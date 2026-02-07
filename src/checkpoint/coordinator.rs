//! Core checkpoint coordination logic
//!
//! Per CHECKPOINT.md §4, checkpoint execution MUST follow:
//!
//! 1. Acquire global execution lock (caller responsibility)
//! 2. fsync WAL
//! 3. Create snapshot (per SNAPSHOT.md)
//! 4. fsync snapshot (handled by snapshot module)
//! 5. Write checkpoint manifest (checkpoint.json)
//! 6. Truncate WAL to zero
//! 7. fsync WAL directory
//! 8. Release global execution lock (caller responsibility)
//!
//! Any failure aborts checkpoint.
//!
//! Checkpoint is NOT recovery. This code does NOT rebuild indexes.

use std::path::Path;

use chrono::Utc;

use super::errors::{CheckpointError, CheckpointResult};
use super::marker::{marker_path, CheckpointMarker};
use super::CheckpointId;
use crate::snapshot::{GlobalExecutionLock, SnapshotManager};
use crate::wal::WalWriter;

/// Generate timestamp in RFC3339 format for created_at field
fn generate_created_at() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Create a checkpoint.
///
/// This function implements the checkpoint creation algorithm per CHECKPOINT.md §4.
///
/// # Arguments
///
/// * `data_dir` - Root data directory
/// * `storage_path` - Path to storage.dat file
/// * `schema_dir` - Path to schema directory
/// * `wal` - Mutable reference to WAL writer
///
/// # Returns
///
/// The checkpoint ID on success (equals snapshot ID).
///
/// # Errors
///
/// Returns `CheckpointError` on any failure. Failure scenarios:
/// - Snapshot creation fails → WAL intact
/// - Marker write fails → snapshot exists, WAL intact
/// - WAL truncation fails → snapshot exists, marker exists, WAL intact
///
/// # Crash Safety
///
/// Per CHECKPOINT.md §7:
/// - Crash before manifest → snapshot ignored, WAL replay
/// - Crash after manifest but before truncation → snapshot used, WAL replayed
/// - Crash after truncation → snapshot used, WAL empty
///
/// No scenario causes data loss.
pub fn create_checkpoint_impl(
    data_dir: &Path,
    storage_path: &Path,
    schema_dir: &Path,
    wal: &mut WalWriter,
    lock: &GlobalExecutionLock,
) -> CheckpointResult<CheckpointId> {
    // Step 2: fsync WAL to ensure all pending writes are durable
    wal.fsync()?;

    // Step 3-4: Create snapshot (includes fsync of all snapshot files)
    let snapshot_id =
        SnapshotManager::create_snapshot(data_dir, storage_path, schema_dir, wal, lock)?;

    // Checkpoint ID equals snapshot ID
    let checkpoint_id = snapshot_id.clone();
    let created_at = generate_created_at();

    // Step 5: Write checkpoint manifest (checkpoint.json)
    // Written AFTER snapshot fsync, BEFORE WAL truncation
    let marker = CheckpointMarker::new(&snapshot_id, &created_at);
    let mp = marker_path(data_dir);
    marker.write_to_file(&mp)?;

    // Step 6: Truncate WAL to zero
    // Per CHECKPOINT.md §6:
    // - WAL file deleted or truncated
    // - New WAL starts empty
    // - Sequence numbers reset to 1
    wal.truncate()?;

    // Step 7: fsync WAL directory is handled by truncate()

    // Update marker to reflect successful truncation
    let final_marker = CheckpointMarker::with_truncation(&snapshot_id, &created_at, true);
    final_marker.write_to_file(&mp)?;

    // Step 9: Return checkpoint_id
    Ok(checkpoint_id)
}

/// Create an MVCC-aware checkpoint.
///
/// Per MVCC_SNAPSHOT_INTEGRATION.md §5:
/// - Produces a snapshot with MVCC boundary
/// - Establishes WAL truncation point
/// - Preserves MVCC correctness
pub fn create_mvcc_checkpoint_impl(
    data_dir: &Path,
    storage_path: &Path,
    schema_dir: &Path,
    wal: &mut WalWriter,
    commit_authority: &crate::mvcc::CommitAuthority,
    lock: &GlobalExecutionLock,
) -> CheckpointResult<CheckpointId> {
    // Step 2: fsync WAL to ensure all pending writes are durable
    wal.fsync()?;

    // Step 3-4: Create MVCC snapshot (includes fsync and commit boundary)
    let snapshot_id = SnapshotManager::create_mvcc_snapshot(
        data_dir,
        storage_path,
        schema_dir,
        wal,
        commit_authority,
        lock,
    )?;

    // Checkpoint ID equals snapshot ID
    let checkpoint_id = snapshot_id.clone();
    let created_at = generate_created_at();

    // Step 5: Write checkpoint manifest (checkpoint.json)
    // Written AFTER snapshot fsync, BEFORE WAL truncation
    let marker = CheckpointMarker::new(&snapshot_id, &created_at);
    let mp = marker_path(data_dir);
    marker.write_to_file(&mp)?;

    // Step 6: Truncate WAL to zero
    // Per CHECKPOINT.md §6:
    // - WAL file deleted or truncated
    // - New WAL starts empty
    // - Sequence numbers reset to 1
    wal.truncate()?;

    // Step 7: fsync WAL directory is handled by truncate()

    // Update marker to reflect successful truncation
    let final_marker = CheckpointMarker::with_truncation(&snapshot_id, &created_at, true);
    final_marker.write_to_file(&mp)?;

    // Step 9: Return checkpoint_id
    Ok(checkpoint_id)
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
    fn test_checkpoint_creates_snapshot() {
        let (temp_dir, storage_path, schema_dir, mut wal) = setup_test_environment();
        let data_dir = temp_dir.path();
        let lock = GlobalExecutionLock::new();

        let checkpoint_id =
            create_checkpoint_impl(data_dir, &storage_path, &schema_dir, &mut wal, &lock).unwrap();

        // Verify snapshot exists
        let snapshot_dir = data_dir.join("snapshots").join(&checkpoint_id);
        assert!(snapshot_dir.exists());
        assert!(snapshot_dir.join("storage.dat").exists());
        assert!(snapshot_dir.join("manifest.json").exists());
    }

    #[test]
    fn test_checkpoint_writes_marker() {
        let (temp_dir, storage_path, schema_dir, mut wal) = setup_test_environment();
        let data_dir = temp_dir.path();
        let lock = GlobalExecutionLock::new();

        let checkpoint_id =
            create_checkpoint_impl(data_dir, &storage_path, &schema_dir, &mut wal, &lock).unwrap();

        // Verify marker exists
        let mp = marker_path(data_dir);
        assert!(mp.exists());

        // Read and verify marker content
        let marker = CheckpointMarker::read_from_file(&mp).unwrap();
        assert_eq!(marker.snapshot_id, checkpoint_id);
        assert!(marker.wal_truncated);
        assert_eq!(marker.format_version, 1);
    }

    #[test]
    fn test_checkpoint_truncates_wal() {
        let (temp_dir, storage_path, schema_dir, mut wal) = setup_test_environment();
        let data_dir = temp_dir.path();
        let lock = GlobalExecutionLock::new();

        // Write some WAL records first
        wal.append(RecordType::Insert, create_test_payload("doc1"))
            .unwrap();
        wal.append(RecordType::Insert, create_test_payload("doc2"))
            .unwrap();
        assert_eq!(wal.next_sequence_number(), 3);

        // Create checkpoint
        create_checkpoint_impl(data_dir, &storage_path, &schema_dir, &mut wal, &lock).unwrap();

        // Verify WAL is truncated (sequence reset to 1)
        assert_eq!(wal.next_sequence_number(), 1);

        // Verify WAL file is empty
        let wal_path = data_dir.join("wal").join("wal.log");
        let mut reader = WalReader::open(&wal_path).unwrap();
        assert!(reader.read_next().unwrap().is_none());
    }

    #[test]
    fn test_checkpoint_id_equals_snapshot_id() {
        let (temp_dir, storage_path, schema_dir, mut wal) = setup_test_environment();
        let data_dir = temp_dir.path();
        let lock = GlobalExecutionLock::new();

        let checkpoint_id =
            create_checkpoint_impl(data_dir, &storage_path, &schema_dir, &mut wal, &lock).unwrap();

        // Verify checkpoint_id format matches snapshot format
        assert_eq!(checkpoint_id.len(), 16); // YYYYMMDDTHHMMSSZ
        assert!(checkpoint_id.ends_with('Z'));
        assert!(checkpoint_id.contains('T'));

        // Verify marker snapshot_id matches
        let mp = marker_path(data_dir);
        let marker = CheckpointMarker::read_from_file(&mp).unwrap();
        assert_eq!(marker.snapshot_id, checkpoint_id);
    }

    #[test]
    fn test_checkpoint_allows_new_wal_writes() {
        let (temp_dir, storage_path, schema_dir, mut wal) = setup_test_environment();
        let data_dir = temp_dir.path();
        let lock = GlobalExecutionLock::new();

        // Write before checkpoint
        wal.append(RecordType::Insert, create_test_payload("old_doc"))
            .unwrap();

        // Checkpoint
        create_checkpoint_impl(data_dir, &storage_path, &schema_dir, &mut wal, &lock).unwrap();

        // New writes should work, starting at sequence 1
        let seq = wal
            .append(RecordType::Insert, create_test_payload("new_doc"))
            .unwrap();
        assert_eq!(seq, 1);
    }

    #[test]
    fn test_checkpoint_failure_on_missing_storage() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path();
        let mut wal = WalWriter::open(data_dir).unwrap();
        let lock = GlobalExecutionLock::new();

        // Non-existent storage
        let storage_path = data_dir.join("nonexistent.dat");
        let schema_dir = data_dir.join("schemas");
        fs::create_dir_all(&schema_dir).unwrap();

        let result = create_checkpoint_impl(data_dir, &storage_path, &schema_dir, &mut wal, &lock);

        // Should fail
        assert!(result.is_err());

        // WAL should be intact (no truncation)
        // (We didn't write anything, so just verify it can still write)
        let seq = wal
            .append(RecordType::Insert, create_test_payload("test"))
            .unwrap();
        assert_eq!(seq, 1);
    }

    #[test]
    fn test_marker_shows_wal_truncated_true() {
        let (temp_dir, storage_path, schema_dir, mut wal) = setup_test_environment();
        let data_dir = temp_dir.path();
        let lock = GlobalExecutionLock::new();

        create_checkpoint_impl(data_dir, &storage_path, &schema_dir, &mut wal, &lock).unwrap();

        let mp = marker_path(data_dir);
        let marker = CheckpointMarker::read_from_file(&mp).unwrap();

        assert!(
            marker.wal_truncated,
            "Marker should show wal_truncated=true after successful checkpoint"
        );
    }

    #[test]
    fn test_multiple_checkpoints() {
        let (temp_dir, storage_path, schema_dir, mut wal) = setup_test_environment();
        let data_dir = temp_dir.path();
        let lock = GlobalExecutionLock::new();

        // First checkpoint
        let cp1 =
            create_checkpoint_impl(data_dir, &storage_path, &schema_dir, &mut wal, &lock).unwrap();

        // Write more data
        wal.append(RecordType::Insert, create_test_payload("doc_after_cp1"))
            .unwrap();

        // Allow time difference in snapshot IDs
        std::thread::sleep(std::time::Duration::from_millis(1100));

        // Second checkpoint
        let cp2 =
            create_checkpoint_impl(data_dir, &storage_path, &schema_dir, &mut wal, &lock).unwrap();

        // Both snapshots should exist
        assert!(data_dir.join("snapshots").join(&cp1).exists());
        assert!(data_dir.join("snapshots").join(&cp2).exists());

        // Marker should point to latest
        let mp = marker_path(data_dir);
        let marker = CheckpointMarker::read_from_file(&mp).unwrap();
        assert_eq!(marker.snapshot_id, cp2);
    }
}
