//! Snapshot crash test scenarios
//!
//! Per CRASH_TESTING.md:
//! - Crash during snapshot → snapshot ignored

use std::fs::{self, File};
use std::io::Write;

use crate::crash::utils::{
    cleanup_temp_data_dir, create_temp_data_dir, validate_snapshot_integrity,
};
use aerodb::crash_point::points;

/// Test: Incomplete snapshot without manifest is ignored
#[test]
fn test_snapshot_incomplete_ignored() {
    let data_dir = create_temp_data_dir("snapshot_incomplete");

    // Create incomplete snapshot (no manifest)
    let snapshot_dir = data_dir.join("snapshots/20260204T120000Z");
    fs::create_dir_all(&snapshot_dir).unwrap();

    // Create storage.dat but no manifest
    let mut f = File::create(snapshot_dir.join("storage.dat")).unwrap();
    f.write_all(b"test data").unwrap();

    // Validate should pass (incomplete snapshots are ignored)
    let result = validate_snapshot_integrity(&data_dir);
    assert!(result.is_ok());

    cleanup_temp_data_dir(&data_dir);
}

/// Test: Complete snapshot with manifest is valid
#[test]
fn test_snapshot_complete_valid() {
    let data_dir = create_temp_data_dir("snapshot_complete");

    // Create complete snapshot
    let snapshot_dir = data_dir.join("snapshots/20260204T120000Z");
    fs::create_dir_all(&snapshot_dir).unwrap();

    // Create storage.dat
    let mut f = File::create(snapshot_dir.join("storage.dat")).unwrap();
    f.write_all(b"test data").unwrap();

    // Create valid manifest
    let mut f = File::create(snapshot_dir.join("manifest.json")).unwrap();
    f.write_all(br#"{"snapshot_id":"20260204T120000Z","created_at":"2026-02-04T12:00:00Z"}"#)
        .unwrap();

    // Validate
    let result = validate_snapshot_integrity(&data_dir);
    assert!(result.is_ok());

    cleanup_temp_data_dir(&data_dir);
}

/// Test: Snapshot crash points defined
#[test]
fn test_snapshot_crash_points_defined() {
    assert_eq!(points::SNAPSHOT_START, "snapshot_start");
    assert_eq!(
        points::SNAPSHOT_AFTER_STORAGE_COPY,
        "snapshot_after_storage_copy"
    );
    assert_eq!(points::SNAPSHOT_BEFORE_MANIFEST, "snapshot_before_manifest");
    assert_eq!(points::SNAPSHOT_AFTER_MANIFEST, "snapshot_after_manifest");
}
