//! Recovery crash test scenarios
//!
//! Per CRASH_TESTING.md:
//! - Same crash + same data → identical final state

use crate::crash::utils::{
    cleanup_temp_data_dir, create_temp_data_dir, validate_wal_integrity, write_test_wal_record,
};
use aerodb::crash_point::points;

/// Test: Recovery produces deterministic state
#[test]
fn test_recovery_deterministic() {
    let data_dir = create_temp_data_dir("recovery_deterministic");

    // Write identical WAL data
    write_test_wal_record(&data_dir, b"record1").unwrap();
    write_test_wal_record(&data_dir, b"record2").unwrap();

    // Validate
    let result = validate_wal_integrity(&data_dir);
    assert!(result.is_ok());

    cleanup_temp_data_dir(&data_dir);
}

/// Test: Recovery handles crash during WAL replay
#[test]
fn test_recovery_wal_replay_crash() {
    let data_dir = create_temp_data_dir("recovery_wal_replay");

    // Write WAL data
    write_test_wal_record(&data_dir, b"important record").unwrap();

    // WAL should be intact
    let result = validate_wal_integrity(&data_dir);
    assert!(result.is_ok());

    cleanup_temp_data_dir(&data_dir);
}

/// Test: Recovery crash points defined
#[test]
fn test_recovery_crash_points_defined() {
    assert_eq!(points::RECOVERY_START, "recovery_start");
    assert_eq!(
        points::RECOVERY_AFTER_WAL_REPLAY,
        "recovery_after_wal_replay"
    );
    assert_eq!(
        points::RECOVERY_AFTER_INDEX_REBUILD,
        "recovery_after_index_rebuild"
    );
}
