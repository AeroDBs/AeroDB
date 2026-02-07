//! Phase 7 Command Definitions
//!
//! Per PHASE7_COMMAND_MODEL.md:
//! - Complete and closed set of operator commands
//! - Each command maps to exactly one kernel action
//! - No command chaining or implicit follow-up actions
//!
//! Command Classes:
//! 1. Inspection Commands (read-only)
//! 2. Diagnostic Commands (read-only but potentially expensive)
//! 3. Control Commands (mutating, high-risk)

use std::fmt;
use uuid::Uuid;

/// All Phase 7 control plane commands.
///
/// Per PHASE7_COMMAND_MODEL.md §3:
/// If a command is not defined in this enum, it must not exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlPlaneCommand {
    /// Read-only inspection command.
    Inspection(InspectionCommand),

    /// Read-only diagnostic command (potentially expensive).
    Diagnostic(DiagnosticCommand),

    /// Mutating control command (requires confirmation).
    Control(ControlCommand),
}

impl ControlPlaneCommand {
    /// Returns whether this command requires confirmation.
    ///
    /// Per PHASE7_CONFIRMATION_MODEL.md §3:
    /// All Control Commands require confirmation.
    pub fn requires_confirmation(&self) -> bool {
        match self {
            ControlPlaneCommand::Inspection(_) => false,
            ControlPlaneCommand::Diagnostic(cmd) => cmd.requires_confirmation(),
            ControlPlaneCommand::Control(_) => true,
        }
    }

    /// Returns whether this command is mutating.
    pub fn is_mutating(&self) -> bool {
        matches!(self, ControlPlaneCommand::Control(_))
    }

    /// Returns the command name for audit logging.
    pub fn command_name(&self) -> &'static str {
        match self {
            ControlPlaneCommand::Inspection(cmd) => cmd.command_name(),
            ControlPlaneCommand::Diagnostic(cmd) => cmd.command_name(),
            ControlPlaneCommand::Control(cmd) => cmd.command_name(),
        }
    }
}

/// Inspection commands — read-only, no confirmation required.
///
/// Per PHASE7_COMMAND_MODEL.md §4:
/// Inspection commands MUST NOT mutate any kernel state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectionCommand {
    /// Retrieve current cluster topology and roles.
    InspectClusterState,

    /// Inspect a specific node's role, WAL position, and health.
    InspectNode { node_id: Uuid },

    /// View replication lag and replica health.
    InspectReplicationStatus,

    /// View current promotion/demotion state machine status.
    InspectPromotionState,
}

impl InspectionCommand {
    /// Returns the command name for audit logging.
    pub fn command_name(&self) -> &'static str {
        match self {
            InspectionCommand::InspectClusterState => "inspect_cluster_state",
            InspectionCommand::InspectNode { .. } => "inspect_node",
            InspectionCommand::InspectReplicationStatus => "inspect_replication_status",
            InspectionCommand::InspectPromotionState => "inspect_promotion_state",
        }
    }
}

/// Diagnostic commands — read-only but potentially expensive.
///
/// Per PHASE7_COMMAND_MODEL.md §5:
/// Diagnostics are read-only but may be disruptive or expensive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticCommand {
    /// Collect kernel diagnostic information.
    /// Requires confirmation due to potential cost.
    RunDiagnostics,

    /// Inspect WAL metadata and boundaries.
    InspectWal,

    /// Inspect available snapshots and checkpoints.
    InspectSnapshots,
}

impl DiagnosticCommand {
    /// Returns the command name for audit logging.
    pub fn command_name(&self) -> &'static str {
        match self {
            DiagnosticCommand::RunDiagnostics => "run_diagnostics",
            DiagnosticCommand::InspectWal => "inspect_wal",
            DiagnosticCommand::InspectSnapshots => "inspect_snapshots",
        }
    }

    /// Returns whether this diagnostic command requires confirmation.
    ///
    /// Per PHASE7_COMMAND_MODEL.md §5.1:
    /// run_diagnostics requires confirmation due to potential cost.
    pub fn requires_confirmation(&self) -> bool {
        matches!(self, DiagnosticCommand::RunDiagnostics)
    }
}

/// Control commands — mutating, require confirmation.
///
/// Per PHASE7_COMMAND_MODEL.md §6:
/// Control commands mutate kernel state and are strictly regulated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlCommand {
    /// Request promotion of a replica to primary.
    /// Confirmation required: Yes (mandatory).
    RequestPromotion {
        replica_id: Uuid,
        reason: Option<String>,
    },

    /// Request demotion of a primary.
    /// Confirmation required: Yes.
    RequestDemotion {
        node_id: Uuid,
        reason: Option<String>,
    },

    /// Explicit operator override for promotion.
    /// Requires enhanced confirmation with risk acknowledgement.
    ForcePromotion {
        replica_id: Uuid,
        reason: String,
        /// Acknowledgement of overridden invariants.
        acknowledged_risks: Vec<String>,
    },
}

impl ControlCommand {
    /// Returns the command name for audit logging.
    pub fn command_name(&self) -> &'static str {
        match self {
            ControlCommand::RequestPromotion { .. } => "request_promotion",
            ControlCommand::RequestDemotion { .. } => "request_demotion",
            ControlCommand::ForcePromotion { .. } => "force_promotion",
        }
    }

    /// Returns whether this command requires enhanced confirmation.
    ///
    /// Per PHASE7_CONFIRMATION_MODEL.md §8:
    /// Override commands require enhanced confirmation.
    pub fn requires_enhanced_confirmation(&self) -> bool {
        matches!(self, ControlCommand::ForcePromotion { .. })
    }

    /// Returns the target node/replica ID for this command.
    pub fn target_id(&self) -> Uuid {
        match self {
            ControlCommand::RequestPromotion { replica_id, .. } => *replica_id,
            ControlCommand::RequestDemotion { node_id, .. } => *node_id,
            ControlCommand::ForcePromotion { replica_id, .. } => *replica_id,
        }
    }
}

impl fmt::Display for ControlPlaneCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.command_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspection_commands_no_confirmation() {
        let cmd = ControlPlaneCommand::Inspection(InspectionCommand::InspectClusterState);
        assert!(!cmd.requires_confirmation());
        assert!(!cmd.is_mutating());
    }

    #[test]
    fn test_diagnostic_run_diagnostics_requires_confirmation() {
        let cmd = ControlPlaneCommand::Diagnostic(DiagnosticCommand::RunDiagnostics);
        assert!(cmd.requires_confirmation());
        assert!(!cmd.is_mutating());
    }

    #[test]
    fn test_diagnostic_inspect_wal_no_confirmation() {
        let cmd = ControlPlaneCommand::Diagnostic(DiagnosticCommand::InspectWal);
        assert!(!cmd.requires_confirmation());
    }

    #[test]
    fn test_control_commands_require_confirmation() {
        let cmd = ControlPlaneCommand::Control(ControlCommand::RequestPromotion {
            replica_id: Uuid::new_v4(),
            reason: None,
        });
        assert!(cmd.requires_confirmation());
        assert!(cmd.is_mutating());
    }

    #[test]
    fn test_force_promotion_requires_enhanced_confirmation() {
        let cmd = ControlCommand::ForcePromotion {
            replica_id: Uuid::new_v4(),
            reason: "Primary unavailable".to_string(),
            acknowledged_risks: vec!["P6-A1 may be violated".to_string()],
        };
        assert!(cmd.requires_enhanced_confirmation());
    }

    #[test]
    fn test_command_names() {
        assert_eq!(
            InspectionCommand::InspectClusterState.command_name(),
            "inspect_cluster_state"
        );
        assert_eq!(
            DiagnosticCommand::RunDiagnostics.command_name(),
            "run_diagnostics"
        );
        assert_eq!(
            ControlCommand::RequestPromotion {
                replica_id: Uuid::nil(),
                reason: None
            }
            .command_name(),
            "request_promotion"
        );
    }
}
