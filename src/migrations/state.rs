//! # Migration State Tracking
//!
//! MANIFESTO ALIGNMENT: Persistent, auditable migration state.
//!
//! Per Design Manifesto: "Every query is logged with its execution details."
//!
//! This module tracks which migrations have been applied to the database,
//! stored in the `_system.migrations` collection.

use super::errors::{MigrationError, MigrationResult};
use super::MigrationVersion;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Status of a migration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MigrationStatus {
    /// Migration is pending (not yet applied)
    Pending,
    /// Migration is currently being applied
    Running,
    /// Migration was successfully applied
    Applied,
    /// Migration failed during application
    Failed,
    /// Migration was rolled back
    RolledBack,
}

/// Record of an applied migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRecord {
    /// Migration version number
    pub version: MigrationVersion,

    /// Migration name (from filename)
    pub name: String,

    /// Checksum at time of application
    pub checksum: String,

    /// Current status
    pub status: MigrationStatus,

    /// When migration was applied
    pub applied_at: Option<DateTime<Utc>>,

    /// Duration of migration execution (ms)
    pub duration_ms: Option<u64>,

    /// Error message if failed
    pub error: Option<String>,

    /// User/process that ran the migration
    pub applied_by: Option<String>,
}

/// Migration state manager
///
/// MANIFESTO ALIGNMENT: Single source of truth for migration state.
/// All state changes are tracked and auditable.
#[derive(Debug)]
pub struct MigrationState {
    /// Path to state file (for file-based storage)
    state_file: PathBuf,

    /// In-memory state cache
    records: RwLock<BTreeMap<MigrationVersion, MigrationRecord>>,

    /// Lock holder (for concurrent access prevention)
    lock_holder: RwLock<Option<(String, DateTime<Utc>)>>,
}

impl MigrationState {
    /// Create new migration state manager
    ///
    /// # Arguments
    /// * `data_dir` - Directory where state file will be stored
    pub fn new(data_dir: PathBuf) -> Self {
        let state_file = data_dir.join("_migrations_state.json");
        Self {
            state_file,
            records: RwLock::new(BTreeMap::new()),
            lock_holder: RwLock::new(None),
        }
    }

    /// Load state from disk
    pub fn load(&self) -> MigrationResult<()> {
        if !self.state_file.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.state_file).map_err(|e| {
            MigrationError::FileRead {
                path: self.state_file.clone(),
                source: e,
            }
        })?;

        let records: BTreeMap<MigrationVersion, MigrationRecord> =
            serde_json::from_str(&content).map_err(|e| MigrationError::StateError {
                message: format!("Failed to parse state file: {}", e),
            })?;

        *self.records.write().unwrap() = records;
        Ok(())
    }

    /// Save state to disk
    ///
    /// MANIFESTO ALIGNMENT: Atomic write to prevent corruption.
    pub fn save(&self) -> MigrationResult<()> {
        let records = self.records.read().unwrap();
        let content = serde_json::to_string_pretty(&*records).map_err(|e| {
            MigrationError::StateError {
                message: format!("Failed to serialize state: {}", e),
            }
        })?;

        // Atomic write: write to temp file, then rename
        let temp_file = self.state_file.with_extension("json.tmp");
        std::fs::write(&temp_file, &content).map_err(|e| MigrationError::FileWrite {
            path: temp_file.clone(),
            source: e,
        })?;

        std::fs::rename(&temp_file, &self.state_file).map_err(|e| MigrationError::FileWrite {
            path: self.state_file.clone(),
            source: e,
        })?;

        Ok(())
    }

    /// Get current version (highest applied migration)
    pub fn current_version(&self) -> MigrationVersion {
        let records = self.records.read().unwrap();
        records
            .values()
            .filter(|r| r.status == MigrationStatus::Applied)
            .map(|r| r.version)
            .max()
            .unwrap_or(0)
    }

    /// Get all applied migrations
    pub fn get_applied(&self) -> Vec<MigrationRecord> {
        let records = self.records.read().unwrap();
        records
            .values()
            .filter(|r| r.status == MigrationStatus::Applied)
            .cloned()
            .collect()
    }

    /// Get all migration records
    pub fn get_all(&self) -> Vec<MigrationRecord> {
        let records = self.records.read().unwrap();
        records.values().cloned().collect()
    }

    /// Check if a specific version is applied
    pub fn is_applied(&self, version: MigrationVersion) -> bool {
        let records = self.records.read().unwrap();
        records
            .get(&version)
            .map(|r| r.status == MigrationStatus::Applied)
            .unwrap_or(false)
    }

    /// Record migration start
    pub fn record_start(
        &self,
        version: MigrationVersion,
        name: String,
        checksum: String,
    ) -> MigrationResult<()> {
        let mut records = self.records.write().unwrap();
        records.insert(
            version,
            MigrationRecord {
                version,
                name,
                checksum,
                status: MigrationStatus::Running,
                applied_at: Some(Utc::now()),
                duration_ms: None,
                error: None,
                applied_by: Some(whoami::username()),
            },
        );
        drop(records);
        self.save()
    }

    /// Record migration success
    pub fn record_success(&self, version: MigrationVersion, duration_ms: u64) -> MigrationResult<()> {
        let mut records = self.records.write().unwrap();
        if let Some(record) = records.get_mut(&version) {
            record.status = MigrationStatus::Applied;
            record.duration_ms = Some(duration_ms);
        }
        drop(records);
        self.save()
    }

    /// Record migration failure
    pub fn record_failure(
        &self,
        version: MigrationVersion,
        error: String,
        duration_ms: u64,
    ) -> MigrationResult<()> {
        let mut records = self.records.write().unwrap();
        if let Some(record) = records.get_mut(&version) {
            record.status = MigrationStatus::Failed;
            record.error = Some(error);
            record.duration_ms = Some(duration_ms);
        }
        drop(records);
        self.save()
    }

    /// Record rollback
    pub fn record_rollback(&self, version: MigrationVersion) -> MigrationResult<()> {
        let mut records = self.records.write().unwrap();
        if let Some(record) = records.get_mut(&version) {
            record.status = MigrationStatus::RolledBack;
        }
        drop(records);
        self.save()
    }

    /// Acquire migration lock
    ///
    /// MANIFESTO ALIGNMENT: Explicit concurrency control.
    pub fn acquire_lock(&self, holder: String) -> MigrationResult<()> {
        let mut lock = self.lock_holder.write().unwrap();
        if let Some((existing_holder, since)) = lock.as_ref() {
            return Err(MigrationError::MigrationLocked {
                holder: existing_holder.clone(),
                since: *since,
            });
        }
        *lock = Some((holder, Utc::now()));
        Ok(())
    }

    /// Release migration lock
    pub fn release_lock(&self) {
        let mut lock = self.lock_holder.write().unwrap();
        *lock = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_migration_state_new() {
        let temp_dir = TempDir::new().unwrap();
        let state = MigrationState::new(temp_dir.path().to_path_buf());
        assert_eq!(state.current_version(), 0);
    }

    #[test]
    fn test_migration_state_record_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let state = MigrationState::new(temp_dir.path().to_path_buf());

        // Start migration
        state
            .record_start(1, "create_users".to_string(), "crc32:ABC".to_string())
            .unwrap();

        assert!(!state.is_applied(1));

        // Complete migration
        state.record_success(1, 100).unwrap();

        assert!(state.is_applied(1));
        assert_eq!(state.current_version(), 1);
    }

    #[test]
    fn test_migration_state_lock() {
        let temp_dir = TempDir::new().unwrap();
        let state = MigrationState::new(temp_dir.path().to_path_buf());

        // Acquire lock
        state.acquire_lock("process-1".to_string()).unwrap();

        // Second acquire should fail
        let result = state.acquire_lock("process-2".to_string());
        assert!(result.is_err());

        // Release and acquire again
        state.release_lock();
        state.acquire_lock("process-2".to_string()).unwrap();
    }
}
