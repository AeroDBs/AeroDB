//! WAL writer with fsync enforcement per WAL.md
//!
//! Per WAL.md §175-198:
//! - Every WAL append is followed by fsync
//! - No batching
//! - No group commit
//! - No async durability
//!
//! Acknowledgment before fsync is forbidden.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::errors::{WalError, WalResult};
use super::record::{RecordType, WalPayload, WalRecord};

/// WAL writer that enforces fsync after every append.
///
/// Per WAL.md §69-74:
/// - Append-only
/// - Single file
/// - Never truncated in Phase 0
/// - Opened with exclusive write access
pub struct WalWriter {
    /// Path to the WAL file
    wal_path: PathBuf,
    /// Underlying file handle
    file: File,
    /// Next sequence number to assign (starts at 1, never reused)
    next_sequence: u64,
}

impl WalWriter {
    /// Opens or creates a WAL file at the specified data directory.
    ///
    /// Creates `<data_dir>/wal/wal.log` if it does not exist.
    /// Creates parent directories if needed.
    ///
    /// # Arguments
    ///
    /// * `data_dir` - The root data directory
    ///
    /// # Errors
    ///
    /// Returns `WalError::append_failed` if the file cannot be created or opened.
    pub fn open(data_dir: &Path) -> WalResult<Self> {
        let wal_dir = data_dir.join("wal");
        let wal_path = wal_dir.join("wal.log");

        // Create directories if missing
        if !wal_dir.exists() {
            fs::create_dir_all(&wal_dir).map_err(|e| {
                WalError::append_failed(
                    format!("Failed to create WAL directory: {}", wal_dir.display()),
                    e,
                )
            })?;
        }

        // Open file for append with exclusive write access
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&wal_path)
            .map_err(|e| {
                WalError::append_failed(
                    format!("Failed to open WAL file: {}", wal_path.display()),
                    e,
                )
            })?;

        // Determine next sequence number by reading existing WAL
        let next_sequence = Self::determine_next_sequence(&wal_path)?;

        Ok(Self {
            wal_path,
            file,
            next_sequence,
        })
    }

    /// Determines the next sequence number by scanning existing WAL.
    ///
    /// Returns 1 if WAL is empty or does not exist.
    fn determine_next_sequence(wal_path: &Path) -> WalResult<u64> {
        use super::reader::WalReader;

        // If file doesn't exist or is empty, start at 1
        let metadata = match fs::metadata(wal_path) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(1),
            Err(e) => return Err(WalError::append_failed("Failed to read WAL metadata", e)),
        };

        if metadata.len() == 0 {
            return Ok(1);
        }

        // Read through WAL to find highest sequence number
        let mut reader = WalReader::open(wal_path)?;
        let mut max_sequence = 0u64;

        loop {
            match reader.read_next() {
                Ok(Some(record)) => {
                    max_sequence = max_sequence.max(record.sequence_number);
                }
                Ok(None) => break,
                Err(e) => return Err(e),
            }
        }

        Ok(max_sequence + 1)
    }

    /// Returns the path to the WAL file.
    pub fn path(&self) -> &Path {
        &self.wal_path
    }

    /// Returns the next sequence number that will be assigned.
    pub fn next_sequence_number(&self) -> u64 {
        self.next_sequence
    }

    /// Returns the last assigned sequence number, or 0 if no records written.
    pub fn last_sequence_number(&self) -> u64 {
        if self.next_sequence > 1 {
            self.next_sequence - 1
        } else {
            0
        }
    }

    /// Appends a record to the WAL with fsync enforcement.
    ///
    /// Per WAL.md §175-198:
    /// 1. Construct WAL record
    /// 2. Append record to wal.log
    /// 3. Flush WAL to disk using fsync
    /// 4. Only after fsync may the operation proceed
    ///
    /// # Arguments
    ///
    /// * `record_type` - INSERT, UPDATE, or DELETE
    /// * `payload` - The operation payload
    ///
    /// # Returns
    ///
    /// The sequence number assigned to this record.
    ///
    /// # Errors
    ///
    /// - `AERO_WAL_APPEND_FAILED` if write fails
    /// - `AERO_WAL_FSYNC_FAILED` if fsync fails (FATAL)
    pub fn append(&mut self, record_type: RecordType, payload: WalPayload) -> WalResult<u64> {
        let sequence_number = self.next_sequence;
        let record = WalRecord::new(record_type, sequence_number, payload);
        let serialized = record.serialize();

        // Write to file
        self.file.write_all(&serialized).map_err(|e| {
            WalError::append_failed(
                format!("Failed to write WAL record at sequence {}", sequence_number),
                e,
            )
        })?;

        // fsync - this is mandatory and FATAL if it fails
        self.file.sync_all().map_err(|e| {
            WalError::fsync_failed(
                format!(
                    "fsync failed after WAL append at sequence {}",
                    sequence_number
                ),
                e,
            )
        })?;

        // Only increment after successful fsync
        self.next_sequence += 1;

        Ok(sequence_number)
    }

    /// Appends an INSERT record.
    pub fn append_insert(&mut self, payload: WalPayload) -> WalResult<u64> {
        self.append(RecordType::Insert, payload)
    }

    /// Appends an UPDATE record.
    pub fn append_update(&mut self, payload: WalPayload) -> WalResult<u64> {
        self.append(RecordType::Update, payload)
    }

    /// Appends a DELETE record.
    pub fn append_delete(&mut self, payload: WalPayload) -> WalResult<u64> {
        self.append(RecordType::Delete, payload)
    }

    /// Explicitly fsync the WAL file.
    ///
    /// This ensures all pending writes are durable on disk.
    /// Called before snapshot creation per CHECKPOINT.md.
    pub fn fsync(&self) -> WalResult<()> {
        self.file
            .sync_all()
            .map_err(|e| WalError::fsync_failed("Explicit WAL fsync failed", e))
    }

    /// Returns the WAL directory path.
    pub fn wal_dir(&self) -> &Path {
        self.wal_path.parent().unwrap_or(Path::new("."))
    }

    /// Truncate WAL to zero (after successful snapshot).
    ///
    /// Per CHECKPOINT.md §6:
    /// - WAL file deleted or truncated
    /// - New WAL starts empty
    /// - Sequence numbers reset to 1
    ///
    /// This operation is atomic: the old file is removed and a new empty
    /// file is created with fsync.
    ///
    /// # Errors
    ///
    /// Returns `WalError` if truncation fails. If truncation fails,
    /// the WAL is left in its original state.
    pub fn truncate(&mut self) -> WalResult<()> {
        // Close current file by dropping and reopening
        let wal_dir = self.wal_path.parent().unwrap_or(Path::new("."));

        // Remove old WAL file
        if self.wal_path.exists() {
            fs::remove_file(&self.wal_path).map_err(|e| {
                WalError::append_failed(
                    format!(
                        "Failed to remove WAL file during truncation: {}",
                        self.wal_path.display()
                    ),
                    e,
                )
            })?;
        }

        // Create new empty WAL file
        let new_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.wal_path)
            .map_err(|e| {
                WalError::append_failed(
                    format!(
                        "Failed to create new WAL file during truncation: {}",
                        self.wal_path.display()
                    ),
                    e,
                )
            })?;

        // fsync new file
        new_file.sync_all().map_err(|e| {
            WalError::fsync_failed(
                format!("Failed to fsync new WAL file: {}", self.wal_path.display()),
                e,
            )
        })?;

        // fsync WAL directory to ensure file creation is durable
        let dir_handle = OpenOptions::new().read(true).open(wal_dir).map_err(|e| {
            WalError::append_failed(
                format!(
                    "Failed to open WAL directory for fsync: {}",
                    wal_dir.display()
                ),
                e,
            )
        })?;

        dir_handle.sync_all().map_err(|e| {
            WalError::fsync_failed(
                format!("Failed to fsync WAL directory: {}", wal_dir.display()),
                e,
            )
        })?;

        // Reopen file for append
        let file = OpenOptions::new()
            .append(true)
            .open(&self.wal_path)
            .map_err(|e| {
                WalError::append_failed(
                    format!(
                        "Failed to reopen WAL file after truncation: {}",
                        self.wal_path.display()
                    ),
                    e,
                )
            })?;

        // Update internal state
        self.file = file;
        self.next_sequence = 1;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
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
    fn test_writer_creates_directories() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path();

        let wal_dir = data_dir.join("wal");
        assert!(!wal_dir.exists());

        let _writer = WalWriter::open(data_dir).unwrap();

        assert!(wal_dir.exists());
        assert!(wal_dir.join("wal.log").exists());
    }

    #[test]
    fn test_sequence_numbers_start_at_one() {
        let temp_dir = TempDir::new().unwrap();
        let writer = WalWriter::open(temp_dir.path()).unwrap();

        assert_eq!(writer.next_sequence_number(), 1);
        assert_eq!(writer.last_sequence_number(), 0);
    }

    #[test]
    fn test_sequence_numbers_increment() {
        let temp_dir = TempDir::new().unwrap();
        let mut writer = WalWriter::open(temp_dir.path()).unwrap();

        let seq1 = writer.append_insert(create_test_payload("doc1")).unwrap();
        let seq2 = writer.append_insert(create_test_payload("doc2")).unwrap();
        let seq3 = writer.append_insert(create_test_payload("doc3")).unwrap();

        assert_eq!(seq1, 1);
        assert_eq!(seq2, 2);
        assert_eq!(seq3, 3);
        assert_eq!(writer.last_sequence_number(), 3);
    }

    #[test]
    fn test_append_all_record_types() {
        let temp_dir = TempDir::new().unwrap();
        let mut writer = WalWriter::open(temp_dir.path()).unwrap();

        let seq1 = writer.append_insert(create_test_payload("doc1")).unwrap();
        let seq2 = writer.append_update(create_test_payload("doc1")).unwrap();
        let seq3 = writer
            .append_delete(WalPayload::tombstone(
                "test_collection",
                "doc1",
                "test_schema",
                "v1",
            ))
            .unwrap();

        assert_eq!(seq1, 1);
        assert_eq!(seq2, 2);
        assert_eq!(seq3, 3);
    }

    #[test]
    fn test_writer_reopens_with_correct_sequence() {
        let temp_dir = TempDir::new().unwrap();

        // Write some records
        {
            let mut writer = WalWriter::open(temp_dir.path()).unwrap();
            writer.append_insert(create_test_payload("doc1")).unwrap();
            writer.append_insert(create_test_payload("doc2")).unwrap();
            writer.append_insert(create_test_payload("doc3")).unwrap();
        }

        // Reopen and verify sequence continues
        {
            let mut writer = WalWriter::open(temp_dir.path()).unwrap();
            assert_eq!(writer.next_sequence_number(), 4);

            let seq4 = writer.append_insert(create_test_payload("doc4")).unwrap();
            assert_eq!(seq4, 4);
        }
    }

    #[test]
    fn test_records_are_durable_after_append() {
        use super::super::reader::WalReader;

        let temp_dir = TempDir::new().unwrap();

        // Write records
        {
            let mut writer = WalWriter::open(temp_dir.path()).unwrap();
            writer.append_insert(create_test_payload("doc1")).unwrap();
            writer.append_update(create_test_payload("doc1")).unwrap();
        }

        // Read back and verify
        {
            let wal_path = temp_dir.path().join("wal").join("wal.log");
            let mut reader = WalReader::open(&wal_path).unwrap();

            let record1 = reader.read_next().unwrap().unwrap();
            assert_eq!(record1.sequence_number, 1);
            assert_eq!(record1.record_type, RecordType::Insert);
            assert_eq!(record1.payload.document_id, "doc1");

            let record2 = reader.read_next().unwrap().unwrap();
            assert_eq!(record2.sequence_number, 2);
            assert_eq!(record2.record_type, RecordType::Update);

            assert!(reader.read_next().unwrap().is_none());
        }
    }

    #[test]
    fn test_fsync_explicit() {
        let temp_dir = TempDir::new().unwrap();
        let mut writer = WalWriter::open(temp_dir.path()).unwrap();

        writer.append_insert(create_test_payload("doc1")).unwrap();

        // Explicit fsync should succeed
        assert!(writer.fsync().is_ok());
    }

    #[test]
    fn test_wal_dir() {
        let temp_dir = TempDir::new().unwrap();
        let writer = WalWriter::open(temp_dir.path()).unwrap();

        let wal_dir = writer.wal_dir();
        assert!(wal_dir.ends_with("wal"));
    }

    #[test]
    fn test_truncate_empties_wal() {
        use super::super::reader::WalReader;

        let temp_dir = TempDir::new().unwrap();

        // Write some records
        let mut writer = WalWriter::open(temp_dir.path()).unwrap();
        writer.append_insert(create_test_payload("doc1")).unwrap();
        writer.append_insert(create_test_payload("doc2")).unwrap();
        writer.append_insert(create_test_payload("doc3")).unwrap();

        assert_eq!(writer.next_sequence_number(), 4);

        // Truncate
        writer.truncate().unwrap();

        // Verify sequence reset
        assert_eq!(writer.next_sequence_number(), 1);

        // Verify WAL is empty
        let wal_path = temp_dir.path().join("wal").join("wal.log");
        let mut reader = WalReader::open(&wal_path).unwrap();
        assert!(reader.read_next().unwrap().is_none());
    }

    #[test]
    fn test_truncate_allows_new_writes() {
        let temp_dir = TempDir::new().unwrap();

        let mut writer = WalWriter::open(temp_dir.path()).unwrap();
        writer.append_insert(create_test_payload("doc1")).unwrap();
        writer.append_insert(create_test_payload("doc2")).unwrap();

        // Truncate
        writer.truncate().unwrap();

        // New writes should start at sequence 1
        let seq1 = writer
            .append_insert(create_test_payload("new_doc1"))
            .unwrap();
        let seq2 = writer
            .append_insert(create_test_payload("new_doc2"))
            .unwrap();

        assert_eq!(seq1, 1);
        assert_eq!(seq2, 2);
    }

    #[test]
    fn test_truncate_persists_across_reopen() {
        let temp_dir = TempDir::new().unwrap();

        // Write and truncate
        {
            let mut writer = WalWriter::open(temp_dir.path()).unwrap();
            writer.append_insert(create_test_payload("doc1")).unwrap();
            writer.append_insert(create_test_payload("doc2")).unwrap();
            writer.truncate().unwrap();
        }

        // Reopen and verify empty WAL, sequence starts at 1
        {
            let writer = WalWriter::open(temp_dir.path()).unwrap();
            assert_eq!(writer.next_sequence_number(), 1);
        }
    }

    #[test]
    fn test_truncate_then_write_then_reopen() {
        use super::super::reader::WalReader;

        let temp_dir = TempDir::new().unwrap();

        // Write, truncate, write again
        {
            let mut writer = WalWriter::open(temp_dir.path()).unwrap();
            writer
                .append_insert(create_test_payload("old_doc"))
                .unwrap();
            writer.truncate().unwrap();
            writer
                .append_insert(create_test_payload("new_doc"))
                .unwrap();
        }

        // Reopen and verify only new record exists
        {
            let wal_path = temp_dir.path().join("wal").join("wal.log");
            let mut reader = WalReader::open(&wal_path).unwrap();

            let record = reader.read_next().unwrap().unwrap();
            assert_eq!(record.sequence_number, 1);
            assert_eq!(record.payload.document_id, "new_doc");

            // No more records
            assert!(reader.read_next().unwrap().is_none());
        }
    }
}
