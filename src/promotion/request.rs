//! Promotion Request Types
//!
//! Per PHASE6_SCOPE.md §2.1:
//! - Promotion requests are explicitly triggered
//! - Deterministically evaluated
//! - Either accepted or rejected atomically
//!
//! Per PHASE6_INVARIANTS.md §P6-A3:
//! A node MUST NOT assume primary authority automatically.
//! Authority is granted only via explicit promotion.

use uuid::Uuid;

/// A request to promote a replica to primary.
///
/// Per PHASE6_STATE_MACHINE.md §4.2:
/// - An explicit promotion request has been issued
/// - No validation has begun yet
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotionRequest {
    /// The replica to promote
    pub replica_id: Uuid,

    /// Reason for promotion (operator-provided)
    /// Used for observability only, does not affect decision.
    pub reason: Option<String>,

    /// Whether to force promotion without validating primary liveness.
    ///
    /// DANGER: Setting this to true may violate P6-A1 if primary is still active.
    /// Use only when primary is provably unavailable.
    pub force: bool,
}

impl PromotionRequest {
    /// Create a new promotion request.
    pub fn new(replica_id: Uuid) -> Self {
        Self {
            replica_id,
            reason: None,
            force: false,
        }
    }

    /// Set the reason for promotion.
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Set force flag.
    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    /// Validate the request format (not safety).
    ///
    /// Returns None if valid, or Some(reason) if invalid.
    pub fn validate_format(&self) -> Option<&'static str> {
        // UUID cannot be nil
        if self.replica_id.is_nil() {
            return Some("replica_id cannot be nil UUID");
        }
        None
    }
}

/// Result of a promotion request submission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromotionRequestResult {
    /// Request accepted and validation will begin.
    Accepted { replica_id: Uuid },

    /// Request rejected immediately.
    Rejected {
        replica_id: Uuid,
        reason: &'static str,
    },

    /// Another promotion is already in progress.
    AlreadyInProgress { existing_replica_id: Uuid },
}

impl PromotionRequestResult {
    /// Check if the request was accepted.
    pub fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted { .. })
    }

    /// Get replica ID if accepted.
    pub fn accepted_replica_id(&self) -> Option<Uuid> {
        match self {
            Self::Accepted { replica_id } => Some(*replica_id),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_creation() {
        let id = Uuid::new_v4();
        let request = PromotionRequest::new(id);

        assert_eq!(request.replica_id, id);
        assert!(request.reason.is_none());
        assert!(!request.force);
    }

    #[test]
    fn test_request_with_reason() {
        let id = Uuid::new_v4();
        let request = PromotionRequest::new(id).with_reason("Primary unavailable");

        assert_eq!(request.reason, Some("Primary unavailable".to_string()));
    }

    #[test]
    fn test_request_with_force() {
        let id = Uuid::new_v4();
        let request = PromotionRequest::new(id).with_force(true);

        assert!(request.force);
    }

    #[test]
    fn test_validate_format_nil_uuid() {
        let request = PromotionRequest::new(Uuid::nil());
        assert!(request.validate_format().is_some());
    }

    #[test]
    fn test_validate_format_valid() {
        let request = PromotionRequest::new(Uuid::new_v4());
        assert!(request.validate_format().is_none());
    }

    #[test]
    fn test_result_is_accepted() {
        let id = Uuid::new_v4();

        let accepted = PromotionRequestResult::Accepted { replica_id: id };
        assert!(accepted.is_accepted());
        assert_eq!(accepted.accepted_replica_id(), Some(id));

        let rejected = PromotionRequestResult::Rejected {
            replica_id: id,
            reason: "test",
        };
        assert!(!rejected.is_accepted());
        assert!(rejected.accepted_replica_id().is_none());
    }
}
