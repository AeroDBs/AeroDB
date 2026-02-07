//! Durable Authority Marker
//!
//! Per PHASE6_ARCHITECTURE.md §4.2 (amended):
//! Phase 6 MAY use non-WAL durability for authority transition:
//! - A single fsynced marker file (`metadata/authority_transition.marker`)
//! - This marker is atomic: present = new authority, absent = old authority
//!
//! Per PHASE6_INVARIANTS.md §P6-F2:
//! There MUST be no ambiguous authority state after recovery.
//!
//! Per PHASE6_INVARIANTS.md §P6-D2:
//! After crash and recovery, authority state MUST be unambiguous.

use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::errors::{PromotionError, PromotionErrorKind, PromotionResult};

/// Marker file name per PHASE6_ARCHITECTURE.md §4.2
const MARKER_FILE_NAME: &str = "authority_transition.marker";

/// Authority transition marker.
///
/// Per PHASE6_ARCHITECTURE.md §4.2:
/// - Present = new authority (Primary)
/// - Absent = old authority (Replica)
///
/// This is the SOLE authority state durability mechanism.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthorityMarker {
    /// ID of the new primary (the promoted replica) as string
    pub new_primary_id: String,

    /// Unix timestamp when marker was written
    pub timestamp_secs: i64,

    /// Previous authority (for audit trail)
    pub previous_state: String,
}

impl AuthorityMarker {
    /// Create a new authority marker.
    pub fn new(new_primary_id: Uuid, previous_state: &str) -> Self {
        Self {
            new_primary_id: new_primary_id.to_string(),
            timestamp_secs: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
            previous_state: previous_state.to_string(),
        }
    }

    /// Get the primary ID as UUID.
    pub fn get_primary_id(&self) -> Option<Uuid> {
        Uuid::parse_str(&self.new_primary_id).ok()
    }
}

/// Durable marker file operations.
///
/// Per PHASE6_INVARIANTS.md §P6-A2:
/// Authority transfer is atomic. All-or-nothing.
///
/// Atomicity is achieved via:
/// 1. Write to temp file
/// 2. fsync temp file
/// 3. Rename temp to final (atomic on POSIX)
pub struct DurableMarker {
    /// Path to marker file
    marker_path: PathBuf,

    /// Path to temp file during atomic write
    temp_path: PathBuf,
}

impl DurableMarker {
    /// Create a new durable marker manager.
    ///
    /// # Arguments
    /// * `data_dir` - Base data directory
    pub fn new(data_dir: &Path) -> Self {
        let metadata_dir = data_dir.join("metadata");
        Self {
            marker_path: metadata_dir.join(MARKER_FILE_NAME),
            temp_path: metadata_dir.join(format!("{}.tmp", MARKER_FILE_NAME)),
        }
    }

    /// Write authority marker atomically.
    ///
    /// Per PHASE6_INVARIANTS.md §P6-F2:
    /// After this completes, new authority is authoritative.
    ///
    /// Uses atomic write pattern:
    /// 1. Write to temp file
    /// 2. fsync temp file (durability)
    /// 3. Rename temp to final (atomicity)
    pub fn write_atomic(&self, marker: &AuthorityMarker) -> PromotionResult<()> {
        // Ensure metadata directory exists
        if let Some(parent) = self.marker_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                PromotionError::new(
                    PromotionErrorKind::AuthorityTransitionFailed,
                    format!("failed to create metadata directory: {}", e),
                )
            })?;
        }

        // Serialize marker
        let content = serde_json::to_string_pretty(marker).map_err(|e| {
            PromotionError::new(
                PromotionErrorKind::AuthorityTransitionFailed,
                format!("failed to serialize marker: {}", e),
            )
        })?;

        // Write to temp file
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.temp_path)
            .map_err(|e| {
                PromotionError::new(
                    PromotionErrorKind::AuthorityTransitionFailed,
                    format!("failed to create temp marker file: {}", e),
                )
            })?;

        file.write_all(content.as_bytes()).map_err(|e| {
            PromotionError::new(
                PromotionErrorKind::AuthorityTransitionFailed,
                format!("failed to write marker content: {}", e),
            )
        })?;

        // fsync for durability - CRITICAL per P6-F2
        file.sync_all().map_err(|e| {
            PromotionError::new(
                PromotionErrorKind::AuthorityTransitionFailed,
                format!("failed to fsync marker file: {}", e),
            )
        })?;

        // Atomic rename - CRITICAL per P6-A2
        fs::rename(&self.temp_path, &self.marker_path).map_err(|e| {
            PromotionError::new(
                PromotionErrorKind::AuthorityTransitionFailed,
                format!("failed to atomically commit marker: {}", e),
            )
        })?;

        // fsync the directory to ensure rename is durable
        if let Some(parent) = self.marker_path.parent() {
            if let Ok(dir) = File::open(parent) {
                let _ = dir.sync_all();
            }
        }

        Ok(())
    }

    /// Read authority marker if present.
    ///
    /// Per PHASE6_INVARIANTS.md §P6-D2:
    /// Recovery reads only durable state.
    ///
    /// Returns:
    /// - Some(marker) if marker exists (new authority)
    /// - None if marker absent (old authority)
    pub fn read(&self) -> PromotionResult<Option<AuthorityMarker>> {
        if !self.marker_path.exists() {
            return Ok(None);
        }

        let mut file = File::open(&self.marker_path).map_err(|e| {
            PromotionError::new(
                PromotionErrorKind::AuthorityTransitionFailed,
                format!("failed to open marker file: {}", e),
            )
        })?;

        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|e| {
            PromotionError::new(
                PromotionErrorKind::AuthorityTransitionFailed,
                format!("failed to read marker file: {}", e),
            )
        })?;

        let marker: AuthorityMarker = serde_json::from_str(&content).map_err(|e| {
            PromotionError::new(
                PromotionErrorKind::AuthorityTransitionFailed,
                format!("failed to parse marker file: {}", e),
            )
        })?;

        Ok(Some(marker))
    }

    /// Check if marker exists.
    ///
    /// Simple existence check for recovery logic.
    pub fn exists(&self) -> bool {
        self.marker_path.exists()
    }

    /// Remove marker after transition is complete.
    ///
    /// Called after the promoted node has successfully started as Primary.
    pub fn remove(&self) -> PromotionResult<()> {
        if self.marker_path.exists() {
            fs::remove_file(&self.marker_path).map_err(|e| {
                PromotionError::new(
                    PromotionErrorKind::AuthorityTransitionFailed,
                    format!("failed to remove marker file: {}", e),
                )
            })?;
        }

        // Clean up temp file if it exists
        if self.temp_path.exists() {
            let _ = fs::remove_file(&self.temp_path);
        }

        Ok(())
    }

    /// Get marker file path (for testing/diagnostics).
    pub fn marker_path(&self) -> &Path {
        &self.marker_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_uuid() -> Uuid {
        Uuid::new_v4()
    }

    #[test]
    fn test_marker_write_and_read() {
        let tmp = TempDir::new().unwrap();
        let dm = DurableMarker::new(tmp.path());

        let id = test_uuid();
        let marker = AuthorityMarker::new(id, "ReplicaActive");

        dm.write_atomic(&marker).unwrap();

        let read_marker = dm.read().unwrap();
        assert!(read_marker.is_some());
        assert_eq!(read_marker.unwrap().get_primary_id(), Some(id));
    }

    #[test]
    fn test_marker_absent_returns_none() {
        let tmp = TempDir::new().unwrap();
        let dm = DurableMarker::new(tmp.path());

        let result = dm.read().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_marker_exists_check() {
        let tmp = TempDir::new().unwrap();
        let dm = DurableMarker::new(tmp.path());

        assert!(!dm.exists());

        let marker = AuthorityMarker::new(test_uuid(), "ReplicaActive");
        dm.write_atomic(&marker).unwrap();

        assert!(dm.exists());
    }

    #[test]
    fn test_marker_remove() {
        let tmp = TempDir::new().unwrap();
        let dm = DurableMarker::new(tmp.path());

        let marker = AuthorityMarker::new(test_uuid(), "ReplicaActive");
        dm.write_atomic(&marker).unwrap();
        assert!(dm.exists());

        dm.remove().unwrap();
        assert!(!dm.exists());
    }

    #[test]
    fn test_marker_atomicity_simulated() {
        // Verify that marker is only visible after full write
        let tmp = TempDir::new().unwrap();
        let dm = DurableMarker::new(tmp.path());

        // Before write
        assert!(!dm.exists());

        // After write
        let marker = AuthorityMarker::new(test_uuid(), "ReplicaActive");
        dm.write_atomic(&marker).unwrap();
        assert!(dm.exists());

        // Read should succeed
        let read = dm.read().unwrap();
        assert!(read.is_some());
    }

    #[test]
    fn test_marker_recovery_determinism() {
        // Per P6-D2: Same disk state → same recovery outcome
        let tmp = TempDir::new().unwrap();
        let dm = DurableMarker::new(tmp.path());

        let marker = AuthorityMarker::new(test_uuid(), "ReplicaActive");
        dm.write_atomic(&marker).unwrap();

        // Multiple reads should return identical results
        let read1 = dm.read().unwrap();
        let read2 = dm.read().unwrap();

        assert_eq!(read1, read2);
    }
}
