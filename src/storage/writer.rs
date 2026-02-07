//! Storage writer with fsync enforcement per STORAGE.md
//!
//! Per STORAGE.md §6.3:
//! - Document record is written after WAL fsync
//! - Storage write failure after WAL fsync: startup recovery will reapply WAL
//! - Operation must not be acknowledged unless storage write completes
//!
//! The storage is append-only with no in-place updates (§6.1).

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use super::errors::{StorageError, StorageResult};
use super::record::{DocumentRecord, StoragePayload};
use crate::wal::WalRecord;

/// Storage writer that maintains the documents.dat file.
///
/// This is an append-only writer with fsync after every write.
/// Multiple records for the same document_id may exist; latest wins.
pub struct StorageWriter {
    /// Path to the storage file
    storage_path: PathBuf,
    /// Underlying file handle
    file: File,
    /// Current file offset (for tracking)
    current_offset: u64,
    /// In-memory index of document_id -> latest offset (for lookups)
    /// This is rebuilt on startup and maintained during writes
    document_offsets: HashMap<String, u64>,
}

impl StorageWriter {
    /// Opens or creates the storage file at the specified data directory.
    ///
    /// Creates `<data_dir>/data/documents.dat` if it does not exist.
    /// Creates parent directories if needed.
    ///
    /// # Arguments
    ///
    /// * `data_dir` - The root data directory
    ///
    /// # Errors
    ///
    /// Returns `StorageError::write_failed` if the file cannot be created or opened.
    pub fn open(data_dir: &Path) -> StorageResult<Self> {
        let data_subdir = data_dir.join("data");
        let storage_path = data_subdir.join("documents.dat");

        // Create directories if missing
        if !data_subdir.exists() {
            fs::create_dir_all(&data_subdir).map_err(|e| {
                StorageError::write_failed(
                    format!("Failed to create data directory: {}", data_subdir.display()),
                    e,
                )
            })?;
        }

        // Open file for append
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&storage_path)
            .map_err(|e| {
                StorageError::write_failed(
                    format!("Failed to open storage file: {}", storage_path.display()),
                    e,
                )
            })?;

        let current_offset = file
            .metadata()
            .map_err(|e| StorageError::write_failed("Failed to read file metadata", e))?
            .len();

        // Build in-memory index by scanning existing records
        let document_offsets = Self::build_offset_index(&storage_path)?;

        Ok(Self {
            storage_path,
            file,
            current_offset,
            document_offsets,
        })
    }

    /// Builds the in-memory offset index by scanning the storage file.
    fn build_offset_index(storage_path: &Path) -> StorageResult<HashMap<String, u64>> {
        use super::reader::StorageReader;

        let mut offsets = HashMap::new();

        // If file doesn't exist or is empty, return empty map
        let metadata = match fs::metadata(storage_path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(offsets),
            Err(e) => {
                return Err(StorageError::read_failed(
                    "Failed to read storage metadata",
                    e,
                ))
            }
        };

        if metadata.len() == 0 {
            return Ok(offsets);
        }

        // Scan all records to build index
        let mut reader = StorageReader::open(storage_path)?;
        loop {
            let offset = reader.current_offset();
            match reader.read_next() {
                Ok(Some(record)) => {
                    // Latest record wins (by file order)
                    offsets.insert(record.document_id.clone(), offset);
                }
                Ok(None) => break,
                Err(e) => return Err(e),
            }
        }

        Ok(offsets)
    }

    /// Returns the path to the storage file.
    pub fn path(&self) -> &Path {
        &self.storage_path
    }

    /// Returns the current file offset.
    pub fn current_offset(&self) -> u64 {
        self.current_offset
    }

    /// Returns the number of unique documents (excluding tombstones).
    pub fn document_count(&self) -> usize {
        self.document_offsets.len()
    }

    /// Writes a document record to storage with fsync enforcement.
    ///
    /// # Arguments
    ///
    /// * `payload` - The storage payload to write
    ///
    /// # Returns
    ///
    /// The byte offset where the record was written.
    ///
    /// # Errors
    ///
    /// Returns `AERO_STORAGE_WRITE_FAILED` if write or fsync fails.
    pub fn write(&mut self, payload: &StoragePayload) -> StorageResult<u64> {
        let record = DocumentRecord::from_payload(payload);
        let serialized = record.serialize();
        let offset = self.current_offset;

        // Write to file
        self.file.write_all(&serialized).map_err(|e| {
            StorageError::write_failed(
                format!("Failed to write document: {}", record.document_id),
                e,
            )
        })?;

        // fsync - mandatory for durability
        self.file.sync_all().map_err(|e| {
            StorageError::write_failed(
                format!(
                    "fsync failed after writing document: {}",
                    record.document_id
                ),
                e,
            )
        })?;

        // Update offset tracking
        self.current_offset += serialized.len() as u64;

        // Update in-memory index (latest record wins)
        self.document_offsets
            .insert(record.document_id.clone(), offset);

        Ok(offset)
    }

    /// Writes a tombstone (DELETE) record.
    ///
    /// Tombstones are preserved forever in Phase 0.
    pub fn write_tombstone(
        &mut self,
        collection_id: &str,
        document_id: &str,
        schema_id: &str,
        schema_version: &str,
    ) -> StorageResult<u64> {
        let payload =
            StoragePayload::tombstone(collection_id, document_id, schema_id, schema_version);
        self.write(&payload)
    }

    /// Applies a WAL record to storage.
    ///
    /// This is the recovery replay hook. It writes the document/tombstone
    /// exactly as the WAL instructs.
    ///
    /// # Idempotency
    ///
    /// This operation is idempotent: applying the same WAL record twice
    /// results in the same final state (the later write is simply appended,
    /// and latest record wins during reads).
    pub fn apply_wal_record(&mut self, wal_record: &WalRecord) -> StorageResult<u64> {
        let payload = StoragePayload::from_wal_record(wal_record);
        self.write(&payload)
    }

    /// Returns the offset for a document, if it exists.
    pub fn get_document_offset(&self, composite_id: &str) -> Option<u64> {
        self.document_offsets.get(composite_id).copied()
    }

    /// Returns whether a document exists (not tombstoned).
    ///
    /// Note: This only checks if there's a record. The actual tombstone
    /// status must be checked by reading the record.
    pub fn has_document(&self, composite_id: &str) -> bool {
        self.document_offsets.contains_key(composite_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_payload(doc_id: &str) -> StoragePayload {
        StoragePayload::new(
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
        let data_path = temp_dir.path().join("data");

        assert!(!data_path.exists());

        let _writer = StorageWriter::open(temp_dir.path()).unwrap();

        assert!(data_path.exists());
        assert!(data_path.join("documents.dat").exists());
    }

    #[test]
    fn test_write_and_read_back() {
        use super::super::reader::StorageReader;

        let temp_dir = TempDir::new().unwrap();

        // Write
        {
            let mut writer = StorageWriter::open(temp_dir.path()).unwrap();
            writer.write(&create_test_payload("doc1")).unwrap();
        }

        // Read back
        {
            let storage_path = temp_dir.path().join("data").join("documents.dat");
            let mut reader = StorageReader::open(&storage_path).unwrap();

            let record = reader.read_next().unwrap().unwrap();
            assert_eq!(record.document_id, "test_collection:doc1");
            assert!(!record.is_tombstone);
        }
    }

    #[test]
    fn test_tombstone_write() {
        use super::super::reader::StorageReader;

        let temp_dir = TempDir::new().unwrap();

        {
            let mut writer = StorageWriter::open(temp_dir.path()).unwrap();
            writer
                .write_tombstone("users", "user_123", "user_schema", "v1")
                .unwrap();
        }

        {
            let storage_path = temp_dir.path().join("data").join("documents.dat");
            let mut reader = StorageReader::open(&storage_path).unwrap();

            let record = reader.read_next().unwrap().unwrap();
            assert!(record.is_tombstone);
            assert!(record.document_body.is_empty());
        }
    }

    #[test]
    fn test_overwrite_semantics() {
        use super::super::reader::StorageReader;

        let temp_dir = TempDir::new().unwrap();

        // Write same document twice
        {
            let mut writer = StorageWriter::open(temp_dir.path()).unwrap();
            writer
                .write(&StoragePayload::new(
                    "users",
                    "doc1",
                    "schema",
                    "v1",
                    b"first".to_vec(),
                ))
                .unwrap();
            writer
                .write(&StoragePayload::new(
                    "users",
                    "doc1",
                    "schema",
                    "v1",
                    b"second".to_vec(),
                ))
                .unwrap();
        }

        // Both records exist in file
        {
            let storage_path = temp_dir.path().join("data").join("documents.dat");
            let mut reader = StorageReader::open(&storage_path).unwrap();

            let records = reader.read_all().unwrap();
            assert_eq!(records.len(), 2);

            // Latest (by file order) wins
            assert_eq!(records[1].document_body, b"second");
        }
    }

    #[test]
    fn test_apply_wal_record() {
        let temp_dir = TempDir::new().unwrap();

        let wal_payload =
            crate::wal::WalPayload::new("users", "user_123", "schema", "v1", b"doc body".to_vec());
        let wal_record = crate::wal::WalRecord::insert(1, wal_payload);

        {
            let mut writer = StorageWriter::open(temp_dir.path()).unwrap();
            writer.apply_wal_record(&wal_record).unwrap();
        }

        // Verify
        use super::super::reader::StorageReader;
        let storage_path = temp_dir.path().join("data").join("documents.dat");
        let mut reader = StorageReader::open(&storage_path).unwrap();

        let record = reader.read_next().unwrap().unwrap();
        assert_eq!(record.document_id, "users:user_123");
    }

    #[test]
    fn test_replay_idempotency() {
        let temp_dir = TempDir::new().unwrap();

        let wal_payload =
            crate::wal::WalPayload::new("users", "user_123", "schema", "v1", b"doc body".to_vec());
        let wal_record = crate::wal::WalRecord::insert(1, wal_payload);

        // Apply same WAL record twice
        {
            let mut writer = StorageWriter::open(temp_dir.path()).unwrap();
            writer.apply_wal_record(&wal_record).unwrap();
            writer.apply_wal_record(&wal_record).unwrap();
        }

        // Both records written, but latest wins semantically
        use super::super::reader::StorageReader;
        let storage_path = temp_dir.path().join("data").join("documents.dat");
        let mut reader = StorageReader::open(&storage_path).unwrap();

        let records = reader.read_all().unwrap();
        assert_eq!(records.len(), 2);

        // Both have identical content (idempotent)
        assert_eq!(records[0].document_body, records[1].document_body);
    }

    #[test]
    fn test_offset_tracking() {
        let temp_dir = TempDir::new().unwrap();

        let mut writer = StorageWriter::open(temp_dir.path()).unwrap();
        assert_eq!(writer.current_offset(), 0);

        let offset1 = writer.write(&create_test_payload("doc1")).unwrap();
        assert_eq!(offset1, 0);
        assert!(writer.current_offset() > 0);

        let offset2 = writer.write(&create_test_payload("doc2")).unwrap();
        assert!(offset2 > offset1);
    }

    #[test]
    fn test_reopens_with_correct_state() {
        let temp_dir = TempDir::new().unwrap();

        // Write records
        {
            let mut writer = StorageWriter::open(temp_dir.path()).unwrap();
            writer.write(&create_test_payload("doc1")).unwrap();
            writer.write(&create_test_payload("doc2")).unwrap();
        }

        // Reopen and continue
        {
            let mut writer = StorageWriter::open(temp_dir.path()).unwrap();
            assert!(writer.current_offset() > 0);

            // Index rebuilt correctly
            assert!(writer.has_document("test_collection:doc1"));
            assert!(writer.has_document("test_collection:doc2"));

            // Can continue writing
            writer.write(&create_test_payload("doc3")).unwrap();
        }
    }
}
