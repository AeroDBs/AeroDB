//! Phase 7 Control Plane Handlers
//!
//! Per PHASE7_CONTROL_PLANE_ARCHITECTURE.md §4.4:
//! - Kernel Boundary Adapter translates control-plane requests into kernel calls
//! - Enforces strict API boundaries
//! - MUST expose only explicitly allowed kernel operations
//! - MUST NOT batch or chain kernel calls
//!
//! This is the ONLY component allowed to call kernel APIs.

use std::sync::Arc;
use std::time::SystemTime;
use uuid::Uuid;

use super::authority::AuthorityContext;
use super::commands::{ControlCommand, ControlPlaneCommand, DiagnosticCommand, InspectionCommand};
use super::confirmation::{ConfirmationFlow, ConfirmationResult, ConfirmationToken};
use super::errors::{ControlPlaneError, ControlPlaneResult};
use super::types::{
    ClusterState, CommandOutcome, CommandRequest, CommandResponse, CommandResponseData,
    DiagnosticResult, DiagnosticSection, NodeHealth, NodeRole, NodeState, PromotionResultData,
    PromotionStateView, ReplicaState, ReplicationStatus, SnapshotInfo, WalInfo,
};

use crate::promotion::{PromotionController, PromotionState};
use crate::replication::ReplicationState;

/// Kernel Adapter trait for accessing kernel subsystems.
///
/// Per PHASE7_CONTROL_PLANE_ARCHITECTURE.md §4.4:
/// - Each method corresponds to exactly one kernel operation
/// - Methods are read-only for inspection, mutating for control
pub trait KernelAdapter: Send + Sync {
    /// Get current replication state
    fn get_replication_state(&self) -> ReplicationState;

    /// Get promotion controller state
    fn get_promotion_state(&self) -> PromotionState;

    /// Get WAL current position
    fn get_wal_position(&self) -> u64;

    /// Get WAL oldest retained position
    fn get_wal_oldest_position(&self) -> u64;

    /// Get WAL size in bytes
    fn get_wal_size_bytes(&self) -> u64;

    /// Get list of active snapshots
    fn get_snapshots(&self) -> Vec<(u64, SystemTime)>;

    /// Get list of checkpoints
    fn get_checkpoints(&self) -> Vec<(u64, SystemTime)>;

    /// Request promotion for a replica
    fn request_promotion(&self, replica_id: Uuid, reason: &str) -> Result<String, String>;

    /// Request demotion for a node
    fn request_demotion(&self, node_id: Uuid, reason: &str) -> Result<String, String>;

    /// Force promotion (with risk acknowledgment)
    fn force_promotion(&self, replica_id: Uuid, reason: &str) -> Result<String, String>;
}

/// Default kernel adapter using actual kernel modules
pub struct DefaultKernelAdapter {
    replication_state: ReplicationState,
    promotion_state: PromotionState,
}

impl Default for DefaultKernelAdapter {
    fn default() -> Self {
        Self {
            replication_state: ReplicationState::default(),
            promotion_state: PromotionState::Steady,
        }
    }
}

impl DefaultKernelAdapter {
    pub fn new(replication_state: ReplicationState, promotion_state: PromotionState) -> Self {
        Self {
            replication_state,
            promotion_state,
        }
    }
}

impl KernelAdapter for DefaultKernelAdapter {
    fn get_replication_state(&self) -> ReplicationState {
        self.replication_state.clone()
    }

    fn get_promotion_state(&self) -> PromotionState {
        self.promotion_state.clone()
    }

    fn get_wal_position(&self) -> u64 {
        // Would read from actual WAL writer
        0
    }

    fn get_wal_oldest_position(&self) -> u64 {
        0
    }

    fn get_wal_size_bytes(&self) -> u64 {
        0
    }

    fn get_snapshots(&self) -> Vec<(u64, SystemTime)> {
        Vec::new()
    }

    fn get_checkpoints(&self) -> Vec<(u64, SystemTime)> {
        Vec::new()
    }

    fn request_promotion(&self, _replica_id: Uuid, _reason: &str) -> Result<String, String> {
        Err("Promotion controller not connected".to_string())
    }

    fn request_demotion(&self, _node_id: Uuid, _reason: &str) -> Result<String, String> {
        Err("Demotion controller not connected".to_string())
    }

    fn force_promotion(&self, _replica_id: Uuid, _reason: &str) -> Result<String, String> {
        Err("Promotion controller not connected".to_string())
    }
}

/// Phase 7 Control Plane Handler.
///
/// Per PHASE7_CONTROL_PLANE_ARCHITECTURE.md:
/// - Routes requests to kernel boundary adapter
/// - Orchestrates confirmation flows
/// - Validates request structure and authority
pub struct ControlPlaneHandler {
    /// Confirmation flow manager (ephemeral state).
    confirmation: ConfirmationFlow,
    /// Kernel adapter for accessing kernel subsystems.
    kernel: Arc<dyn KernelAdapter>,
}

impl Default for ControlPlaneHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ControlPlaneHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ControlPlaneHandler")
            .field("confirmation", &self.confirmation)
            .finish()
    }
}

impl ControlPlaneHandler {
    /// Create a new control plane handler with default kernel adapter.
    pub fn new() -> Self {
        Self {
            confirmation: ConfirmationFlow::new(),
            kernel: Arc::new(DefaultKernelAdapter::default()),
        }
    }

    /// Create with a custom kernel adapter.
    pub fn with_kernel(kernel: Arc<dyn KernelAdapter>) -> Self {
        Self {
            confirmation: ConfirmationFlow::new(),
            kernel,
        }
    }

    /// Handle a command request.
    ///
    /// Per PHASE7_CONTROL_PLANE_ARCHITECTURE.md §6:
    /// 1. Validate structure and authority
    /// 2. Generate pre-execution explanation (if mutating)
    /// 3. Request confirmation (if required)
    /// 4. Forward to kernel boundary adapter
    /// 5. Return result
    pub fn handle_command(
        &mut self,
        request: CommandRequest,
    ) -> ControlPlaneResult<CommandResponse> {
        // Step 1: Validate authority
        self.validate_authority(&request)?;

        // Step 2: Check if confirmation is required
        if request.command.requires_confirmation() {
            return self.handle_confirmable_command(request);
        }

        // Step 3: Execute non-confirmable command directly
        self.execute_command(&request)
    }

    /// Validate authority for a command.
    fn validate_authority(&self, request: &CommandRequest) -> ControlPlaneResult<()> {
        // Per PHASE7_AUTHORITY_MODEL.md §3.2:
        // Operator authority is required for all state mutation.
        if request.command.is_mutating() && !request.authority.can_mutate() {
            return Err(ControlPlaneError::insufficient_authority(
                "OPERATOR",
                &request.authority.level.to_string(),
            ));
        }
        Ok(())
    }

    /// Handle a command that requires confirmation.
    fn handle_confirmable_command(
        &mut self,
        request: CommandRequest,
    ) -> ControlPlaneResult<CommandResponse> {
        let command_name = request.command.command_name();
        let target_id = self.extract_target_id(&request.command);

        // Check if confirmation token is provided
        match request.confirmation_token {
            None => {
                // Request confirmation
                let token = self
                    .confirmation
                    .request_confirmation(command_name, target_id);
                Ok(CommandResponse::awaiting_confirmation(
                    request.request_id,
                    command_name,
                    token.id(),
                ))
            }
            Some(token_id) => {
                // Validate and consume confirmation
                match self.confirmation.confirm(token_id, command_name, target_id) {
                    ConfirmationResult::Proceed { .. } => {
                        // Execute the command
                        self.execute_command(&request)
                    }
                    ConfirmationResult::Abort { reason } => {
                        Err(ControlPlaneError::missing_confirmation(&reason))
                    }
                }
            }
        }
    }

    /// Extract target ID from a command.
    fn extract_target_id(&self, command: &ControlPlaneCommand) -> Option<Uuid> {
        match command {
            ControlPlaneCommand::Inspection(InspectionCommand::InspectNode { node_id }) => {
                Some(*node_id)
            }
            ControlPlaneCommand::Control(cmd) => Some(cmd.target_id()),
            _ => None,
        }
    }

    /// Execute a command against the kernel.
    ///
    /// Per PHASE7_INVARIANTS.md §P7-E1:
    /// Each operator command MUST correspond to exactly one kernel action.
    fn execute_command(&self, request: &CommandRequest) -> ControlPlaneResult<CommandResponse> {
        match &request.command {
            ControlPlaneCommand::Inspection(cmd) => {
                self.execute_inspection(request.request_id, cmd)
            }
            ControlPlaneCommand::Diagnostic(cmd) => {
                self.execute_diagnostic(request.request_id, cmd)
            }
            ControlPlaneCommand::Control(cmd) => self.execute_control(request.request_id, cmd),
        }
    }

    // =========================================================================
    // INSPECTION COMMANDS
    // =========================================================================

    fn execute_inspection(
        &self,
        request_id: Uuid,
        cmd: &InspectionCommand,
    ) -> ControlPlaneResult<CommandResponse> {
        match cmd {
            InspectionCommand::InspectClusterState => {
                let repl_state = self.kernel.get_replication_state();
                let state = ClusterState {
                    cluster_id: None,
                    primary_id: if repl_state.is_primary() || repl_state.is_disabled() {
                        Some(Uuid::nil()) // This node is primary
                    } else {
                        None
                    },
                    replicas: if let Some(replica_id) = repl_state.replica_id() {
                        vec![replica_id]
                    } else {
                        Vec::new()
                    },
                    snapshot_time: SystemTime::now(),
                };
                Ok(CommandResponse::success(
                    request_id,
                    cmd.command_name(),
                    CommandResponseData::ClusterState(state),
                ))
            }
            InspectionCommand::InspectNode { node_id } => {
                let repl_state = self.kernel.get_replication_state();
                let role = if repl_state.is_primary() {
                    NodeRole::Primary
                } else if repl_state.is_replica() {
                    NodeRole::Replica
                } else {
                    NodeRole::Unknown
                };
                let health = if repl_state.is_halted() {
                    NodeHealth::Unavailable
                } else if repl_state.can_read() {
                    NodeHealth::Healthy
                } else {
                    NodeHealth::Unknown
                };
                let state = NodeState {
                    node_id: *node_id,
                    role,
                    wal_position: self.kernel.get_wal_position(),
                    health,
                    snapshot_time: SystemTime::now(),
                };
                Ok(CommandResponse::success(
                    request_id,
                    cmd.command_name(),
                    CommandResponseData::NodeState(state),
                ))
            }
            InspectionCommand::InspectReplicationStatus => {
                let repl_state = self.kernel.get_replication_state();
                let status = ReplicationStatus {
                    primary_id: if repl_state.is_primary() || repl_state.is_disabled() {
                        Some(Uuid::nil())
                    } else {
                        None
                    },
                    replicas: if let Some(replica_id) = repl_state.replica_id() {
                        vec![ReplicaState {
                            replica_id,
                            lag_bytes: 0,
                            health: NodeHealth::Healthy,
                        }]
                    } else {
                        Vec::new()
                    },
                    snapshot_time: SystemTime::now(),
                };
                Ok(CommandResponse::success(
                    request_id,
                    cmd.command_name(),
                    CommandResponseData::ReplicationStatus(status),
                ))
            }
            InspectionCommand::InspectPromotionState => {
                let promo_state = self.kernel.get_promotion_state();
                let state = PromotionStateView {
                    state: promo_state.state_name().to_string(),
                    pending_replica: promo_state.replica_id(),
                    last_promotion: None,
                    snapshot_time: SystemTime::now(),
                };
                Ok(CommandResponse::success(
                    request_id,
                    cmd.command_name(),
                    CommandResponseData::PromotionState(state),
                ))
            }
        }
    }

    // =========================================================================
    // DIAGNOSTIC COMMANDS
    // =========================================================================

    fn execute_diagnostic(
        &self,
        request_id: Uuid,
        cmd: &DiagnosticCommand,
    ) -> ControlPlaneResult<CommandResponse> {
        match cmd {
            DiagnosticCommand::RunDiagnostics => {
                let repl_state = self.kernel.get_replication_state();
                let promo_state = self.kernel.get_promotion_state();

                let sections = vec![
                    DiagnosticSection {
                        name: "replication".to_string(),
                        entries: vec![
                            (
                                "status".to_string(),
                                if repl_state.is_halted() {
                                    "error"
                                } else {
                                    "ok"
                                }
                                .to_string(),
                            ),
                            ("state".to_string(), repl_state.state_name().to_string()),
                            ("can_write".to_string(), repl_state.can_write().to_string()),
                            ("can_read".to_string(), repl_state.can_read().to_string()),
                        ],
                    },
                    DiagnosticSection {
                        name: "promotion".to_string(),
                        entries: vec![
                            ("status".to_string(), "ok".to_string()),
                            ("state".to_string(), promo_state.state_name().to_string()),
                        ],
                    },
                    DiagnosticSection {
                        name: "wal".to_string(),
                        entries: vec![
                            ("status".to_string(), "ok".to_string()),
                            (
                                "position".to_string(),
                                self.kernel.get_wal_position().to_string(),
                            ),
                            (
                                "oldest".to_string(),
                                self.kernel.get_wal_oldest_position().to_string(),
                            ),
                            (
                                "size_bytes".to_string(),
                                self.kernel.get_wal_size_bytes().to_string(),
                            ),
                        ],
                    },
                ];

                let result = DiagnosticResult {
                    sections,
                    collected_at: SystemTime::now(),
                };
                Ok(CommandResponse::success(
                    request_id,
                    cmd.command_name(),
                    CommandResponseData::Diagnostics(result),
                ))
            }
            DiagnosticCommand::InspectWal => {
                let info = WalInfo {
                    current_position: self.kernel.get_wal_position(),
                    oldest_position: self.kernel.get_wal_oldest_position(),
                    size_bytes: self.kernel.get_wal_size_bytes(),
                    snapshot_time: SystemTime::now(),
                };
                Ok(CommandResponse::success(
                    request_id,
                    cmd.command_name(),
                    CommandResponseData::WalInfo(info),
                ))
            }
            DiagnosticCommand::InspectSnapshots => {
                let snapshots_data = self.kernel.get_snapshots();
                let checkpoints_data = self.kernel.get_checkpoints();

                let info = SnapshotInfo {
                    snapshots: snapshots_data
                        .into_iter()
                        .map(|(wal_position, created_at)| super::types::SnapshotMeta {
                            id: format!("snapshot-{}", wal_position),
                            created_at,
                            wal_position,
                        })
                        .collect(),
                    checkpoints: checkpoints_data
                        .into_iter()
                        .map(|(wal_position, created_at)| super::types::CheckpointMeta {
                            id: format!("checkpoint-{}", wal_position),
                            created_at,
                            wal_position,
                        })
                        .collect(),
                    snapshot_time: SystemTime::now(),
                };
                Ok(CommandResponse::success(
                    request_id,
                    cmd.command_name(),
                    CommandResponseData::SnapshtoInfo(info),
                ))
            }
        }
    }

    // =========================================================================
    // CONTROL COMMANDS
    // =========================================================================

    fn execute_control(
        &self,
        request_id: Uuid,
        cmd: &ControlCommand,
    ) -> ControlPlaneResult<CommandResponse> {
        match cmd {
            ControlCommand::RequestPromotion { replica_id, reason } => {
                let result_msg = self.kernel.request_promotion(
                    *replica_id,
                    reason.as_deref().unwrap_or("operator request"),
                );
                let (success, explanation) = match result_msg {
                    Ok(msg) => (true, msg),
                    Err(msg) => (false, msg),
                };
                let result = PromotionResultData {
                    replica_id: *replica_id,
                    success,
                    new_role: if success {
                        Some(NodeRole::Primary)
                    } else {
                        None
                    },
                    explanation,
                };
                Ok(CommandResponse::success(
                    request_id,
                    cmd.command_name(),
                    CommandResponseData::PromotionResult(result),
                ))
            }
            ControlCommand::RequestDemotion { node_id, reason } => {
                let result_msg = self
                    .kernel
                    .request_demotion(*node_id, reason.as_deref().unwrap_or("operator request"));
                let (success, explanation) = match result_msg {
                    Ok(msg) => (true, msg),
                    Err(msg) => (false, msg),
                };
                let result = PromotionResultData {
                    replica_id: *node_id,
                    success,
                    new_role: if success {
                        Some(NodeRole::Replica)
                    } else {
                        None
                    },
                    explanation,
                };
                Ok(CommandResponse::success(
                    request_id,
                    cmd.command_name(),
                    CommandResponseData::PromotionResult(result),
                ))
            }
            ControlCommand::ForcePromotion {
                replica_id,
                reason,
                acknowledged_risks: _,
            } => {
                let result_msg = self.kernel.force_promotion(*replica_id, reason.as_str());
                let (success, explanation) = match result_msg {
                    Ok(msg) => (true, msg),
                    Err(msg) => (false, msg),
                };
                let result = PromotionResultData {
                    replica_id: *replica_id,
                    success,
                    new_role: if success {
                        Some(NodeRole::Primary)
                    } else {
                        None
                    },
                    explanation,
                };
                Ok(CommandResponse::success(
                    request_id,
                    cmd.command_name(),
                    CommandResponseData::PromotionResult(result),
                ))
            }
        }
    }

    /// Request confirmation for a command (without executing).
    pub fn request_confirmation(&mut self, command: &ControlPlaneCommand) -> ConfirmationToken {
        let command_name = command.command_name();
        let target_id = self.extract_target_id(command);
        self.confirmation
            .request_confirmation(command_name, target_id)
    }

    /// Reject a pending confirmation.
    pub fn reject_confirmation(&mut self, token_id: Uuid) {
        self.confirmation.reject(token_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dx::api::control_plane::authority::AuthorityContext;

    #[test]
    fn test_inspection_no_confirmation() {
        let mut handler = ControlPlaneHandler::new();
        let cmd = ControlPlaneCommand::Inspection(InspectionCommand::InspectClusterState);
        let request = CommandRequest::new(cmd, AuthorityContext::observer());

        let response = handler.handle_command(request).unwrap();
        assert_eq!(response.outcome, CommandOutcome::Success);
    }

    #[test]
    fn test_control_requires_confirmation() {
        let mut handler = ControlPlaneHandler::new();
        let cmd = ControlPlaneCommand::Control(ControlCommand::RequestPromotion {
            replica_id: Uuid::new_v4(),
            reason: None,
        });
        let request = CommandRequest::new(cmd, AuthorityContext::operator());

        let response = handler.handle_command(request).unwrap();
        assert_eq!(response.outcome, CommandOutcome::AwaitingConfirmation);
        assert!(response.confirmation_token.is_some());
    }

    #[test]
    fn test_control_with_confirmation() {
        let mut handler = ControlPlaneHandler::new();
        let replica_id = Uuid::new_v4();
        let cmd = ControlPlaneCommand::Control(ControlCommand::RequestPromotion {
            replica_id,
            reason: None,
        });

        // First request - get confirmation token
        let request1 = CommandRequest::new(cmd.clone(), AuthorityContext::operator());
        let response1 = handler.handle_command(request1).unwrap();
        let token_id = response1.confirmation_token.unwrap();

        // Second request - with confirmation token
        let request2 =
            CommandRequest::new(cmd, AuthorityContext::operator()).with_confirmation(token_id);
        let response2 = handler.handle_command(request2).unwrap();

        assert_eq!(response2.outcome, CommandOutcome::Success);
    }

    #[test]
    fn test_insufficient_authority_rejected() {
        let mut handler = ControlPlaneHandler::new();
        let cmd = ControlPlaneCommand::Control(ControlCommand::RequestPromotion {
            replica_id: Uuid::new_v4(),
            reason: None,
        });

        // Observer cannot execute mutating commands
        let request = CommandRequest::new(cmd, AuthorityContext::observer());
        let result = handler.handle_command(request);

        assert!(result.is_err());
    }
}
