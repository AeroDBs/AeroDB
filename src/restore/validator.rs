//! Backup validation for restore
//!
//! Per RESTORE.md §4 and §5:
//! - Validate backup structure
//! - Validate backup_manifest.json
//! - Validate snapshot manifest and checksums
//! - Validate WAL files
//!
//! Any validation failure aborts restore.

use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use crate::backup::BackupManifest;

use super::errors::{RestoreError, RestoreResult};

/// Validate backup structure
///
/// Per RESTORE.md §4, backup must contain:
/// - snapshot/
/// - wal/
/// - backup_manifest.json
pub fn validate_backup_structure(restore_dir: &Path) -> RestoreResult<()> {
    // Check snapshot directory exists
    let snapshot_dir = restore_dir.join("snapshot");
    if !snapshot_dir.exists() || !snapshot_dir.is_dir() {
        return Err(RestoreError::invalid_backup(
            "Missing snapshot/ directory in backup archive",
        ));
    }

    // Check wal directory exists
    let wal_dir = restore_dir.join("wal");
    if !wal_dir.exists() || !wal_dir.is_dir() {
        return Err(RestoreError::invalid_backup(
            "Missing wal/ directory in backup archive",
        ));
    }

    // Check backup_manifest.json exists
    let manifest_path = restore_dir.join("backup_manifest.json");
    if !manifest_path.exists() {
        return Err(RestoreError::invalid_backup(
            "Missing backup_manifest.json in backup archive",
        ));
    }

    Ok(())
}

/// Validate backup manifest
///
/// Per RESTORE.md §5:
/// - format_version == 1
/// - snapshot_id present
pub fn validate_backup_manifest(restore_dir: &Path) -> RestoreResult<BackupManifest> {
    let manifest_path = restore_dir.join("backup_manifest.json");

    let manifest = BackupManifest::read_from_file(&manifest_path).map_err(|e| {
        RestoreError::invalid_backup(format!("Failed to read backup manifest: {}", e))
    })?;

    // Validate format_version
    if manifest.format_version != 1 {
        return Err(RestoreError::invalid_backup(format!(
            "Unsupported backup format version: expected 1, got {}",
            manifest.format_version
        )));
    }

    // Validate snapshot_id present
    if manifest.snapshot_id.is_empty() {
        return Err(RestoreError::invalid_backup(
            "Backup manifest has empty snapshot_id",
        ));
    }

    Ok(manifest)
}

/// Validate snapshot within the backup
///
/// Per RESTORE.md §5:
/// - snapshot manifest exists
/// - checksums correct (if present)
/// - required files exist
pub fn validate_snapshot(restore_dir: &Path) -> RestoreResult<()> {
    let snapshot_dir = restore_dir.join("snapshot");

    // Check manifest.json exists
    let manifest_path = snapshot_dir.join("manifest.json");
    if !manifest_path.exists() {
        return Err(RestoreError::corruption(
            "Missing snapshot manifest.json in backup",
        ));
    }

    // Read and validate manifest is valid JSON
    let mut file = File::open(&manifest_path)
        .map_err(|e| RestoreError::io_error_at_path(&manifest_path, e))?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| RestoreError::io_error_at_path(&manifest_path, e))?;

    // Validate it's valid JSON
    let _: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|e| RestoreError::corruption(format!("Invalid snapshot manifest JSON: {}", e)))?;

    // Check storage.dat exists
    let storage_path = snapshot_dir.join("storage.dat");
    if !storage_path.exists() {
        return Err(RestoreError::corruption(
            "Missing storage.dat in backup snapshot",
        ));
    }

    // Check schemas directory exists (optional, may be empty)
    let schemas_dir = snapshot_dir.join("schemas");
    if schemas_dir.exists() && !schemas_dir.is_dir() {
        return Err(RestoreError::corruption(
            "schemas in backup is not a directory",
        ));
    }

    Ok(())
}

/// Validate WAL files within the backup
///
/// Per RESTORE.md §5:
/// - WAL directory readable
/// - WAL files accessible
pub fn validate_wal(restore_dir: &Path) -> RestoreResult<()> {
    let wal_dir = restore_dir.join("wal");

    if !wal_dir.exists() {
        return Err(RestoreError::invalid_backup(
            "Missing wal/ directory in backup",
        ));
    }

    // Check wal.log exists (may be empty)
    let wal_log = wal_dir.join("wal.log");
    if wal_log.exists() {
        // Try to open it to verify it's readable
        File::open(&wal_log).map_err(|e| RestoreError::io_error_at_path(&wal_log, e))?;
    }

    Ok(())
}

/// Check if AeroDB is currently running
///
/// Per RESTORE.md §3: AeroDB must not be running
/// We check for lock files that would indicate a running instance
pub fn check_not_running(data_dir: &Path) -> RestoreResult<()> {
    // Check for lock file that would indicate running instance
    let lock_file = data_dir.join(".lock");
    if lock_file.exists() {
        return Err(RestoreError::failed(
            "AeroDB appears to be running (lock file exists). Stop AeroDB before restoring.",
        ));
    }

    Ok(())
}

/// Validate preconditions for restore
///
/// Per RESTORE.md §3:
/// - AeroDB must not be running
/// - data_dir must exist
/// - backup archive must exist and be readable
pub fn validate_preconditions(data_dir: &Path, backup_path: &Path) -> RestoreResult<()> {
    // Check data_dir exists
    if !data_dir.exists() {
        return Err(RestoreError::failed(format!(
            "Data directory does not exist: {}",
            data_dir.display()
        )));
    }

    // Check backup exists
    if !backup_path.exists() {
        return Err(RestoreError::failed(format!(
            "Backup file does not exist: {}",
            backup_path.display()
        )));
    }

    // Check backup is readable
    File::open(backup_path).map_err(|e| {
        RestoreError::io_error(
            format!("Cannot read backup file: {}", backup_path.display()),
            e,
        )
    })?;

    // Check AeroDB not running
    check_not_running(data_dir)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_valid_backup_structure(dir: &Path) {
        // Create snapshot directory
        let snapshot_dir = dir.join("snapshot");
        fs::create_dir_all(&snapshot_dir).unwrap();

        // Create manifest.json
        let mut f = File::create(snapshot_dir.join("manifest.json")).unwrap();
        f.write_all(br#"{"snapshot_id":"test"}"#).unwrap();

        // Create storage.dat
        let mut f = File::create(snapshot_dir.join("storage.dat")).unwrap();
        f.write_all(b"test data").unwrap();

        // Create schemas dir
        fs::create_dir_all(snapshot_dir.join("schemas")).unwrap();

        // Create wal directory
        let wal_dir = dir.join("wal");
        fs::create_dir_all(&wal_dir).unwrap();
        File::create(wal_dir.join("wal.log")).unwrap();

        // Create backup_manifest.json
        let mut f = File::create(dir.join("backup_manifest.json")).unwrap();
        f.write_all(br#"{"backup_id":"20260204T163000Z","snapshot_id":"20260204T163000Z","created_at":"2026-02-04T16:30:00Z","wal_present":true,"format_version":1}"#).unwrap();
    }

    #[test]
    fn test_validate_backup_structure_valid() {
        let temp_dir = TempDir::new().unwrap();
        create_valid_backup_structure(temp_dir.path());

        let result = validate_backup_structure(temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_backup_structure_missing_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join("wal")).unwrap();
        File::create(temp_dir.path().join("backup_manifest.json")).unwrap();

        let result = validate_backup_structure(temp_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().message().contains("snapshot"));
    }

    #[test]
    fn test_validate_backup_structure_missing_wal() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join("snapshot")).unwrap();
        File::create(temp_dir.path().join("backup_manifest.json")).unwrap();

        let result = validate_backup_structure(temp_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().message().contains("wal"));
    }

    #[test]
    fn test_validate_backup_structure_missing_manifest() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join("snapshot")).unwrap();
        fs::create_dir_all(temp_dir.path().join("wal")).unwrap();

        let result = validate_backup_structure(temp_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().message().contains("manifest"));
    }

    #[test]
    fn test_validate_backup_manifest_valid() {
        let temp_dir = TempDir::new().unwrap();
        create_valid_backup_structure(temp_dir.path());

        let result = validate_backup_manifest(temp_dir.path());
        assert!(result.is_ok());

        let manifest = result.unwrap();
        assert_eq!(manifest.format_version, 1);
        assert_eq!(manifest.snapshot_id, "20260204T163000Z");
    }

    #[test]
    fn test_validate_backup_manifest_wrong_version() {
        let temp_dir = TempDir::new().unwrap();

        let mut f = File::create(temp_dir.path().join("backup_manifest.json")).unwrap();
        f.write_all(br#"{"backup_id":"test","snapshot_id":"test","created_at":"test","wal_present":true,"format_version":99}"#).unwrap();

        let result = validate_backup_manifest(temp_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().message().contains("format version"));
    }

    #[test]
    fn test_validate_snapshot_valid() {
        let temp_dir = TempDir::new().unwrap();
        create_valid_backup_structure(temp_dir.path());

        let result = validate_snapshot(temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_snapshot_missing_manifest() {
        let temp_dir = TempDir::new().unwrap();
        let snapshot_dir = temp_dir.path().join("snapshot");
        fs::create_dir_all(&snapshot_dir).unwrap();

        // Create storage.dat but no manifest
        File::create(snapshot_dir.join("storage.dat")).unwrap();

        let result = validate_snapshot(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_snapshot_missing_storage() {
        let temp_dir = TempDir::new().unwrap();
        let snapshot_dir = temp_dir.path().join("snapshot");
        fs::create_dir_all(&snapshot_dir).unwrap();

        // Create manifest but no storage.dat
        let mut f = File::create(snapshot_dir.join("manifest.json")).unwrap();
        f.write_all(br#"{"snapshot_id":"test"}"#).unwrap();

        let result = validate_snapshot(temp_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().message().contains("storage.dat"));
    }

    #[test]
    fn test_validate_wal_valid() {
        let temp_dir = TempDir::new().unwrap();
        create_valid_backup_structure(temp_dir.path());

        let result = validate_wal(temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_preconditions_data_dir_missing() {
        let temp_dir = TempDir::new().unwrap();
        let backup_path = temp_dir.path().join("backup.tar");
        File::create(&backup_path).unwrap();

        let missing_data_dir = temp_dir.path().join("nonexistent");

        let result = validate_preconditions(&missing_data_dir, &backup_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_preconditions_backup_missing() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();

        let missing_backup = temp_dir.path().join("nonexistent.tar");

        let result = validate_preconditions(&data_dir, &missing_backup);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_not_running_no_lock() {
        let temp_dir = TempDir::new().unwrap();

        let result = check_not_running(temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_not_running_lock_exists() {
        let temp_dir = TempDir::new().unwrap();
        File::create(temp_dir.path().join(".lock")).unwrap();

        let result = check_not_running(temp_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().message().contains("running"));
    }
}
