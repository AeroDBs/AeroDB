//! WAL replay for recovery
//!
//! Replays WAL records sequentially from byte 0 to restore state.
//!
//! Per WAL.md:
//! - Must start at byte 0 (no checkpoints in Phase 0)
//! - Must read sequentially
//! - Must validate checksum for every record
//! - On ANY corruption: FATAL error, abort immediately

use crate::wal::{RecordType, WalPayload, WalRecord};

use super::errors::{RecoveryError, RecoveryResult};

/// Trait for applying WAL records to storage
pub trait StorageApply {
    /// Apply a WAL record to storage
    fn apply_wal_record(&mut self, record: &WalRecord) -> RecoveryResult<()>;
}

/// Trait for reading WAL records
pub trait WalRead {
    /// Read the next WAL record
    /// Returns None if at end of WAL
    /// Returns Err if corruption detected
    fn read_next(&mut self) -> RecoveryResult<Option<WalRecord>>;

    /// Get current byte offset in WAL
    fn current_offset(&self) -> u64;

    /// Reset to beginning of WAL
    fn reset(&mut self) -> RecoveryResult<()>;
}

/// Statistics from WAL replay
#[derive(Debug, Clone, Default)]
pub struct ReplayStats {
    /// Number of records replayed
    pub records_replayed: u64,
    /// Number of inserts
    pub inserts: u64,
    /// Number of updates
    pub updates: u64,
    /// Number of deletes
    pub deletes: u64,
    /// Number of MVCC commits
    pub mvcc_commits: u64,
    /// Number of MVCC versions
    pub mvcc_versions: u64,
    /// Number of MVCC garbage collection events
    pub mvcc_gc: u64,
    /// Final WAL offset
    pub final_offset: u64,
    /// Final sequence number
    pub final_sequence: u64,
}

/// WAL replayer that processes WAL records sequentially
pub struct WalReplayer;

impl WalReplayer {
    /// Replay all WAL records to storage.
    ///
    /// This method:
    /// 1. Starts at byte 0
    /// 2. Reads each record sequentially
    /// 3. Validates checksum on every record
    /// 4. Applies each record to storage
    /// 5. Aborts on any corruption
    ///
    /// Replay is idempotent: same WAL replayed twice produces identical state.
    pub fn replay<W: WalRead, S: StorageApply>(
        wal: &mut W,
        storage: &mut S,
    ) -> RecoveryResult<ReplayStats> {
        // Reset to beginning of WAL
        wal.reset()?;

        let mut stats = ReplayStats::default();

        loop {
            let offset_before = wal.current_offset();

            // Read next record (checksum validated by reader)
            let record = match wal.read_next() {
                Ok(Some(r)) => r,
                Ok(None) => break, // End of WAL
                Err(e) => {
                    // WAL corruption detected - abort immediately
                    return Err(RecoveryError::wal_corruption(offset_before, e.message()));
                }
            };

            // Apply to storage
            storage.apply_wal_record(&record)?;

            // Update stats based on record type
            stats.records_replayed += 1;
            stats.final_sequence = record.sequence_number;

            match record.record_type {
                RecordType::Insert => stats.inserts += 1,
                RecordType::Update => stats.updates += 1,
                RecordType::Delete => stats.deletes += 1,
                RecordType::MvccCommit => stats.mvcc_commits += 1,
                RecordType::MvccVersion => stats.mvcc_versions += 1,
                RecordType::MvccGc => stats.mvcc_gc += 1,
            }
        }

        stats.final_offset = wal.current_offset();

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockWal {
        records: Vec<WalRecord>,
        position: usize,
        offset: u64,
        corrupt_at: Option<usize>,
    }

    impl MockWal {
        fn new(records: Vec<WalRecord>) -> Self {
            Self {
                records,
                position: 0,
                offset: 0,
                corrupt_at: None,
            }
        }

        fn with_corruption_at(mut self, index: usize) -> Self {
            self.corrupt_at = Some(index);
            self
        }
    }

    impl WalRead for MockWal {
        fn read_next(&mut self) -> RecoveryResult<Option<WalRecord>> {
            if self.position >= self.records.len() {
                return Ok(None);
            }

            if self.corrupt_at == Some(self.position) {
                return Err(RecoveryError::wal_corruption(
                    self.offset,
                    "checksum mismatch",
                ));
            }

            let record = self.records[self.position].clone();
            self.position += 1;
            self.offset += 100; // Fake offset increment
            Ok(Some(record))
        }

        fn current_offset(&self) -> u64 {
            self.offset
        }

        fn reset(&mut self) -> RecoveryResult<()> {
            self.position = 0;
            self.offset = 0;
            Ok(())
        }
    }

    struct MockStorage {
        applied: Vec<WalRecord>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                applied: Vec::new(),
            }
        }
    }

    impl StorageApply for MockStorage {
        fn apply_wal_record(&mut self, record: &WalRecord) -> RecoveryResult<()> {
            self.applied.push(record.clone());
            Ok(())
        }
    }

    fn make_insert_record(seq: u64, id: &str) -> WalRecord {
        WalRecord::insert(
            seq,
            WalPayload::new("users", id, "users", "v1", b"{}".to_vec()),
        )
    }

    fn make_delete_record(seq: u64, id: &str) -> WalRecord {
        WalRecord::delete(seq, WalPayload::tombstone("users", id, "users", "v1"))
    }

    #[test]
    fn test_full_replay() {
        let records = vec![
            make_insert_record(1, "user_1"),
            make_insert_record(2, "user_2"),
            make_delete_record(3, "user_1"),
        ];

        let mut wal = MockWal::new(records);
        let mut storage = MockStorage::new();

        let stats = WalReplayer::replay(&mut wal, &mut storage).unwrap();

        assert_eq!(stats.records_replayed, 3);
        assert_eq!(stats.inserts, 2);
        assert_eq!(stats.deletes, 1);
        assert_eq!(stats.final_sequence, 3);
        assert_eq!(storage.applied.len(), 3);
    }

    #[test]
    fn test_replay_idempotency() {
        let records = vec![
            make_insert_record(1, "user_1"),
            make_insert_record(2, "user_2"),
        ];

        // First replay
        let mut wal1 = MockWal::new(records.clone());
        let mut storage1 = MockStorage::new();
        let stats1 = WalReplayer::replay(&mut wal1, &mut storage1).unwrap();

        // Second replay (same WAL)
        let mut wal2 = MockWal::new(records);
        let mut storage2 = MockStorage::new();
        let stats2 = WalReplayer::replay(&mut wal2, &mut storage2).unwrap();

        // Results must be identical
        assert_eq!(stats1.records_replayed, stats2.records_replayed);
        assert_eq!(stats1.final_sequence, stats2.final_sequence);
        assert_eq!(storage1.applied.len(), storage2.applied.len());
    }

    #[test]
    fn test_corruption_aborts_replay() {
        let records = vec![
            make_insert_record(1, "user_1"),
            make_insert_record(2, "user_2"),
            make_insert_record(3, "user_3"),
        ];

        let mut wal = MockWal::new(records).with_corruption_at(1);
        let mut storage = MockStorage::new();

        let result = WalReplayer::replay(&mut wal, &mut storage);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code().code(), "AERO_WAL_CORRUPTION");

        // Only first record should have been applied
        assert_eq!(storage.applied.len(), 1);
    }

    #[test]
    fn test_empty_wal_replay() {
        let mut wal = MockWal::new(vec![]);
        let mut storage = MockStorage::new();

        let stats = WalReplayer::replay(&mut wal, &mut storage).unwrap();

        assert_eq!(stats.records_replayed, 0);
        assert_eq!(storage.applied.len(), 0);
    }
}
