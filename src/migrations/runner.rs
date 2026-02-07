//! # Migration Runner
//!
//! MANIFESTO ALIGNMENT: Deterministic, transactional migration execution.
//!
//! Per Design Manifesto: "fail loudly, execute predictably, leave no surprises."
//!
//! This module orchestrates migration execution with:
//! - Sequential version ordering
//! - Checksum verification
//! - Transactional semantics
//! - State tracking

use super::checksum::{generate_checksum_for_file, verify_checksum};
use super::errors::{MigrationError, MigrationResult};
use super::operations::OperationExecutor;
use super::state::MigrationState;
use super::{Migration, MigrationOperation, MigrationVersion};
use chrono::Utc;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

/// Migration runner
///
/// MANIFESTO ALIGNMENT: Single entry point for migration execution.
/// Ensures deterministic ordering and transactional semantics.
pub struct MigrationRunner {
    /// Directory containing migration files
    migrations_dir: PathBuf,

    /// Migration state tracker
    state: Arc<MigrationState>,

    /// Operation executor
    executor: Arc<dyn OperationExecutor>,
}

impl MigrationRunner {
    /// Create a new migration runner
    pub fn new(
        migrations_dir: PathBuf,
        data_dir: PathBuf,
        executor: Arc<dyn OperationExecutor>,
    ) -> MigrationResult<Self> {
        let state = Arc::new(MigrationState::new(data_dir));
        state.load()?;

        Ok(Self {
            migrations_dir,
            state,
            executor,
        })
    }

    /// Load all migrations from disk
    pub fn load_migrations(&self) -> MigrationResult<BTreeMap<MigrationVersion, Migration>> {
        if !self.migrations_dir.exists() {
            return Err(MigrationError::DirectoryNotFound {
                path: self.migrations_dir.clone(),
            });
        }

        let mut migrations = BTreeMap::new();

        for entry in fs::read_dir(&self.migrations_dir).map_err(|e| MigrationError::FileRead {
            path: self.migrations_dir.clone(),
            source: e,
        })? {
            let entry = entry.map_err(|e| MigrationError::FileRead {
                path: self.migrations_dir.clone(),
                source: e,
            })?;

            let path = entry.path();
            if path.extension().map(|e| e == "yaml" || e == "yml").unwrap_or(false) {
                let migration = self.load_migration(&path)?;
                if migrations.contains_key(&migration.version) {
                    return Err(MigrationError::DuplicateVersion {
                        version: migration.version,
                    });
                }
                migrations.insert(migration.version, migration);
            }
        }

        Ok(migrations)
    }

    /// Load a single migration file
    fn load_migration(&self, path: &Path) -> MigrationResult<Migration> {
        let content = fs::read_to_string(path).map_err(|e| MigrationError::FileRead {
            path: path.to_path_buf(),
            source: e,
        })?;

        let mut migration: Migration = serde_yaml::from_str(&content).map_err(|e| {
            MigrationError::ParseError {
                path: path.to_path_buf(),
                message: e.to_string(),
            }
        })?;

        migration.file_path = Some(path.to_path_buf());

        // Extract name from filename: 001_create_users.yaml -> create_users
        if migration.name.is_empty() {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if let Some(idx) = stem.find('_') {
                migration.name = stem[idx + 1..].to_string();
            } else {
                migration.name = stem.to_string();
            }
        }

        // Verify checksum
        let computed = generate_checksum_for_file(&content);
        if !migration.checksum.is_empty() && migration.checksum != computed {
            return Err(MigrationError::ChecksumMismatch {
                migration: migration.name.clone(),
                expected: migration.checksum.clone(),
                actual: computed,
            });
        }

        migration.validate()?;
        Ok(migration)
    }

    /// Get pending migrations
    pub fn get_pending(&self) -> MigrationResult<Vec<Migration>> {
        let all_migrations = self.load_migrations()?;
        let current = self.state.current_version();

        Ok(all_migrations
            .into_values()
            .filter(|m| m.version > current && !self.state.is_applied(m.version))
            .collect())
    }

    /// Get migration status
    pub fn status(&self) -> MigrationResult<MigrationStatusReport> {
        let all_migrations = self.load_migrations()?;
        let applied = self.state.get_applied();
        let current = self.state.current_version();

        let pending: Vec<_> = all_migrations
            .values()
            .filter(|m| !self.state.is_applied(m.version))
            .cloned()
            .collect();

        Ok(MigrationStatusReport {
            current_version: current,
            total_migrations: all_migrations.len(),
            applied_count: applied.len(),
            pending_count: pending.len(),
            pending,
        })
    }

    /// Apply all pending migrations
    ///
    /// MANIFESTO ALIGNMENT: Sequential, deterministic application.
    pub fn migrate_up(&self) -> MigrationResult<MigrationRunReport> {
        self.state.acquire_lock(format!("runner-{}", std::process::id()))?;

        let result = self.migrate_up_internal();

        self.state.release_lock();
        result
    }

    fn migrate_up_internal(&self) -> MigrationResult<MigrationRunReport> {
        let pending = self.get_pending()?;

        if pending.is_empty() {
            return Ok(MigrationRunReport {
                applied: vec![],
                failed: None,
            });
        }

        let mut applied = Vec::new();

        for migration in pending {
            match self.apply_migration(&migration) {
                Ok(duration_ms) => {
                    applied.push(AppliedMigration {
                        version: migration.version,
                        name: migration.name.clone(),
                        duration_ms,
                    });
                }
                Err(e) => {
                    return Ok(MigrationRunReport {
                        applied,
                        failed: Some(FailedMigration {
                            version: migration.version,
                            name: migration.name,
                            error: e.to_string(),
                        }),
                    });
                }
            }
        }

        Ok(MigrationRunReport {
            applied,
            failed: None,
        })
    }

    /// Apply a single migration
    fn apply_migration(&self, migration: &Migration) -> MigrationResult<u64> {
        let start = Instant::now();

        // Record start
        self.state.record_start(
            migration.version,
            migration.name.clone(),
            migration.checksum.clone(),
        )?;

        // Execute operations
        for (i, op) in migration.up.iter().enumerate() {
            if let Err(e) = self.executor.execute(op) {
                let duration_ms = start.elapsed().as_millis() as u64;
                self.state.record_failure(
                    migration.version,
                    format!("Operation {} failed: {}", i, e),
                    duration_ms,
                )?;
                return Err(MigrationError::ExecutionFailed {
                    version: migration.version,
                    operation: format!("operation[{}]", i),
                    reason: e.to_string(),
                });
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        self.state.record_success(migration.version, duration_ms)?;

        Ok(duration_ms)
    }

    /// Rollback the last applied migration
    ///
    /// MANIFESTO ALIGNMENT: Explicit, reversible rollback.
    pub fn migrate_down(&self) -> MigrationResult<Option<AppliedMigration>> {
        self.state.acquire_lock(format!("runner-{}", std::process::id()))?;

        let result = self.migrate_down_internal();

        self.state.release_lock();
        result
    }

    fn migrate_down_internal(&self) -> MigrationResult<Option<AppliedMigration>> {
        let current = self.state.current_version();
        if current == 0 {
            return Ok(None);
        }

        let migrations = self.load_migrations()?;
        let migration = migrations.get(&current).ok_or(MigrationError::MigrationNotFound {
            version: current,
        })?;

        let start = Instant::now();

        // Execute down operations in reverse order
        for (i, op) in migration.down.iter().enumerate() {
            if let Err(e) = self.executor.execute(op) {
                return Err(MigrationError::CannotRollback {
                    version: migration.version,
                    reason: format!("Down operation {} failed: {}", i, e),
                });
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        self.state.record_rollback(current)?;

        Ok(Some(AppliedMigration {
            version: migration.version,
            name: migration.name.clone(),
            duration_ms,
        }))
    }
}

/// Status report for migrations
#[derive(Debug)]
pub struct MigrationStatusReport {
    pub current_version: MigrationVersion,
    pub total_migrations: usize,
    pub applied_count: usize,
    pub pending_count: usize,
    pub pending: Vec<Migration>,
}

/// Report from a migration run
#[derive(Debug)]
pub struct MigrationRunReport {
    pub applied: Vec<AppliedMigration>,
    pub failed: Option<FailedMigration>,
}

/// Successfully applied migration
#[derive(Debug)]
pub struct AppliedMigration {
    pub version: MigrationVersion,
    pub name: String,
    pub duration_ms: u64,
}

/// Failed migration
#[derive(Debug)]
pub struct FailedMigration {
    pub version: MigrationVersion,
    pub name: String,
    pub error: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::operations::InMemoryExecutor;
    use tempfile::TempDir;
    use std::fs;
    use std::path::Path; // Added for Path type
    use chrono; // Added for chrono::Utc
    use serde_json; // Added for serde_json::json!
    use serde_yaml; // Added for serde_yaml::to_string

    fn create_test_migration(dir: &Path, version: u64, name: &str) {
        // Use serde to generate properly formatted YAML
        let migration = Migration {
            version,
            name: name.to_string(),
            checksum: "".to_string(),
            timestamp: chrono::Utc::now(),
            file_path: None,
            up: vec![MigrationOperation::CreateCollection {
                name: name.to_string(),
                schema: serde_json::json!({}),
            }],
            down: vec![MigrationOperation::DropCollection {
                name: name.to_string(),
            }],
        };
        
        let content = serde_yaml::to_string(&migration).unwrap();
        let filename = format!("{:03}_{}.yaml", version, name);
        fs::write(dir.join(&filename), &content).unwrap();
    }

    #[test]
    fn test_load_migrations() {
        let temp_dir = TempDir::new().unwrap();
        let migrations_dir = temp_dir.path().join("migrations");
        fs::create_dir_all(&migrations_dir).unwrap();

        create_test_migration(&migrations_dir, 1, "create_users");
        create_test_migration(&migrations_dir, 2, "create_posts");

        let data_dir = temp_dir.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();

        let executor = Arc::new(InMemoryExecutor::new());
        let runner = MigrationRunner::new(migrations_dir, data_dir, executor).unwrap();

        let migrations = runner.load_migrations().unwrap();
        assert_eq!(migrations.len(), 2);
        assert!(migrations.contains_key(&1));
        assert!(migrations.contains_key(&2));
    }

    #[test]
    fn test_migrate_up() {
        let temp_dir = TempDir::new().unwrap();
        let migrations_dir = temp_dir.path().join("migrations");
        fs::create_dir_all(&migrations_dir).unwrap();

        create_test_migration(&migrations_dir, 1, "users");
        create_test_migration(&migrations_dir, 2, "posts");

        let data_dir = temp_dir.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();

        let executor = Arc::new(InMemoryExecutor::new());
        let runner = MigrationRunner::new(migrations_dir, data_dir, executor.clone()).unwrap();

        let report = runner.migrate_up().unwrap();
        assert_eq!(report.applied.len(), 2);
        assert!(report.failed.is_none());

        // Verify collections were created
        assert!(executor.collection_exists("users").unwrap());
        assert!(executor.collection_exists("posts").unwrap());
    }

    #[test]
    fn test_migrate_down() {
        let temp_dir = TempDir::new().unwrap();
        let migrations_dir = temp_dir.path().join("migrations");
        fs::create_dir_all(&migrations_dir).unwrap();

        create_test_migration(&migrations_dir, 1, "users");

        let data_dir = temp_dir.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();

        let executor = Arc::new(InMemoryExecutor::new());
        let runner = MigrationRunner::new(migrations_dir, data_dir, executor.clone()).unwrap();

        // Apply
        runner.migrate_up().unwrap();
        assert!(executor.collection_exists("users").unwrap());

        // Rollback
        let result = runner.migrate_down().unwrap();
        assert!(result.is_some());
        assert!(!executor.collection_exists("users").unwrap());
    }
}
