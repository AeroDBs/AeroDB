//! Recovery Crash Safety Tests
//!
//! Tests for invariants:
//! - K2: Halt-on-corruption policy
//! - Crash-safe recovery
//! - Partial replay prevention
//!
//! Per WAL.md, any corruption detected during recovery halts immediately
//! with no partial replay and no repair attempts.

use aerodb::recovery::{RecoveryStorage, WalReplayer};
use aerodb::storage::StorageReader;
use aerodb::wal::{WalPayload, WalReader, WalWriter};
use std::fs;
use tempfile::TempDir;

// =============================================================================
// Test Utilities
// =============================================================================

fn create_test_payload(doc_id: &str) -> WalPayload {
    WalPayload::new(
        "test_collection",
        doc_id,
        "test_schema",
        "v1",
        format!(r#"{{"id": "{}"}}"#, doc_id).into_bytes(),
    )
}

fn create_temp_data_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp dir")
}

// =============================================================================
// INVARIANT K2: Halt-on-Corruption Policy
// =============================================================================

/// K2: WAL corruption halts recovery immediately.
#[test]
fn test_k2_wal_corruption_halts_recovery() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();
    let wal_path = data_dir.join("wal/wal.log");

    // Write valid records
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        for i in 1..=5 {
            writer
                .append_insert(create_test_payload(&format!("doc{}", i)))
                .unwrap();
        }
    }

    // Corrupt WAL
    {
        let mut contents = fs::read(&wal_path).unwrap();
        let mid = contents.len() / 2;
        contents[mid] ^= 0xFF;
        fs::write(&wal_path, contents).unwrap();
    }

    // Recovery must fail
    let result = {
        let mut wal = WalReader::open_from_data_dir(data_dir).unwrap();
        let mut storage = RecoveryStorage::open(data_dir).unwrap();
        WalReplayer::replay(&mut wal, &mut storage)
    };

    assert!(
        result.is_err(),
        "K2 VIOLATION: Corrupted WAL must halt recovery"
    );
}

/// K2: Truncated WAL halts recovery.
#[test]
fn test_k2_truncated_wal_halts_recovery() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();
    let wal_path = data_dir.join("wal/wal.log");

    // Write a record
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        writer.append_insert(create_test_payload("doc1")).unwrap();
    }

    // Truncate WAL mid-record
    {
        let contents = fs::read(&wal_path).unwrap();
        fs::write(&wal_path, &contents[..contents.len() - 10]).unwrap();
    }

    // Recovery must fail
    let result = {
        let mut wal = WalReader::open_from_data_dir(data_dir).unwrap();
        let mut storage = RecoveryStorage::open(data_dir).unwrap();
        WalReplayer::replay(&mut wal, &mut storage)
    };

    assert!(result.is_err(), "K2: Truncated WAL must halt recovery");
}

// =============================================================================
// Partial Replay Prevention Tests
// =============================================================================

/// Corruption in middle prevents full replay.
#[test]
fn test_corruption_prevents_later_records() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();
    let wal_path = data_dir.join("wal/wal.log");

    // Write 5 records
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        for i in 1..=5 {
            writer
                .append_insert(create_test_payload(&format!("doc{}", i)))
                .unwrap();
        }
    }

    // Corrupt somewhere in the middle
    {
        let mut contents = fs::read(&wal_path).unwrap();
        let corrupt_pos = contents.len() / 2;
        contents[corrupt_pos] ^= 0xFF;
        fs::write(&wal_path, contents).unwrap();
    }

    // Recovery must fail - no partial replay
    let result = {
        let mut wal = WalReader::open_from_data_dir(data_dir).unwrap();
        let mut storage = RecoveryStorage::open(data_dir).unwrap();
        WalReplayer::replay(&mut wal, &mut storage)
    };

    assert!(result.is_err(), "Corruption must halt, no partial replay");
}

// =============================================================================
// Recovery Error Handling Tests
// =============================================================================

/// Missing WAL directory is handled gracefully.
#[test]
fn test_missing_wal_handled() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // Don't create WAL - try to open reader
    let result = WalReader::open_from_data_dir(data_dir);

    // Should fail with appropriate error
    assert!(result.is_err(), "Missing WAL should cause error");
}

/// Empty WAL is valid and produces empty replay.
#[test]
fn test_empty_wal_valid_recovery() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // Create empty WAL
    {
        let _writer = WalWriter::open(data_dir).unwrap();
    }

    // Empty replay should succeed
    let result = {
        let mut wal = WalReader::open_from_data_dir(data_dir).unwrap();
        let mut storage = RecoveryStorage::open(data_dir).unwrap();
        WalReplayer::replay(&mut wal, &mut storage)
    };

    assert!(result.is_ok(), "Empty WAL is valid");
    assert_eq!(result.unwrap().records_replayed, 0);
}

// =============================================================================
// Recovery Sequence Tests
// =============================================================================

/// Recovery processes records in correct order.
#[test]
fn test_recovery_processes_in_order() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // Write records in specific order
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        writer.append_insert(create_test_payload("first")).unwrap();
        writer.append_insert(create_test_payload("second")).unwrap();
        writer.append_insert(create_test_payload("third")).unwrap();
    }

    // Replay should process all in order
    let stats = {
        let mut wal = WalReader::open_from_data_dir(data_dir).unwrap();
        let mut storage = RecoveryStorage::open(data_dir).unwrap();
        WalReplayer::replay(&mut wal, &mut storage).unwrap()
    };

    assert_eq!(stats.records_replayed, 3);
    assert_eq!(stats.final_sequence, 3);

    // Verify storage has all records
    let mut reader = StorageReader::open_from_data_dir(data_dir).unwrap();
    let records = reader.read_all().unwrap();

    assert_eq!(records.len(), 3, "All records should be in storage");
}
