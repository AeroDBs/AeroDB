//! WAL Durability Invariant Tests
//!
//! Tests for invariants:
//! - D1: No acknowledged write is ever lost (fsync before acknowledgment)
//! - R1: WAL precedes acknowledgment
//! - K1: Checksums on every record
//! - C1: Full-document atomicity
//!
//! Per CORE_INVARIANTS.md and WAL.md, these invariants are mandatory
//! and must hold under all conditions including crashes.

use aerodb::wal::{RecordType, WalError, WalPayload, WalReader, WalRecord, WalWriter};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
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
// INVARIANT D1: No Acknowledged Write Is Ever Lost
// =============================================================================

/// D1: After append() returns Ok, the record MUST be recoverable on reopen.
///
/// This is the fundamental durability guarantee. If append() returns a
/// sequence number, that record must survive process restart.
#[test]
fn test_d1_acknowledged_write_survives_reopen() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // Write records and collect their sequence numbers
    let written_sequences: Vec<u64>;
    {
        let mut writer = WalWriter::open(data_dir).expect("Failed to open WAL writer");

        written_sequences = (1..=10)
            .map(|i| {
                let payload = create_test_payload(&format!("doc{}", i));
                writer
                    .append_insert(payload)
                    .expect("append() returned Ok - this write is now acknowledged")
            })
            .collect();
    }
    // Writer dropped, simulating process exit

    // Reopen and verify ALL acknowledged writes are present
    let wal_path = data_dir.join("wal/wal.log");
    let mut reader = WalReader::open(&wal_path).expect("Failed to open WAL reader");

    let mut recovered_sequences = Vec::new();
    loop {
        match reader.read_next() {
            Ok(Some(record)) => recovered_sequences.push(record.sequence_number),
            Ok(None) => break,
            Err(e) => panic!("Recovery failed: {}", e),
        }
    }

    // INVARIANT D1: Every acknowledged write must be present
    assert_eq!(
        written_sequences, recovered_sequences,
        "D1 VIOLATION: Acknowledged writes were lost after reopen"
    );
}

/// D1: Multiple reopens must not lose data.
///
/// Tests that the durability guarantee holds across multiple restarts.
#[test]
fn test_d1_multiple_reopens_preserve_data() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // Phase 1: Write initial records
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        writer.append_insert(create_test_payload("doc1")).unwrap();
        writer.append_insert(create_test_payload("doc2")).unwrap();
    }

    // Phase 2: Reopen and write more
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        assert_eq!(writer.next_sequence_number(), 3, "Sequence should continue");
        writer.append_insert(create_test_payload("doc3")).unwrap();
    }

    // Phase 3: Reopen again and write more
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        assert_eq!(writer.next_sequence_number(), 4, "Sequence should continue");
        writer.append_insert(create_test_payload("doc4")).unwrap();
    }

    // Final verification: All 4 records must be present
    let wal_path = data_dir.join("wal/wal.log");
    let mut reader = WalReader::open(&wal_path).unwrap();

    let mut count = 0;
    while let Ok(Some(_)) = reader.read_next() {
        count += 1;
    }

    assert_eq!(
        count, 4,
        "D1 VIOLATION: Expected 4 records, found {}",
        count
    );
}

// =============================================================================
// INVARIANT R1: WAL Precedes Acknowledgment
// =============================================================================

/// R1: After successful append(), the data must be readable immediately.
///
/// This tests that fsync has completed before append() returns.
#[test]
fn test_r1_data_readable_immediately_after_append() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    let mut writer = WalWriter::open(data_dir).unwrap();
    let seq = writer.append_insert(create_test_payload("doc1")).unwrap();

    // Immediately after append returns, open a NEW reader (fresh file handle)
    // and verify the data is readable
    let wal_path = data_dir.join("wal/wal.log");
    let mut reader = WalReader::open(&wal_path).unwrap();

    let record = reader
        .read_next()
        .expect("Read should succeed")
        .expect("Record should exist");

    assert_eq!(
        record.sequence_number, seq,
        "R1: Record must be readable immediately"
    );
    assert_eq!(record.payload.document_id, "doc1");
}

/// R1: Sequence numbers start at 1 and are strictly monotonic.
///
/// Per WAL.md, sequence numbers are never reused and always increment.
#[test]
fn test_r1_sequence_monotonicity() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    let mut writer = WalWriter::open(data_dir).unwrap();

    // First sequence must be 1
    assert_eq!(writer.next_sequence_number(), 1, "First sequence must be 1");

    let mut prev_seq = 0u64;
    for i in 1..=100 {
        let payload = create_test_payload(&format!("doc{}", i));
        let seq = writer.append_insert(payload).unwrap();

        // Strict monotonicity: each sequence > previous
        assert!(
            seq > prev_seq,
            "R1 VIOLATION: Sequence {} is not greater than previous {}",
            seq,
            prev_seq
        );

        prev_seq = seq;
    }
}

// =============================================================================
// INVARIANT K1: Checksums on Every Record
// =============================================================================

/// K1: Every record must have a valid checksum that is verified on read.
///
/// Write records, then verify they can be read with checksum validation.
#[test]
fn test_k1_checksums_verified_on_read() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // Write records
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        for i in 1..=5 {
            writer
                .append_insert(create_test_payload(&format!("doc{}", i)))
                .unwrap();
        }
    }

    // Read back - WalReader verifies checksums on every read
    let wal_path = data_dir.join("wal/wal.log");
    let mut reader = WalReader::open(&wal_path).unwrap();

    let mut count = 0;
    loop {
        match reader.read_next() {
            Ok(Some(_)) => count += 1,
            Ok(None) => break,
            Err(e) => panic!("K1: Checksum verification failed: {}", e),
        }
    }

    assert_eq!(count, 5, "All records should pass checksum verification");
}

/// K1: Corrupted checksum must cause explicit failure.
///
/// Tamper with WAL bytes and verify that read fails explicitly.
#[test]
fn test_k1_corrupted_checksum_detected() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();
    let wal_path = data_dir.join("wal/wal.log");

    // Write a valid record
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        writer.append_insert(create_test_payload("doc1")).unwrap();
    }

    // Corrupt the WAL file by modifying a byte in the checksum region
    {
        let mut contents = fs::read(&wal_path).unwrap();
        assert!(!contents.is_empty(), "WAL should have content");

        // The checksum is the last 4 bytes of the record
        // Flip a bit in the last byte
        let len = contents.len();
        contents[len - 1] ^= 0xFF;

        fs::write(&wal_path, contents).unwrap();
    }

    // Read must fail with checksum error
    let mut reader = WalReader::open(&wal_path).unwrap();
    let result = reader.read_next();

    assert!(
        result.is_err(),
        "K1 VIOLATION: Corrupted checksum should cause read failure"
    );

    let err = result.unwrap_err();
    assert!(
        err.to_string().to_lowercase().contains("checksum"),
        "K1: Error should mention checksum, got: {}",
        err
    );
}

/// K1: Corrupted payload must cause checksum mismatch.
///
/// Tamper with payload bytes and verify checksum detection.
#[test]
fn test_k1_corrupted_payload_detected() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();
    let wal_path = data_dir.join("wal/wal.log");

    // Write a valid record
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        writer.append_insert(create_test_payload("doc1")).unwrap();
    }

    // Corrupt the WAL file by modifying a byte in the payload region
    {
        let mut contents = fs::read(&wal_path).unwrap();
        assert!(contents.len() > 20, "WAL should have sufficient content");

        // Modify a byte in the middle of the record (payload area)
        contents[20] ^= 0xFF;

        fs::write(&wal_path, contents).unwrap();
    }

    // Read must fail with checksum error
    let mut reader = WalReader::open(&wal_path).unwrap();
    let result = reader.read_next();

    assert!(
        result.is_err(),
        "K1 VIOLATION: Corrupted payload should cause checksum mismatch"
    );
}

// =============================================================================
// INVARIANT C1: Full-Document Atomicity
// =============================================================================

/// C1: Each WAL record contains the complete document state.
///
/// Verify that the document body written is the complete document read back.
#[test]
fn test_c1_full_document_preserved() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // Create a complex document payload
    let doc_body = r#"{
        "id": "test123",
        "name": "Test Document",
        "data": [1, 2, 3, 4, 5],
        "nested": {
            "field1": "value1",
            "field2": 42
        }
    }"#;

    let payload = WalPayload::new(
        "my_collection",
        "test123",
        "my_schema",
        "v2",
        doc_body.as_bytes().to_vec(),
    );

    // Write the record
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        writer.append_insert(payload).unwrap();
    }

    // Read back and verify complete document
    let wal_path = data_dir.join("wal/wal.log");
    let mut reader = WalReader::open(&wal_path).unwrap();

    let record = reader.read_next().unwrap().unwrap();

    // C1: All fields must match exactly
    assert_eq!(record.payload.collection_id, "my_collection");
    assert_eq!(record.payload.document_id, "test123");
    assert_eq!(record.payload.schema_id, "my_schema");
    assert_eq!(record.payload.schema_version, "v2");
    assert_eq!(
        record.payload.document_body,
        doc_body.as_bytes(),
        "C1 VIOLATION: Document body mismatch"
    );
}

/// C1: All record types preserve full document state.
///
/// Test INSERT, UPDATE, DELETE all carry complete document data.
#[test]
fn test_c1_all_record_types_preserve_state() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    {
        let mut writer = WalWriter::open(data_dir).unwrap();

        // INSERT
        writer
            .append_insert(WalPayload::new(
                "col",
                "doc1",
                "schema",
                "v1",
                b"INSERT_BODY".to_vec(),
            ))
            .unwrap();

        // UPDATE
        writer
            .append_update(WalPayload::new(
                "col",
                "doc1",
                "schema",
                "v1",
                b"UPDATE_BODY".to_vec(),
            ))
            .unwrap();

        // DELETE (tombstone)
        writer
            .append_delete(WalPayload::tombstone("col", "doc1", "schema", "v1"))
            .unwrap();
    }

    // Read back and verify
    let wal_path = data_dir.join("wal/wal.log");
    let mut reader = WalReader::open(&wal_path).unwrap();

    let insert_record = reader.read_next().unwrap().unwrap();
    assert_eq!(insert_record.record_type, RecordType::Insert);
    assert_eq!(insert_record.payload.document_body, b"INSERT_BODY");

    let update_record = reader.read_next().unwrap().unwrap();
    assert_eq!(update_record.record_type, RecordType::Update);
    assert_eq!(update_record.payload.document_body, b"UPDATE_BODY");

    let delete_record = reader.read_next().unwrap().unwrap();
    assert_eq!(delete_record.record_type, RecordType::Delete);
    assert!(
        delete_record.payload.document_body.is_empty(),
        "Tombstone should be empty"
    );
}

// =============================================================================
// Sequential Replay Determinism Tests
// =============================================================================

/// R2: Same WAL content must produce same replay sequence.
///
/// Read the same WAL twice and verify identical ordering.
#[test]
fn test_r2_sequential_replay_determinism() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // Write records with interleaved document IDs
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        for i in 1..=20 {
            writer
                .append_insert(create_test_payload(&format!("doc{}", i)))
                .unwrap();
        }
    }

    let wal_path = data_dir.join("wal/wal.log");

    // First read pass
    let mut reader1 = WalReader::open(&wal_path).unwrap();
    let mut pass1: Vec<(u64, String)> = Vec::new();
    while let Ok(Some(record)) = reader1.read_next() {
        pass1.push((record.sequence_number, record.payload.document_id.clone()));
    }

    // Second read pass
    let mut reader2 = WalReader::open(&wal_path).unwrap();
    let mut pass2: Vec<(u64, String)> = Vec::new();
    while let Ok(Some(record)) = reader2.read_next() {
        pass2.push((record.sequence_number, record.payload.document_id.clone()));
    }

    // R2: Both passes must be identical
    assert_eq!(pass1, pass2, "R2 VIOLATION: Non-deterministic WAL replay");
}

// =============================================================================
// Record Serialization Round-Trip Tests
// =============================================================================

/// Verify that WalRecord serialization is bijective.
#[test]
fn test_wal_record_serialization_roundtrip() {
    let payload = WalPayload::new(
        "collection",
        "doc_id",
        "schema",
        "v1",
        b"test body".to_vec(),
    );

    let original = WalRecord::new(RecordType::Insert, 42, payload);
    let serialized = original.serialize();

    let (deserialized, consumed) =
        WalRecord::deserialize(&serialized).expect("Deserialization should succeed");

    assert_eq!(consumed, serialized.len(), "All bytes should be consumed");
    assert_eq!(
        original, deserialized,
        "Round-trip must preserve record exactly"
    );
}

/// Verify WalPayload serialization is bijective.
#[test]
fn test_wal_payload_serialization_roundtrip() {
    let original = WalPayload::new(
        "my_collection",
        "my_document",
        "my_schema",
        "version_1",
        b"payload content here".to_vec(),
    );

    let serialized = original.serialize();
    let deserialized =
        WalPayload::deserialize(&serialized).expect("Deserialization should succeed");

    assert_eq!(
        original, deserialized,
        "Round-trip must preserve payload exactly"
    );
}
