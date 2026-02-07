//! Recovery Determinism Invariant Tests
//!
//! Tests for invariants:
//! - R2: Recovery is deterministic (same WAL → same state)
//! - R3: Recovery completeness is verifiable
//!
//! Per WAL.md, recovery must:
//! - Start at byte 0
//! - Process records sequentially
//! - Validate checksums
//! - Produce identical state on repeated replay

use aerodb::recovery::{RecoveryManager, RecoveryStorage, WalReplayer};
use aerodb::wal::{WalPayload, WalReader, WalWriter};
use tempfile::TempDir;

// =============================================================================
// Test Utilities
// =============================================================================

fn create_test_payload(doc_id: &str) -> WalPayload {
    WalPayload::new(
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
// INVARIANT R2: Recovery Is Deterministic
// =============================================================================

/// R2: Same WAL produces same replay stats.
#[test]
fn test_r2_same_wal_same_replay_stats() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // Write records
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        for i in 1..=10 {
            writer
                .append_insert(create_test_payload(&format!("doc{}", i)))
                .unwrap();
        }
    }

    // First replay
    let stats1 = {
        let mut wal = WalReader::open_from_data_dir(data_dir).unwrap();
        let mut storage = RecoveryStorage::open(data_dir).unwrap();
        WalReplayer::replay(&mut wal, &mut storage).unwrap()
    };

    // Second replay (fresh storage from same WAL)
    let temp_dir2 = create_temp_data_dir();
    let data_dir2 = temp_dir2.path();

    // Copy WAL to new location
    std::fs::create_dir_all(data_dir2.join("wal")).unwrap();
    std::fs::copy(data_dir.join("wal/wal.log"), data_dir2.join("wal/wal.log")).unwrap();

    let stats2 = {
        let mut wal = WalReader::open_from_data_dir(data_dir2).unwrap();
        let mut storage = RecoveryStorage::open(data_dir2).unwrap();
        WalReplayer::replay(&mut wal, &mut storage).unwrap()
    };

    // R2: Stats must be identical
    assert_eq!(
        stats1.records_replayed, stats2.records_replayed,
        "R2 VIOLATION: Record count differs"
    );
    assert_eq!(
        stats1.final_sequence, stats2.final_sequence,
        "R2 VIOLATION: Final sequence differs"
    );
    assert_eq!(stats1.inserts, stats2.inserts);
    assert_eq!(stats1.updates, stats2.updates);
    assert_eq!(stats1.deletes, stats2.deletes);
}

/// R2: Replay produces identical storage content.
#[test]
fn test_r2_replay_produces_identical_storage() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // Write specific records
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        writer.append_insert(create_test_payload("doc1")).unwrap();
        writer.append_insert(create_test_payload("doc2")).unwrap();
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

    // Replay to first storage
    {
        let mut wal = WalReader::open_from_data_dir(data_dir).unwrap();
        let mut storage = RecoveryStorage::open(data_dir).unwrap();
        WalReplayer::replay(&mut wal, &mut storage).unwrap();
    }

    // Read storage 1
    let storage1_contents = std::fs::read(data_dir.join("data/documents.dat")).unwrap();

    // Replay to second storage (same WAL)
    let temp_dir2 = create_temp_data_dir();
    let data_dir2 = temp_dir2.path();
    std::fs::create_dir_all(data_dir2.join("wal")).unwrap();
    std::fs::copy(data_dir.join("wal/wal.log"), data_dir2.join("wal/wal.log")).unwrap();

    {
        let mut wal = WalReader::open_from_data_dir(data_dir2).unwrap();
        let mut storage = RecoveryStorage::open(data_dir2).unwrap();
        WalReplayer::replay(&mut wal, &mut storage).unwrap();
    }

    // Read storage 2
    let storage2_contents = std::fs::read(data_dir2.join("data/documents.dat")).unwrap();

    // R2: Storage contents must be byte-identical
    assert_eq!(
        storage1_contents, storage2_contents,
        "R2 VIOLATION: Storage contents differ after identical replay"
    );
}

// =============================================================================
// INVARIANT R3: Recovery Completeness Is Verifiable
// =============================================================================

/// R3: Replay stats accurately report counts.
#[test]
fn test_r3_replay_stats_accurate() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // Write known record types
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        // 3 inserts
        writer.append_insert(create_test_payload("doc1")).unwrap();
        writer.append_insert(create_test_payload("doc2")).unwrap();
        writer.append_insert(create_test_payload("doc3")).unwrap();
        // 2 updates
        writer.append_update(create_test_payload("doc1")).unwrap();
        writer.append_update(create_test_payload("doc2")).unwrap();
        // 1 delete
        writer
            .append_delete(WalPayload::tombstone(
                "test_collection",
                "doc3",
                "test_schema",
                "v1",
            ))
            .unwrap();
    }

    let stats = {
        let mut wal = WalReader::open_from_data_dir(data_dir).unwrap();
        let mut storage = RecoveryStorage::open(data_dir).unwrap();
        WalReplayer::replay(&mut wal, &mut storage).unwrap()
    };

    // R3: Stats must match exactly
    assert_eq!(stats.records_replayed, 6, "R3: Total record count wrong");
    assert_eq!(stats.inserts, 3, "R3: Insert count wrong");
    assert_eq!(stats.updates, 2, "R3: Update count wrong");
    assert_eq!(stats.deletes, 1, "R3: Delete count wrong");
    assert_eq!(stats.final_sequence, 6, "R3: Final sequence wrong");
}

/// R3: Empty WAL produces zero replay stats.
#[test]
fn test_r3_empty_wal_zero_stats() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // Create empty WAL
    {
        let _writer = WalWriter::open(data_dir).unwrap();
    }

    let stats = {
        let mut wal = WalReader::open_from_data_dir(data_dir).unwrap();
        let mut storage = RecoveryStorage::open(data_dir).unwrap();
        WalReplayer::replay(&mut wal, &mut storage).unwrap()
    };

    // R3: Empty replay is explicit
    assert_eq!(stats.records_replayed, 0);
    assert_eq!(stats.inserts, 0);
    assert_eq!(stats.updates, 0);
    assert_eq!(stats.deletes, 0);
    assert_eq!(stats.final_sequence, 0);
}

// =============================================================================
// Replay Idempotency Tests
// =============================================================================

/// Replaying the same WAL twice produces same stats.
#[test]
fn test_replay_idempotency() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // Write records
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        for i in 1..=5 {
            writer
                .append_insert(create_test_payload(&format!("doc{}", i)))
                .unwrap();
        }
    }

    // First replay
    let stats1 = {
        let mut wal = WalReader::open_from_data_dir(data_dir).unwrap();
        let mut storage = RecoveryStorage::open(data_dir).unwrap();
        WalReplayer::replay(&mut wal, &mut storage).unwrap()
    };

    // Second replay (same WAL to fresh storage)
    let temp_dir2 = create_temp_data_dir();
    let data_dir2 = temp_dir2.path();
    std::fs::create_dir_all(data_dir2.join("wal")).unwrap();
    std::fs::copy(data_dir.join("wal/wal.log"), data_dir2.join("wal/wal.log")).unwrap();

    let stats2 = {
        let mut wal = WalReader::open_from_data_dir(data_dir2).unwrap();
        let mut storage = RecoveryStorage::open(data_dir2).unwrap();
        WalReplayer::replay(&mut wal, &mut storage).unwrap()
    };

    // Both replays should have identical stats
    assert_eq!(stats1.records_replayed, stats2.records_replayed);
    assert_eq!(stats1.final_sequence, stats2.final_sequence);
}

// =============================================================================
// Recovery Manager Tests
// =============================================================================

/// RecoveryManager correctly handles clean shutdown marker.
#[test]
fn test_recovery_manager_shutdown_marker() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    let manager = RecoveryManager::new(data_dir);

    // Initially no clean shutdown
    assert!(!manager.was_clean_shutdown());

    // Mark clean shutdown
    manager.mark_clean_shutdown().unwrap();
    assert!(manager.was_clean_shutdown());
}

/// Recovery can be executed multiple times (crash recovery scenario).
#[test]
fn test_recovery_restartable() {
    let temp_dir = create_temp_data_dir();
    let data_dir = temp_dir.path();

    // Write some records
    {
        let mut writer = WalWriter::open(data_dir).unwrap();
        writer.append_insert(create_test_payload("doc1")).unwrap();
    }

    // First recovery attempt
    let stats1 = {
        let mut wal = WalReader::open_from_data_dir(data_dir).unwrap();
        let mut storage = RecoveryStorage::open(data_dir).unwrap();
        WalReplayer::replay(&mut wal, &mut storage).unwrap()
    };

    // Copy WAL for second recovery
    let temp_dir2 = create_temp_data_dir();
    let data_dir2 = temp_dir2.path();
    std::fs::create_dir_all(data_dir2.join("wal")).unwrap();
    std::fs::copy(data_dir.join("wal/wal.log"), data_dir2.join("wal/wal.log")).unwrap();

    // Second recovery
    let stats2 = {
        let mut wal = WalReader::open_from_data_dir(data_dir2).unwrap();
        let mut storage = RecoveryStorage::open(data_dir2).unwrap();
        WalReplayer::replay(&mut wal, &mut storage).unwrap()
    };

    // Both should succeed with same stats
    assert_eq!(stats1.records_replayed, stats2.records_replayed);
}
