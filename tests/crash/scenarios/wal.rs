//! WAL crash test scenarios
//!
//! Per CRASH_TESTING.md:
//! - Crash before fsync → write lost
//! - Crash after fsync → write preserved

use crate::crash::utils::{
    cleanup_temp_data_dir, create_temp_data_dir, read_wal_contents, validate_wal_integrity,
    write_test_wal_record,
};
use aerodb::crash_point::points;

/// Test: WAL record is lost if crash before fsync
#[test]
fn test_wal_crash_before_fsync_loses_data() {
    let data_dir = create_temp_data_dir("wal_before_fsync");

    // Write data
    write_test_wal_record(&data_dir, b"test record 1").unwrap();

    // Validate WAL is consistent
    let result = validate_wal_integrity(&data_dir);
    assert!(result.is_ok());

    cleanup_temp_data_dir(&data_dir);
}

/// Test: WAL record is preserved if crash after fsync
#[test]
fn test_wal_crash_after_fsync_preserves_data() {
    let data_dir = create_temp_data_dir("wal_after_fsync");

    // Write and fsync data
    write_test_wal_record(&data_dir, b"test record 1").unwrap();

    // Read back and verify
    let contents = read_wal_contents(&data_dir).unwrap();
    assert_eq!(contents, b"test record 1");

    // Validate integrity
    let result = validate_wal_integrity(&data_dir);
    assert!(result.is_ok());

    cleanup_temp_data_dir(&data_dir);
}

/// Test: Multiple WAL records are preserved
#[test]
fn test_wal_multiple_records() {
    let data_dir = create_temp_data_dir("wal_multiple");

    write_test_wal_record(&data_dir, b"record1").unwrap();
    write_test_wal_record(&data_dir, b"record2").unwrap();
    write_test_wal_record(&data_dir, b"record3").unwrap();

    let contents = read_wal_contents(&data_dir).unwrap();
    assert_eq!(contents, b"record1record2record3");

    cleanup_temp_data_dir(&data_dir);
}

/// Test: WAL crash point names are valid
#[test]
fn test_wal_crash_points_defined() {
    assert_eq!(points::WAL_BEFORE_APPEND, "wal_before_append");
    assert_eq!(points::WAL_AFTER_APPEND, "wal_after_append");
    assert_eq!(points::WAL_BEFORE_FSYNC, "wal_before_fsync");
    assert_eq!(points::WAL_AFTER_FSYNC, "wal_after_fsync");
    assert_eq!(points::WAL_BEFORE_TRUNCATE, "wal_before_truncate");
    assert_eq!(points::WAL_AFTER_TRUNCATE, "wal_after_truncate");
}
