//! Storage Integrity Invariant Tests
//!
//! Tests for invariants:
//! - D2: Data corruption is never ignored
//! - D3: Reads never observe invalid state
//! - K1: Checksums on every record
//! - C1: Full-document writes
//!
//! Per STORAGE.md, storage is append-only, checksum-verified,
//! and WAL-driven (writes occur after WAL fsync).

use aerodb::storage::{DocumentRecord, StoragePayload, StorageReader, StorageWriter};
use aerodb::wal::{RecordType, WalPayload, WalRecord};
use std::fs;
use tempfile::TempDir;

// =============================================================================
// Test Utilities
// =============================================================================

fn create_test_payload(doc_id: &str) -> StoragePayload {
    StoragePayload::new(
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
// INVARIANT D2: Data Corruption Is Never Ignored
// =============================================================================

/// D2: Corrupted storage record must cause explicit failure.
#[test]
fn test_d2_corruption_causes_explicit_failure() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();
    let storage_path = data_dir.join("data/documents.dat");

    // Write a valid record
    {
        let mut writer = StorageWriter::open(data_dir).unwrap();
        writer.write(&create_test_payload("doc1")).unwrap();
    }

    // Corrupt the storage file
    {
        let mut contents = fs::read(&storage_path).unwrap();
        let mid = contents.len() / 2;
        contents[mid] ^= 0xFF;
        fs::write(&storage_path, contents).unwrap();
    }

    // Read must fail with corruption error
    let mut reader = StorageReader::open_from_data_dir(data_dir).unwrap();
    let result = reader.read_at(0);

    assert!(
        result.is_err(),
        "D2 VIOLATION: Corruption must cause explicit failure"
    );

    let err = result.unwrap_err();
    assert!(
        err.to_string().to_lowercase().contains("checksum"),
        "D2: Error should mention checksum, got: {}",
        err
    );
}

/// D2: Checksum mismatch is detected on every read.
#[test]
fn test_d2_checksum_verified_on_every_read() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // Write multiple records
    {
        let mut writer = StorageWriter::open(data_dir).unwrap();
        for i in 1..=5 {
            writer
                .write(&create_test_payload(&format!("doc{}", i)))
                .unwrap();
        }
    }

    // Read all records - each read verifies checksum
    let mut reader = StorageReader::open_from_data_dir(data_dir).unwrap();
    let records = reader.read_all();

    assert!(
        records.is_ok(),
        "All records should pass checksum verification"
    );
    assert_eq!(records.unwrap().len(), 5);
}

// =============================================================================
// INVARIANT D3: Reads Never Observe Invalid State
// =============================================================================

/// D3: Read returns complete, valid document.
#[test]
fn test_d3_reads_return_complete_documents() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    let expected_body = r#"{"id": "test123", "name": "Complete Document"}"#;

    // Write a document
    {
        let mut writer = StorageWriter::open(data_dir).unwrap();
        let payload = StoragePayload::new(
            "col",
            "test123",
            "schema",
            "v1",
            expected_body.as_bytes().to_vec(),
        );
        writer.write(&payload).unwrap();
    }

    // Read and verify complete document
    let mut reader = StorageReader::open_from_data_dir(data_dir).unwrap();
    let record = reader.read_at(0).unwrap();

    // D3: All fields must be present and valid
    assert_eq!(record.document_id, "col:test123");
    assert_eq!(record.schema_id, "schema");
    assert_eq!(record.schema_version, "v1");
    assert!(!record.is_tombstone);
    assert_eq!(record.document_body, expected_body.as_bytes());
}

/// D3: Schema information is always present.
#[test]
fn test_d3_schema_info_always_present() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    {
        let mut writer = StorageWriter::open(data_dir).unwrap();
        writer
            .write(&StoragePayload::new(
                "col",
                "doc1",
                "my_schema",
                "v2",
                b"{}".to_vec(),
            ))
            .unwrap();
    }

    let mut reader = StorageReader::open_from_data_dir(data_dir).unwrap();
    let record = reader.read_at(0).unwrap();

    // D3: Schema fields must not be empty
    assert!(
        !record.schema_id.is_empty(),
        "D3: schema_id must be present"
    );
    assert!(
        !record.schema_version.is_empty(),
        "D3: schema_version must be present"
    );
}

// =============================================================================
// INVARIANT K1: Checksums on Every Record
// =============================================================================

/// K1: Record serialization includes checksum.
#[test]
fn test_k1_checksum_included_in_serialization() {
    let payload = create_test_payload("doc1");
    let record = DocumentRecord::from_payload(&payload);
    let serialized = record.serialize();

    // Checksum is last 4 bytes
    assert!(serialized.len() > 4, "Record must have checksum bytes");

    // Deserialize should verify checksum
    let (deserialized, _) = DocumentRecord::deserialize(&serialized).unwrap();
    assert_eq!(record, deserialized);
}

/// K1: Corrupted checksum bytes are detected.
#[test]
fn test_k1_corrupted_checksum_bytes_detected() {
    let payload = create_test_payload("doc1");
    let record = DocumentRecord::from_payload(&payload);
    let mut serialized = record.serialize();

    // Corrupt the last byte (checksum region)
    let len = serialized.len();
    serialized[len - 1] ^= 0xFF;

    let result = DocumentRecord::deserialize(&serialized);
    assert!(result.is_err(), "K1: Corrupted checksum must be detected");
}

// =============================================================================
// INVARIANT C1: Full-Document Writes
// =============================================================================

/// C1: Storage records contain complete document state.
#[test]
fn test_c1_complete_document_state_preserved() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    let original_body = br#"{"complex": {"nested": [1, 2, 3]}, "flag": true}"#;

    {
        let mut writer = StorageWriter::open(data_dir).unwrap();
        writer
            .write(&StoragePayload::new(
                "collection",
                "document",
                "schema",
                "v1",
                original_body.to_vec(),
            ))
            .unwrap();
    }

    let mut reader = StorageReader::open_from_data_dir(data_dir).unwrap();
    let record = reader.read_at(0).unwrap();

    // C1: Document body must match exactly
    assert_eq!(
        record.document_body,
        original_body.to_vec(),
        "C1 VIOLATION: Document body mismatch"
    );
}

/// C1: Tombstones have empty body but preserve metadata.
#[test]
fn test_c1_tombstone_preserves_metadata() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    {
        let mut writer = StorageWriter::open(data_dir).unwrap();
        writer
            .write_tombstone("col", "deleted_doc", "schema", "v1")
            .unwrap();
    }

    let mut reader = StorageReader::open_from_data_dir(data_dir).unwrap();
    let record = reader.read_at(0).unwrap();

    // C1: Tombstone has empty body but preserves identity
    assert!(record.is_tombstone);
    assert!(record.document_body.is_empty());
    assert_eq!(record.document_id, "col:deleted_doc");
    assert_eq!(record.schema_id, "schema");
}

// =============================================================================
// WAL-Storage Integration Tests
// =============================================================================

/// Storage correctly applies WAL INSERT record.
#[test]
fn test_apply_wal_insert_record() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    let wal_payload = WalPayload::new("users", "user1", "user_schema", "v1", b"user data".to_vec());
    let wal_record = WalRecord::new(RecordType::Insert, 1, wal_payload);

    {
        let mut writer = StorageWriter::open(data_dir).unwrap();
        writer.apply_wal_record(&wal_record).unwrap();
    }

    let mut reader = StorageReader::open_from_data_dir(data_dir).unwrap();
    let record = reader.read_at(0).unwrap();

    assert_eq!(record.document_id, "users:user1");
    assert!(!record.is_tombstone);
    assert_eq!(record.document_body, b"user data");
}

/// Storage correctly applies WAL DELETE record.
#[test]
fn test_apply_wal_delete_record() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    let wal_payload = WalPayload::tombstone("users", "user1", "user_schema", "v1");
    let wal_record = WalRecord::new(RecordType::Delete, 1, wal_payload);

    {
        let mut writer = StorageWriter::open(data_dir).unwrap();
        writer.apply_wal_record(&wal_record).unwrap();
    }

    let mut reader = StorageReader::open_from_data_dir(data_dir).unwrap();
    let record = reader.read_at(0).unwrap();

    assert!(record.is_tombstone);
    assert!(record.document_body.is_empty());
}

// =============================================================================
// Append-Only Semantics Tests
// =============================================================================

/// Multiple records for same document are all preserved.
#[test]
fn test_append_only_preserves_all_versions() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    {
        let mut writer = StorageWriter::open(data_dir).unwrap();
        // Write 3 versions of same document
        writer
            .write(&StoragePayload::new(
                "col",
                "doc1",
                "s",
                "v1",
                b"version1".to_vec(),
            ))
            .unwrap();
        writer
            .write(&StoragePayload::new(
                "col",
                "doc1",
                "s",
                "v1",
                b"version2".to_vec(),
            ))
            .unwrap();
        writer
            .write(&StoragePayload::new(
                "col",
                "doc1",
                "s",
                "v1",
                b"version3".to_vec(),
            ))
            .unwrap();
    }

    // All 3 records must be readable
    let mut reader = StorageReader::open_from_data_dir(data_dir).unwrap();
    let records = reader.read_all().unwrap();

    assert_eq!(records.len(), 3, "Append-only: all versions preserved");
}

/// Offset tracking enables random access.
#[test]
fn test_offset_tracking() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    let offsets: Vec<u64>;
    {
        let mut writer = StorageWriter::open(data_dir).unwrap();
        offsets = (1..=3)
            .map(|i| {
                writer
                    .write(&create_test_payload(&format!("doc{}", i)))
                    .unwrap()
            })
            .collect();
    }

    // Read at specific offsets
    let mut reader = StorageReader::open_from_data_dir(data_dir).unwrap();

    for (i, &offset) in offsets.iter().enumerate() {
        let record = reader.read_at(offset).unwrap();
        let expected_id = format!("test_collection:doc{}", i + 1);
        assert_eq!(record.document_id, expected_id);
    }
}

// =============================================================================
// Serialization Round-Trip Tests
// =============================================================================

/// DocumentRecord serialization is bijective.
#[test]
fn test_document_record_roundtrip() {
    let payload = StoragePayload::new(
        "my_collection",
        "my_document",
        "my_schema",
        "version_1",
        b"payload content here".to_vec(),
    );
    let original = DocumentRecord::from_payload(&payload);

    let serialized = original.serialize();
    let (deserialized, consumed) = DocumentRecord::deserialize(&serialized).unwrap();

    assert_eq!(consumed, serialized.len());
    assert_eq!(original, deserialized);
}

/// StoragePayload correctly generates composite key.
#[test]
fn test_composite_key_generation() {
    let payload = StoragePayload::new("users", "user123", "schema", "v1", b"{}".to_vec());
    let record = DocumentRecord::from_payload(&payload);

    assert_eq!(record.document_id, "users:user123");
}

// =============================================================================
// Persistence Tests
// =============================================================================

/// Data persists across reopens.
#[test]
fn test_data_persists_across_reopens() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // Write records
    {
        let mut writer = StorageWriter::open(data_dir).unwrap();
        for i in 1..=5 {
            writer
                .write(&create_test_payload(&format!("doc{}", i)))
                .unwrap();
        }
    }

    // Reopen and verify
    {
        let mut reader = StorageReader::open_from_data_dir(data_dir).unwrap();
        let records = reader.read_all().unwrap();
        assert_eq!(records.len(), 5);
    }
}

/// Writer reopens with correct state.
#[test]
fn test_writer_reopens_with_correct_state() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // First session
    {
        let mut writer = StorageWriter::open(data_dir).unwrap();
        writer.write(&create_test_payload("doc1")).unwrap();
    }

    // Second session - should continue from correct offset
    {
        let mut writer = StorageWriter::open(data_dir).unwrap();
        assert!(
            writer.current_offset() > 0,
            "Writer should have non-zero offset"
        );
        writer.write(&create_test_payload("doc2")).unwrap();
    }

    // Verify both records exist
    let mut reader = StorageReader::open_from_data_dir(data_dir).unwrap();
    let records = reader.read_all().unwrap();
    assert_eq!(records.len(), 2);
}
