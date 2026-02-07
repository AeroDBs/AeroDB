//! WAL reader with strict corruption detection per WAL.md
//!
//! Per WAL.md §216-229 (Zero Tolerance Policy):
//! - If any WAL corruption is detected, startup halts immediately
//! - No partial replay
//! - No skipping records
//! - No repair attempts
//!
//! Per WAL.md §233-252 (Replay Rules):
//! - WAL records are replayed strictly in sequence number order
//! - Replay always starts from the first record
//! - Replay is single-threaded

use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use super::errors::{WalError, WalResult};
use super::record::WalRecord;

/// WAL reader for sequential replay.
///
/// Reads records from the WAL in strict order, validating checksums
/// and record structure. Any corruption causes immediate failure.
pub struct WalReader {
    /// Path to the WAL file
    wal_path: PathBuf,
    /// Buffered reader for efficient sequential reads
    reader: BufReader<File>,
    /// Current byte offset in the file
    current_offset: u64,
    /// Total file size
    file_size: u64,
    /// Last successfully read sequence number
    last_sequence: u64,
}

impl WalReader {
    /// Opens a WAL file for reading.
    ///
    /// # Arguments
    ///
    /// * `wal_path` - Path to the WAL file
    ///
    /// # Errors
    ///
    /// Returns `WalError` if the file cannot be opened.
    pub fn open(wal_path: &Path) -> WalResult<Self> {
        let file = File::open(wal_path).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                WalError::corruption(format!("WAL file not found: {}", wal_path.display()))
            } else {
                WalError::corruption(format!(
                    "Failed to open WAL file: {}: {}",
                    wal_path.display(),
                    e
                ))
            }
        })?;

        let metadata = file
            .metadata()
            .map_err(|e| WalError::corruption(format!("Failed to read WAL metadata: {}", e)))?;

        let file_size = metadata.len();

        Ok(Self {
            wal_path: wal_path.to_path_buf(),
            reader: BufReader::new(file),
            current_offset: 0,
            file_size,
            last_sequence: 0,
        })
    }

    /// Opens a WAL file from a data directory.
    ///
    /// Expects the WAL at `<data_dir>/wal/wal.log`.
    pub fn open_from_data_dir(data_dir: &Path) -> WalResult<Self> {
        let wal_path = data_dir.join("wal").join("wal.log");
        Self::open(&wal_path)
    }

    /// Returns the path to the WAL file.
    pub fn path(&self) -> &Path {
        &self.wal_path
    }

    /// Returns the current byte offset in the file.
    pub fn current_offset(&self) -> u64 {
        self.current_offset
    }

    /// Returns the last successfully read sequence number.
    pub fn last_sequence_number(&self) -> u64 {
        self.last_sequence
    }

    /// Reads the next record from the WAL.
    ///
    /// Per WAL.md §243-252:
    /// 1. Validate checksum
    /// 2. Validate record structure
    /// 3. Validate schema version existence (deferred to recovery manager)
    ///
    /// # Returns
    ///
    /// - `Ok(Some(record))` if a record was successfully read
    /// - `Ok(None)` if end of file reached cleanly
    /// - `Err(WalError)` if any corruption or read error occurs
    ///
    /// # Errors
    ///
    /// Returns `AERO_WAL_CORRUPTION` if:
    /// - Checksum validation fails
    /// - Record structure is invalid
    /// - File is truncated mid-record
    /// - Sequence numbers are not strictly increasing
    pub fn read_next(&mut self) -> WalResult<Option<WalRecord>> {
        // Check if we've reached end of file
        if self.current_offset >= self.file_size {
            return Ok(None);
        }

        let remaining = self.file_size - self.current_offset;

        // Minimum record size check
        const MIN_RECORD_SIZE: u64 = 4 + 1 + 8 + 20 + 4; // len + type + seq + min_payload + checksum
        if remaining < MIN_RECORD_SIZE {
            return Err(WalError::corruption_at_offset(
                self.current_offset,
                format!(
                    "Truncated WAL: {} bytes remaining, minimum record size is {}",
                    remaining, MIN_RECORD_SIZE
                ),
            ));
        }

        // Read record length first
        let mut len_buf = [0u8; 4];
        self.reader.read_exact(&mut len_buf).map_err(|e| {
            WalError::corruption_at_offset(
                self.current_offset,
                format!("Failed to read record length: {}", e),
            )
        })?;
        let record_length = u32::from_le_bytes(len_buf) as u64;

        // Validate record length
        if record_length < MIN_RECORD_SIZE {
            return Err(WalError::corruption_at_offset(
                self.current_offset,
                format!("Invalid record length: {}", record_length),
            ));
        }

        if record_length > remaining {
            return Err(WalError::corruption_at_offset(
                self.current_offset,
                format!(
                    "Record length {} exceeds remaining file size {}",
                    record_length, remaining
                ),
            ));
        }

        // Read the rest of the record
        let mut record_buf = vec![0u8; record_length as usize];
        record_buf[0..4].copy_from_slice(&len_buf);

        self.reader.read_exact(&mut record_buf[4..]).map_err(|e| {
            WalError::corruption_at_offset(
                self.current_offset,
                format!("Failed to read record body: {}", e),
            )
        })?;

        // Parse and validate record (includes checksum verification)
        let (record, bytes_consumed) = WalRecord::deserialize(&record_buf)
            .map_err(|e| WalError::corruption_at_offset(self.current_offset, e.to_string()))?;

        // Validate sequence number ordering
        if self.last_sequence > 0 && record.sequence_number != self.last_sequence + 1 {
            return Err(WalError::corruption_at_sequence(
                record.sequence_number,
                format!(
                    "Non-sequential sequence number: expected {}, got {}",
                    self.last_sequence + 1,
                    record.sequence_number
                ),
            ));
        }

        // Validate sequence number starts at 1
        if self.last_sequence == 0 && record.sequence_number != 1 {
            return Err(WalError::corruption_at_sequence(
                record.sequence_number,
                format!(
                    "First sequence number must be 1, got {}",
                    record.sequence_number
                ),
            ));
        }

        // Update state
        self.current_offset += bytes_consumed as u64;
        self.last_sequence = record.sequence_number;

        Ok(Some(record))
    }

    /// Reads all records from the WAL.
    ///
    /// This is a convenience method for full WAL replay.
    /// Any corruption causes immediate failure.
    ///
    /// # Returns
    ///
    /// A vector of all WAL records in sequence order.
    ///
    /// # Errors
    ///
    /// Returns `AERO_WAL_CORRUPTION` if any record is corrupted.
    pub fn read_all(&mut self) -> WalResult<Vec<WalRecord>> {
        let mut records = Vec::new();

        loop {
            match self.read_next()? {
                Some(record) => records.push(record),
                None => break,
            }
        }

        Ok(records)
    }

    /// Resets the reader to the beginning of the WAL.
    pub fn reset(&mut self) -> WalResult<()> {
        self.reader
            .seek(SeekFrom::Start(0))
            .map_err(|e| WalError::corruption(format!("Failed to seek to start of WAL: {}", e)))?;
        self.current_offset = 0;
        self.last_sequence = 0;
        Ok(())
    }

    /// Returns whether there are more records to read.
    pub fn has_more(&self) -> bool {
        self.current_offset < self.file_size
    }
}

/// Iterator adapter for WalReader.
///
/// Stops iteration on any error, which should be treated as fatal.
pub struct WalRecordIterator {
    reader: WalReader,
    error: Option<WalError>,
}

impl WalRecordIterator {
    /// Creates a new iterator from a reader.
    pub fn new(reader: WalReader) -> Self {
        Self {
            reader,
            error: None,
        }
    }

    /// Returns the error if iteration failed.
    pub fn error(&self) -> Option<&WalError> {
        self.error.as_ref()
    }

    /// Consumes the iterator and returns the error if any.
    pub fn into_error(self) -> Option<WalError> {
        self.error
    }
}

impl Iterator for WalRecordIterator {
    type Item = WalRecord;

    fn next(&mut self) -> Option<Self::Item> {
        if self.error.is_some() {
            return None;
        }

        match self.reader.read_next() {
            Ok(Some(record)) => Some(record),
            Ok(None) => None,
            Err(e) => {
                self.error = Some(e);
                None
            }
        }
    }
}

impl IntoIterator for WalReader {
    type Item = WalRecord;
    type IntoIter = WalRecordIterator;

    fn into_iter(self) -> Self::IntoIter {
        WalRecordIterator::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::super::record::{RecordType, WalPayload};
    use super::super::writer::WalWriter;
    use super::*;
    use tempfile::TempDir;

    fn create_test_payload(doc_id: &str) -> WalPayload {
        WalPayload::new(
            "test_collection",
            doc_id,
            "test_schema",
            "v1",
            format!(r#"{{"id": "{}"}}"#, doc_id).into_bytes(),
        )
    }

    #[test]
    fn test_read_empty_wal() {
        let temp_dir = TempDir::new().unwrap();

        // Create empty WAL
        {
            let _writer = WalWriter::open(temp_dir.path()).unwrap();
        }

        let wal_path = temp_dir.path().join("wal").join("wal.log");
        let mut reader = WalReader::open(&wal_path).unwrap();

        assert!(reader.read_next().unwrap().is_none());
    }

    #[test]
    fn test_read_single_record() {
        let temp_dir = TempDir::new().unwrap();

        // Write a record
        {
            let mut writer = WalWriter::open(temp_dir.path()).unwrap();
            writer.append_insert(create_test_payload("doc1")).unwrap();
        }

        // Read it back
        let wal_path = temp_dir.path().join("wal").join("wal.log");
        let mut reader = WalReader::open(&wal_path).unwrap();

        let record = reader.read_next().unwrap().unwrap();
        assert_eq!(record.sequence_number, 1);
        assert_eq!(record.record_type, RecordType::Insert);
        assert_eq!(record.payload.document_id, "doc1");

        assert!(reader.read_next().unwrap().is_none());
    }

    #[test]
    fn test_read_multiple_records() {
        let temp_dir = TempDir::new().unwrap();

        // Write records
        {
            let mut writer = WalWriter::open(temp_dir.path()).unwrap();
            writer.append_insert(create_test_payload("doc1")).unwrap();
            writer.append_update(create_test_payload("doc1")).unwrap();
            writer
                .append_delete(WalPayload::tombstone(
                    "test_collection",
                    "doc1",
                    "test_schema",
                    "v1",
                ))
                .unwrap();
        }

        // Read all
        let wal_path = temp_dir.path().join("wal").join("wal.log");
        let mut reader = WalReader::open(&wal_path).unwrap();
        let records = reader.read_all().unwrap();

        assert_eq!(records.len(), 3);
        assert_eq!(records[0].sequence_number, 1);
        assert_eq!(records[0].record_type, RecordType::Insert);
        assert_eq!(records[1].sequence_number, 2);
        assert_eq!(records[1].record_type, RecordType::Update);
        assert_eq!(records[2].sequence_number, 3);
        assert_eq!(records[2].record_type, RecordType::Delete);
    }

    #[test]
    fn test_corruption_detected_on_checksum_failure() {
        let temp_dir = TempDir::new().unwrap();

        // Write a record
        {
            let mut writer = WalWriter::open(temp_dir.path()).unwrap();
            writer.append_insert(create_test_payload("doc1")).unwrap();
        }

        // Corrupt the WAL file
        let wal_path = temp_dir.path().join("wal").join("wal.log");
        {
            use std::fs::OpenOptions;
            use std::io::{Seek, SeekFrom, Write};

            let mut file = OpenOptions::new().write(true).open(&wal_path).unwrap();
            // Corrupt byte in the middle
            file.seek(SeekFrom::Start(10)).unwrap();
            file.write_all(&[0xFF]).unwrap();
        }

        // Try to read - should fail with corruption
        let mut reader = WalReader::open(&wal_path).unwrap();
        let result = reader.read_next();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_fatal());
        assert_eq!(err.code().code(), "AERO_WAL_CORRUPTION");
    }

    #[test]
    fn test_truncation_detected() {
        let temp_dir = TempDir::new().unwrap();

        // Write a record
        {
            let mut writer = WalWriter::open(temp_dir.path()).unwrap();
            writer.append_insert(create_test_payload("doc1")).unwrap();
        }

        // Truncate the WAL file
        let wal_path = temp_dir.path().join("wal").join("wal.log");
        {
            use std::fs::OpenOptions;

            let file = OpenOptions::new().write(true).open(&wal_path).unwrap();
            let original_len = file.metadata().unwrap().len();
            file.set_len(original_len - 5).unwrap();
        }

        // Try to read - should fail with corruption
        let mut reader = WalReader::open(&wal_path).unwrap();
        let result = reader.read_next();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_fatal());
    }

    #[test]
    fn test_sequence_ordering_enforced() {
        let temp_dir = TempDir::new().unwrap();

        // Write records normally
        {
            let mut writer = WalWriter::open(temp_dir.path()).unwrap();
            writer.append_insert(create_test_payload("doc1")).unwrap();
        }

        // Read and verify sequence starts at 1
        let wal_path = temp_dir.path().join("wal").join("wal.log");
        let mut reader = WalReader::open(&wal_path).unwrap();
        let record = reader.read_next().unwrap().unwrap();
        assert_eq!(record.sequence_number, 1);
    }

    #[test]
    fn test_read_all_roundtrip() {
        let temp_dir = TempDir::new().unwrap();

        let payloads: Vec<_> = (1..=5)
            .map(|i| create_test_payload(&format!("doc{}", i)))
            .collect();

        // Write
        {
            let mut writer = WalWriter::open(temp_dir.path()).unwrap();
            for payload in &payloads {
                writer.append_insert(payload.clone()).unwrap();
            }
        }

        // Read
        let wal_path = temp_dir.path().join("wal").join("wal.log");
        let mut reader = WalReader::open(&wal_path).unwrap();
        let records = reader.read_all().unwrap();

        assert_eq!(records.len(), 5);
        for (i, record) in records.iter().enumerate() {
            assert_eq!(record.sequence_number, (i + 1) as u64);
            assert_eq!(record.payload.document_id, format!("doc{}", i + 1));
        }
    }

    #[test]
    fn test_iterator_adapter() {
        let temp_dir = TempDir::new().unwrap();

        // Write records
        {
            let mut writer = WalWriter::open(temp_dir.path()).unwrap();
            for i in 1..=3 {
                writer
                    .append_insert(create_test_payload(&format!("doc{}", i)))
                    .unwrap();
            }
        }

        // Read via iterator
        let wal_path = temp_dir.path().join("wal").join("wal.log");
        let reader = WalReader::open(&wal_path).unwrap();
        let records: Vec<_> = reader.into_iter().collect();

        assert_eq!(records.len(), 3);
    }

    #[test]
    fn test_reset_allows_rereading() {
        let temp_dir = TempDir::new().unwrap();

        // Write
        {
            let mut writer = WalWriter::open(temp_dir.path()).unwrap();
            writer.append_insert(create_test_payload("doc1")).unwrap();
        }

        // Read, reset, read again
        let wal_path = temp_dir.path().join("wal").join("wal.log");
        let mut reader = WalReader::open(&wal_path).unwrap();

        let record1 = reader.read_next().unwrap().unwrap();
        assert!(reader.read_next().unwrap().is_none());

        reader.reset().unwrap();

        let record2 = reader.read_next().unwrap().unwrap();
        assert_eq!(record1.sequence_number, record2.sequence_number);
        assert_eq!(record1.payload.document_id, record2.payload.document_id);
    }

    #[test]
    fn test_replay_idempotency() {
        // Replaying the same WAL twice produces the same sequence of records
        let temp_dir = TempDir::new().unwrap();

        // Write
        {
            let mut writer = WalWriter::open(temp_dir.path()).unwrap();
            writer.append_insert(create_test_payload("doc1")).unwrap();
            writer.append_update(create_test_payload("doc1")).unwrap();
        }

        let wal_path = temp_dir.path().join("wal").join("wal.log");

        // First replay
        let records1: Vec<_> = {
            let mut reader = WalReader::open(&wal_path).unwrap();
            reader.read_all().unwrap()
        };

        // Second replay
        let records2: Vec<_> = {
            let mut reader = WalReader::open(&wal_path).unwrap();
            reader.read_all().unwrap()
        };

        // Verify identical results (deterministic replay)
        assert_eq!(records1.len(), records2.len());
        for (r1, r2) in records1.iter().zip(records2.iter()) {
            assert_eq!(r1.sequence_number, r2.sequence_number);
            assert_eq!(r1.record_type, r2.record_type);
            assert_eq!(r1.payload, r2.payload);
        }
    }
}
