//! Promotion Controller
//!
//! Per PHASE6_ARCHITECTURE.md §3.1:
//! - Coordinates promotion attempts
//! - Orchestrates validation and decision flow
//! - Emits observability and explanation data
//!
//! Non-Responsibilities:
//! - Does not write WAL
//! - Does not modify replication protocol
//! - Does not infer liveness
//! - Does not retry automatically
//!
//! The Promotion Controller is purely coordinating.

use super::errors::{PromotionError, PromotionResult};
use super::request::{PromotionRequest, PromotionRequestResult};
use super::state::{DenialReason, PromotionState};
use uuid::Uuid;

/// Promotion Controller
///
/// Per PHASE6_ARCHITECTURE.md:
/// - Purely coordinating; no authority decisions
/// - Relies on Validator for allow/deny decisions
/// - Relies on Authority Transition Manager for apply
pub struct PromotionController {
    /// Current promotion state machine state
    state: PromotionState,
}

impl Default for PromotionController {
    fn default() -> Self {
        Self::new()
    }
}

impl PromotionController {
    /// Create a new promotion controller in Steady state.
    pub fn new() -> Self {
        Self {
            state: PromotionState::Steady,
        }
    }

    /// Get the current state.
    pub fn state(&self) -> &PromotionState {
        &self.state
    }

    /// Get the current state name for observability.
    pub fn state_name(&self) -> &'static str {
        self.state.state_name()
    }

    /// Check if a promotion is in progress.
    pub fn is_promotion_in_progress(&self) -> bool {
        self.state.is_promotion_in_progress()
    }

    // =========================================================================
    // REQUEST HANDLING (Stage 6.2)
    // =========================================================================

    /// Submit a promotion request.
    ///
    /// Per PHASE6_SCOPE.md §2.1:
    /// - Promotion is explicitly triggered
    /// - Deterministically evaluated
    /// - Either accepted or rejected atomically
    ///
    /// Per PHASE6_INVARIANTS.md §P6-F3:
    /// - Promotion failures MUST NOT be retried automatically
    /// - MUST require explicit re-attempt
    pub fn request_promotion(&mut self, request: PromotionRequest) -> PromotionRequestResult {
        // Validate request format first
        if let Some(reason) = request.validate_format() {
            return PromotionRequestResult::Rejected {
                replica_id: request.replica_id,
                reason,
            };
        }

        // Check if promotion already in progress
        if let Some(existing_id) = self.state.replica_id() {
            return PromotionRequestResult::AlreadyInProgress {
                existing_replica_id: existing_id,
            };
        }

        // Attempt state transition: Steady → PromotionRequested
        match std::mem::replace(&mut self.state, PromotionState::Steady)
            .request_promotion(request.replica_id)
        {
            Ok(new_state) => {
                self.state = new_state;
                PromotionRequestResult::Accepted {
                    replica_id: request.replica_id,
                }
            }
            Err(_) => {
                // State doesn't allow promotion request
                PromotionRequestResult::Rejected {
                    replica_id: request.replica_id,
                    reason: "promotion request not allowed in current state",
                }
            }
        }
    }

    /// Reject the current promotion request immediately.
    ///
    /// Used when the request is invalid (e.g., unknown replica).
    pub fn reject_request(&mut self) -> PromotionResult<()> {
        self.state = std::mem::replace(&mut self.state, PromotionState::Steady).reject_request()?;
        Ok(())
    }

    /// Begin validation of the current promotion request.
    ///
    /// Called after initial request checks pass.
    pub fn begin_validation(&mut self) -> PromotionResult<Uuid> {
        let replica_id = self
            .state
            .replica_id()
            .ok_or_else(PromotionError::no_promotion_in_progress)?;

        self.state =
            std::mem::replace(&mut self.state, PromotionState::Steady).begin_validation()?;

        Ok(replica_id)
    }

    /// Approve the promotion after validation succeeds.
    pub fn approve_promotion(&mut self) -> PromotionResult<Uuid> {
        let replica_id = self
            .state
            .replica_id()
            .ok_or_else(PromotionError::no_promotion_in_progress)?;

        self.state =
            std::mem::replace(&mut self.state, PromotionState::Steady).approve_promotion()?;

        Ok(replica_id)
    }

    /// Deny the promotion with an explicit reason.
    pub fn deny_promotion(&mut self, reason: DenialReason) -> PromotionResult<Uuid> {
        let replica_id = self
            .state
            .replica_id()
            .ok_or_else(PromotionError::no_promotion_in_progress)?;

        self.state =
            std::mem::replace(&mut self.state, PromotionState::Steady).deny_promotion(reason)?;

        Ok(replica_id)
    }

    /// Begin authority transition after approval.
    pub fn begin_authority_transition(&mut self) -> PromotionResult<Uuid> {
        let replica_id = self
            .state
            .replica_id()
            .ok_or_else(PromotionError::no_promotion_in_progress)?;

        self.state = std::mem::replace(&mut self.state, PromotionState::Steady)
            .begin_authority_transition()?;

        Ok(replica_id)
    }

    /// Complete the authority transition.
    pub fn complete_transition(&mut self) -> PromotionResult<Uuid> {
        let replica_id = self
            .state
            .replica_id()
            .ok_or_else(PromotionError::no_promotion_in_progress)?;

        self.state =
            std::mem::replace(&mut self.state, PromotionState::Steady).complete_transition()?;

        Ok(replica_id)
    }

    /// Acknowledge successful promotion and return to steady state.
    pub fn acknowledge_success(&mut self) -> PromotionResult<Uuid> {
        let new_primary_id = self
            .state
            .replica_id()
            .ok_or_else(PromotionError::no_promotion_in_progress)?;

        self.state =
            std::mem::replace(&mut self.state, PromotionState::Steady).acknowledge_success()?;

        Ok(new_primary_id)
    }

    /// Acknowledge promotion denial and return to steady state.
    pub fn acknowledge_denial(&mut self) -> PromotionResult<()> {
        self.state =
            std::mem::replace(&mut self.state, PromotionState::Steady).acknowledge_denial()?;
        Ok(())
    }

    /// Recover after crash.
    ///
    /// Per PHASE6_STATE_MACHINE.md §5:
    /// - System MUST re-enter Steady
    /// - Authority state MUST be reconstructed deterministically
    pub fn recover_after_crash(
        authority_transition_was_atomic: bool,
        new_primary_id: Option<Uuid>,
    ) -> Self {
        Self {
            state: PromotionState::recover_after_crash(
                authority_transition_was_atomic,
                new_primary_id,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_uuid() -> Uuid {
        Uuid::new_v4()
    }

    #[test]
    fn test_controller_starts_steady() {
        let controller = PromotionController::new();
        assert_eq!(controller.state_name(), "Steady");
        assert!(!controller.is_promotion_in_progress());
    }

    #[test]
    fn test_request_promotion_accepted() {
        let mut controller = PromotionController::new();
        let replica_id = test_uuid();

        let result = controller.request_promotion(PromotionRequest::new(replica_id));

        assert!(result.is_accepted());
        assert_eq!(result.accepted_replica_id(), Some(replica_id));
        assert_eq!(controller.state_name(), "PromotionRequested");
    }

    #[test]
    fn test_request_promotion_nil_uuid_rejected() {
        let mut controller = PromotionController::new();

        let result = controller.request_promotion(PromotionRequest::new(Uuid::nil()));

        assert!(!result.is_accepted());
        match result {
            PromotionRequestResult::Rejected { reason, .. } => {
                assert!(reason.contains("nil"));
            }
            _ => panic!("expected Rejected"),
        }
    }

    #[test]
    fn test_request_promotion_already_in_progress() {
        let mut controller = PromotionController::new();
        let replica1 = test_uuid();
        let replica2 = test_uuid();

        // First request succeeds
        controller.request_promotion(PromotionRequest::new(replica1));

        // Second request fails
        let result = controller.request_promotion(PromotionRequest::new(replica2));

        match result {
            PromotionRequestResult::AlreadyInProgress {
                existing_replica_id,
            } => {
                assert_eq!(existing_replica_id, replica1);
            }
            _ => panic!("expected AlreadyInProgress"),
        }
    }

    #[test]
    fn test_controller_full_lifecycle() {
        let mut controller = PromotionController::new();
        let replica_id = test_uuid();

        // Request
        let result = controller.request_promotion(PromotionRequest::new(replica_id));
        assert!(result.is_accepted());

        // Begin validation
        let id = controller.begin_validation().unwrap();
        assert_eq!(id, replica_id);
        assert_eq!(controller.state_name(), "PromotionValidating");

        // Approve
        let id = controller.approve_promotion().unwrap();
        assert_eq!(id, replica_id);
        assert_eq!(controller.state_name(), "PromotionApproved");

        // Begin transition
        let id = controller.begin_authority_transition().unwrap();
        assert_eq!(id, replica_id);
        assert_eq!(controller.state_name(), "AuthorityTransitioning");

        // Complete transition
        let id = controller.complete_transition().unwrap();
        assert_eq!(id, replica_id);
        assert_eq!(controller.state_name(), "PromotionSucceeded");

        // Acknowledge success
        let id = controller.acknowledge_success().unwrap();
        assert_eq!(id, replica_id);
        assert_eq!(controller.state_name(), "Steady");
    }

    #[test]
    fn test_controller_denial_lifecycle() {
        let mut controller = PromotionController::new();
        let replica_id = test_uuid();

        // Request
        controller.request_promotion(PromotionRequest::new(replica_id));

        // Begin validation
        controller.begin_validation().unwrap();

        // Deny
        controller
            .deny_promotion(DenialReason::ReplicaBehindWal)
            .unwrap();
        assert_eq!(controller.state_name(), "PromotionDenied");

        // Acknowledge denial
        controller.acknowledge_denial().unwrap();
        assert_eq!(controller.state_name(), "Steady");
    }

    #[test]
    fn test_controller_reject_request() {
        let mut controller = PromotionController::new();
        let replica_id = test_uuid();

        // Request
        controller.request_promotion(PromotionRequest::new(replica_id));

        // Reject (e.g., unknown replica)
        controller.reject_request().unwrap();
        assert_eq!(controller.state_name(), "Steady");
    }

    #[test]
    fn test_controller_crash_recovery() {
        // Non-atomic crash → Steady
        let controller = PromotionController::recover_after_crash(false, None);
        assert_eq!(controller.state_name(), "Steady");

        // Atomic crash → PromotionSucceeded
        let new_id = test_uuid();
        let controller = PromotionController::recover_after_crash(true, Some(new_id));
        assert_eq!(controller.state_name(), "PromotionSucceeded");
    }
}
