//! Promotion Subsystem
//!
//! Phase 6: Failover & Promotion
//!
//! Per PHASE6_VISION.md:
//! - Explicit, correctness-preserving failover and promotion
//! - Correctness over availability
//! - Determinism over automation
//! - Explicit authority over heuristics
//!
//! Per PHASE6_STATE_MACHINE.md:
//! - States are explicit and enumerable
//! - Transitions are event-driven, never inferred
//! - All transitions are deterministic
//! - No background or time-based transitions
//! - All authority changes are atomic
//! - All failures are explicit
//!
//! This module is ORTHOGONAL to Phase 5 replication.
//! It observes and constrains Phase 5 transitions but does not replace them.

mod controller;
#[cfg(test)]
mod crash_tests;
mod errors;
mod integration;
mod marker;
mod observability;
mod request;
mod state;
mod transition;
mod validator;

pub use controller::PromotionController;
pub use errors::{PromotionError, PromotionErrorKind, PromotionResult};
pub use integration::{RebindResult, ReplicationIntegration};
pub use marker::{AuthorityMarker, DurableMarker};
pub use observability::{
    InvariantCheck, PromotionEvent, PromotionExplanation, PromotionObserver, PromotionOutcome,
};
pub use request::{PromotionRequest, PromotionRequestResult};
pub use state::{DenialReason, PromotionState};
pub use transition::{AuthorityTransitionManager, TransitionFailureReason, TransitionResult};
pub use validator::{PromotionValidator, ValidationContext, ValidationResult};
