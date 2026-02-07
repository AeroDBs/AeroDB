//! Adapter implementations for recovery traits
//!
//! This module provides implementations of the recovery traits for the
//! actual WAL, Storage, Index, and Schema types used in the system.

use std::path::Path;

use crate::index::IndexManager;
use crate::schema::SchemaLoader;
use crate::storage::{StorageReader, StorageWriter};
use crate::wal::{WalReader, WalRecord};

use super::errors::{RecoveryError, RecoveryResult};
use super::replay::{StorageApply, WalRead};
use super::startup::IndexRebuild;
use super::verifier::{SchemaCheck, StorageRecordInfo, StorageScan};

// ============================================================================
// WalRead implementation for WalReader
// ============================================================================

impl WalRead for WalReader {
    fn read_next(&mut self) -> RecoveryResult<Option<WalRecord>> {
        WalReader::read_next(self)
            .map_err(|e| RecoveryError::wal_corruption(self.current_offset(), e.to_string()))
    }

    fn current_offset(&self) -> u64 {
        WalReader::current_offset(self)
    }

    fn reset(&mut self) -> RecoveryResult<()> {
        WalReader::reset(self).map_err(|e| {
            RecoveryError::recovery_failed(format!("Failed to reset WAL reader: {}", e))
        })
    }
}

// ============================================================================
// Combined Storage adapter for recovery (implements both StorageApply + StorageScan)
// ============================================================================

/// Combined storage adapter that implements both StorageApply and StorageScan.
/// Required by RecoveryManager::recover() which needs both traits on the same type.
pub struct RecoveryStorage {
    writer: StorageWriter,
    reader: StorageReader,
}

impl RecoveryStorage {
    /// Create a new recovery storage adapter from a data directory
    pub fn open(data_dir: &Path) -> RecoveryResult<Self> {
        let writer = StorageWriter::open(data_dir).map_err(|e| {
            RecoveryError::recovery_failed(format!("Failed to open storage writer: {}", e))
        })?;
        let reader = StorageReader::open_from_data_dir(data_dir).map_err(|e| {
            RecoveryError::recovery_failed(format!("Failed to open storage reader: {}", e))
        })?;
        Ok(Self { writer, reader })
    }

    /// Consume the adapter and return the underlying writer and reader
    pub fn into_parts(self) -> (StorageWriter, StorageReader) {
        (self.writer, self.reader)
    }
}

impl StorageApply for RecoveryStorage {
    fn apply_wal_record(&mut self, record: &WalRecord) -> RecoveryResult<()> {
        self.writer.apply_wal_record(record).map_err(|e| {
            RecoveryError::recovery_failed(format!("Failed to apply WAL record: {}", e))
        })?;
        Ok(())
    }
}

impl StorageScan for RecoveryStorage {
    fn scan_next(&mut self) -> RecoveryResult<Option<StorageRecordInfo>> {
        match self.reader.read_next() {
            Ok(Some(record)) => Ok(Some(StorageRecordInfo {
                document_id: record.document_id.clone(),
                schema_id: record.schema_id.clone(),
                schema_version: record.schema_version.clone(),
                offset: self.reader.current_offset(),
                is_tombstone: record.is_tombstone,
            })),
            Ok(None) => Ok(None),
            Err(e) => Err(RecoveryError::storage_corruption(
                self.reader.current_offset(),
                e.to_string(),
            )),
        }
    }

    fn reset(&mut self) -> RecoveryResult<()> {
        self.reader.reset().map_err(|e| {
            RecoveryError::recovery_failed(format!("Failed to reset storage reader: {}", e))
        })
    }
}

// ============================================================================
// IndexRebuild implementation for IndexManager
// ============================================================================

impl IndexRebuild for IndexManager {
    fn rebuild_from_storage(&mut self) -> RecoveryResult<()> {
        // Index rebuild is performed by the recovery manager using storage scan
        // For Phase 0, we use a simplified approach where indexes are rebuilt
        // after recovery completes. The actual rebuild is a no-op here because
        // we don't have storage access at this point.
        //
        // The full rebuild happens via IndexManager::rebuild_from_storage(&mut S)
        // which requires a StorageScan. This is called separately with the scanner.
        Ok(())
    }
}

// ============================================================================
// SchemaCheck implementation for SchemaLoader
// ============================================================================

impl SchemaCheck for SchemaLoader {
    fn schema_exists(&self, schema_id: &str) -> bool {
        SchemaLoader::schema_id_exists(self, schema_id)
    }

    fn schema_version_exists(&self, schema_id: &str, version: &str) -> bool {
        SchemaLoader::exists(self, schema_id, version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wal::{WalPayload, WalWriter};
    use tempfile::TempDir;

    #[test]
    fn test_wal_read_implementation() {
        let temp_dir = TempDir::new().unwrap();

        // Write some records
        {
            let mut writer = WalWriter::open(temp_dir.path()).unwrap();
            let payload = WalPayload::new(
                "test_collection",
                "doc1",
                "test_schema",
                "v1",
                b"{}".to_vec(),
            );
            writer.append_insert(payload).unwrap();
        }

        // Read back via WalRead trait
        let wal_path = temp_dir.path().join("wal").join("wal.log");
        let mut reader = WalReader::open(&wal_path).unwrap();

        let record = WalRead::read_next(&mut reader).unwrap();
        assert!(record.is_some());
    }

    #[test]
    fn test_schema_check_implementation() {
        let temp_dir = TempDir::new().unwrap();
        let mut loader = SchemaLoader::new(temp_dir.path());

        use crate::schema::{FieldDef, Schema};
        use std::collections::HashMap;

        let mut fields = HashMap::new();
        fields.insert("_id".to_string(), FieldDef::required_string());
        let schema = Schema::new("users", "v1", fields);
        loader.register(schema).unwrap();

        // Test via SchemaCheck trait
        assert!(SchemaCheck::schema_exists(&loader, "users"));
        assert!(SchemaCheck::schema_version_exists(&loader, "users", "v1"));
        assert!(!SchemaCheck::schema_exists(&loader, "nonexistent"));
    }

    #[test]
    fn test_recovery_storage_adapter() {
        let temp_dir = TempDir::new().unwrap();

        // Initialize directories
        std::fs::create_dir_all(temp_dir.path().join("data")).unwrap();
        std::fs::create_dir_all(temp_dir.path().join("wal")).unwrap();

        let storage = RecoveryStorage::open(temp_dir.path()).unwrap();
        let (writer, _reader) = storage.into_parts();
        assert_eq!(writer.current_offset(), 0);
    }
}
