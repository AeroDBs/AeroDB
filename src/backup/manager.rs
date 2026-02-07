//! Backup Manager for AeroDB.
//!
//! Provides backup creation, listing, deletion, and retention enforcement.
//!
//! Per BACKUP.md:
//! - Backups are tar archives containing snapshot + WAL
//! - Backups are atomic and crash-safe
//! - Backups are compatible with RestoreManager

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use tar::Builder;

use crate::backup::errors::{BackupError, BackupResult};
use crate::backup::{BackupConfig, BackupManifest, BackupMetadata, BackupStatus};
use crate::snapshot::{GlobalExecutionLock, SnapshotManager};
use crate::wal::WalWriter;

/// Backup format version
const BACKUP_FORMAT_VERSION: u32 = 1;

/// Backup manager for creating and managing database backups.
///
/// The BackupManager creates tar archives containing:
/// - `backup_manifest.json` - Backup metadata
/// - `snapshot/` - Database snapshot files
/// - `wal/` - Write-ahead log files
///
/// This format is compatible with RestoreManager.
pub struct BackupManager {
    config: BackupConfig,
    backup_dir: PathBuf,
}

impl BackupManager {
    /// Create a new BackupManager with the given configuration.
    ///
    /// # Arguments
    /// * `config` - Backup configuration
    ///
    /// # Returns
    /// BackupManager instance or error if backup directory is not accessible
    pub fn new(config: BackupConfig) -> BackupResult<Self> {
        let backup_dir = PathBuf::from(&config.backup_dir);

        // Ensure backup directory exists
        if !backup_dir.exists() {
            fs::create_dir_all(&backup_dir).map_err(|e| {
                BackupError::io_error(e, format!("Failed to create backup directory: {}", backup_dir.display()))
            })?;
        }

        // Verify directory is accessible
        if !backup_dir.is_dir() {
            return Err(BackupError::dir_not_accessible(&backup_dir));
        }

        Ok(Self { config, backup_dir })
    }

    /// Create a new backup from the current database state.
    ///
    /// # Algorithm
    /// 1. Create snapshot using SnapshotManager
    /// 2. Create temp directory for backup assembly
    /// 3. Copy snapshot files to temp/snapshot/
    /// 4. Copy WAL files to temp/wal/
    /// 5. Generate backup_manifest.json
    /// 6. Create tar archive
    /// 7. fsync archive file
    /// 8. Clean up temp directory
    /// 9. Enforce retention policy
    ///
    /// # Arguments
    /// * `data_dir` - Root data directory
    /// * `storage_path` - Path to storage file
    /// * `schema_dir` - Path to schema directory
    /// * `wal` - WAL writer reference
    /// * `description` - Optional backup description
    /// * `lock` - Global execution lock (held by caller)
    ///
    /// # Returns
    /// BackupMetadata on success
    #[allow(clippy::too_many_arguments)]
    pub fn create_backup(
        &self,
        data_dir: &Path,
        storage_path: &Path,
        schema_dir: &Path,
        wal: &WalWriter,
        description: Option<String>,
        lock: &GlobalExecutionLock,
    ) -> BackupResult<BackupMetadata> {
        // Step 1: Create snapshot
        let snapshot_id = SnapshotManager::create_snapshot(
            data_dir,
            storage_path,
            schema_dir,
            wal,
            lock,
        )
        .map_err(|e| BackupError::snapshot_failed(format!("Snapshot creation failed: {}", e)))?;

        let snapshot_id_str = snapshot_id.to_string();
        let backup_id = format!("backup_{}", snapshot_id_str);
        let created_at = Utc::now();
        let created_at_str = created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();

        // Step 2: Create temp directory
        let temp_dir = self.backup_dir.join(format!("{}.tmp", backup_id));
        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir).map_err(|e| {
                BackupError::io_error(e, "Failed to clean existing temp directory")
            })?;
        }
        fs::create_dir_all(&temp_dir).map_err(|e| {
            BackupError::io_error(e, "Failed to create temp directory")
        })?;

        // Ensure cleanup on error
        let _cleanup_guard = CleanupGuard::new(&temp_dir);

        // Step 3: Copy snapshot files to temp/snapshot/
        let snapshot_src = data_dir.join("snapshots").join(&snapshot_id_str);
        let snapshot_dest = temp_dir.join("snapshot");
        self.copy_dir_recursive(&snapshot_src, &snapshot_dest)?;

        // Step 4: Copy WAL files to temp/wal/
        let wal_src = data_dir.join("wal");
        let wal_dest = temp_dir.join("wal");
        let wal_present = if wal_src.exists() {
            self.copy_dir_recursive(&wal_src, &wal_dest)?;
            true
        } else {
            fs::create_dir_all(&wal_dest).map_err(|e| {
                BackupError::io_error(e, "Failed to create WAL directory")
            })?;
            false
        };

        // Step 5: Generate backup_manifest.json
        let manifest = BackupManifest {
            backup_id: backup_id.clone(),
            snapshot_id: snapshot_id_str.clone(),
            created_at: created_at_str.clone(),
            wal_present,
            format_version: BACKUP_FORMAT_VERSION,
        };

        let manifest_path = temp_dir.join("backup_manifest.json");
        manifest.write_to_file(&manifest_path).map_err(|e| {
            BackupError::io_error(e, "Failed to write backup manifest")
        })?;

        // Step 6: Create tar archive
        let archive_path = self.backup_dir.join(format!("{}.tar", backup_id));
        self.create_tar_archive(&temp_dir, &archive_path)?;

        // Step 7: fsync archive file
        self.fsync_file(&archive_path)?;

        // Step 8: Clean up temp directory (handled by CleanupGuard drop)

        // Calculate backup size
        let size_bytes = fs::metadata(&archive_path)
            .map(|m| m.len())
            .unwrap_or(0);

        // Step 9: Enforce retention policy
        let _ = self.enforce_retention();

        let metadata = BackupMetadata {
            id: backup_id,
            created_at: created_at_str,
            size_bytes,
            description,
        };

        Ok(metadata)
    }

    /// List all available backups.
    ///
    /// Returns backups sorted by creation time (newest first).
    pub fn list_backups(&self) -> BackupResult<Vec<BackupMetadata>> {
        let mut backups = Vec::new();

        if !self.backup_dir.exists() {
            return Ok(backups);
        }

        for entry in fs::read_dir(&self.backup_dir).map_err(|e| {
            BackupError::io_error(e, "Failed to read backup directory")
        })? {
            let entry = entry.map_err(|e| {
                BackupError::io_error(e, "Failed to read directory entry")
            })?;

            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "tar") {
                if let Some(metadata) = self.read_backup_metadata(&path)? {
                    backups.push(metadata);
                }
            }
        }

        // Sort by creation time (newest first)
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(backups)
    }

    /// Get a specific backup by ID.
    pub fn get_backup(&self, backup_id: &str) -> BackupResult<BackupMetadata> {
        let archive_path = self.backup_dir.join(format!("{}.tar", backup_id));
        
        if !archive_path.exists() {
            return Err(BackupError::not_found(backup_id));
        }

        self.read_backup_metadata(&archive_path)?
            .ok_or_else(|| BackupError::not_found(backup_id))
    }

    /// Delete a specific backup.
    pub fn delete_backup(&self, backup_id: &str) -> BackupResult<()> {
        let archive_path = self.backup_dir.join(format!("{}.tar", backup_id));

        if !archive_path.exists() {
            return Err(BackupError::not_found(backup_id));
        }

        fs::remove_file(&archive_path).map_err(|e| {
            BackupError::io_error(e, format!("Failed to delete backup: {}", backup_id))
        })?;

        Ok(())
    }

    /// Enforce retention policy by deleting old backups.
    ///
    /// Keeps only the `max_backups` most recent backups.
    ///
    /// # Returns
    /// Number of backups deleted
    pub fn enforce_retention(&self) -> BackupResult<u32> {
        let mut backups = self.list_backups()?;
        let max_backups = self.config.max_backups as usize;

        if backups.len() <= max_backups {
            return Ok(0);
        }

        // Backups are sorted newest first, so delete from the end
        let mut deleted = 0u32;
        while backups.len() > max_backups {
            if let Some(oldest) = backups.pop() {
                if let Err(e) = self.delete_backup(&oldest.id) {
                    // Log but don't fail
                    eprintln!("Warning: Failed to delete old backup {}: {}", oldest.id, e);
                } else {
                    deleted += 1;
                }
            }
        }

        Ok(deleted)
    }

    /// Get current backup status.
    pub fn status(&self) -> BackupResult<BackupStatus> {
        let backups = self.list_backups()?;
        
        let last_backup = backups.first().map(|b| b.created_at.clone());
        let backup_count = backups.len() as u32;
        let total_size_bytes: u64 = backups.iter().map(|b| b.size_bytes).sum();

        // Calculate next backup time based on last backup and interval
        let next_backup = if self.config.enabled {
            last_backup.as_ref().and_then(|last| {
                DateTime::parse_from_rfc3339(last).ok().map(|dt| {
                    let next = dt + chrono::Duration::hours(self.config.interval_hours as i64);
                    next.format("%Y-%m-%dT%H:%M:%SZ").to_string()
                })
            })
        } else {
            None
        };

        Ok(BackupStatus {
            last_backup,
            next_backup,
            backup_count,
            total_size_bytes,
        })
    }

    /// Read backup metadata from a tar archive.
    fn read_backup_metadata(&self, archive_path: &Path) -> BackupResult<Option<BackupMetadata>> {
        let file = File::open(archive_path).map_err(|e| {
            BackupError::io_error(e, format!("Failed to open backup: {}", archive_path.display()))
        })?;

        let mut archive = tar::Archive::new(file);

        for entry in archive.entries().map_err(|e| {
            BackupError::io_error(e, "Failed to read archive entries")
        })? {
            let mut entry = entry.map_err(|e| {
                BackupError::io_error(e, "Failed to read archive entry")
            })?;

            let path = entry.path().map_err(|e| {
                BackupError::io_error(e, "Failed to get entry path")
            })?;

            if path.to_string_lossy() == "backup_manifest.json" {
                let mut contents = String::new();
                entry.read_to_string(&mut contents).map_err(|e| {
                    BackupError::io_error(e, "Failed to read manifest")
                })?;

                let manifest: BackupManifest = serde_json::from_str(&contents).map_err(|e| {
                    BackupError::archive_failed(format!("Invalid manifest: {}", e))
                })?;

                let size_bytes = fs::metadata(archive_path)
                    .map(|m| m.len())
                    .unwrap_or(0);

                return Ok(Some(BackupMetadata {
                    id: manifest.backup_id,
                    created_at: manifest.created_at,
                    size_bytes,
                    description: None,
                }));
            }
        }

        Ok(None)
    }

    /// Create a tar archive from a directory.
    fn create_tar_archive(&self, source_dir: &Path, archive_path: &Path) -> BackupResult<()> {
        let file = File::create(archive_path).map_err(|e| {
            BackupError::io_error(e, format!("Failed to create archive: {}", archive_path.display()))
        })?;

        let mut builder = Builder::new(file);

        // Add all files from source directory recursively
        self.add_dir_to_archive(&mut builder, source_dir, source_dir)?;

        builder.finish().map_err(|e| {
            BackupError::io_error(e, "Failed to finish archive")
        })?;

        Ok(())
    }

    /// Recursively add directory contents to tar archive.
    fn add_dir_to_archive(
        &self,
        builder: &mut Builder<File>,
        base_dir: &Path,
        current_dir: &Path,
    ) -> BackupResult<()> {
        for entry in fs::read_dir(current_dir).map_err(|e| {
            BackupError::io_error(e, format!("Failed to read directory: {}", current_dir.display()))
        })? {
            let entry = entry.map_err(|e| {
                BackupError::io_error(e, "Failed to read directory entry")
            })?;

            let path = entry.path();
            let relative_path = path.strip_prefix(base_dir).unwrap_or(&path);

            if path.is_file() {
                let mut file = File::open(&path).map_err(|e| {
                    BackupError::io_error(e, format!("Failed to open file: {}", path.display()))
                })?;
                builder.append_file(relative_path, &mut file).map_err(|e| {
                    BackupError::io_error(e, format!("Failed to add file to archive: {}", path.display()))
                })?;
            } else if path.is_dir() {
                builder.append_dir(relative_path, &path).map_err(|e| {
                    BackupError::io_error(e, format!("Failed to add directory to archive: {}", path.display()))
                })?;
                // Recursively add directory contents
                self.add_dir_to_archive(builder, base_dir, &path)?;
            }
        }

        Ok(())
    }

    /// Copy directory recursively.
    fn copy_dir_recursive(&self, src: &Path, dst: &Path) -> BackupResult<()> {
        fs::create_dir_all(dst).map_err(|e| {
            BackupError::io_error(e, format!("Failed to create directory: {}", dst.display()))
        })?;

        for entry in fs::read_dir(src).map_err(|e| {
            BackupError::io_error(e, format!("Failed to read directory: {}", src.display()))
        })? {
            let entry = entry.map_err(|e| {
                BackupError::io_error(e, "Failed to read directory entry")
            })?;

            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                self.copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path).map_err(|e| {
                    BackupError::io_error(e, format!("Failed to copy file: {}", src_path.display()))
                })?;
            }
        }

        Ok(())
    }

    /// Fsync a file to disk.
    fn fsync_file(&self, path: &Path) -> BackupResult<()> {
        let file = File::open(path).map_err(|e| {
            BackupError::io_error(e, format!("Failed to open file for fsync: {}", path.display()))
        })?;

        file.sync_all().map_err(|e| {
            BackupError::io_error(e, format!("Failed to fsync file: {}", path.display()))
        })?;

        Ok(())
    }
}

/// RAII guard for cleaning up temp directories.
struct CleanupGuard<'a> {
    path: &'a Path,
}

impl<'a> CleanupGuard<'a> {
    fn new(path: &'a Path) -> Self {
        Self { path }
    }
}

impl<'a> Drop for CleanupGuard<'a> {
    fn drop(&mut self) {
        if self.path.exists() {
            let _ = fs::remove_dir_all(self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config(backup_dir: &Path) -> BackupConfig {
        BackupConfig {
            enabled: true,
            interval_hours: 24,
            max_backups: 3,
            backup_dir: backup_dir.to_string_lossy().to_string(),
        }
    }

    #[test]
    fn test_backup_manager_new() {
        let temp = TempDir::new().unwrap();
        let config = create_test_config(temp.path());
        
        let manager = BackupManager::new(config);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_backup_manager_creates_dir() {
        let temp = TempDir::new().unwrap();
        let backup_dir = temp.path().join("backups");
        
        assert!(!backup_dir.exists());
        
        let config = BackupConfig {
            enabled: true,
            interval_hours: 24,
            max_backups: 7,
            backup_dir: backup_dir.to_string_lossy().to_string(),
        };
        
        let manager = BackupManager::new(config);
        assert!(manager.is_ok());
        assert!(backup_dir.exists());
    }

    #[test]
    fn test_list_backups_empty() {
        let temp = TempDir::new().unwrap();
        let config = create_test_config(temp.path());
        let manager = BackupManager::new(config).unwrap();
        
        let backups = manager.list_backups().unwrap();
        assert!(backups.is_empty());
    }

    #[test]
    fn test_backup_status_empty() {
        let temp = TempDir::new().unwrap();
        let config = create_test_config(temp.path());
        let manager = BackupManager::new(config).unwrap();
        
        let status = manager.status().unwrap();
        assert!(status.last_backup.is_none());
        assert_eq!(status.backup_count, 0);
        assert_eq!(status.total_size_bytes, 0);
    }

    #[test]
    fn test_delete_nonexistent_backup() {
        let temp = TempDir::new().unwrap();
        let config = create_test_config(temp.path());
        let manager = BackupManager::new(config).unwrap();
        
        let result = manager.delete_backup("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_nonexistent_backup() {
        let temp = TempDir::new().unwrap();
        let config = create_test_config(temp.path());
        let manager = BackupManager::new(config).unwrap();
        
        let result = manager.get_backup("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_retention_no_backups() {
        let temp = TempDir::new().unwrap();
        let config = create_test_config(temp.path());
        let manager = BackupManager::new(config).unwrap();
        
        let deleted = manager.enforce_retention().unwrap();
        assert_eq!(deleted, 0);
    }
}
