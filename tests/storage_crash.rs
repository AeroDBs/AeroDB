//! Storage Crash Safety Tests
//!
//! Tests for invariants:
//! - K2: Halt-on-corruption policy
//! - Crash recovery from storage corruption
//! - Partial write detection
//!
//! Per STORAGE.md, storage halts on any corruption with no repair attempts.

use aerodb::storage::{StoragePayload, StorageReader, StorageWriter};
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
// INVARIANT K2: Halt-on-Corruption Policy
// =============================================================================

/// K2: Corrupted record in middle of file halts reading.
#[test]
fn test_k2_corruption_in_middle_halts() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();
    let storage_path = data_dir.join("data/documents.dat");

    // Write multiple records
    {
        let mut writer = StorageWriter::open(data_dir).unwrap();
        for i in 1..=5 {
            writer
                .write(&create_test_payload(&format!("doc{}", i)))
                .unwrap();
        }
    }

    // Corrupt a byte in the middle of the file
    {
        let mut contents = fs::read(&storage_path).unwrap();
        let mid = contents.len() / 2;
        contents[mid] ^= 0xFF;
        fs::write(&storage_path, contents).unwrap();
    }

    // Read all must fail
    let mut reader = StorageReader::open_from_data_dir(data_dir).unwrap();
    let result = reader.read_all();

    assert!(
        result.is_err(),
        "K2 VIOLATION: Corruption must halt, not skip records"
    );
}

/// K2: Truncated record at EOF halts reading.
#[test]
fn test_k2_truncated_record_halts() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();
    let storage_path = data_dir.join("data/documents.dat");

    // Write a record
    {
        let mut writer = StorageWriter::open(data_dir).unwrap();
        writer.write(&create_test_payload("doc1")).unwrap();
    }

    // Truncate the file mid-record
    {
        let contents = fs::read(&storage_path).unwrap();
        fs::write(&storage_path, &contents[..contents.len() - 10]).unwrap();
    }

    // Read must fail
    let mut reader = StorageReader::open_from_data_dir(data_dir).unwrap();
    let result = reader.read_at(0);

    assert!(result.is_err(), "K2: Truncated record must halt");
}

/// K2: Invalid record length halts reading.
#[test]
fn test_k2_invalid_record_length_halts() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();
    let storage_path = data_dir.join("data/documents.dat");

    // Write a record
    {
        let mut writer = StorageWriter::open(data_dir).unwrap();
        writer.write(&create_test_payload("doc1")).unwrap();
    }

    // Corrupt the record length (first 4 bytes)
    {
        let mut contents = fs::read(&storage_path).unwrap();
        // Set record length to an impossibly large value
        contents[0..4].copy_from_slice(&[0xFF, 0xFF, 0x00, 0x00]);
        fs::write(&storage_path, contents).unwrap();
    }

    // Read must fail
    let mut reader = StorageReader::open_from_data_dir(data_dir).unwrap();
    let result = reader.read_at(0);

    assert!(result.is_err(), "K2: Invalid record length must halt");
}

// =============================================================================
// Partial Write Simulation Tests
// =============================================================================

/// Partial write at EOF should be detected.
#[test]
fn test_partial_write_detection() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();
    let storage_path = data_dir.join("data/documents.dat");

    // Write a complete record
    {
        let mut writer = StorageWriter::open(data_dir).unwrap();
        writer.write(&create_test_payload("doc1")).unwrap();
    }

    // Append garbage bytes (simulating partial write crash)
    {
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&storage_path)
            .unwrap();
        // Write partial record header
        file.write_all(&[0x50, 0x00, 0x00, 0x00, 0x01]).unwrap();
    }

    // First record should be readable
    let mut reader = StorageReader::open_from_data_dir(data_dir).unwrap();
    let first = reader.read_at(0);
    assert!(first.is_ok(), "First complete record should be readable");

    // But read_all should fail on the partial trailing record
    reader = StorageReader::open_from_data_dir(data_dir).unwrap();
    let result = reader.read_all();
    assert!(result.is_err(), "Partial record at EOF must cause halt");
}

// =============================================================================
// Recovery After Corruption Tests
// =============================================================================

/// Valid records before corruption can be recovered individually.
#[test]
fn test_partial_recovery_before_corruption() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();
    let storage_path = data_dir.join("data/documents.dat");

    // Write multiple records
    let offsets: Vec<u64>;
    {
        let mut writer = StorageWriter::open(data_dir).unwrap();
        offsets = (1..=5)
            .map(|i| {
                writer
                    .write(&create_test_payload(&format!("doc{}", i)))
                    .unwrap()
            })
            .collect();
    }

    // Corrupt the last record only
    {
        let contents = fs::read(&storage_path).unwrap();
        let last_offset = offsets[4] as usize;
        let mut modified = contents;
        // Corrupt last record's checksum
        let corrupt_pos = modified.len() - 2;
        modified[corrupt_pos] ^= 0xFF;
        fs::write(&storage_path, modified).unwrap();
    }

    // First 4 records should be individually recoverable
    let mut reader = StorageReader::open_from_data_dir(data_dir).unwrap();
    for i in 0..4 {
        let result = reader.read_at(offsets[i]);
        assert!(
            result.is_ok(),
            "Record {} before corruption should be recoverable",
            i
        );
    }

    // Last record should fail
    let last_result = reader.read_at(offsets[4]);
    assert!(last_result.is_err(), "Corrupted record should fail");
}

// =============================================================================
// Determinism Tests
// =============================================================================

/// Same storage content produces same read results.
#[test]
fn test_deterministic_reads() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // Write records
    {
        let mut writer = StorageWriter::open(data_dir).unwrap();
        for i in 1..=10 {
            writer
                .write(&create_test_payload(&format!("doc{}", i)))
                .unwrap();
        }
    }

    // Read multiple times
    let mut reader1 = StorageReader::open_from_data_dir(data_dir).unwrap();
    let records1 = reader1.read_all().unwrap();

    let mut reader2 = StorageReader::open_from_data_dir(data_dir).unwrap();
    let records2 = reader2.read_all().unwrap();

    let mut reader3 = StorageReader::open_from_data_dir(data_dir).unwrap();
    let records3 = reader3.read_all().unwrap();

    // All reads must be identical
    assert_eq!(records1.len(), records2.len());
    assert_eq!(records2.len(), records3.len());

    for i in 0..records1.len() {
        assert_eq!(records1[i], records2[i]);
        assert_eq!(records2[i], records3[i]);
    }
}

// =============================================================================
// Empty Storage Tests
// =============================================================================

/// Empty storage is valid and produces empty read.
#[test]
fn test_empty_storage_valid() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // Create empty storage
    {
        let _writer = StorageWriter::open(data_dir).unwrap();
    }

    // Empty read should succeed
    let mut reader = StorageReader::open_from_data_dir(data_dir).unwrap();
    let records = reader.read_all().unwrap();

    assert!(records.is_empty(), "Empty storage produces empty result");
}
