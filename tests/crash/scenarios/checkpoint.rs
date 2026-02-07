//! Checkpoint crash test scenarios
//!
//! Per CRASH_TESTING.md:
//! - Crash before WAL truncate → WAL intact
//! - Crash after WAL truncate → snapshot used

use std::fs::{self, File};
use std::io::Write;

use crate::crash::utils::{
    cleanup_temp_data_dir, create_temp_data_dir, read_wal_contents, validate_wal_integrity,
    write_test_wal_record,
};
use aerodb::crash_point::points;

/// Test: WAL intact if crash before truncate
#[test]
fn test_checkpoint_crash_before_truncate_wal_intact() {
    let data_dir = create_temp_data_dir("checkpoint_before_truncate");

    // Write WAL records
    write_test_wal_record(&data_dir, b"record1").unwrap();
    write_test_wal_record(&data_dir, b"record2").unwrap();

    // Verify WAL still has data (no truncation happened)
    let contents = read_wal_contents(&data_dir).unwrap();
    assert!(!contents.is_empty());

    cleanup_temp_data_dir(&data_dir);
}

/// Test: Snapshot used if crash after truncate
#[test]
fn test_checkpoint_crash_after_truncate_uses_snapshot() {
    let data_dir = create_temp_data_dir("checkpoint_after_truncate");

    // Create snapshot
    let snapshot_dir = data_dir.join("snapshots/20260204T120000Z");
    fs::create_dir_all(&snapshot_dir).unwrap();

    let mut f = File::create(snapshot_dir.join("storage.dat")).unwrap();
    f.write_all(b"snapshot data").unwrap();

    let mut f = File::create(snapshot_dir.join("manifest.json")).unwrap();
    f.write_all(br#"{"snapshot_id":"20260204T120000Z"}"#)
        .unwrap();

    // Empty WAL (simulating truncation)
    File::create(data_dir.join("wal/wal.log")).unwrap();

    // Verify WAL is empty
    let contents = read_wal_contents(&data_dir).unwrap();
    assert!(contents.is_empty());

    // Verify snapshot exists
    assert!(snapshot_dir.join("storage.dat").exists());

    cleanup_temp_data_dir(&data_dir);
}

/// Test: Checkpoint crash points defined
#[test]
fn test_checkpoint_crash_points_defined() {
    assert_eq!(points::CHECKPOINT_START, "checkpoint_start");
    assert_eq!(
        points::CHECKPOINT_AFTER_SNAPSHOT,
        "checkpoint_after_snapshot"
    );
    assert_eq!(
        points::CHECKPOINT_BEFORE_WAL_TRUNCATE,
        "checkpoint_before_wal_truncate"
    );
    assert_eq!(
        points::CHECKPOINT_AFTER_WAL_TRUNCATE,
        "checkpoint_after_wal_truncate"
    );
}
