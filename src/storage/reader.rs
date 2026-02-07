//! Storage reader with strict corruption detection per STORAGE.md
//!
//! Per STORAGE.md §6.4:
//! - Every read validates checksum
//! - Storage is accessed by primary key lookup, recovery, and index rebuild
//!
//! Per STORAGE.md §11:
//! - Any checksum failure on read → operation abort
//! - During recovery → startup abort

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use super::errors::{StorageError, StorageResult};
use super::record::DocumentRecord;

/// Storage reader for sequential scans and primary key lookups.
///
/// Validates checksums on every read. Any corruption is fatal.
pub struct StorageReader {
    /// Path to the storage file
    storage_path: PathBuf,
    /// Buffered reader
    reader: BufReader<File>,
    /// Current byte offset
    current_offset: u64,
    /// Total file size
    file_size: u64,
}

impl StorageReader {
    /// Opens the storage file for reading.
    pub fn open(storage_path: &Path) -> StorageResult<Self> {
        let file = File::open(storage_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::data_corruption(format!(
                    "Storage file not found: {}",
                    storage_path.display()
                ))
            } else {
                StorageError::read_failed(
                    format!("Failed to open storage file: {}", storage_path.display()),
                    e,
                )
            }
        })?;

        let file_size = file
            .metadata()
            .map_err(|e| StorageError::read_failed("Failed to read file metadata", e))?
            .len();

        Ok(Self {
            storage_path: storage_path.to_path_buf(),
            reader: BufReader::new(file),
            current_offset: 0,
            file_size,
        })
    }

    /// Opens storage from data directory.
    pub fn open_from_data_dir(data_dir: &Path) -> StorageResult<Self> {
        let storage_path = data_dir.join("data").join("documents.dat");
        Self::open(&storage_path)
    }

    /// Returns the storage file path.
    pub fn path(&self) -> &Path {
        &self.storage_path
    }

    /// Returns the current read offset.
    pub fn current_offset(&self) -> u64 {
        self.current_offset
    }

    /// Returns whether there are more records to read.
    pub fn has_more(&self) -> bool {
        self.current_offset < self.file_size
    }

    /// Reads the next record from storage.
    ///
    /// Validates checksum on read. Any corruption is fatal.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(record))` if a record was read
    /// - `Ok(None)` if end of file
    /// - `Err(AERO_DATA_CORRUPTION)` if checksum fails (FATAL)
    pub fn read_next(&mut self) -> StorageResult<Option<DocumentRecord>> {
        if self.current_offset >= self.file_size {
            return Ok(None);
        }

        let remaining = self.file_size - self.current_offset;
        const MIN_RECORD_SIZE: u64 = 4 + 4 + 4 + 4 + 1 + 4 + 4;

        if remaining < MIN_RECORD_SIZE {
            return Err(StorageError::corruption_at_offset(
                self.current_offset,
                format!(
                    "Truncated storage: {} bytes remaining, minimum record size is {}",
                    remaining, MIN_RECORD_SIZE
                ),
            ));
        }

        // Read record length
        let mut len_buf = [0u8; 4];
        self.reader.read_exact(&mut len_buf).map_err(|e| {
            StorageError::corruption_at_offset(
                self.current_offset,
                format!("Failed to read record length: {}", e),
            )
        })?;
        let record_length = u32::from_le_bytes(len_buf) as u64;

        if record_length < MIN_RECORD_SIZE {
            return Err(StorageError::corruption_at_offset(
                self.current_offset,
                format!("Invalid record length: {}", record_length),
            ));
        }

        if record_length > remaining {
            return Err(StorageError::corruption_at_offset(
                self.current_offset,
                format!(
                    "Record length {} exceeds remaining file size {}",
                    record_length, remaining
                ),
            ));
        }

        // Read full record
        let mut record_buf = vec![0u8; record_length as usize];
        record_buf[0..4].copy_from_slice(&len_buf);

        self.reader.read_exact(&mut record_buf[4..]).map_err(|e| {
            StorageError::corruption_at_offset(
                self.current_offset,
                format!("Failed to read record body: {}", e),
            )
        })?;

        // Parse and validate (includes checksum verification)
        let (record, bytes_consumed) = DocumentRecord::deserialize(&record_buf)
            .map_err(|e| StorageError::corruption_at_offset(self.current_offset, e.to_string()))?;

        self.current_offset += bytes_consumed as u64;

        Ok(Some(record))
    }

    /// Reads all records from storage.
    ///
    /// Any corruption causes immediate failure.
    pub fn read_all(&mut self) -> StorageResult<Vec<DocumentRecord>> {
        let mut records = Vec::new();

        loop {
            match self.read_next()? {
                Some(record) => records.push(record),
                None => break,
            }
        }

        Ok(records)
    }

    /// Seeks to a specific offset in the file.
    pub fn seek_to(&mut self, offset: u64) -> StorageResult<()> {
        self.reader.seek(SeekFrom::Start(offset)).map_err(|e| {
            StorageError::read_failed(format!("Failed to seek to offset {}", offset), e)
        })?;
        self.current_offset = offset;
        Ok(())
    }

    /// Reads a single record at the specified offset.
    ///
    /// Validates checksum. Returns AERO_DATA_CORRUPTION if invalid.
    pub fn read_at(&mut self, offset: u64) -> StorageResult<DocumentRecord> {
        self.seek_to(offset)?;
        match self.read_next()? {
            Some(record) => Ok(record),
            None => Err(StorageError::corruption_at_offset(
                offset,
                "No record at specified offset",
            )),
        }
    }

    /// Resets reader to beginning of file.
    pub fn reset(&mut self) -> StorageResult<()> {
        self.seek_to(0)
    }

    /// Finds the latest record for a document by sequential scan.
    ///
    /// This is O(n) but acceptable for Phase 0.
    /// Latest record (by file order) wins.
    pub fn find_latest(&mut self, composite_id: &str) -> StorageResult<Option<DocumentRecord>> {
        self.reset()?;

        let mut latest: Option<DocumentRecord> = None;

        loop {
            match self.read_next()? {
                Some(record) => {
                    if record.document_id == composite_id {
                        latest = Some(record);
                    }
                }
                None => break,
            }
        }

        Ok(latest)
    }

    /// Builds a map of document_id -> latest record by scanning the file.
    ///
    /// This resolves overwrites: only the latest record per document is returned.
    pub fn build_document_map(
        &mut self,
    ) -> StorageResult<std::collections::HashMap<String, DocumentRecord>> {
        use std::collections::HashMap;

        self.reset()?;

        let mut map: HashMap<String, DocumentRecord> = HashMap::new();

        loop {
            match self.read_next()? {
                Some(record) => {
                    // Latest wins
                    map.insert(record.document_id.clone(), record);
                }
                None => break,
            }
        }

        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::super::record::StoragePayload;
    use super::super::writer::StorageWriter;
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
    fn test_read_empty_file() {
        let temp_dir = TempDir::new().unwrap();

        // Create empty storage
        {
            let _writer = StorageWriter::open(temp_dir.path()).unwrap();
        }

        let storage_path = temp_dir.path().join("data").join("documents.dat");
        let mut reader = StorageReader::open(&storage_path).unwrap();

        assert!(reader.read_next().unwrap().is_none());
    }

    #[test]
    fn test_read_single_record() {
        let temp_dir = TempDir::new().unwrap();

        {
            let mut writer = StorageWriter::open(temp_dir.path()).unwrap();
            writer.write(&create_test_payload("doc1")).unwrap();
        }

        let storage_path = temp_dir.path().join("data").join("documents.dat");
        let mut reader = StorageReader::open(&storage_path).unwrap();

        let record = reader.read_next().unwrap().unwrap();
        assert_eq!(record.document_id, "test_collection:doc1");
        assert!(!record.is_tombstone);

        assert!(reader.read_next().unwrap().is_none());
    }

    #[test]
    fn test_read_multiple_records() {
        let temp_dir = TempDir::new().unwrap();

        {
            let mut writer = StorageWriter::open(temp_dir.path()).unwrap();
            writer.write(&create_test_payload("doc1")).unwrap();
            writer.write(&create_test_payload("doc2")).unwrap();
            writer.write(&create_test_payload("doc3")).unwrap();
        }

        let storage_path = temp_dir.path().join("data").join("documents.dat");
        let mut reader = StorageReader::open(&storage_path).unwrap();

        let records = reader.read_all().unwrap();
        assert_eq!(records.len(), 3);
    }

    #[test]
    fn test_corruption_detected() {
        let temp_dir = TempDir::new().unwrap();

        {
            let mut writer = StorageWriter::open(temp_dir.path()).unwrap();
            writer.write(&create_test_payload("doc1")).unwrap();
        }

        // Corrupt the file
        let storage_path = temp_dir.path().join("data").join("documents.dat");
        {
            use std::fs::OpenOptions;
            use std::io::{Seek, SeekFrom, Write};

            let mut file = OpenOptions::new().write(true).open(&storage_path).unwrap();
            file.seek(SeekFrom::Start(10)).unwrap();
            file.write_all(&[0xFF]).unwrap();
        }

        let mut reader = StorageReader::open(&storage_path).unwrap();
        let result = reader.read_next();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_fatal());
        assert_eq!(err.code().code(), "AERO_DATA_CORRUPTION");
    }

    #[test]
    fn test_find_latest_overwrites() {
        let temp_dir = TempDir::new().unwrap();

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
            writer
                .write(&StoragePayload::new(
                    "users",
                    "doc1",
                    "schema",
                    "v1",
                    b"third".to_vec(),
                ))
                .unwrap();
        }

        let storage_path = temp_dir.path().join("data").join("documents.dat");
        let mut reader = StorageReader::open(&storage_path).unwrap();

        let latest = reader.find_latest("users:doc1").unwrap().unwrap();
        assert_eq!(latest.document_body, b"third");
    }

    #[test]
    fn test_build_document_map() {
        let temp_dir = TempDir::new().unwrap();

        {
            let mut writer = StorageWriter::open(temp_dir.path()).unwrap();
            writer.write(&create_test_payload("doc1")).unwrap();
            writer.write(&create_test_payload("doc2")).unwrap();
            writer
                .write(&StoragePayload::new(
                    "test_collection",
                    "doc1",
                    "test_schema",
                    "v1",
                    b"updated".to_vec(),
                ))
                .unwrap();
        }

        let storage_path = temp_dir.path().join("data").join("documents.dat");
        let mut reader = StorageReader::open(&storage_path).unwrap();

        let map = reader.build_document_map().unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(
            map.get("test_collection:doc1").unwrap().document_body,
            b"updated"
        );
    }

    #[test]
    fn test_read_at_offset() {
        let temp_dir = TempDir::new().unwrap();

        let offset2;
        {
            let mut writer = StorageWriter::open(temp_dir.path()).unwrap();
            writer.write(&create_test_payload("doc1")).unwrap();
            offset2 = writer.write(&create_test_payload("doc2")).unwrap();
        }

        let storage_path = temp_dir.path().join("data").join("documents.dat");
        let mut reader = StorageReader::open(&storage_path).unwrap();

        let record = reader.read_at(offset2).unwrap();
        assert_eq!(record.document_id, "test_collection:doc2");
    }

    #[test]
    fn test_tombstone_in_document_map() {
        let temp_dir = TempDir::new().unwrap();

        {
            let mut writer = StorageWriter::open(temp_dir.path()).unwrap();
            writer.write(&create_test_payload("doc1")).unwrap();
            writer
                .write_tombstone("test_collection", "doc1", "test_schema", "v1")
                .unwrap();
        }

        let storage_path = temp_dir.path().join("data").join("documents.dat");
        let mut reader = StorageReader::open(&storage_path).unwrap();

        let map = reader.build_document_map().unwrap();
        let record = map.get("test_collection:doc1").unwrap();
        assert!(record.is_tombstone);
    }
}
