//! Recovery Manager startup sequence
//!
//! Orchestrates the full recovery process per WAL.md and STORAGE.md.
//!
//! # Startup Sequence (strict order)
//!
//! 1. Load schemas via schema loader
//! 2. Open WAL reader
//! 3. Open document storage
//! 4. Replay WAL from offset 0 sequentially
//! 5. Apply each WAL record via storage.apply_wal_record
//! 6. After replay completes, call index.rebuild_from_storage
//! 7. Run consistency verification
//! 8. Enter serving state

use std::fs;
use std::path::{Path, PathBuf};

use super::errors::{RecoveryError, RecoveryResult};
use super::replay::{ReplayStats, StorageApply, WalRead, WalReplayer};
use super::verifier::{ConsistencyVerifier, SchemaCheck, StorageScan, VerificationStats};

/// Clean shutdown marker filename
const CLEAN_SHUTDOWN_MARKER: &str = "clean_shutdown";

/// Trait for index rebuild
pub trait IndexRebuild {
    /// Rebuild indexes from storage
    fn rebuild_from_storage(&mut self) -> RecoveryResult<()>;
}

/// Recovery state after successful startup
#[derive(Debug, Clone)]
pub struct RecoveryState {
    /// WAL replay statistics
    pub replay_stats: ReplayStats,
    /// Verification statistics
    pub verification_stats: VerificationStats,
    /// Whether clean shutdown marker was present
    pub was_clean_shutdown: bool,
}

/// Recovery Manager that orchestrates startup
pub struct RecoveryManager {
    data_dir: PathBuf,
}

impl RecoveryManager {
    /// Creates a new recovery manager
    pub fn new(data_dir: impl AsRef<Path>) -> Self {
        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
        }
    }

    /// Returns the path to the clean shutdown marker
    fn marker_path(&self) -> PathBuf {
        self.data_dir.join(CLEAN_SHUTDOWN_MARKER)
    }

    /// Check if clean shutdown marker exists
    pub fn was_clean_shutdown(&self) -> bool {
        self.marker_path().exists()
    }

    /// Remove clean shutdown marker (called during startup)
    fn remove_shutdown_marker(&self) -> RecoveryResult<()> {
        let path = self.marker_path();
        if path.exists() {
            fs::remove_file(&path).map_err(|e| {
                RecoveryError::recovery_failed(format!("Failed to remove shutdown marker: {}", e))
            })?;
        }
        Ok(())
    }

    /// Write clean shutdown marker (called on graceful shutdown)
    pub fn mark_clean_shutdown(&self) -> RecoveryResult<()> {
        let path = self.marker_path();

        // Ensure data directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                RecoveryError::recovery_failed(format!("Failed to create data directory: {}", e))
            })?;
        }

        fs::write(&path, b"").map_err(|e| {
            RecoveryError::recovery_failed(format!("Failed to write shutdown marker: {}", e))
        })?;

        Ok(())
    }

    /// Execute the full recovery sequence.
    ///
    /// Steps (must be exact order):
    /// 1. Check for clean shutdown marker
    /// 2. Replay WAL from offset 0
    /// 3. Rebuild indexes
    /// 4. Verify consistency
    /// 5. Remove shutdown marker
    ///
    /// Returns RecoveryState on success, FATAL error on any failure.
    pub fn recover<W, S, I, C>(
        &self,
        wal: &mut W,
        storage: &mut S,
        index: &mut I,
        schema_registry: &C,
    ) -> RecoveryResult<RecoveryState>
    where
        W: WalRead,
        S: StorageApply + StorageScan,
        I: IndexRebuild,
        C: SchemaCheck,
    {
        // Step 1: Check for clean shutdown marker
        let was_clean_shutdown = self.was_clean_shutdown();

        // Step 2: Replay WAL (always replay in Phase 0, even after clean shutdown)
        let replay_stats = WalReplayer::replay(wal, storage)?;

        // Step 3: Rebuild indexes from storage
        index.rebuild_from_storage()?;

        // Step 4: Verify consistency
        let verification_stats = ConsistencyVerifier::verify(storage, schema_registry)?;

        // Step 5: Remove shutdown marker
        self.remove_shutdown_marker()?;

        Ok(RecoveryState {
            replay_stats,
            verification_stats,
            was_clean_shutdown,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wal::{RecordType, WalPayload, WalRecord};
    use std::collections::HashSet;
    use tempfile::TempDir;

    // Mock implementations for testing

    struct MockWal {
        records: Vec<WalRecord>,
        position: usize,
    }

    impl MockWal {
        fn new(records: Vec<WalRecord>) -> Self {
            Self {
                records,
                position: 0,
            }
        }
    }

    impl WalRead for MockWal {
        fn read_next(&mut self) -> RecoveryResult<Option<WalRecord>> {
            if self.position >= self.records.len() {
                return Ok(None);
            }
            let record = self.records[self.position].clone();
            self.position += 1;
            Ok(Some(record))
        }

        fn current_offset(&self) -> u64 {
            self.position as u64 * 100
        }

        fn reset(&mut self) -> RecoveryResult<()> {
            self.position = 0;
            Ok(())
        }
    }

    struct MockStorage {
        applied_records: Vec<WalRecord>,
        scan_records: Vec<super::super::verifier::StorageRecordInfo>,
        scan_position: usize,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                applied_records: Vec::new(),
                scan_records: Vec::new(),
                scan_position: 0,
            }
        }
    }

    impl StorageApply for MockStorage {
        fn apply_wal_record(&mut self, record: &WalRecord) -> RecoveryResult<()> {
            self.applied_records.push(record.clone());

            // Also add to scan records based on record type
            let is_tombstone = record.record_type == RecordType::Delete;
            self.scan_records
                .push(super::super::verifier::StorageRecordInfo {
                    document_id: record.payload.document_id.clone(),
                    schema_id: record.payload.schema_id.clone(),
                    schema_version: record.payload.schema_version.clone(),
                    offset: self.scan_records.len() as u64 * 100,
                    is_tombstone,
                });
            Ok(())
        }
    }

    impl StorageScan for MockStorage {
        fn scan_next(
            &mut self,
        ) -> RecoveryResult<Option<super::super::verifier::StorageRecordInfo>> {
            if self.scan_position >= self.scan_records.len() {
                return Ok(None);
            }
            let record = self.scan_records[self.scan_position].clone();
            self.scan_position += 1;
            Ok(Some(record))
        }

        fn reset(&mut self) -> RecoveryResult<()> {
            self.scan_position = 0;
            Ok(())
        }
    }

    struct MockIndex {
        rebuild_called: bool,
    }

    impl MockIndex {
        fn new() -> Self {
            Self {
                rebuild_called: false,
            }
        }
    }

    impl IndexRebuild for MockIndex {
        fn rebuild_from_storage(&mut self) -> RecoveryResult<()> {
            self.rebuild_called = true;
            Ok(())
        }
    }

    struct MockSchemaRegistry {
        schemas: HashSet<(String, String)>,
    }

    impl MockSchemaRegistry {
        fn new() -> Self {
            let mut schemas = HashSet::new();
            schemas.insert(("users".to_string(), "v1".to_string()));
            Self { schemas }
        }
    }

    impl SchemaCheck for MockSchemaRegistry {
        fn schema_exists(&self, schema_id: &str) -> bool {
            self.schemas.iter().any(|(id, _)| id == schema_id)
        }

        fn schema_version_exists(&self, schema_id: &str, version: &str) -> bool {
            self.schemas
                .contains(&(schema_id.to_string(), version.to_string()))
        }
    }

    fn make_insert_record(seq: u64, id: &str) -> WalRecord {
        WalRecord::insert(
            seq,
            WalPayload::new("users", id, "users", "v1", b"{}".to_vec()),
        )
    }

    #[test]
    fn test_full_recovery() {
        let temp_dir = TempDir::new().unwrap();
        let manager = RecoveryManager::new(temp_dir.path());

        let records = vec![
            make_insert_record(1, "user_1"),
            make_insert_record(2, "user_2"),
        ];

        let mut wal = MockWal::new(records);
        let mut storage = MockStorage::new();
        let mut index = MockIndex::new();
        let schema = MockSchemaRegistry::new();

        let state = manager
            .recover(&mut wal, &mut storage, &mut index, &schema)
            .unwrap();

        assert_eq!(state.replay_stats.records_replayed, 2);
        assert!(index.rebuild_called);
        assert_eq!(state.verification_stats.live_documents, 2);
    }

    #[test]
    fn test_index_rebuilt_after_replay() {
        let temp_dir = TempDir::new().unwrap();
        let manager = RecoveryManager::new(temp_dir.path());

        let mut wal = MockWal::new(vec![make_insert_record(1, "user_1")]);
        let mut storage = MockStorage::new();
        let mut index = MockIndex::new();
        let schema = MockSchemaRegistry::new();

        assert!(!index.rebuild_called);

        manager
            .recover(&mut wal, &mut storage, &mut index, &schema)
            .unwrap();

        assert!(index.rebuild_called);
    }

    #[test]
    fn test_clean_shutdown_marker() {
        let temp_dir = TempDir::new().unwrap();
        let manager = RecoveryManager::new(temp_dir.path());

        // Initially no marker
        assert!(!manager.was_clean_shutdown());

        // Write marker
        manager.mark_clean_shutdown().unwrap();
        assert!(manager.was_clean_shutdown());

        // Recovery should detect and remove marker
        let mut wal = MockWal::new(vec![]);
        let mut storage = MockStorage::new();
        let mut index = MockIndex::new();
        let schema = MockSchemaRegistry::new();

        let state = manager
            .recover(&mut wal, &mut storage, &mut index, &schema)
            .unwrap();

        assert!(state.was_clean_shutdown);
        assert!(!manager.was_clean_shutdown()); // Marker removed
    }

    #[test]
    fn test_replay_restores_documents() {
        let temp_dir = TempDir::new().unwrap();
        let manager = RecoveryManager::new(temp_dir.path());

        let records = vec![
            make_insert_record(1, "user_1"),
            make_insert_record(2, "user_2"),
            make_insert_record(3, "user_3"),
        ];

        let mut wal = MockWal::new(records);
        let mut storage = MockStorage::new();
        let mut index = MockIndex::new();
        let schema = MockSchemaRegistry::new();

        manager
            .recover(&mut wal, &mut storage, &mut index, &schema)
            .unwrap();

        // All records should be applied to storage
        assert_eq!(storage.applied_records.len(), 3);
    }

    #[test]
    fn test_replay_idempotency() {
        let temp_dir = TempDir::new().unwrap();
        let manager = RecoveryManager::new(temp_dir.path());

        let records = vec![make_insert_record(1, "user_1")];

        // First run
        let mut wal1 = MockWal::new(records.clone());
        let mut storage1 = MockStorage::new();
        let mut index1 = MockIndex::new();
        let schema = MockSchemaRegistry::new();
        let state1 = manager
            .recover(&mut wal1, &mut storage1, &mut index1, &schema)
            .unwrap();

        // Second run (same WAL)
        let mut wal2 = MockWal::new(records);
        let mut storage2 = MockStorage::new();
        let mut index2 = MockIndex::new();
        let state2 = manager
            .recover(&mut wal2, &mut storage2, &mut index2, &schema)
            .unwrap();

        // Results must be identical
        assert_eq!(
            state1.replay_stats.records_replayed,
            state2.replay_stats.records_replayed
        );
        assert_eq!(
            storage1.applied_records.len(),
            storage2.applied_records.len()
        );
    }
}
