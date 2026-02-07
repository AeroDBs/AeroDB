//! Checkpoint subsystem for aerodb
//!
//! Checkpoint is the ONLY mechanism that truncates WAL.
//!
//! Per CHECKPOINT.md:
//! - Checkpoint = Snapshot + WAL Reset
//! - WAL truncation is atomic
//! - Crash safety guaranteed at every stage
//!
//! # Design Principles
//!
//! - Atomicity
//! - Durability
//! - Determinism
//! - No partial success
//! - Explicit failure
//!
//! # Algorithm (CHECKPOINT.md §4)
//!
//! 1. Acquire global execution lock
//! 2. fsync WAL
//! 3. Create snapshot (per SNAPSHOT.md)
//! 4. fsync snapshot
//! 5. Write checkpoint manifest (checkpoint.json)
//! 6. Truncate WAL to zero
//! 7. fsync WAL directory
//! 8. Release global execution lock
//!
//! # Crash Safety
//!
//! Per CHECKPOINT.md §7:
//! - Crash before marker → snapshot ignored, WAL intact
//! - Crash after marker but before truncation → snapshot used, WAL replay
//! - Crash after truncation → snapshot used, WAL empty
//!
//! No scenario causes data loss.
//!
//! # Important
//!
//! Checkpoint is NOT recovery.
//! Checkpoint does NOT rebuild indexes.
//! Checkpoint orchestrates only.
//!
//! # Phase 3 Optimizations
//!
//! - Pipelining: Overlap Phase A (prep) work with normal operation (optional, disabled by default)

mod coordinator;
mod errors;
mod marker;
mod pipeline;

pub use errors::{CheckpointError, CheckpointErrorCode, CheckpointResult, Severity};
pub use marker::{marker_path, CheckpointMarker};
pub use pipeline::{
    CheckpointPath, CheckpointPipeline, CheckpointPipelineError, PhaseA, PhaseAResult, PhaseB,
    PhaseBResult, PipelineConfig, PipelineState, PipelineStats,
};

use std::path::Path;

use crate::snapshot::{GlobalExecutionLock, SnapshotManager};
use crate::wal::WalWriter;

/// Checkpoint ID type (equals SnapshotId per spec)
pub type CheckpointId = String;

/// Checkpoint manager for creating checkpoints with WAL truncation.
///
/// This struct provides the public API for checkpoint operations.
///
/// # Usage
///
/// ```ignore
/// let lock = GlobalExecutionLock::new();
/// let checkpoint_id = CheckpointManager::create_checkpoint(
///     &data_dir,
///     &snapshot_mgr,
///     &mut wal,
///     &lock,
/// )?;
/// ```
pub struct CheckpointManager;

impl CheckpointManager {
    /// Create a checkpoint with snapshot and WAL truncation.
    ///
    /// Per CHECKPOINT.md §4, this follows the exact sequence:
    /// 1. Acquire global execution lock (caller responsibility, verified by lock param)
    /// 2. fsync WAL
    /// 3. Create snapshot via SnapshotManager
    /// 4. Write checkpoint manifest (checkpoint.json)
    /// 5. Truncate WAL to zero (sequence resets to 1)
    /// 6. fsync WAL directory
    /// 7. Release lock (caller responsibility)
    /// 8. Return checkpoint_id (equals snapshot_id)
    ///
    /// # Arguments
    ///
    /// * `data_dir` - Root data directory
    /// * `storage_path` - Path to storage.dat file
    /// * `schema_dir` - Path to schema directory
    /// * `_snapshot_mgr` - Reference to SnapshotManager (used for type safety)
    /// * `wal` - Mutable reference to WAL writer
    /// * `lock` - Marker proving the caller holds the global execution lock
    ///
    /// # Returns
    ///
    /// The checkpoint ID on success (equals snapshot ID).
    ///
    /// # Errors
    ///
    /// Returns `CheckpointError` on any failure:
    /// - Snapshot creation fails → WAL intact
    /// - Marker write fails → snapshot exists, WAL intact
    /// - WAL truncation fails → snapshot exists, marker exists, WAL intact
    ///
    /// Checkpoint failure does NOT corrupt serving state.
    ///
    /// # Forbidden Operations
    ///
    /// This function does NOT:
    /// - Modify storage directly
    /// - Modify schemas
    /// - Rebuild indexes
    /// - Perform recovery
    /// - Spawn threads
    /// - Perform async IO
    pub fn create_checkpoint(
        data_dir: &Path,
        storage_path: &Path,
        schema_dir: &Path,
        _snapshot_mgr: &SnapshotManager,
        wal: &mut WalWriter,
        lock: &GlobalExecutionLock,
    ) -> Result<CheckpointId, CheckpointError> {
        coordinator::create_checkpoint_impl(data_dir, storage_path, schema_dir, wal, lock)
    }

    /// Create an MVCC-aware checkpoint with commit boundary.
    ///
    /// Per MVCC_SNAPSHOT_INTEGRATION.md §5:
    /// - Produces a snapshot with MVCC boundary
    /// - Establishes WAL truncation point
    /// - Preserves MVCC correctness
    ///
    /// # Arguments
    ///
    /// * `data_dir` - Root data directory
    /// * `storage_path` - Path to storage.dat file
    /// * `schema_dir` - Path to schema directory
    /// * `_snapshot_mgr` - Reference to SnapshotManager
    /// * `wal` - Mutable reference to WAL writer
    /// * `commit_authority` - Provides the current commit boundary
    /// * `lock` - Marker proving the caller holds the global execution lock
    pub fn create_mvcc_checkpoint(
        data_dir: &Path,
        storage_path: &Path,
        schema_dir: &Path,
        _snapshot_mgr: &SnapshotManager,
        wal: &mut WalWriter,
        commit_authority: &crate::mvcc::CommitAuthority,
        lock: &GlobalExecutionLock,
    ) -> Result<CheckpointId, CheckpointError> {
        coordinator::create_mvcc_checkpoint_impl(
            data_dir,
            storage_path,
            schema_dir,
            wal,
            commit_authority,
            lock,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wal::{RecordType, WalPayload, WalReader};
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    fn setup_test_environment() -> (TempDir, std::path::PathBuf, std::path::PathBuf, WalWriter) {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        // Create WAL
        let wal = WalWriter::open(&data_dir).unwrap();

        // Create storage.dat
        let storage_path = data_dir.join("storage.dat");
        let mut storage_file = File::create(&storage_path).unwrap();
        storage_file.write_all(b"test storage data").unwrap();
        storage_file.sync_all().unwrap();

        // Create schema directory with files
        let schema_dir = data_dir.join("metadata").join("schemas");
        fs::create_dir_all(&schema_dir).unwrap();

        let schema_path = schema_dir.join("user_v1.json");
        let mut schema_file = File::create(&schema_path).unwrap();
        schema_file
            .write_all(br#"{"name": "user", "version": 1}"#)
            .unwrap();
        schema_file.sync_all().unwrap();

        (temp_dir, storage_path, schema_dir, wal)
    }

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
    fn test_checkpoint_manager_create_checkpoint() {
        let (temp_dir, storage_path, schema_dir, mut wal) = setup_test_environment();
        let data_dir = temp_dir.path();
        let lock = GlobalExecutionLock::new();
        let snapshot_mgr = SnapshotManager;

        let result = CheckpointManager::create_checkpoint(
            data_dir,
            &storage_path,
            &schema_dir,
            &snapshot_mgr,
            &mut wal,
            &lock,
        );

        assert!(result.is_ok());
        let checkpoint_id = result.unwrap();

        // Verify snapshot exists
        let snapshot_dir = data_dir.join("snapshots").join(&checkpoint_id);
        assert!(snapshot_dir.exists());

        // Verify marker exists
        let mp = marker_path(data_dir);
        assert!(mp.exists());

        // Verify WAL truncated
        assert_eq!(wal.next_sequence_number(), 1);
    }

    #[test]
    fn test_lock_required() {
        let (temp_dir, storage_path, schema_dir, mut wal) = setup_test_environment();
        let data_dir = temp_dir.path();

        // The lock marker must be provided - compile-time check
        let lock = GlobalExecutionLock::new();
        let snapshot_mgr = SnapshotManager;

        let _ = CheckpointManager::create_checkpoint(
            data_dir,
            &storage_path,
            &schema_dir,
            &snapshot_mgr,
            &mut wal,
            &lock,
        );

        // If this compiles, the test passes
    }

    #[test]
    fn test_checkpoint_id_format() {
        let (temp_dir, storage_path, schema_dir, mut wal) = setup_test_environment();
        let data_dir = temp_dir.path();
        let lock = GlobalExecutionLock::new();
        let snapshot_mgr = SnapshotManager;

        let checkpoint_id = CheckpointManager::create_checkpoint(
            data_dir,
            &storage_path,
            &schema_dir,
            &snapshot_mgr,
            &mut wal,
            &lock,
        )
        .unwrap();

        // Verify RFC3339 basic format: YYYYMMDDTHHMMSSZ
        assert_eq!(checkpoint_id.len(), 16);
        assert!(checkpoint_id.ends_with('Z'));
        assert!(checkpoint_id.contains('T'));
    }

    #[test]
    fn test_wal_truncation_resets_sequence() {
        let (temp_dir, storage_path, schema_dir, mut wal) = setup_test_environment();
        let data_dir = temp_dir.path();
        let lock = GlobalExecutionLock::new();
        let snapshot_mgr = SnapshotManager;

        // Write WAL records
        wal.append(RecordType::Insert, create_test_payload("doc1"))
            .unwrap();
        wal.append(RecordType::Insert, create_test_payload("doc2"))
            .unwrap();
        assert_eq!(wal.next_sequence_number(), 3);

        // Checkpoint
        CheckpointManager::create_checkpoint(
            data_dir,
            &storage_path,
            &schema_dir,
            &snapshot_mgr,
            &mut wal,
            &lock,
        )
        .unwrap();

        // Sequence should be reset
        assert_eq!(wal.next_sequence_number(), 1);
    }

    #[test]
    fn test_wal_empty_after_checkpoint() {
        let (temp_dir, storage_path, schema_dir, mut wal) = setup_test_environment();
        let data_dir = temp_dir.path();
        let lock = GlobalExecutionLock::new();
        let snapshot_mgr = SnapshotManager;

        // Write WAL records
        wal.append(RecordType::Insert, create_test_payload("doc1"))
            .unwrap();

        // Checkpoint
        CheckpointManager::create_checkpoint(
            data_dir,
            &storage_path,
            &schema_dir,
            &snapshot_mgr,
            &mut wal,
            &lock,
        )
        .unwrap();

        // WAL should be empty
        let wal_path = data_dir.join("wal").join("wal.log");
        let mut reader = WalReader::open(&wal_path).unwrap();
        assert!(reader.read_next().unwrap().is_none());
    }

    #[test]
    fn test_marker_wal_truncated_true() {
        let (temp_dir, storage_path, schema_dir, mut wal) = setup_test_environment();
        let data_dir = temp_dir.path();
        let lock = GlobalExecutionLock::new();
        let snapshot_mgr = SnapshotManager;

        CheckpointManager::create_checkpoint(
            data_dir,
            &storage_path,
            &schema_dir,
            &snapshot_mgr,
            &mut wal,
            &lock,
        )
        .unwrap();

        let mp = marker_path(data_dir);
        let marker = CheckpointMarker::read_from_file(&mp).unwrap();

        assert!(marker.wal_truncated);
        assert_eq!(marker.format_version, 1);
    }

    #[test]
    fn test_partial_failure_wal_intact() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path();
        let mut wal = WalWriter::open(data_dir).unwrap();
        let lock = GlobalExecutionLock::new();
        let snapshot_mgr = SnapshotManager;

        // Write to WAL
        wal.append(RecordType::Insert, create_test_payload("important_doc"))
            .unwrap();
        let original_seq = wal.next_sequence_number();

        // Non-existent storage - checkpoint should fail
        let storage_path = data_dir.join("nonexistent.dat");
        let schema_dir = data_dir.join("schemas");
        fs::create_dir_all(&schema_dir).unwrap();

        let result = CheckpointManager::create_checkpoint(
            data_dir,
            &storage_path,
            &schema_dir,
            &snapshot_mgr,
            &mut wal,
            &lock,
        );

        assert!(result.is_err());

        // WAL should be intact (not truncated)
        assert_eq!(wal.next_sequence_number(), original_seq);
    }

    #[test]
    fn test_checkpoint_error_not_fatal() {
        let err = CheckpointError::failed("test");
        assert!(!err.is_fatal());
    }
}
