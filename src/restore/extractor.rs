//! Archive extraction for restore
//!
//! Per RESTORE.md §5:
//! - Extract backup.tar to temp directory
//! - Validate extraction was complete
//! - Handle cleanup on failure

use std::fs::{self, File};
use std::path::{Path, PathBuf};

use tar::Archive;

use super::errors::{RestoreError, RestoreResult};

/// Create temp restore directory
///
/// Per RESTORE.md §5: Create <data_dir>.restore_tmp
pub fn create_temp_restore_dir(data_dir: &Path) -> RestoreResult<PathBuf> {
    let parent = data_dir.parent().unwrap_or(Path::new("."));
    let data_dir_name = data_dir
        .file_name()
        .ok_or_else(|| RestoreError::failed("Invalid data directory name"))?;

    let temp_name = format!("{}.restore_tmp", data_dir_name.to_string_lossy());
    let temp_dir = parent.join(temp_name);

    // Clean up any existing temp directory from failed restore
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).map_err(|e| {
            RestoreError::io_error(
                format!(
                    "Failed to clean up existing temp directory: {}",
                    temp_dir.display()
                ),
                e,
            )
        })?;
    }

    fs::create_dir_all(&temp_dir).map_err(|e| {
        RestoreError::io_error(
            format!(
                "Failed to create temp restore directory: {}",
                temp_dir.display()
            ),
            e,
        )
    })?;

    Ok(temp_dir)
}

/// Extract backup.tar to destination directory
///
/// Per RESTORE.md §5: Extract backup.tar into temp directory
pub fn extract_archive(archive_path: &Path, dest_dir: &Path) -> RestoreResult<()> {
    let file = File::open(archive_path).map_err(|e| {
        RestoreError::io_error(
            format!("Failed to open backup archive: {}", archive_path.display()),
            e,
        )
    })?;

    let mut archive = Archive::new(file);

    archive.unpack(dest_dir).map_err(|e| {
        RestoreError::invalid_backup_with_source(
            format!(
                "Failed to extract backup archive: {}",
                archive_path.display()
            ),
            std::io::Error::new(std::io::ErrorKind::Other, e),
        )
    })?;

    Ok(())
}

/// Cleanup temp directory
///
/// Per RESTORE.md §5: Delete temp directory on failure
pub fn cleanup_temp_dir(temp_dir: &Path) {
    if temp_dir.exists() {
        let _ = fs::remove_dir_all(temp_dir);
    }
}

/// Cleanup old data directory
///
/// Per RESTORE.md §5: Delete data_dir.old after successful restore
pub fn cleanup_old_dir(old_dir: &Path) {
    if old_dir.exists() {
        let _ = fs::remove_dir_all(old_dir);
    }
}

/// Get the path for the backup of the old data directory
pub fn get_old_data_dir_path(data_dir: &Path) -> RestoreResult<PathBuf> {
    let parent = data_dir.parent().unwrap_or(Path::new("."));
    let data_dir_name = data_dir
        .file_name()
        .ok_or_else(|| RestoreError::failed("Invalid data directory name"))?;

    let old_name = format!("{}.old", data_dir_name.to_string_lossy());
    Ok(parent.join(old_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tar::Builder;
    use tempfile::TempDir;

    fn create_test_archive(archive_path: &Path) {
        let file = File::create(archive_path).unwrap();
        let mut builder = Builder::new(file);

        // Create a temp dir with test structure
        let temp = TempDir::new().unwrap();

        // Create snapshot directory
        let snapshot_dir = temp.path().join("snapshot");
        fs::create_dir_all(&snapshot_dir).unwrap();

        let mut f = File::create(snapshot_dir.join("manifest.json")).unwrap();
        f.write_all(br#"{"id":"test"}"#).unwrap();

        let mut f = File::create(snapshot_dir.join("storage.dat")).unwrap();
        f.write_all(b"test storage").unwrap();

        // Create wal directory
        let wal_dir = temp.path().join("wal");
        fs::create_dir_all(&wal_dir).unwrap();
        File::create(wal_dir.join("wal.log")).unwrap();

        // Create backup manifest
        let mut f = File::create(temp.path().join("backup_manifest.json")).unwrap();
        f.write_all(br#"{"backup_id":"test","snapshot_id":"test","created_at":"test","wal_present":true,"format_version":1}"#).unwrap();

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

    #[test]
    fn test_create_temp_restore_dir() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();

        let result = create_temp_restore_dir(&data_dir);
        assert!(result.is_ok());

        let restore_dir = result.unwrap();
        assert!(restore_dir.exists());
        assert!(restore_dir.to_string_lossy().contains("restore_tmp"));
    }

    #[test]
    fn test_create_temp_restore_dir_cleans_existing() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();

        // Create existing temp dir
        let existing_temp = temp_dir.path().join("data.restore_tmp");
        fs::create_dir_all(&existing_temp).unwrap();
        File::create(existing_temp.join("old_file")).unwrap();

        let result = create_temp_restore_dir(&data_dir);
        assert!(result.is_ok());

        let restore_dir = result.unwrap();
        // Should have cleaned up and recreated
        assert!(restore_dir.exists());
        assert!(!restore_dir.join("old_file").exists());
    }

    #[test]
    fn test_extract_archive() {
        let temp_dir = TempDir::new().unwrap();

        // Create test archive
        let archive_path = temp_dir.path().join("backup.tar");
        create_test_archive(&archive_path);

        // Create extraction directory
        let dest_dir = temp_dir.path().join("extracted");
        fs::create_dir_all(&dest_dir).unwrap();

        let result = extract_archive(&archive_path, &dest_dir);
        assert!(result.is_ok());

        // Verify extraction
        assert!(dest_dir.join("snapshot").exists());
        assert!(dest_dir.join("snapshot").join("manifest.json").exists());
        assert!(dest_dir.join("wal").exists());
        assert!(dest_dir.join("backup_manifest.json").exists());
    }

    #[test]
    fn test_extract_archive_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("nonexistent.tar");
        let dest_dir = temp_dir.path().join("extracted");
        fs::create_dir_all(&dest_dir).unwrap();

        let result = extract_archive(&archive_path, &dest_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_cleanup_temp_dir() {
        let temp_dir = TempDir::new().unwrap();
        let restore_temp = temp_dir.path().join("data.restore_tmp");
        fs::create_dir_all(&restore_temp).unwrap();
        File::create(restore_temp.join("file")).unwrap();

        assert!(restore_temp.exists());

        cleanup_temp_dir(&restore_temp);

        assert!(!restore_temp.exists());
    }

    #[test]
    fn test_cleanup_nonexistent_dir() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("nonexistent");

        // Should not panic
        cleanup_temp_dir(&nonexistent);
    }

    #[test]
    fn test_get_old_data_dir_path() {
        let data_dir = Path::new("/var/lib/aerodb/data");
        let old_path = get_old_data_dir_path(data_dir).unwrap();

        assert_eq!(old_path, Path::new("/var/lib/aerodb/data.old"));
    }
}
