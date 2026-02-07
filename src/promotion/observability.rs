//! Promotion Observability
//!
//! Per PHASE6_OBSERVABILITY_MAPPING.md §2:
//! - Side-effect free
//! - Deterministic ordering
//! - Non-authoritative
//! - No feedback loops
//! - No gating behavior
//!
//! Observability DESCRIBES what happened; it never DECIDES what happens.
//!
//! Per PHASE6_INVARIANTS.md §P6-O1:
//! Every promotion attempt MUST emit: Start event, Validation result, Final decision.
//! Silent promotion is forbidden.
//!
//! Per PHASE6_INVARIANTS.md §P6-O2:
//! For every promotion decision, the system MUST be able to explain
//! why promotion was allowed or denied.

use super::state::{DenialReason, PromotionState};
use uuid::Uuid;

/// Promotion event types for observability.
///
/// Per PHASE6_OBSERVABILITY_MAPPING.md §3.1:
/// Promotion Lifecycle Events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromotionEvent {
    /// replication.promotion.requested
    /// Emitted when a promotion request is accepted for processing.
    Requested {
        replica_id: Uuid,
        reason: Option<String>,
    },

    /// replication.promotion.validation_started
    /// Emitted when transition to PromotionValidating occurs.
    ValidationStarted { replica_id: Uuid },

    /// replication.promotion.validation_failed
    /// Emitted when promotion is denied during validation.
    ValidationFailed {
        replica_id: Uuid,
        failed_invariant: String,
        failure_reason: String,
    },

    /// replication.promotion.validation_succeeded
    /// Emitted when promotion validation completes successfully.
    ValidationSucceeded { replica_id: Uuid },

    /// replication.promotion.transition_started
    /// Emitted when authority transition begins.
    TransitionStarted { replica_id: Uuid },

    /// replication.promotion.transition_completed
    /// Emitted when authority transition completes successfully.
    TransitionCompleted { new_primary_id: Uuid },

    /// replication.promotion.aborted_on_crash
    /// Emitted on recovery if a promotion was in progress but not completed.
    AbortedOnCrash {
        last_known_state: String,
        replica_id: Option<Uuid>,
    },
}

impl PromotionEvent {
    /// Get the event name for logging/metrics.
    pub fn event_name(&self) -> &'static str {
        match self {
            Self::Requested { .. } => "replication.promotion.requested",
            Self::ValidationStarted { .. } => "replication.promotion.validation_started",
            Self::ValidationFailed { .. } => "replication.promotion.validation_failed",
            Self::ValidationSucceeded { .. } => "replication.promotion.validation_succeeded",
            Self::TransitionStarted { .. } => "replication.promotion.transition_started",
            Self::TransitionCompleted { .. } => "replication.promotion.transition_completed",
            Self::AbortedOnCrash { .. } => "replication.promotion.aborted_on_crash",
        }
    }
}

/// Explanation artifact for promotion decisions.
///
/// Per PHASE6_OBSERVABILITY_MAPPING.md §5:
/// Every promotion attempt MUST produce an explanation artifact.
#[derive(Debug, Clone)]
pub struct PromotionExplanation {
    /// The replica involved.
    pub replica_id: Uuid,

    /// The final outcome.
    pub outcome: PromotionOutcome,

    /// Invariants checked.
    pub checked_invariants: Vec<InvariantCheck>,

    /// Human-readable explanation.
    pub explanation: String,
}

/// Outcome of a promotion attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromotionOutcome {
    /// Promotion succeeded.
    Succeeded,

    /// Promotion was denied.
    Denied { reason: DenialReason },

    /// Promotion was aborted (crash, operator abort, etc.).
    Aborted,
}

/// Result of checking an invariant.
#[derive(Debug, Clone)]
pub struct InvariantCheck {
    /// Invariant ID (e.g., "P6-S1").
    pub invariant_id: &'static str,

    /// Invariant description.
    pub description: &'static str,

    /// Whether the invariant was satisfied.
    pub satisfied: bool,
}

impl PromotionExplanation {
    /// Create an explanation for a successful promotion.
    pub fn success(replica_id: Uuid, checked: Vec<InvariantCheck>) -> Self {
        Self {
            replica_id,
            outcome: PromotionOutcome::Succeeded,
            checked_invariants: checked,
            explanation: "Promotion allowed: all Phase 6 invariants satisfied".to_string(),
        }
    }

    /// Create an explanation for a denied promotion.
    pub fn denied(replica_id: Uuid, reason: DenialReason, checked: Vec<InvariantCheck>) -> Self {
        let explanation = format!(
            "Promotion denied: {} (invariant {})",
            reason.description(),
            reason.invariant_reference()
        );
        Self {
            replica_id,
            outcome: PromotionOutcome::Denied { reason },
            checked_invariants: checked,
            explanation,
        }
    }

    /// Create an explanation for an aborted promotion.
    pub fn aborted(replica_id: Uuid) -> Self {
        Self {
            replica_id,
            outcome: PromotionOutcome::Aborted,
            checked_invariants: vec![],
            explanation: "Promotion aborted before completion".to_string(),
        }
    }
}

/// Observability collector for promotion events.
///
/// Per PHASE6_OBSERVABILITY_MAPPING.md §6:
/// If observability emission fails, promotion behavior MUST NOT change.
/// Observability failure is NEVER fatal to correctness.
pub struct PromotionObserver {
    /// Collected events (for testing/debugging).
    events: Vec<PromotionEvent>,
}

impl Default for PromotionObserver {
    fn default() -> Self {
        Self::new()
    }
}

impl PromotionObserver {
    /// Create a new observer.
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Emit an event.
    ///
    /// Per PHASE6_OBSERVABILITY_MAPPING.md §7:
    /// Phase 6 MUST NOT block promotion on logging failure.
    pub fn emit(&mut self, event: PromotionEvent) {
        // In production, this would integrate with the observability subsystem
        // Here we just collect for testing
        self.events.push(event);
    }

    /// Get all emitted events.
    pub fn events(&self) -> &[PromotionEvent] {
        &self.events
    }

    /// Clear all events.
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Emit event for state transition.
    pub fn emit_state_transition(&mut self, from: &PromotionState, to: &PromotionState) {
        match to {
            PromotionState::PromotionRequested { replica_id } => {
                self.emit(PromotionEvent::Requested {
                    replica_id: *replica_id,
                    reason: None,
                });
            }
            PromotionState::PromotionValidating { replica_id } => {
                self.emit(PromotionEvent::ValidationStarted {
                    replica_id: *replica_id,
                });
            }
            PromotionState::PromotionApproved { replica_id } => {
                self.emit(PromotionEvent::ValidationSucceeded {
                    replica_id: *replica_id,
                });
            }
            PromotionState::AuthorityTransitioning { replica_id } => {
                self.emit(PromotionEvent::TransitionStarted {
                    replica_id: *replica_id,
                });
            }
            PromotionState::PromotionSucceeded { new_primary_id } => {
                self.emit(PromotionEvent::TransitionCompleted {
                    new_primary_id: *new_primary_id,
                });
            }
            PromotionState::PromotionDenied { replica_id, reason } => {
                self.emit(PromotionEvent::ValidationFailed {
                    replica_id: *replica_id,
                    failed_invariant: reason.invariant_reference().to_string(),
                    failure_reason: reason.description().to_string(),
                });
            }
            PromotionState::Steady => {
                // No event for returning to steady
                let _ = from; // Silence unused variable warning
            }
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
    fn test_event_names() {
        let replica_id = test_uuid();

        assert_eq!(
            PromotionEvent::Requested {
                replica_id,
                reason: None
            }
            .event_name(),
            "replication.promotion.requested"
        );
        assert_eq!(
            PromotionEvent::ValidationStarted { replica_id }.event_name(),
            "replication.promotion.validation_started"
        );
        assert_eq!(
            PromotionEvent::TransitionCompleted {
                new_primary_id: replica_id
            }
            .event_name(),
            "replication.promotion.transition_completed"
        );
    }

    #[test]
    fn test_observer_collects_events() {
        let mut observer = PromotionObserver::new();
        let replica_id = test_uuid();

        observer.emit(PromotionEvent::Requested {
            replica_id,
            reason: None,
        });
        observer.emit(PromotionEvent::ValidationStarted { replica_id });

        assert_eq!(observer.events().len(), 2);
    }

    #[test]
    fn test_explanation_success() {
        let replica_id = test_uuid();
        let explanation = PromotionExplanation::success(replica_id, vec![]);

        assert_eq!(explanation.outcome, PromotionOutcome::Succeeded);
        assert!(explanation.explanation.contains("allowed"));
    }

    #[test]
    fn test_explanation_denied() {
        let replica_id = test_uuid();
        let reason = DenialReason::ReplicaBehindWal;
        let explanation = PromotionExplanation::denied(replica_id, reason, vec![]);

        match explanation.outcome {
            PromotionOutcome::Denied { reason: r } => {
                assert_eq!(r, DenialReason::ReplicaBehindWal);
            }
            _ => panic!("expected denied"),
        }
        assert!(explanation.explanation.contains("P6-S1"));
    }

    #[test]
    fn test_emit_state_transition() {
        let mut observer = PromotionObserver::new();
        let replica_id = test_uuid();

        let from = PromotionState::Steady;
        let to = PromotionState::PromotionRequested { replica_id };

        observer.emit_state_transition(&from, &to);

        assert_eq!(observer.events().len(), 1);
        assert_eq!(
            observer.events()[0].event_name(),
            "replication.promotion.requested"
        );
    }
}
