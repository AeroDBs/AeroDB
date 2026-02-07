//! Phase 7 Authority Model
//!
//! Per PHASE7_AUTHORITY_MODEL.md:
//! - Phase 7 grants the ability to ask, never the ability to decide
//! - Authority levels: Observer, Operator, Auditor
//! - Authority does not imply trust
//! - All requests are validated equally

use std::fmt;

/// Authority level for Phase 7 operations.
///
/// Per PHASE7_AUTHORITY_MODEL.md §3:
/// - Observer: read-only access
/// - Operator: can issue mutating commands with confirmation
/// - Auditor: can review action logs and reconstruct events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityLevel {
    /// May view cluster state, logs, metrics, and explanations.
    /// May NOT trigger any mutating action.
    Observer,

    /// May issue explicit, mutating commands with confirmation.
    /// Must accept responsibility for outcomes.
    Operator,

    /// May review action logs and reconstruct historical events.
    /// May NOT execute commands or replay actions.
    Auditor,
}

impl AuthorityLevel {
    /// Returns whether this authority level allows mutations.
    pub fn can_mutate(&self) -> bool {
        matches!(self, AuthorityLevel::Operator)
    }

    /// Returns whether this authority level allows observation.
    pub fn can_observe(&self) -> bool {
        true // All levels can observe
    }

    /// Returns whether this authority level allows audit access.
    pub fn can_audit(&self) -> bool {
        matches!(self, AuthorityLevel::Operator | AuthorityLevel::Auditor)
    }
}

impl fmt::Display for AuthorityLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthorityLevel::Observer => write!(f, "OBSERVER"),
            AuthorityLevel::Operator => write!(f, "OPERATOR"),
            AuthorityLevel::Auditor => write!(f, "AUDITOR"),
        }
    }
}

/// Authority context for a request.
///
/// Per PHASE7_AUTHORITY_MODEL.md §8:
/// Every mutating action must have a clear responsibility chain.
#[derive(Debug, Clone)]
pub struct AuthorityContext {
    /// Authority level of the requester.
    pub level: AuthorityLevel,

    /// Operator identity (if available).
    /// May be None for anonymous requests in early implementation.
    pub operator_id: Option<String>,

    /// Session identifier for correlation.
    pub session_id: Option<String>,
}

impl AuthorityContext {
    /// Create a new authority context.
    pub fn new(level: AuthorityLevel) -> Self {
        Self {
            level,
            operator_id: None,
            session_id: None,
        }
    }

    /// Create an observer context.
    pub fn observer() -> Self {
        Self::new(AuthorityLevel::Observer)
    }

    /// Create an operator context.
    pub fn operator() -> Self {
        Self::new(AuthorityLevel::Operator)
    }

    /// Create an auditor context.
    pub fn auditor() -> Self {
        Self::new(AuthorityLevel::Auditor)
    }

    /// Set operator identity.
    pub fn with_operator_id(mut self, id: impl Into<String>) -> Self {
        self.operator_id = Some(id.into());
        self
    }

    /// Set session ID.
    pub fn with_session_id(mut self, id: impl Into<String>) -> Self {
        self.session_id = Some(id.into());
        self
    }

    /// Check if mutations are allowed.
    pub fn can_mutate(&self) -> bool {
        self.level.can_mutate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authority_levels() {
        assert!(!AuthorityLevel::Observer.can_mutate());
        assert!(AuthorityLevel::Operator.can_mutate());
        assert!(!AuthorityLevel::Auditor.can_mutate());

        assert!(AuthorityLevel::Observer.can_observe());
        assert!(AuthorityLevel::Operator.can_observe());
        assert!(AuthorityLevel::Auditor.can_observe());

        assert!(!AuthorityLevel::Observer.can_audit());
        assert!(AuthorityLevel::Operator.can_audit());
        assert!(AuthorityLevel::Auditor.can_audit());
    }

    #[test]
    fn test_authority_context() {
        let ctx = AuthorityContext::operator()
            .with_operator_id("admin@example.com")
            .with_session_id("sess-123");

        assert!(ctx.can_mutate());
        assert_eq!(ctx.operator_id, Some("admin@example.com".to_string()));
        assert_eq!(ctx.session_id, Some("sess-123".to_string()));
    }
}
