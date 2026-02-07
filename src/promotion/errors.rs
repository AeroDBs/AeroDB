//! Promotion Error Types
//!
//! Per PHASE6_INVARIANTS.md §P6-F1:
//! - If promotion safety cannot be proven, promotion MUST be rejected
//! - System MUST NOT guess
//! - Explicit failure is preferred over unsafe success

use std::fmt;

/// Promotion error type
#[derive(Debug, Clone)]
pub struct PromotionError {
    /// Error kind
    pub kind: PromotionErrorKind,
    /// Error message
    pub message: String,
}

/// Promotion error kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromotionErrorKind {
    /// Forbidden state transition attempted
    ForbiddenTransition,

    /// Promotion validation failed
    ValidationFailed,

    /// No promotion in progress
    NoPromotionInProgress,

    /// Promotion already in progress
    PromotionAlreadyInProgress,

    /// Invalid replica for promotion
    InvalidReplica,

    /// Authority transition failed
    AuthorityTransitionFailed,

    /// Promotion denied by validator
    PromotionDenied,
}

impl PromotionError {
    /// Create a new promotion error.
    pub fn new(kind: PromotionErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Create a forbidden transition error.
    pub fn forbidden_transition(from: &str, to: &str) -> Self {
        Self::new(
            PromotionErrorKind::ForbiddenTransition,
            format!("forbidden transition: {} → {}", from, to),
        )
    }

    /// Create a validation failed error.
    pub fn validation_failed(reason: impl Into<String>) -> Self {
        Self::new(PromotionErrorKind::ValidationFailed, reason)
    }

    /// Create a no promotion in progress error.
    pub fn no_promotion_in_progress() -> Self {
        Self::new(
            PromotionErrorKind::NoPromotionInProgress,
            "no promotion attempt is in progress",
        )
    }

    /// Create a promotion already in progress error.
    pub fn promotion_already_in_progress() -> Self {
        Self::new(
            PromotionErrorKind::PromotionAlreadyInProgress,
            "a promotion attempt is already in progress",
        )
    }
}

impl fmt::Display for PromotionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PromotionError({:?}): {}", self.kind, self.message)
    }
}

impl std::error::Error for PromotionError {}

/// Result type for promotion operations
pub type PromotionResult<T> = Result<T, PromotionError>;
