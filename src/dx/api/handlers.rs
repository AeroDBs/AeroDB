//! API Handlers
//!
//! Per DX_OBSERVABILITY_API.md §2 (Core Principles):
//! - OAPI-1: All endpoints are strictly read-only
//! - OAPI-2: Every response states which snapshot it observes
//! - OAPI-3: Responses MUST be deterministic
//!
//! Read-only, Phase 4, no semantic authority.

use super::response::{ApiError, ApiResponse, ObservedAt};
use crate::replication::ReplicationState;
use serde::{Deserialize, Serialize};

// ============================================================================
// Status Endpoint Data (§5.1)
// ============================================================================

/// Database lifecycle state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    /// System is starting up.
    Booting,
    /// System is running normally.
    Running,
    /// System is recovering from crash.
    Recovering,
    /// System is shutting down.
    ShuttingDown,
}

/// Status response data per §5.1.
///
/// Returns:
/// - Lifecycle state (booting, running, recovering)
/// - Current CommitId high-water mark
/// - WAL durability boundary
/// - Phase enablement flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusData {
    /// Current lifecycle state.
    pub lifecycle_state: LifecycleState,
    /// Current CommitId high-water mark.
    pub commit_id_high_water: u64,
    /// WAL durability boundary (last fsync'd offset).
    pub wal_durability_boundary: u64,
    /// Phase 3 performance optimizations enabled.
    pub phase3_enabled: bool,
    /// Phase 4 observability enabled.
    pub phase4_enabled: bool,
}

// ============================================================================
// WAL Endpoint Data (§5.2)
// ============================================================================

/// WAL state response per §5.2.
///
/// Returns:
/// - WAL file identifiers
/// - Current append offset
/// - Last durable offset
/// - Checksum status
/// - Truncation point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalData {
    /// Active WAL file identifier.
    pub active_file_id: u64,
    /// Current append offset (may not be durable).
    pub append_offset: u64,
    /// Last durable (fsync'd) offset.
    pub durable_offset: u64,
    /// Whether all checksums are valid.
    pub checksum_valid: bool,
    /// Truncation point (oldest retained offset).
    pub truncation_point: Option<u64>,
    /// Number of WAL segments.
    pub segment_count: usize,
}

// ============================================================================
// MVCC Endpoint Data (§5.3)
// ============================================================================

/// MVCC state response per §5.3.
///
/// Returns:
/// - Oldest retained CommitId
/// - Latest committed CommitId
/// - Active snapshots
/// - GC watermark
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MvccData {
    /// Oldest retained CommitId (versions older than this may be GC'd).
    pub oldest_retained_commit_id: u64,
    /// Latest committed CommitId.
    pub latest_commit_id: u64,
    /// Number of active snapshots.
    pub active_snapshot_count: usize,
    /// Active snapshot IDs.
    pub active_snapshot_ids: Vec<u64>,
    /// GC watermark CommitId.
    pub gc_watermark: Option<u64>,
}

// ============================================================================
// Snapshots Endpoint Data (§5.4)
// ============================================================================

/// Individual snapshot info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotInfo {
    /// Snapshot identifier.
    pub snapshot_id: u64,
    /// CommitId this snapshot observes.
    pub commit_id: u64,
    /// Snapshot source.
    pub source: SnapshotSource,
}

/// Snapshot creation source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotSource {
    /// User-requested snapshot.
    User,
    /// Internal system snapshot.
    Internal,
    /// Recovery-created snapshot.
    Recovery,
}

/// Snapshots response per §5.4.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotsData {
    /// List of active snapshots.
    pub snapshots: Vec<SnapshotInfo>,
}

// ============================================================================
// Checkpoints Endpoint Data (§5.5)
// ============================================================================

/// Checkpoint state response per §5.5.
///
/// Returns:
/// - Last completed checkpoint CommitId
/// - Checkpoint durability status
/// - WAL range covered
/// - Pending checkpoint info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointsData {
    /// Last completed checkpoint CommitId.
    pub last_checkpoint_commit_id: Option<u64>,
    /// Whether last checkpoint is fully durable.
    pub last_checkpoint_durable: bool,
    /// WAL start offset covered by checkpoint.
    pub wal_range_start: Option<u64>,
    /// WAL end offset covered by checkpoint.
    pub wal_range_end: Option<u64>,
    /// Whether a checkpoint is currently pending.
    pub checkpoint_pending: bool,
}

// ============================================================================
// Indexes Endpoint Data (§5.6)
// ============================================================================

/// Index info per §5.6.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    /// Index identifier.
    pub index_id: String,
    /// Index type.
    pub index_type: String,
    /// Build status.
    pub build_status: IndexBuildStatus,
    /// Entry count.
    pub entry_count: u64,
}

/// Index build status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IndexBuildStatus {
    /// Index is fully built.
    Complete,
    /// Index is currently building.
    Building,
    /// Index is being rebuilt.
    Rebuilding,
    /// Index build failed.
    Failed,
}

/// Indexes response per §5.6.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexesData {
    /// List of indexes.
    pub indexes: Vec<IndexInfo>,
}

// ============================================================================
// Replication Endpoint Data (§5.7)
// ============================================================================

/// Replication role for DX observability.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReplicationRole {
    /// Primary (writer) node.
    Primary,
    /// Replica (read-only) node.
    Replica,
    /// Standalone (no replication) - when replication is disabled.
    Standalone,
}

/// Read safety status for replicas.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReadSafetyStatus {
    /// Reads are safe (replica is caught up).
    Safe,
    /// Reads may be stale.
    Unsafe,
    /// Read safety is disabled (standalone or primary).
    Disabled,
}

/// Replication state response per §5.7 and PHASE5_OBSERVABILITY_MAPPING.md §3.1.
///
/// Per PHASE5_IMPLEMENTATION_ORDER.md Stage 1:
/// - role: Primary/Replica/Standalone
/// - replica_state: State machine state name
/// - replica_id: UUID if replica
/// - read_safety: Safe/Unsafe/Disabled
/// - blocking_reason: Why blocked (if any)
/// - replication_enabled: Whether replication is on
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationData {
    /// Node role.
    pub role: ReplicationRole,
    /// State machine state name per PHASE5_OBSERVABILITY_MAPPING.md.
    pub replica_state: String,
    /// Replica UUID (None for primary/standalone).
    pub replica_id: Option<String>,
    /// Read safety status.
    pub read_safety: ReadSafetyStatus,
    /// Blocking reason if any operation is blocked.
    pub blocking_reason: Option<String>,
    /// Whether replication is enabled.
    pub replication_enabled: bool,
    /// WAL prefix position (replica's applied CommitId) - Phase 2+.
    pub wal_prefix_commit_id: Option<u64>,
    /// Replica lag in CommitIds - Phase 2+.
    pub replica_lag: Option<u64>,
    /// Whether snapshot bootstrap is in progress - Phase 5+.
    pub snapshot_bootstrap_active: bool,
}

// ============================================================================
// Handler Placeholder Functions
// ============================================================================

/// Get current status.
///
/// Read-only, Phase 4, no semantic authority.
/// Per DX_OBSERVABILITY_API.md §5.1: Must NOT trigger refresh.
#[allow(dead_code)]
pub fn handle_status(
    lifecycle: LifecycleState,
    commit_id: u64,
    wal_durable: u64,
) -> ApiResponse<StatusData> {
    let data = StatusData {
        lifecycle_state: lifecycle,
        commit_id_high_water: commit_id,
        wal_durability_boundary: wal_durable,
        phase3_enabled: true, // Would be read from actual config
        phase4_enabled: true,
    };
    ApiResponse::new(ObservedAt::live(commit_id), data)
}

/// Get WAL state.
///
/// Read-only, Phase 4, no semantic authority.
#[allow(dead_code)]
pub fn handle_wal(commit_id: u64, wal_data: WalData) -> ApiResponse<WalData> {
    ApiResponse::new(ObservedAt::live(commit_id), wal_data)
}

/// Get MVCC state.
///
/// Read-only, Phase 4, no semantic authority.
/// Per DX_OBSERVABILITY_API.md §5.3: Must NOT trigger GC or snapshot creation.
#[allow(dead_code)]
pub fn handle_mvcc(commit_id: u64, mvcc_data: MvccData) -> ApiResponse<MvccData> {
    ApiResponse::new(ObservedAt::live(commit_id), mvcc_data)
}

/// Get replication state.
///
/// Per PHASE5_OBSERVABILITY_MAPPING.md §3.1:
/// - Role, state machine state, replica ID
/// - Read safety status
/// - Blocking reason if any
///
/// Read-only, Phase 4-5, no semantic authority.
#[allow(dead_code)]
pub fn handle_replication(
    commit_id: u64,
    state: &ReplicationState,
    wal_prefix_commit_id: Option<u64>,
    replica_lag_ms: Option<u64>,
    snapshot_bootstrap_active: bool,
) -> ApiResponse<ReplicationData> {
    let (role, replica_id, read_safety, blocking_reason) = match state {
        ReplicationState::Disabled => (
            ReplicationRole::Standalone,
            None,
            ReadSafetyStatus::Disabled,
            None,
        ),
        ReplicationState::Uninitialized => (
            ReplicationRole::Standalone,
            None,
            ReadSafetyStatus::Disabled,
            Some("replication not initialized".to_string()),
        ),
        ReplicationState::PrimaryActive => (
            ReplicationRole::Primary,
            None,
            ReadSafetyStatus::Disabled, // Primary doesn't have read safety concern
            None,
        ),
        ReplicationState::ReplicaActive { replica_id } => (
            ReplicationRole::Replica,
            Some(replica_id.to_string()),
            ReadSafetyStatus::Unsafe, // Stage 1: no read safety gate yet
            None,
        ),
        ReplicationState::ReplicationHalted { reason } => (
            ReplicationRole::Standalone,
            None,
            ReadSafetyStatus::Disabled,
            Some(format!("halted: {:?}", reason)),
        ),
    };

    let data = ReplicationData {
        role,
        replica_state: state.state_name().to_string(),
        replica_id,
        read_safety,
        blocking_reason,
        replication_enabled: !state.is_disabled(),
        // Phase 2+ fields from actual replication state
        wal_prefix_commit_id,
        replica_lag: replica_lag_ms,
        snapshot_bootstrap_active,
    };

    ApiResponse::new(ObservedAt::live(commit_id), data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_response_deterministic() {
        let resp1 = handle_status(LifecycleState::Running, 100, 95);
        let resp2 = handle_status(LifecycleState::Running, 100, 95);

        // Per OAPI-3: Given identical state, responses MUST be identical
        assert_eq!(resp1.data.lifecycle_state, resp2.data.lifecycle_state);
        assert_eq!(
            resp1.data.commit_id_high_water,
            resp2.data.commit_id_high_water
        );
    }

    #[test]
    fn test_response_includes_commit_id() {
        let resp = handle_status(LifecycleState::Running, 42, 40);

        // Per OAPI-2: Every response MUST state which snapshot it observes
        assert_eq!(resp.observed_at.commit_id, 42);
    }

    #[test]
    fn test_lifecycle_states() {
        assert_eq!(
            serde_json::to_string(&LifecycleState::Running).unwrap(),
            "\"running\""
        );
        assert_eq!(
            serde_json::to_string(&LifecycleState::Recovering).unwrap(),
            "\"recovering\""
        );
    }

    // ===== Stage 1: DX Replication Endpoint Tests =====

    #[test]
    fn test_replication_disabled_returns_standalone() {
        // Per P5-I16: Disabled replication returns standalone role
        let state = ReplicationState::Disabled;
        let resp = handle_replication(100, &state, None, None, false);

        assert_eq!(resp.data.role, ReplicationRole::Standalone);
        assert_eq!(resp.data.replica_state, "disabled");
        assert!(!resp.data.replication_enabled);
        assert!(resp.data.replica_id.is_none());
        assert_eq!(resp.data.read_safety, ReadSafetyStatus::Disabled);
    }

    #[test]
    fn test_replication_primary_returns_correct_role() {
        let state = ReplicationState::PrimaryActive;
        let resp = handle_replication(100, &state, None, None, false);

        assert_eq!(resp.data.role, ReplicationRole::Primary);
        assert_eq!(resp.data.replica_state, "primary_active");
        assert!(resp.data.replication_enabled);
        assert!(resp.data.replica_id.is_none());
    }

    #[test]
    fn test_replication_replica_returns_correct_role_and_uuid() {
        let replica_id = uuid::Uuid::new_v4();
        let state = ReplicationState::ReplicaActive { replica_id };
        let resp = handle_replication(100, &state, None, None, false);

        assert_eq!(resp.data.role, ReplicationRole::Replica);
        assert_eq!(resp.data.replica_state, "replica_active");
        assert!(resp.data.replication_enabled);
        assert_eq!(resp.data.replica_id, Some(replica_id.to_string()));
        assert_eq!(resp.data.read_safety, ReadSafetyStatus::Unsafe);
    }

    #[test]
    fn test_replication_halted_returns_blocking_reason() {
        use crate::replication::HaltReason;

        let state = ReplicationState::ReplicationHalted {
            reason: HaltReason::WalGapDetected,
        };
        let resp = handle_replication(100, &state, None, None, false);

        assert_eq!(resp.data.role, ReplicationRole::Standalone);
        assert!(resp.data.blocking_reason.is_some());
        assert!(resp
            .data
            .blocking_reason
            .as_ref()
            .unwrap()
            .contains("halted"));
    }

    #[test]
    fn test_replication_response_deterministic() {
        // Per OAPI-3: Given identical state, outputs MUST be identical
        let state = ReplicationState::PrimaryActive;
        let resp1 = handle_replication(100, &state, None, None, false);
        let resp2 = handle_replication(100, &state, None, None, false);

        assert_eq!(resp1.data.role, resp2.data.role);
        assert_eq!(resp1.data.replica_state, resp2.data.replica_state);
        assert_eq!(
            resp1.data.replication_enabled,
            resp2.data.replication_enabled
        );
    }
}
