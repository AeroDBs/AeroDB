//! Backup crash test scenarios
//!
//! Per CRASH_TESTING.md:
//! - Crash during backup → no partial archive

use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::crash::utils::{cleanup_temp_data_dir, create_temp_data_dir, validate_backup_integrity};
use aerodb::crash_point::points;

/// Helper to create a valid backup archive
fn create_valid_backup(backup_path: &Path) {
    use tar::Builder;

    let file = File::create(backup_path).unwrap();
    let mut builder = Builder::new(file);

    // Create temp content
    let temp = std::env::temp_dir().join("backup_test_content");
    std::fs::create_dir_all(temp.join("snapshot")).unwrap();
    std::fs::create_dir_all(temp.join("wal")).unwrap();

    let mut f = File::create(temp.join("snapshot/manifest.json")).unwrap();
    f.write_all(br#"{"id":"test"}"#).unwrap();

    let mut f = File::create(temp.join("snapshot/storage.dat")).unwrap();
    f.write_all(b"test data").unwrap();

    let mut f = File::create(temp.join("backup_manifest.json")).unwrap();
    f.write_all(br#"{"backup_id":"test","snapshot_id":"test","format_version":1}"#)
        .unwrap();

    builder
        .append_dir_all("snapshot", temp.join("snapshot"))
        .unwrap();
    builder.append_dir_all("wal", temp.join("wal")).unwrap();

    let manifest_path = temp.join("backup_manifest.json");
    let mut manifest_file = File::open(&manifest_path).unwrap();
    builder
        .append_file("backup_manifest.json", &mut manifest_file)
        .unwrap();

    builder.finish().unwrap();

    // Cleanup temp
    let _ = std::fs::remove_dir_all(&temp);
}

/// Test: Complete backup is valid
#[test]
fn test_backup_complete_valid() {
    let data_dir = create_temp_data_dir("backup_complete");
    let backup_path = data_dir.join("backup.tar");

    create_valid_backup(&backup_path);

    let result = validate_backup_integrity(&backup_path);
    assert!(result.is_ok());

    cleanup_temp_data_dir(&data_dir);
}

/// Test: Missing backup is valid (no partial)
#[test]
fn test_backup_missing_valid() {
    let data_dir = create_temp_data_dir("backup_missing");
    let backup_path = data_dir.join("backup.tar");

    // No backup file created
    let result = validate_backup_integrity(&backup_path);
    assert!(result.is_ok()); // Missing is OK

    cleanup_temp_data_dir(&data_dir);
}

/// Test: Backup crash points defined
#[test]
fn test_backup_crash_points_defined() {
    assert_eq!(points::BACKUP_START, "backup_start");
    assert_eq!(
        points::BACKUP_AFTER_SNAPSHOT_COPY,
        "backup_after_snapshot_copy"
    );
    assert_eq!(points::BACKUP_AFTER_WAL_COPY, "backup_after_wal_copy");
    assert_eq!(points::BACKUP_BEFORE_ARCHIVE, "backup_before_archive");
}
