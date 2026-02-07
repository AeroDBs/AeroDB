//! Phase 7 Control Plane Types
//!
//! Per PHASE7_CONTROL_PLANE_ARCHITECTURE.md:
//! Request/response types for the control plane API.
//!
//! Per PHASE7_STATE_MODEL.md §5:
//! Derived presentation state is computed for human consumption.
//! No derived view is authoritative.

use std::time::SystemTime;
use uuid::Uuid;

use super::authority::AuthorityContext;
use super::commands::ControlPlaneCommand;

/// Command request — operator-initiated action.
///
/// Per PHASE7_INVARIANTS.md §P7-A2:
/// Every mutating action MUST be traceable to a conscious human decision.
#[derive(Debug, Clone)]
pub struct CommandRequest {
    /// Unique request ID for correlation and audit.
    pub request_id: Uuid,

    /// The command to execute.
    pub command: ControlPlaneCommand,

    /// Authority context of the requester.
    pub authority: AuthorityContext,

    /// Confirmation token ID (required for mutating commands).
    pub confirmation_token: Option<Uuid>,

    /// Request timestamp.
    pub timestamp: SystemTime,
}

impl CommandRequest {
    /// Create a new command request.
    pub fn new(command: ControlPlaneCommand, authority: AuthorityContext) -> Self {
        Self {
            request_id: Uuid::new_v4(),
            command,
            authority,
            confirmation_token: None,
            timestamp: SystemTime::now(),
        }
    }

    /// Add confirmation token.
    pub fn with_confirmation(mut self, token_id: Uuid) -> Self {
        self.confirmation_token = Some(token_id);
        self
    }
}

/// Command execution outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandOutcome {
    /// Command executed successfully.
    Success,

    /// Command was rejected before execution.
    Rejected,

    /// Command failed during execution.
    Failed,

    /// Awaiting confirmation.
    AwaitingConfirmation,
}

/// Command response.
#[derive(Debug, Clone)]
pub struct CommandResponse {
    /// Request ID for correlation.
    pub request_id: Uuid,

    /// Command that was executed.
    pub command_name: String,

    /// Outcome of the command.
    pub outcome: CommandOutcome,

    /// Response timestamp.
    pub timestamp: SystemTime,

    /// Confirmation token (if awaiting confirmation).
    pub confirmation_token: Option<Uuid>,

    /// Result data (specific to command type).
    pub data: Option<CommandResponseData>,

    /// Error message (if rejected/failed).
    pub error_message: Option<String>,
}

impl CommandResponse {
    /// Create a success response.
    pub fn success(request_id: Uuid, command_name: &str, data: CommandResponseData) -> Self {
        Self {
            request_id,
            command_name: command_name.to_string(),
            outcome: CommandOutcome::Success,
            timestamp: SystemTime::now(),
            confirmation_token: None,
            data: Some(data),
            error_message: None,
        }
    }

    /// Create a rejection response.
    pub fn rejected(request_id: Uuid, command_name: &str, reason: &str) -> Self {
        Self {
            request_id,
            command_name: command_name.to_string(),
            outcome: CommandOutcome::Rejected,
            timestamp: SystemTime::now(),
            confirmation_token: None,
            data: None,
            error_message: Some(reason.to_string()),
        }
    }

    /// Create an awaiting confirmation response.
    pub fn awaiting_confirmation(request_id: Uuid, command_name: &str, token_id: Uuid) -> Self {
        Self {
            request_id,
            command_name: command_name.to_string(),
            outcome: CommandOutcome::AwaitingConfirmation,
            timestamp: SystemTime::now(),
            confirmation_token: Some(token_id),
            data: None,
            error_message: None,
        }
    }
}

/// Response data variants for different command types.
#[derive(Debug, Clone)]
pub enum CommandResponseData {
    /// Cluster state inspection result.
    ClusterState(ClusterState),

    /// Node state inspection result.
    NodeState(NodeState),

    /// Replication status inspection result.
    ReplicationStatus(ReplicationStatus),

    /// Promotion state inspection result.
    PromotionState(PromotionStateView),

    /// Diagnostic results.
    Diagnostics(DiagnosticResult),

    /// WAL inspection result.
    WalInfo(WalInfo),

    /// Snapshots inspection result.
    SnapshtoInfo(SnapshotInfo),

    /// Promotion request result.
    PromotionResult(PromotionResultData),
}

// ============================================================================
// DERIVED STATE VIEWS (per PHASE7_STATE_MODEL.md §5)
// ============================================================================

/// Cluster state view.
///
/// Per PHASE7_OBSERVABILITY_MODEL.md §4.1:
/// Views must reflect a single kernel snapshot.
#[derive(Debug, Clone)]
pub struct ClusterState {
    /// Cluster identifier.
    pub cluster_id: Option<String>,

    /// Current primary node (if known).
    pub primary_id: Option<Uuid>,

    /// Known replica nodes.
    pub replicas: Vec<Uuid>,

    /// Snapshot timestamp.
    pub snapshot_time: SystemTime,
}

/// Node state view.
#[derive(Debug, Clone)]
pub struct NodeState {
    /// Node identifier.
    pub node_id: Uuid,

    /// Current role.
    pub role: NodeRole,

    /// WAL position.
    pub wal_position: u64,

    /// Health status.
    pub health: NodeHealth,

    /// Snapshot timestamp.
    pub snapshot_time: SystemTime,
}

/// Node role (derived from kernel state).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRole {
    Primary,
    Replica,
    Unknown,
}

/// Node health status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeHealth {
    Healthy,
    Degraded,
    Unavailable,
    Unknown,
}

/// Replication status view.
#[derive(Debug, Clone)]
pub struct ReplicationStatus {
    /// Primary node ID.
    pub primary_id: Option<Uuid>,

    /// Replica states.
    pub replicas: Vec<ReplicaState>,

    /// Snapshot timestamp.
    pub snapshot_time: SystemTime,
}

/// Individual replica state.
#[derive(Debug, Clone)]
pub struct ReplicaState {
    /// Replica node ID.
    pub replica_id: Uuid,

    /// WAL position lag (bytes behind primary).
    pub lag_bytes: u64,

    /// Health status.
    pub health: NodeHealth,
}

/// Promotion state machine view.
#[derive(Debug, Clone)]
pub struct PromotionStateView {
    /// Current state name.
    pub state: String,

    /// Promotion in progress for replica.
    pub pending_replica: Option<Uuid>,

    /// Last promotion timestamp (if any).
    pub last_promotion: Option<SystemTime>,

    /// Snapshot timestamp.
    pub snapshot_time: SystemTime,
}

// ============================================================================
// DIAGNOSTIC RESULTS
// ============================================================================

/// Diagnostic result.
#[derive(Debug, Clone)]
pub struct DiagnosticResult {
    /// Diagnostic sections.
    pub sections: Vec<DiagnosticSection>,

    /// Collection timestamp.
    pub collected_at: SystemTime,
}

/// Diagnostic section.
#[derive(Debug, Clone)]
pub struct DiagnosticSection {
    /// Section name.
    pub name: String,

    /// Key-value entries.
    pub entries: Vec<(String, String)>,
}

/// WAL information.
#[derive(Debug, Clone)]
pub struct WalInfo {
    /// Current WAL position.
    pub current_position: u64,

    /// Oldest retained position.
    pub oldest_position: u64,

    /// WAL size in bytes.
    pub size_bytes: u64,

    /// Snapshot timestamp.
    pub snapshot_time: SystemTime,
}

/// Snapshot information.
#[derive(Debug, Clone)]
pub struct SnapshotInfo {
    /// Available snapshots.
    pub snapshots: Vec<SnapshotMeta>,

    /// Available checkpoints.
    pub checkpoints: Vec<CheckpointMeta>,

    /// Snapshot timestamp.
    pub snapshot_time: SystemTime,
}

/// Snapshot metadata.
#[derive(Debug, Clone)]
pub struct SnapshotMeta {
    /// Snapshot ID.
    pub id: String,

    /// Creation timestamp.
    pub created_at: SystemTime,

    /// WAL position at snapshot.
    pub wal_position: u64,
}

/// Checkpoint metadata.
#[derive(Debug, Clone)]
pub struct CheckpointMeta {
    /// Checkpoint ID.
    pub id: String,

    /// Creation timestamp.
    pub created_at: SystemTime,

    /// WAL position at checkpoint.
    pub wal_position: u64,
}

/// Promotion result data.
#[derive(Debug, Clone)]
pub struct PromotionResultData {
    /// Replica that was promoted.
    pub replica_id: Uuid,

    /// Whether promotion succeeded.
    pub success: bool,

    /// New role after promotion.
    pub new_role: Option<NodeRole>,

    /// Explanation of result.
    pub explanation: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dx::api::control_plane::authority::AuthorityContext;
    use crate::dx::api::control_plane::commands::InspectionCommand;

    #[test]
    fn test_command_request_creation() {
        let cmd = ControlPlaneCommand::Inspection(InspectionCommand::InspectClusterState);
        let auth = AuthorityContext::observer();
        let request = CommandRequest::new(cmd, auth);

        assert!(!request.request_id.is_nil());
        assert!(request.confirmation_token.is_none());
    }

    #[test]
    fn test_command_response_success() {
        let data = CommandResponseData::ClusterState(ClusterState {
            cluster_id: Some("test-cluster".to_string()),
            primary_id: Some(Uuid::new_v4()),
            replicas: vec![],
            snapshot_time: SystemTime::now(),
        });
        let response = CommandResponse::success(Uuid::new_v4(), "inspect_cluster_state", data);

        assert_eq!(response.outcome, CommandOutcome::Success);
    }
}
