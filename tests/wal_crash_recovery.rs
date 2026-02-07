//! WAL Crash Recovery Invariant Tests
//!
//! Tests for invariants:
//! - R2: Recovery is deterministic (same WAL → same state)
//! - R3: Recovery completeness is verifiable
//! - K2: Halt-on-corruption policy
//!
//! Per WAL.md §216-229, any corruption detected during recovery
//! must halt immediately with no partial replay and no repair attempts.

use aerodb::wal::{RecordType, WalPayload, WalReader, WalWriter};
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
// INVARIANT R2: Recovery Is Deterministic
// =============================================================================

/// R2: Given identical WAL content, recovery produces identical state.
///
/// This tests that WAL replay is a pure function of the WAL bytes.
#[test]
fn test_r2_recovery_determinism_same_wal_same_result() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // Write N records
    {
        let mut writer = WalWriter::open(data_dir).expect("Failed to open WAL");
        for i in 1..=20 {
            writer
                .append_insert(create_test_payload(&format!("doc{}", i)))
                .unwrap();
        }
    }

    let wal_path = data_dir.join("wal/wal.log");

    // First recovery pass
    let pass1_records: Vec<_> = {
        let mut reader = WalReader::open(&wal_path).unwrap();
        reader.read_all().unwrap()
    };

    // Second recovery pass
    let pass2_records: Vec<_> = {
        let mut reader = WalReader::open(&wal_path).unwrap();
        reader.read_all().unwrap()
    };

    // Third recovery pass
    let pass3_records: Vec<_> = {
        let mut reader = WalReader::open(&wal_path).unwrap();
        reader.read_all().unwrap()
    };

    // R2: All passes must produce identical results
    assert_eq!(pass1_records.len(), pass2_records.len());
    assert_eq!(pass2_records.len(), pass3_records.len());

    for i in 0..pass1_records.len() {
        assert_eq!(
            pass1_records[i].sequence_number, pass2_records[i].sequence_number,
            "R2 VIOLATION: Sequence mismatch at index {}",
            i
        );
        assert_eq!(
            pass2_records[i].sequence_number, pass3_records[i].sequence_number,
            "R2 VIOLATION: Sequence mismatch at index {}",
            i
        );
        assert_eq!(
            pass1_records[i].payload, pass2_records[i].payload,
            "R2 VIOLATION: Payload mismatch at index {}",
            i
        );
    }
}

/// R2: Recovery order matches write order exactly.
///
/// WAL replay must restore records in the exact order they were written.
#[test]
fn test_r2_recovery_preserves_write_order() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // Write records with specific ordering
    let expected_order: Vec<String> = (1..=10).map(|i| format!("doc{}", i)).collect();

    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        for doc_id in &expected_order {
            writer.append_insert(create_test_payload(doc_id)).unwrap();
        }
    }

    // Recover and verify order
    let wal_path = data_dir.join("wal/wal.log");
    let mut reader = WalReader::open(&wal_path).unwrap();
    let records = reader.read_all().unwrap();

    let recovered_order: Vec<String> = records
        .iter()
        .map(|r| r.payload.document_id.clone())
        .collect();

    assert_eq!(
        expected_order, recovered_order,
        "R2 VIOLATION: Recovery order differs from write order"
    );
}

/// R2: Interleaved record types maintain order.
#[test]
fn test_r2_interleaved_record_types_maintain_order() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // Write interleaved INSERT, UPDATE, DELETE
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        writer.append_insert(create_test_payload("doc1")).unwrap(); // seq 1
        writer.append_update(create_test_payload("doc1")).unwrap(); // seq 2
        writer.append_insert(create_test_payload("doc2")).unwrap(); // seq 3
        writer
            .append_delete(WalPayload::tombstone("test", "doc1", "s", "v1"))
            .unwrap(); // seq 4
        writer.append_update(create_test_payload("doc2")).unwrap(); // seq 5
    }

    let wal_path = data_dir.join("wal/wal.log");
    let mut reader = WalReader::open(&wal_path).unwrap();
    let records = reader.read_all().unwrap();

    // Verify exact sequence and type order
    assert_eq!(records[0].sequence_number, 1);
    assert_eq!(records[0].record_type, RecordType::Insert);

    assert_eq!(records[1].sequence_number, 2);
    assert_eq!(records[1].record_type, RecordType::Update);

    assert_eq!(records[2].sequence_number, 3);
    assert_eq!(records[2].record_type, RecordType::Insert);

    assert_eq!(records[3].sequence_number, 4);
    assert_eq!(records[3].record_type, RecordType::Delete);

    assert_eq!(records[4].sequence_number, 5);
    assert_eq!(records[4].record_type, RecordType::Update);
}

// =============================================================================
// INVARIANT R3: Recovery Completeness Is Verifiable
// =============================================================================

/// R3: After recovery, we must know how many records were replayed.
#[test]
fn test_r3_recovery_count_is_verifiable() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    let expected_count = 15;

    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        for i in 1..=expected_count {
            writer
                .append_insert(create_test_payload(&format!("doc{}", i)))
                .unwrap();
        }
    }

    let wal_path = data_dir.join("wal/wal.log");
    let mut reader = WalReader::open(&wal_path).unwrap();
    let records = reader.read_all().unwrap();

    // R3: Count must be deterministic and verifiable
    assert_eq!(
        records.len(),
        expected_count,
        "R3: Recovery count must be verifiable"
    );
}

/// R3: Empty WAL recovery is explicitly verifiable.
#[test]
fn test_r3_empty_wal_recovery_verifiable() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // Create empty WAL
    {
        let _writer = WalWriter::open(data_dir).unwrap();
    }

    let wal_path = data_dir.join("wal/wal.log");
    let mut reader = WalReader::open(&wal_path).unwrap();
    let records = reader.read_all().unwrap();

    // R3: Empty WAL produces empty result - not an error
    assert!(records.is_empty(), "Empty WAL must recover to empty state");
}

/// R3: Recovery reports last sequence number accurately.
#[test]
fn test_r3_last_sequence_number_accurate() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        for i in 1..=7 {
            writer
                .append_insert(create_test_payload(&format!("doc{}", i)))
                .unwrap();
        }
    }

    let wal_path = data_dir.join("wal/wal.log");
    let mut reader = WalReader::open(&wal_path).unwrap();

    // Read all records
    while let Ok(Some(_)) = reader.read_next() {}

    // R3: Last sequence number is accurately reported
    assert_eq!(
        reader.last_sequence_number(),
        7,
        "R3: Last sequence number must be accurately reported"
    );
}

// =============================================================================
// INVARIANT K2: Halt-on-Corruption Policy
// =============================================================================

/// K2: Corrupted record halts recovery immediately.
///
/// Per WAL.md §216-229: No partial replay, no skipping, no repair.
#[test]
fn test_k2_corruption_halts_immediately() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();
    let wal_path = data_dir.join("wal/wal.log");

    // Write multiple valid records
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        for i in 1..=5 {
            writer
                .append_insert(create_test_payload(&format!("doc{}", i)))
                .unwrap();
        }
    }

    // Corrupt a byte in the middle of the WAL (affects record 2 or 3)
    {
        let mut contents = fs::read(&wal_path).unwrap();
        // Corrupt byte somewhere in the middle (not the first record)
        let mid = contents.len() / 2;
        contents[mid] ^= 0xFF;
        fs::write(&wal_path, contents).unwrap();
    }

    // Recovery must halt with error
    let mut reader = WalReader::open(&wal_path).unwrap();
    let result = reader.read_all();

    assert!(
        result.is_err(),
        "K2 VIOLATION: Corruption must halt recovery, not skip records"
    );

    let err = result.unwrap_err();
    assert!(
        err.is_fatal(),
        "K2: Corruption errors must be marked as fatal"
    );
}

/// K2: Truncated record at EOF is detected.
#[test]
fn test_k2_truncated_record_detected() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();
    let wal_path = data_dir.join("wal/wal.log");

    // Write a record
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        writer.append_insert(create_test_payload("doc1")).unwrap();
    }

    // Truncate the file mid-record
    {
        let contents = fs::read(&wal_path).unwrap();
        // Remove last 10 bytes (partial checksum and body)
        fs::write(&wal_path, &contents[..contents.len() - 10]).unwrap();
    }

    // Recovery must fail
    let mut reader = WalReader::open(&wal_path).unwrap();
    let result = reader.read_next();

    assert!(
        result.is_err(),
        "K2 VIOLATION: Truncated record must cause halt"
    );
}

/// K2: Invalid record type causes halt.
#[test]
fn test_k2_invalid_record_type_halts() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();
    let wal_path = data_dir.join("wal/wal.log");

    // Write a valid record
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        writer.append_insert(create_test_payload("doc1")).unwrap();
    }

    // Corrupt the record type byte (byte 4 after length)
    {
        let mut contents = fs::read(&wal_path).unwrap();
        // Set record type to invalid value (e.g., 255)
        contents[4] = 255;
        fs::write(&wal_path, contents).unwrap();
    }

    // Recovery must fail (checksum mismatch due to corruption)
    let mut reader = WalReader::open(&wal_path).unwrap();
    let result = reader.read_next();

    assert!(
        result.is_err(),
        "K2: Invalid record type must halt recovery"
    );
}

/// K2: Non-sequential sequence numbers cause halt.
///
/// WAL must contain strictly sequential records (1, 2, 3, ...).
#[test]
fn test_k2_sequence_gap_detected() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();
    let wal_path = data_dir.join("wal/wal.log");

    // Write two valid records
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        writer.append_insert(create_test_payload("doc1")).unwrap();
        writer.append_insert(create_test_payload("doc2")).unwrap();
    }

    // Read the WAL to get record structure
    let contents = fs::read(&wal_path).unwrap();

    // Parse first record to find its length
    let first_record_len =
        u32::from_le_bytes([contents[0], contents[1], contents[2], contents[3]]) as usize;

    // The second record starts at first_record_len
    // Sequence number is at offset 5-12 within each record (after length + type)
    let seq_offset = first_record_len + 5;

    // Create a modified version with sequence gap (change seq 2 to seq 5)
    // Note: This will also corrupt the checksum, which will be caught
    let mut modified = contents.clone();
    if seq_offset + 8 <= modified.len() {
        modified[seq_offset..seq_offset + 8].copy_from_slice(&5u64.to_le_bytes());
    }
    fs::write(&wal_path, modified).unwrap();

    // Recovery must fail (either checksum or sequence validation)
    let mut reader = WalReader::open(&wal_path).unwrap();

    // Read first record (should succeed)
    let first = reader.read_next();
    assert!(first.is_ok(), "First record should be readable");

    // Second record should fail
    let second = reader.read_next();
    assert!(
        second.is_err(),
        "K2: Sequence gap must be detected (via checksum or sequence validation)"
    );
}

// =============================================================================
// Partial Write Simulation Tests
// =============================================================================

/// Simulate crash after partial record write.
///
/// A record that is only partially written should not be considered durable.
#[test]
fn test_partial_write_at_eof_not_durable() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();
    let wal_path = data_dir.join("wal/wal.log");

    // Write a complete record
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        writer.append_insert(create_test_payload("doc1")).unwrap();
    }

    // Append garbage bytes to simulate partial write
    {
        use std::io::Write;
        let mut file = fs::OpenOptions::new().append(true).open(&wal_path).unwrap();
        // Write a partial record header
        file.write_all(&[0x50, 0x00, 0x00, 0x00, 0x01]).unwrap(); // Length + type, incomplete
    }

    // Recovery should fail on the partial record
    let mut reader = WalReader::open(&wal_path).unwrap();

    // First record should succeed
    let first = reader.read_next().unwrap();
    assert!(first.is_some(), "First complete record should be readable");

    // Second "record" should fail
    let second = reader.read_next();
    assert!(second.is_err(), "Partial record at EOF must cause halt");
}

// =============================================================================
// Recovery Restart Tests
// =============================================================================

/// Recovery can be restarted after failure (idempotent).
#[test]
fn test_recovery_can_restart() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();
    let wal_path = data_dir.join("wal/wal.log");

    // Write valid records
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        for i in 1..=3 {
            writer
                .append_insert(create_test_payload(&format!("doc{}", i)))
                .unwrap();
        }
    }

    // First recovery attempt
    let records1: Vec<_> = {
        let mut reader = WalReader::open(&wal_path).unwrap();
        reader.read_all().unwrap()
    };

    // Simulate process restart - second recovery attempt
    let records2: Vec<_> = {
        let mut reader = WalReader::open(&wal_path).unwrap();
        reader.read_all().unwrap()
    };

    // Both should succeed with identical results
    assert_eq!(records1.len(), records2.len());
    assert_eq!(records1.len(), 3);
}

/// Reader reset allows re-reading (useful for verification passes).
#[test]
fn test_reader_reset_for_verification() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();
    let wal_path = data_dir.join("wal/wal.log");

    // Write records
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        writer.append_insert(create_test_payload("doc1")).unwrap();
        writer.append_insert(create_test_payload("doc2")).unwrap();
    }

    let mut reader = WalReader::open(&wal_path).unwrap();

    // First pass
    let pass1 = reader.read_all().unwrap();

    // Reset
    reader.reset().unwrap();

    // Second pass (verification)
    let pass2 = reader.read_all().unwrap();

    assert_eq!(pass1.len(), pass2.len());
    for (r1, r2) in pass1.iter().zip(pass2.iter()) {
        assert_eq!(r1.sequence_number, r2.sequence_number);
    }
}
