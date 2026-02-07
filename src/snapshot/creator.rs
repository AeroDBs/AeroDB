//! Core snapshot creation logic
//!
//! Per SNAPSHOT.md §4, snapshot creation must follow this exact sequence:
//!
//! 1. Pause writes (acquire global execution lock) - done by caller
//! 2. fsync WAL
//! 3. Copy storage.dat → snapshot/storage.dat
//! 4. fsync snapshot/storage.dat
//! 5. Copy schemas → snapshot/schemas/
//! 6. fsync snapshot/schemas directory
//! 7. Generate manifest.json
//! 8. fsync manifest.json
//! 9. fsync snapshot directory
//! 10. Release global lock - done by caller
//!
//! Any failure aborts snapshot and cleans up partial directory.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use chrono::Utc;

use super::checksum::{compute_file_checksum, format_checksum};
use super::errors::{SnapshotError, SnapshotResult};
use super::manifest::SnapshotManifest;
use super::SnapshotId;

/// Generates a snapshot ID in RFC3339 basic format.
///
/// Format: YYYYMMDDTHHMMSSZ
///
/// Example: 20260204T113000Z
pub fn generate_snapshot_id() -> SnapshotId {
    Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

/// Generates the RFC3339 timestamp for created_at field.
///
/// Format: YYYY-MM-DDTHH:MM:SSZ
fn generate_created_at() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// fsync a directory to ensure durability.
///
/// On Unix, this opens the directory and calls fsync on it.
fn fsync_dir(path: &Path) -> SnapshotResult<()> {
    let dir = OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|e| SnapshotError::io_error_at_path(path, e))?;

    dir.sync_all().map_err(|e| {
        SnapshotError::io_error(format!("fsync directory failed: {}", path.display()), e)
    })
}

/// Copy a file byte-for-byte with fsync.
///
/// Per SNAPSHOT.md §3.1:
/// - byte-for-byte copy
/// - fsync before manifest creation
fn copy_file_with_fsync(src: &Path, dst: &Path) -> SnapshotResult<()> {
    let mut src_file = File::open(src).map_err(|e| {
        SnapshotError::io_error(format!("Failed to open source file: {}", src.display()), e)
    })?;

    let mut dst_file = File::create(dst).map_err(|e| {
        SnapshotError::io_error(
            format!("Failed to create destination file: {}", dst.display()),
            e,
        )
    })?;

    // Copy in chunks for large files
    let mut buffer = [0u8; 8192];
    loop {
        let bytes_read = src_file.read(&mut buffer).map_err(|e| {
            SnapshotError::io_error(format!("Failed to read from: {}", src.display()), e)
        })?;

        if bytes_read == 0 {
            break;
        }

        dst_file.write_all(&buffer[..bytes_read]).map_err(|e| {
            SnapshotError::io_error(format!("Failed to write to: {}", dst.display()), e)
        })?;
    }

    // fsync is mandatory
    dst_file
        .sync_all()
        .map_err(|e| SnapshotError::io_error(format!("fsync failed for: {}", dst.display()), e))
}

/// Recursively copy a directory.
///
/// Per SNAPSHOT.md §3.2:
/// - copied recursively
/// - filenames preserved
fn copy_dir_recursive(src: &Path, dst: &Path) -> SnapshotResult<()> {
    fs::create_dir_all(dst).map_err(|e| {
        SnapshotError::io_error(format!("Failed to create directory: {}", dst.display()), e)
    })?;

    let entries = fs::read_dir(src).map_err(|e| {
        SnapshotError::io_error(format!("Failed to read directory: {}", src.display()), e)
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            SnapshotError::io_error(
                format!("Failed to read directory entry in: {}", src.display()),
                e,
            )
        })?;

        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if src_path.is_file() {
            copy_file_with_fsync(&src_path, &dst_path)?;
        }
        // Skip symlinks and other file types
    }

    Ok(())
}

/// Remove a snapshot directory (cleanup on failure).
fn cleanup_snapshot(path: &Path) {
    if path.exists() {
        // Best effort removal - we're already in an error path
        let _ = fs::remove_dir_all(path);
    }
}

/// Create a snapshot.
///
/// This function implements the snapshot creation algorithm per SNAPSHOT.md §4.
///
/// # Arguments
///
/// * `data_dir` - Root data directory (contains snapshots/)
/// * `storage_path` - Path to storage.dat file
/// * `schema_dir` - Path to schema directory
///
/// # Returns
///
/// The snapshot ID on success.
///
/// # Errors
///
/// Returns `SnapshotError` on any failure. The snapshot directory is cleaned up
/// on failure (no partial snapshots left behind).
///
/// # Panics
///
/// This function does not panic.
pub fn create_snapshot_impl(
    data_dir: &Path,
    storage_path: &Path,
    schema_dir: &Path,
) -> SnapshotResult<SnapshotId> {
    // Generate snapshot ID and timestamp
    let snapshot_id = generate_snapshot_id();
    let created_at = generate_created_at();

    // Create snapshot directory path
    let snapshots_dir = data_dir.join("snapshots");
    let snapshot_dir = snapshots_dir.join(&snapshot_id);

    // Create snapshot directory
    fs::create_dir_all(&snapshot_dir).map_err(|e| {
        SnapshotError::io_error(
            format!(
                "Failed to create snapshot directory: {}",
                snapshot_dir.display()
            ),
            e,
        )
    })?;

    // From here on, any error must clean up the snapshot directory
    let result = create_snapshot_contents(
        &snapshot_dir,
        storage_path,
        schema_dir,
        &snapshot_id,
        &created_at,
        None, // Phase-1: no MVCC boundary
    );

    if result.is_err() {
        cleanup_snapshot(&snapshot_dir);
    }

    result
}

/// Create snapshot contents (storage, schemas, manifest).
///
/// This is separated to make cleanup easier on failure.
fn create_snapshot_contents(
    snapshot_dir: &Path,
    storage_path: &Path,
    schema_dir: &Path,
    snapshot_id: &str,
    created_at: &str,
    commit_boundary: Option<u64>,
) -> SnapshotResult<SnapshotId> {
    // Step 3-4: Copy storage.dat and fsync
    let snapshot_storage = snapshot_dir.join("storage.dat");
    copy_file_with_fsync(storage_path, &snapshot_storage)?;

    // Step 5-6: Copy schemas recursively and fsync directory
    let snapshot_schemas = snapshot_dir.join("schemas");
    if schema_dir.exists() && schema_dir.is_dir() {
        copy_dir_recursive(schema_dir, &snapshot_schemas)?;
        fsync_dir(&snapshot_schemas)?;
    } else {
        // Create empty schemas directory if source doesn't exist
        fs::create_dir_all(&snapshot_schemas).map_err(|e| {
            SnapshotError::io_error(
                format!(
                    "Failed to create schemas directory: {}",
                    snapshot_schemas.display()
                ),
                e,
            )
        })?;
        fsync_dir(&snapshot_schemas)?;
    }

    // Compute checksums
    let storage_checksum = compute_file_checksum(&snapshot_storage)?;
    let storage_checksum_str = format_checksum(storage_checksum);

    let schema_checksums = compute_schema_checksums(&snapshot_schemas)?;

    // Step 7-8: Generate and write manifest with fsync
    // Use Phase-2 manifest if commit_boundary is provided
    let manifest = match commit_boundary {
        Some(boundary) => SnapshotManifest::with_mvcc_boundary(
            snapshot_id,
            created_at,
            storage_checksum_str,
            schema_checksums,
            boundary,
        ),
        None => SnapshotManifest::new(
            snapshot_id,
            created_at,
            storage_checksum_str,
            schema_checksums,
        ),
    };

    let manifest_path = snapshot_dir.join("manifest.json");
    manifest.write_to_file(&manifest_path)?;

    // Step 9: fsync snapshot directory
    fsync_dir(snapshot_dir)?;

    Ok(snapshot_id.to_string())
}

/// Create an MVCC-aware snapshot with commit boundary.
///
/// Per MVCC_SNAPSHOT_INTEGRATION.md:
/// - Captures all versions with commit_id ≤ boundary
/// - Records boundary in manifest (format_version = 2)
///
/// # Arguments
///
/// * `data_dir` - Root data directory (contains snapshots/)
/// * `storage_path` - Path to storage.dat file
/// * `schema_dir` - Path to schema directory
/// * `commit_boundary` - The MVCC commit identity boundary
///
/// # Returns
///
/// The snapshot ID on success.
pub fn create_mvcc_snapshot_impl(
    data_dir: &Path,
    storage_path: &Path,
    schema_dir: &Path,
    commit_boundary: u64,
) -> SnapshotResult<SnapshotId> {
    // Generate snapshot ID and timestamp
    let snapshot_id = generate_snapshot_id();
    let created_at = generate_created_at();

    // Create snapshot directory path
    let snapshots_dir = data_dir.join("snapshots");
    let snapshot_dir = snapshots_dir.join(&snapshot_id);

    // Create snapshot directory
    fs::create_dir_all(&snapshot_dir).map_err(|e| {
        SnapshotError::io_error(
            format!(
                "Failed to create snapshot directory: {}",
                snapshot_dir.display()
            ),
            e,
        )
    })?;

    // From here on, any error must clean up the snapshot directory
    let result = create_snapshot_contents(
        &snapshot_dir,
        storage_path,
        schema_dir,
        &snapshot_id,
        &created_at,
        Some(commit_boundary),
    );

    if result.is_err() {
        cleanup_snapshot(&snapshot_dir);
    }

    result
}

/// Compute checksums for all schema files.
fn compute_schema_checksums(schema_dir: &Path) -> SnapshotResult<HashMap<String, String>> {
    let mut checksums = HashMap::new();

    if !schema_dir.exists() {
        return Ok(checksums);
    }

    let entries = fs::read_dir(schema_dir).map_err(|e| {
        SnapshotError::io_error(
            format!("Failed to read schema directory: {}", schema_dir.display()),
            e,
        )
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            SnapshotError::io_error(
                format!("Failed to read schema entry in: {}", schema_dir.display()),
                e,
            )
        })?;

        let path = entry.path();
        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                let checksum = compute_file_checksum(&path)?;
                checksums.insert(filename.to_string(), format_checksum(checksum));
            }
        }
    }

    Ok(checksums)
}

/// Get the path to the snapshots directory.
pub fn snapshots_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("snapshots")
}

/// Get the path to a specific snapshot directory.
pub fn snapshot_path(data_dir: &Path, snapshot_id: &str) -> PathBuf {
    snapshots_dir(data_dir).join(snapshot_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn setup_test_environment() -> (TempDir, PathBuf, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        // Create storage.dat
        let storage_path = data_dir.join("storage.dat");
        let mut storage_file = File::create(&storage_path).unwrap();
        storage_file.write_all(b"test storage data").unwrap();
        storage_file.sync_all().unwrap();

        // Create schema directory with files
        let schema_dir = data_dir.join("metadata").join("schemas");
        fs::create_dir_all(&schema_dir).unwrap();

        let schema_path = schema_dir.join("user_v1.json");
        let mut schema_file = File::create(&schema_path).unwrap();
        schema_file
            .write_all(br#"{"name": "user", "version": 1}"#)
            .unwrap();
        schema_file.sync_all().unwrap();

        (temp_dir, storage_path, schema_dir)
    }

    #[test]
    fn test_generate_snapshot_id_format() {
        let id = generate_snapshot_id();

        // Should be 16 characters: YYYYMMDDTHHMMSSZ
        assert_eq!(id.len(), 16);
        assert!(id.ends_with('Z'));
        assert!(id.contains('T'));

        // Verify it's a valid format with digits
        let parts: Vec<&str> = id.split('T').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].len(), 8); // YYYYMMDD
        assert_eq!(parts[1].len(), 7); // HHMMSSZ
    }

    #[test]
    fn test_snapshot_creation_creates_directory() {
        let (temp_dir, storage_path, schema_dir) = setup_test_environment();
        let data_dir = temp_dir.path();

        let snapshot_id = create_snapshot_impl(data_dir, &storage_path, &schema_dir).unwrap();

        let snapshot_dir = data_dir.join("snapshots").join(&snapshot_id);
        assert!(snapshot_dir.exists());
        assert!(snapshot_dir.is_dir());
    }

    #[test]
    fn test_snapshot_contains_required_files() {
        let (temp_dir, storage_path, schema_dir) = setup_test_environment();
        let data_dir = temp_dir.path();

        let snapshot_id = create_snapshot_impl(data_dir, &storage_path, &schema_dir).unwrap();

        let snapshot_dir = data_dir.join("snapshots").join(&snapshot_id);

        // Verify required files exist
        assert!(snapshot_dir.join("storage.dat").exists());
        assert!(snapshot_dir.join("schemas").exists());
        assert!(snapshot_dir.join("manifest.json").exists());
    }

    #[test]
    fn test_manifest_correct_format() {
        let (temp_dir, storage_path, schema_dir) = setup_test_environment();
        let data_dir = temp_dir.path();

        let snapshot_id = create_snapshot_impl(data_dir, &storage_path, &schema_dir).unwrap();

        let manifest_path = data_dir
            .join("snapshots")
            .join(&snapshot_id)
            .join("manifest.json");
        let manifest = SnapshotManifest::read_from_file(&manifest_path).unwrap();

        assert_eq!(manifest.snapshot_id, snapshot_id);
        assert_eq!(manifest.format_version, 1);
        assert!(manifest.storage_checksum.starts_with("crc32:"));
        assert!(!manifest.created_at.is_empty());
    }

    #[test]
    fn test_checksums_deterministic() {
        let (temp_dir, storage_path, schema_dir) = setup_test_environment();
        let data_dir = temp_dir.path();

        // Create snapshot
        let snapshot_id = create_snapshot_impl(data_dir, &storage_path, &schema_dir).unwrap();

        // Read checksums
        let manifest_path = data_dir
            .join("snapshots")
            .join(&snapshot_id)
            .join("manifest.json");
        let manifest = SnapshotManifest::read_from_file(&manifest_path).unwrap();

        // Compute checksums of copied files
        let snapshot_dir = data_dir.join("snapshots").join(&snapshot_id);
        let storage_checksum = compute_file_checksum(&snapshot_dir.join("storage.dat")).unwrap();

        assert_eq!(manifest.storage_checksum, format_checksum(storage_checksum));
    }

    #[test]
    fn test_schemas_copied() {
        let (temp_dir, storage_path, schema_dir) = setup_test_environment();
        let data_dir = temp_dir.path();

        let snapshot_id = create_snapshot_impl(data_dir, &storage_path, &schema_dir).unwrap();

        // Verify schema file copied
        let snapshot_schemas = data_dir
            .join("snapshots")
            .join(&snapshot_id)
            .join("schemas");
        assert!(snapshot_schemas.join("user_v1.json").exists());

        // Verify manifest has schema checksum
        let manifest_path = data_dir
            .join("snapshots")
            .join(&snapshot_id)
            .join("manifest.json");
        let manifest = SnapshotManifest::read_from_file(&manifest_path).unwrap();
        assert!(manifest.schema_checksums.contains_key("user_v1.json"));
    }

    #[test]
    fn test_storage_byte_for_byte_copy() {
        let (temp_dir, storage_path, schema_dir) = setup_test_environment();
        let data_dir = temp_dir.path();

        let snapshot_id = create_snapshot_impl(data_dir, &storage_path, &schema_dir).unwrap();

        // Compare original and copied storage
        let original = fs::read(&storage_path).unwrap();
        let copied = fs::read(
            data_dir
                .join("snapshots")
                .join(&snapshot_id)
                .join("storage.dat"),
        )
        .unwrap();

        assert_eq!(original, copied);
    }

    #[test]
    fn test_cleanup_on_missing_storage() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path();

        // Non-existent storage path
        let storage_path = data_dir.join("nonexistent.dat");
        let schema_dir = data_dir.join("schemas");
        fs::create_dir_all(&schema_dir).unwrap();

        let result = create_snapshot_impl(data_dir, &storage_path, &schema_dir);

        // Should fail
        assert!(result.is_err());

        // Snapshots directory may or may not exist, but no partial snapshots
        let snapshots_dir = data_dir.join("snapshots");
        if snapshots_dir.exists() {
            let entries: Vec<_> = fs::read_dir(&snapshots_dir).unwrap().collect();
            assert!(entries.is_empty(), "Partial snapshot should be cleaned up");
        }
    }

    #[test]
    fn test_empty_schema_directory() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path();

        // Create storage.dat
        let storage_path = data_dir.join("storage.dat");
        fs::write(&storage_path, b"test data").unwrap();

        // Empty schema directory
        let schema_dir = data_dir.join("schemas");
        fs::create_dir_all(&schema_dir).unwrap();

        let snapshot_id = create_snapshot_impl(data_dir, &storage_path, &schema_dir).unwrap();

        // Verify manifest has empty schema checksums
        let manifest_path = data_dir
            .join("snapshots")
            .join(&snapshot_id)
            .join("manifest.json");
        let manifest = SnapshotManifest::read_from_file(&manifest_path).unwrap();
        assert!(manifest.schema_checksums.is_empty());
    }

    #[test]
    fn test_nonexistent_schema_directory() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path();

        // Create storage.dat
        let storage_path = data_dir.join("storage.dat");
        fs::write(&storage_path, b"test data").unwrap();

        // Non-existent schema directory
        let schema_dir = data_dir.join("nonexistent_schemas");

        let snapshot_id = create_snapshot_impl(data_dir, &storage_path, &schema_dir).unwrap();

        // Should succeed with empty schemas
        let manifest_path = data_dir
            .join("snapshots")
            .join(&snapshot_id)
            .join("manifest.json");
        let manifest = SnapshotManifest::read_from_file(&manifest_path).unwrap();
        assert!(manifest.schema_checksums.is_empty());
    }
}
