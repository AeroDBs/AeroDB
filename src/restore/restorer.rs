//! Atomic directory replacement for restore
//!
//! Per RESTORE.md §6:
//! - Atomic replacement via directory rename + fsync
//! - Either old data_dir or new data_dir exists
//! - Never partial mixed state
//!
//! The replacement follows:
//! 1. Move data_dir → data_dir.old
//! 2. Move temp_dir → data_dir
//! 3. fsync parent directory
//! 4. Delete data_dir.old

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use super::errors::{RestoreError, RestoreResult};
use super::extractor::get_old_data_dir_path;

/// fsync a directory
pub fn fsync_dir(dir: &Path) -> RestoreResult<()> {
    let d = OpenOptions::new()
        .read(true)
        .open(dir)
        .map_err(|e| RestoreError::io_error_at_path(dir, e))?;

    d.sync_all()
        .map_err(|e| RestoreError::io_error(format!("Failed to fsync {}", dir.display()), e))
}

/// fsync a directory recursively
pub fn fsync_recursive(dir: &Path) -> RestoreResult<()> {
    if !dir.exists() {
        return Ok(());
    }

    let entries = fs::read_dir(dir).map_err(|e| RestoreError::io_error_at_path(dir, e))?;

    for entry in entries {
        let entry = entry.map_err(|e| RestoreError::io_error_at_path(dir, e))?;
        let path = entry.path();

        if path.is_dir() {
            fsync_recursive(&path)?;
        } else {
            // fsync the file
            let file = OpenOptions::new()
                .read(true)
                .open(&path)
                .map_err(|e| RestoreError::io_error_at_path(&path, e))?;

            file.sync_all().map_err(|e| {
                RestoreError::io_error(format!("Failed to fsync {}", path.display()), e)
            })?;
        }
    }

    // fsync the directory itself
    fsync_dir(dir)?;

    Ok(())
}

/// Copy file with fsync
fn copy_file_with_fsync(src: &Path, dst: &Path) -> RestoreResult<()> {
    // Create parent directories if needed
    if let Some(parent) = dst.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| {
                RestoreError::io_error(format!("Failed to create {}", parent.display()), e)
            })?;
        }
    }

    // Read source
    let mut src_file = File::open(src).map_err(|e| RestoreError::io_error_at_path(src, e))?;
    let mut contents = Vec::new();
    src_file
        .read_to_end(&mut contents)
        .map_err(|e| RestoreError::io_error_at_path(src, e))?;

    // Write to destination
    let mut dst_file = File::create(dst).map_err(|e| RestoreError::io_error_at_path(dst, e))?;
    dst_file
        .write_all(&contents)
        .map_err(|e| RestoreError::io_error_at_path(dst, e))?;
    dst_file
        .sync_all()
        .map_err(|e| RestoreError::io_error_at_path(dst, e))?;

    Ok(())
}

/// Copy directory recursively with fsync
fn copy_dir_recursive(src: &Path, dst: &Path) -> RestoreResult<()> {
    fs::create_dir_all(dst)
        .map_err(|e| RestoreError::io_error(format!("Failed to create {}", dst.display()), e))?;

    let entries = fs::read_dir(src).map_err(|e| RestoreError::io_error_at_path(src, e))?;

    for entry in entries {
        let entry = entry.map_err(|e| RestoreError::io_error_at_path(src, e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            copy_file_with_fsync(&src_path, &dst_path)?;
        }
    }

    fsync_dir(dst)?;
    Ok(())
}

/// Reorganize extracted files into proper data_dir structure
///
/// Backup structure:
/// - snapshot/storage.dat
/// - snapshot/manifest.json
/// - snapshot/schemas/
/// - wal/wal.log
/// - backup_manifest.json
///
/// Data dir structure:
/// - data/storage.dat
/// - metadata/schemas/
/// - snapshots/<snapshot_id>/
/// - wal/wal.log
/// - checkpoint.json (optional, created from backup manifest)
pub fn reorganize_extracted_files(temp_dir: &Path, snapshot_id: &str) -> RestoreResult<PathBuf> {
    let reorganized = temp_dir
        .parent()
        .ok_or_else(|| RestoreError::failed("Invalid temp directory"))?
        .join(format!(
            "{}.reorganized",
            temp_dir.file_name().unwrap().to_string_lossy()
        ));

    if reorganized.exists() {
        fs::remove_dir_all(&reorganized).map_err(|e| {
            RestoreError::io_error(format!("Failed to clean up {}", reorganized.display()), e)
        })?;
    }

    fs::create_dir_all(&reorganized).map_err(|e| {
        RestoreError::io_error(format!("Failed to create {}", reorganized.display()), e)
    })?;

    let snapshot_src = temp_dir.join("snapshot");
    let wal_src = temp_dir.join("wal");

    // 1. Copy storage.dat to data/storage.dat
    let data_dir = reorganized.join("data");
    fs::create_dir_all(&data_dir).map_err(|e| {
        RestoreError::io_error(format!("Failed to create {}", data_dir.display()), e)
    })?;

    let storage_src = snapshot_src.join("storage.dat");
    if storage_src.exists() {
        let storage_dst = data_dir.join("storage.dat");
        copy_file_with_fsync(&storage_src, &storage_dst)?;
    }

    // 2. Copy schemas to metadata/schemas/
    let schemas_src = snapshot_src.join("schemas");
    if schemas_src.exists() {
        let schemas_dst = reorganized.join("metadata").join("schemas");
        copy_dir_recursive(&schemas_src, &schemas_dst)?;
    } else {
        // Create empty schemas directory
        let schemas_dst = reorganized.join("metadata").join("schemas");
        fs::create_dir_all(&schemas_dst).map_err(|e| {
            RestoreError::io_error(format!("Failed to create {}", schemas_dst.display()), e)
        })?;
    }

    // 3. Create snapshots/<snapshot_id>/ with snapshot manifest
    let snapshots_dir = reorganized.join("snapshots").join(snapshot_id);
    fs::create_dir_all(&snapshots_dir).map_err(|e| {
        RestoreError::io_error(format!("Failed to create {}", snapshots_dir.display()), e)
    })?;

    // Copy snapshot manifest
    let manifest_src = snapshot_src.join("manifest.json");
    if manifest_src.exists() {
        let manifest_dst = snapshots_dir.join("manifest.json");
        copy_file_with_fsync(&manifest_src, &manifest_dst)?;
    }

    // Copy storage.dat to snapshot directory as well (for snapshot consistency)
    if storage_src.exists() {
        let storage_dst = snapshots_dir.join("storage.dat");
        copy_file_with_fsync(&storage_src, &storage_dst)?;
    }

    // Copy schemas to snapshot directory
    if schemas_src.exists() {
        let schemas_dst = snapshots_dir.join("schemas");
        copy_dir_recursive(&schemas_src, &schemas_dst)?;
    }

    // 4. Copy WAL to wal/
    let wal_dst = reorganized.join("wal");
    if wal_src.exists() {
        copy_dir_recursive(&wal_src, &wal_dst)?;
    } else {
        fs::create_dir_all(&wal_dst).map_err(|e| {
            RestoreError::io_error(format!("Failed to create {}", wal_dst.display()), e)
        })?;
    }

    // 5. fsync everything
    fsync_recursive(&reorganized)?;

    Ok(reorganized)
}

/// Atomically replace data directory
///
/// Per RESTORE.md §6:
/// 1. Move data_dir → data_dir.old
/// 2. Move temp_dir → data_dir
/// 3. fsync parent directory
/// 4. Delete data_dir.old
pub fn atomic_replace(data_dir: &Path, reorganized_dir: &Path) -> RestoreResult<()> {
    let old_dir = get_old_data_dir_path(data_dir)?;
    let parent = data_dir.parent().unwrap_or(Path::new("."));

    // Clean up any stale .old directory
    if old_dir.exists() {
        fs::remove_dir_all(&old_dir).map_err(|e| {
            RestoreError::io_error(format!("Failed to remove stale {}", old_dir.display()), e)
        })?;
    }

    // Step 1: Move data_dir → data_dir.old
    fs::rename(data_dir, &old_dir).map_err(|e| {
        RestoreError::io_error(
            format!(
                "Failed to move {} to {}",
                data_dir.display(),
                old_dir.display()
            ),
            e,
        )
    })?;

    // Step 2: Move reorganized_dir → data_dir
    let rename_result = fs::rename(reorganized_dir, data_dir);
    if let Err(e) = rename_result {
        // Rollback: restore old_dir to data_dir
        let _ = fs::rename(&old_dir, data_dir);
        return Err(RestoreError::io_error(
            format!(
                "Failed to move {} to {} (rolled back)",
                reorganized_dir.display(),
                data_dir.display()
            ),
            e,
        ));
    }

    // Step 3: fsync parent directory
    fsync_dir(parent)?;

    // Step 4: Delete data_dir.old
    // This is best-effort, not critical to restore success
    let _ = fs::remove_dir_all(&old_dir);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_extracted_backup(dir: &Path) {
        // Create extracted backup structure
        let snapshot_dir = dir.join("snapshot");
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

        let wal_dir = dir.join("wal");
        fs::create_dir_all(&wal_dir).unwrap();
        let mut f = File::create(wal_dir.join("wal.log")).unwrap();
        f.write_all(b"wal data").unwrap();

        let mut f = File::create(dir.join("backup_manifest.json")).unwrap();
        f.write_all(br#"{"backup_id":"20260204T163000Z"}"#).unwrap();
    }

    #[test]
    fn test_fsync_dir() {
        let temp_dir = TempDir::new().unwrap();

        // Should not error
        let result = fsync_dir(temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_fsync_recursive() {
        let temp_dir = TempDir::new().unwrap();

        // Create nested structure
        let nested = temp_dir.path().join("a").join("b").join("c");
        fs::create_dir_all(&nested).unwrap();

        let mut f = File::create(nested.join("file.txt")).unwrap();
        f.write_all(b"content").unwrap();

        let result = fsync_recursive(temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_reorganize_extracted_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create extracted backup structure
        let extracted = temp_dir.path().join("extracted");
        fs::create_dir_all(&extracted).unwrap();
        create_test_extracted_backup(&extracted);

        let result = reorganize_extracted_files(&extracted, "20260204T163000Z");
        assert!(result.is_ok());

        let reorganized = result.unwrap();

        // Verify structure
        assert!(reorganized.join("data").join("storage.dat").exists());
        assert!(reorganized.join("metadata").join("schemas").exists());
        assert!(reorganized
            .join("snapshots")
            .join("20260204T163000Z")
            .exists());
        assert!(reorganized.join("wal").exists());
    }

    #[test]
    fn test_atomic_replace() {
        let temp_dir = TempDir::new().unwrap();

        // Create existing data directory
        let data_dir = temp_dir.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();
        let mut f = File::create(data_dir.join("old_file.txt")).unwrap();
        f.write_all(b"old content").unwrap();

        // Create new reorganized directory
        let reorganized = temp_dir.path().join("data.reorganized");
        fs::create_dir_all(&reorganized).unwrap();
        let mut f = File::create(reorganized.join("new_file.txt")).unwrap();
        f.write_all(b"new content").unwrap();

        let result = atomic_replace(&data_dir, &reorganized);
        assert!(result.is_ok());

        // Verify new data is in place
        assert!(data_dir.join("new_file.txt").exists());
        assert!(!data_dir.join("old_file.txt").exists());

        // Verify reorganized directory is gone
        assert!(!reorganized.exists());
    }

    #[test]
    fn test_atomic_replace_cleans_stale_old() {
        let temp_dir = TempDir::new().unwrap();

        // Create existing data directory
        let data_dir = temp_dir.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();

        // Create stale .old directory
        let old_dir = temp_dir.path().join("data.old");
        fs::create_dir_all(&old_dir).unwrap();

        // Create new reorganized directory
        let reorganized = temp_dir.path().join("data.reorganized");
        fs::create_dir_all(&reorganized).unwrap();

        let result = atomic_replace(&data_dir, &reorganized);
        assert!(result.is_ok());
    }
}
