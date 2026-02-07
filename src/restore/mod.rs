//! Restore subsystem for aerodb
//!
//! Restore reconstructs a complete data directory from a backup archive.
//!
//! Per RESTORE.md:
//! - Atomic replacement
//! - Full integrity verification
//! - Deterministic outcome
//! - Zero partial success
//! - Explicit failure
//!
//! # Algorithm (RESTORE.md §5)
//!
//! 1. Verify AeroDB not running
//! 2. Create temp directory
//! 3. Extract backup.tar
//! 4. Validate structure
//! 5. Validate manifest
//! 6. Validate snapshot
//! 7. Validate WAL
//! 8. fsync temp
//! 9. Reorganize files
//! 10. Move data_dir → .old
//! 11. Move temp → data_dir
//! 12. fsync parent
//! 13. Delete .old
//!
//! # Important
//!
//! Restore is offline-only.
//! Restore does NOT start AeroDB.
//! Restore does NOT replay WAL.
//! Restore does NOT rebuild indexes.
//! Restore prepares data for next `aerodb start`.

mod errors;
mod extractor;
mod restorer;
mod validator;

pub use errors::{RestoreError, RestoreErrorCode, RestoreResult, Severity};

use std::path::Path;

use extractor::{
    cleanup_old_dir, cleanup_temp_dir, create_temp_restore_dir, extract_archive,
    get_old_data_dir_path,
};
use restorer::{atomic_replace, fsync_recursive, reorganize_extracted_files};
use validator::{
    validate_backup_manifest, validate_backup_structure, validate_preconditions, validate_snapshot,
    validate_wal,
};

/// Restore manager for restoring from backup archives.
///
/// This struct provides the public API for restore operations.
///
/// # Usage
///
/// ```ignore
/// RestoreManager::restore_from_backup(
///     &data_dir,
///     &backup_path,
/// )?;
/// ```
pub struct RestoreManager;

impl RestoreManager {
    /// Restore from a backup archive.
    ///
    /// Per RESTORE.md §5, this follows the exact sequence:
    /// 1. Verify AeroDB not running
    /// 2. Create temp directory
    /// 3. Extract backup.tar
    /// 4. Validate backup structure
    /// 5. Validate backup manifest
    /// 6. Validate snapshot
    /// 7. Validate WAL
    /// 8. fsync temp directory
    /// 9. Reorganize files to data_dir structure
    /// 10. Atomic directory replacement
    ///
    /// # Arguments
    ///
    /// * `data_dir` - Root data directory to restore to
    /// * `backup_path` - Path to the backup.tar file
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns `RestoreError` on any failure:
    /// - AeroDB is running
    /// - Backup file not found
    /// - Invalid backup format
    /// - Corruption detected
    /// - I/O error
    ///
    /// All errors are FATAL. Original data is preserved on failure.
    ///
    /// # Forbidden Operations
    ///
    /// This function does NOT:
    /// - Acquire global execution lock
    /// - Start API server
    /// - Rebuild indexes
    /// - Replay WAL
    /// - Truncate WAL
    /// - Spawn threads
    /// - Perform async IO
    pub fn restore_from_backup(data_dir: &Path, backup_path: &Path) -> Result<(), RestoreError> {
        // Step 1: Validate preconditions
        validate_preconditions(data_dir, backup_path)?;

        // Step 2: Create temp directory
        let temp_dir = create_temp_restore_dir(data_dir)?;

        // All remaining operations must clean up temp_dir on failure
        let result = Self::restore_inner(data_dir, backup_path, &temp_dir);

        if result.is_err() {
            // Clean up temp directory
            cleanup_temp_dir(&temp_dir);

            // Clean up reorganized directory if it exists
            if let Some(parent) = temp_dir.parent() {
                let reorganized = parent.join(format!(
                    "{}.reorganized",
                    temp_dir.file_name().unwrap().to_string_lossy()
                ));
                cleanup_temp_dir(&reorganized);
            }
        }

        result
    }

    fn restore_inner(
        data_dir: &Path,
        backup_path: &Path,
        temp_dir: &Path,
    ) -> Result<(), RestoreError> {
        // Step 3: Extract backup.tar
        extract_archive(backup_path, temp_dir)?;

        // Step 4: Validate backup structure
        validate_backup_structure(temp_dir)?;

        // Step 5: Validate backup manifest
        let manifest = validate_backup_manifest(temp_dir)?;

        // Step 6: Validate snapshot
        validate_snapshot(temp_dir)?;

        // Step 7: Validate WAL
        validate_wal(temp_dir)?;

        // Step 8: fsync temp directory
        fsync_recursive(temp_dir)?;

        // Step 9: Reorganize files to data_dir structure
        let reorganized = reorganize_extracted_files(temp_dir, &manifest.snapshot_id)?;

        // Clean up original temp directory (we have reorganized now)
        cleanup_temp_dir(temp_dir);

        // Step 10-13: Atomic directory replacement
        atomic_replace(data_dir, &reorganized)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tar::Builder;
    use tempfile::TempDir;

    fn create_test_backup_archive(archive_path: &Path) {
        let file = File::create(archive_path).unwrap();
        let mut builder = Builder::new(file);

        let temp = TempDir::new().unwrap();

        // Create snapshot directory
        let snapshot_dir = temp.path().join("snapshot");
        fs::create_dir_all(&snapshot_dir).unwrap();

        let mut f = File::create(snapshot_dir.join("manifest.json")).unwrap();
        f.write_all(br#"{"snapshot_id":"20260204T163000Z"}"#)
            .unwrap();

        let mut f = File::create(snapshot_dir.join("storage.dat")).unwrap();
        f.write_all(b"test storage data").unwrap();

        let schemas_dir = snapshot_dir.join("schemas");
        fs::create_dir_all(&schemas_dir).unwrap();
        let mut f = File::create(schemas_dir.join("user_v1.json")).unwrap();
        f.write_all(br#"{"name":"user"}"#).unwrap();

        // Create wal directory
        let wal_dir = temp.path().join("wal");
        fs::create_dir_all(&wal_dir).unwrap();
        let mut f = File::create(wal_dir.join("wal.log")).unwrap();
        f.write_all(b"wal data").unwrap();

        // Create backup manifest
        let mut f = File::create(temp.path().join("backup_manifest.json")).unwrap();
        f.write_all(br#"{"backup_id":"20260204T163000Z","snapshot_id":"20260204T163000Z","created_at":"2026-02-04T16:30:00Z","wal_present":true,"format_version":1}"#).unwrap();

        // Add to archive
        builder.append_dir_all("snapshot", &snapshot_dir).unwrap();
        builder.append_dir_all("wal", &wal_dir).unwrap();

        let manifest_path = temp.path().join("backup_manifest.json");
        let mut manifest_file = File::open(&manifest_path).unwrap();
        builder
            .append_file("backup_manifest.json", &mut manifest_file)
            .unwrap();

        builder.finish().unwrap();
    }

    fn create_existing_data_dir(data_dir: &Path) {
        fs::create_dir_all(data_dir.join("data")).unwrap();
        fs::create_dir_all(data_dir.join("wal")).unwrap();
        fs::create_dir_all(data_dir.join("metadata").join("schemas")).unwrap();

        let mut f = File::create(data_dir.join("data").join("storage.dat")).unwrap();
        f.write_all(b"old data").unwrap();
    }

    #[test]
    fn test_restore_from_backup_valid() {
        let temp_dir = TempDir::new().unwrap();

        // Create existing data directory
        let data_dir = temp_dir.path().join("data");
        create_existing_data_dir(&data_dir);

        // Create backup archive
        let backup_path = temp_dir.path().join("backup.tar");
        create_test_backup_archive(&backup_path);

        let result = RestoreManager::restore_from_backup(&data_dir, &backup_path);
        assert!(result.is_ok());

        // Verify restored structure
        assert!(data_dir.join("data").join("storage.dat").exists());
        assert!(data_dir.join("wal").exists());
        assert!(data_dir.join("metadata").join("schemas").exists());
        assert!(data_dir.join("snapshots").join("20260204T163000Z").exists());
    }

    #[test]
    fn test_restore_invalid_backup_rejected() {
        let temp_dir = TempDir::new().unwrap();

        // Create existing data directory
        let data_dir = temp_dir.path().join("data");
        create_existing_data_dir(&data_dir);

        // Create invalid backup (empty tar)
        let backup_path = temp_dir.path().join("backup.tar");
        let file = File::create(&backup_path).unwrap();
        let builder = Builder::new(file);
        builder.into_inner().unwrap();

        let result = RestoreManager::restore_from_backup(&data_dir, &backup_path);
        assert!(result.is_err());

        // Original data should be preserved
        assert!(data_dir.join("data").join("storage.dat").exists());
    }

    #[test]
    fn test_restore_backup_not_found() {
        let temp_dir = TempDir::new().unwrap();

        let data_dir = temp_dir.path().join("data");
        create_existing_data_dir(&data_dir);

        let backup_path = temp_dir.path().join("nonexistent.tar");

        let result = RestoreManager::restore_from_backup(&data_dir, &backup_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_restore_data_dir_not_found() {
        let temp_dir = TempDir::new().unwrap();

        let data_dir = temp_dir.path().join("nonexistent");

        let backup_path = temp_dir.path().join("backup.tar");
        create_test_backup_archive(&backup_path);

        let result = RestoreManager::restore_from_backup(&data_dir, &backup_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_restore_aerodb_running() {
        let temp_dir = TempDir::new().unwrap();

        let data_dir = temp_dir.path().join("data");
        create_existing_data_dir(&data_dir);

        // Create lock file to simulate running instance
        File::create(data_dir.join(".lock")).unwrap();

        let backup_path = temp_dir.path().join("backup.tar");
        create_test_backup_archive(&backup_path);

        let result = RestoreManager::restore_from_backup(&data_dir, &backup_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().message().contains("running"));
    }

    #[test]
    fn test_restore_temp_cleanup_on_error() {
        let temp_dir = TempDir::new().unwrap();

        let data_dir = temp_dir.path().join("data");
        create_existing_data_dir(&data_dir);

        // Create invalid backup
        let backup_path = temp_dir.path().join("backup.tar");
        let file = File::create(&backup_path).unwrap();
        let builder = Builder::new(file);
        builder.into_inner().unwrap();

        let _ = RestoreManager::restore_from_backup(&data_dir, &backup_path);

        // Temp directory should be cleaned up
        let temp_restore = temp_dir.path().join("data.restore_tmp");
        assert!(!temp_restore.exists());
    }

    #[test]
    fn test_restore_error_is_fatal() {
        let err = RestoreError::failed("test");
        assert!(err.is_fatal());
    }

    #[test]
    fn test_restore_preserves_original_on_failure() {
        let temp_dir = TempDir::new().unwrap();

        let data_dir = temp_dir.path().join("data");
        create_existing_data_dir(&data_dir);

        // Make a backup of original content
        let original_content = fs::read(data_dir.join("data").join("storage.dat")).unwrap();

        // Create invalid backup
        let backup_path = temp_dir.path().join("backup.tar");
        let file = File::create(&backup_path).unwrap();
        let builder = Builder::new(file);
        builder.into_inner().unwrap();

        let _ = RestoreManager::restore_from_backup(&data_dir, &backup_path);

        // Original data should be preserved
        let current_content = fs::read(data_dir.join("data").join("storage.dat")).unwrap();
        assert_eq!(original_content, current_content);
    }
}
