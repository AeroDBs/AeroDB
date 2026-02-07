//! Phase 7 Control Plane API
//!
//! Per PHASE7_CONTROL_PLANE_ARCHITECTURE.md:
//! - Control Plane API validates request structure
//! - Enforces Phase 7 invariants
//! - Orchestrates confirmation flows
//! - Routes requests to kernel boundary adapter
//!
//! Per PHASE7_INVARIANTS.md §P7-S2:
//! - Phase 7 MUST NOT alter kernel timing, execution order, or durability
//! - Kernel behavior must be identical with or without Phase 7

mod authority;
mod commands;
mod confirmation;
mod errors;
mod handlers;
mod types;

pub use authority::{AuthorityContext, AuthorityLevel};
pub use commands::{ControlCommand, ControlPlaneCommand, DiagnosticCommand, InspectionCommand};
pub use confirmation::{
    ConfirmationFlow, ConfirmationResult, ConfirmationStatus, ConfirmationToken,
    EnhancedConfirmation,
};
pub use errors::{ControlPlaneError, ControlPlaneErrorDomain, ControlPlaneResult};
pub use handlers::ControlPlaneHandler;
pub use types::{
    ClusterState, CommandOutcome, CommandRequest, CommandResponse, NodeState, PromotionStateView,
    ReplicationStatus,
};
