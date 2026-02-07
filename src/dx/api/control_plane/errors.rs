//! Phase 7 Error Model
//!
//! Per PHASE7_ERROR_MODEL.md:
//! - Errors are explicit, deterministic, and meaningful
//! - Errors never mask or reinterpret kernel decisions
//! - Errors do not trigger retries, fallbacks, or hidden behavior
//!
//! Error domains:
//! 1. Operator Input Errors
//! 2. Phase 7 Validation Errors
//! 3. Kernel Rejection Errors
//! 4. Infrastructure / Transport Errors

use std::fmt;

/// Phase 7 error domain classification.
///
/// Per PHASE7_ERROR_MODEL.md §3:
/// Errors MUST be classified into one and only one domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlPlaneErrorDomain {
    /// Invalid or incomplete operator input.
    OperatorInput,

    /// Phase 7 invariant enforcement or precondition check.
    ValidationError,

    /// Error returned by the correctness kernel (Phases 0–6).
    KernelRejection,

    /// Network, crash, or timeout errors.
    TransportError,
}

impl ControlPlaneErrorDomain {
    /// Returns the domain name string.
    pub fn as_str(&self) -> &'static str {
        match self {
            ControlPlaneErrorDomain::OperatorInput => "OPERATOR_INPUT",
            ControlPlaneErrorDomain::ValidationError => "VALIDATION_ERROR",
            ControlPlaneErrorDomain::KernelRejection => "KERNEL_REJECTION",
            ControlPlaneErrorDomain::TransportError => "TRANSPORT_ERROR",
        }
    }
}

impl fmt::Display for ControlPlaneErrorDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Execution outcome indicator.
///
/// Per PHASE7_ERROR_MODEL.md §9:
/// Errors MUST include execution outcome (executed / not executed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionOutcome {
    /// Action was definitely not executed.
    NotExecuted,

    /// Action was executed (error occurred after).
    Executed,

    /// Execution outcome is unknown (treat as not executed per PHASE7_FAILURE_MODEL.md).
    Unknown,
}

impl fmt::Display for ExecutionOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionOutcome::NotExecuted => write!(f, "NOT_EXECUTED"),
            ExecutionOutcome::Executed => write!(f, "EXECUTED"),
            ExecutionOutcome::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Phase 7 Control Plane Error.
///
/// Per PHASE7_ERROR_MODEL.md §9:
/// Errors MUST include domain, stable error code, human-readable message,
/// referenced invariant(s), and execution outcome.
#[derive(Debug)]
pub struct ControlPlaneError {
    /// Error domain classification.
    domain: ControlPlaneErrorDomain,

    /// Stable error code (PHASE7_category_name format).
    code: String,

    /// Human-readable error message.
    message: String,

    /// Referenced Phase 7 invariant, if applicable.
    invariant: Option<String>,

    /// Execution outcome.
    outcome: ExecutionOutcome,
}

impl ControlPlaneError {
    // =========================================================================
    // OPERATOR INPUT ERRORS (§4)
    // =========================================================================

    /// Create an error for missing required argument.
    pub fn missing_argument(field: &str) -> Self {
        Self {
            domain: ControlPlaneErrorDomain::OperatorInput,
            code: "PHASE7_MISSING_ARGUMENT".to_string(),
            message: format!("Missing required argument: {}", field),
            invariant: None,
            outcome: ExecutionOutcome::NotExecuted,
        }
    }

    /// Create an error for invalid node identifier.
    pub fn invalid_node_id(id: &str) -> Self {
        Self {
            domain: ControlPlaneErrorDomain::OperatorInput,
            code: "PHASE7_INVALID_NODE_ID".to_string(),
            message: format!("Invalid node identifier: {}", id),
            invariant: None,
            outcome: ExecutionOutcome::NotExecuted,
        }
    }

    /// Create an error for malformed request.
    pub fn malformed_request(reason: &str) -> Self {
        Self {
            domain: ControlPlaneErrorDomain::OperatorInput,
            code: "PHASE7_MALFORMED_REQUEST".to_string(),
            message: format!("Malformed request: {}", reason),
            invariant: None,
            outcome: ExecutionOutcome::NotExecuted,
        }
    }

    // =========================================================================
    // PHASE 7 VALIDATION ERRORS (§5)
    // =========================================================================

    /// Create an error for missing confirmation.
    ///
    /// Per PHASE7_CONFIRMATION_MODEL.md:
    /// No execution without confirmation.
    pub fn missing_confirmation(command: &str) -> Self {
        Self {
            domain: ControlPlaneErrorDomain::ValidationError,
            code: "PHASE7_MISSING_CONFIRMATION".to_string(),
            message: format!("Command '{}' requires confirmation", command),
            invariant: Some("P7-A2".to_string()),
            outcome: ExecutionOutcome::NotExecuted,
        }
    }

    /// Create an error for invalid authority level.
    pub fn insufficient_authority(required: &str, actual: &str) -> Self {
        Self {
            domain: ControlPlaneErrorDomain::ValidationError,
            code: "PHASE7_INSUFFICIENT_AUTHORITY".to_string(),
            message: format!("Required authority: {}, actual: {}", required, actual),
            invariant: Some("P7-A1".to_string()),
            outcome: ExecutionOutcome::NotExecuted,
        }
    }

    /// Create an error for confirmation reuse attempt.
    pub fn confirmation_reused() -> Self {
        Self {
            domain: ControlPlaneErrorDomain::ValidationError,
            code: "PHASE7_CONFIRMATION_REUSED".to_string(),
            message: "Confirmation token already consumed; new confirmation required".to_string(),
            invariant: Some("PHASE7_CONFIRMATION_MODEL.md §7".to_string()),
            outcome: ExecutionOutcome::NotExecuted,
        }
    }

    /// Create an error for confirmation expiry.
    pub fn confirmation_expired() -> Self {
        Self {
            domain: ControlPlaneErrorDomain::ValidationError,
            code: "PHASE7_CONFIRMATION_EXPIRED".to_string(),
            message: "Confirmation token expired; please reconfirm".to_string(),
            invariant: Some("P7-A2".to_string()),
            outcome: ExecutionOutcome::NotExecuted,
        }
    }

    /// Create an error for incomplete enhanced confirmation.
    pub fn incomplete_enhanced_confirmation() -> Self {
        Self {
            domain: ControlPlaneErrorDomain::ValidationError,
            code: "PHASE7_INCOMPLETE_ENHANCED_CONFIRMATION".to_string(),
            message: "Override command requires complete enhanced confirmation with all risks acknowledged".to_string(),
            invariant: Some("PHASE7_CONFIRMATION_MODEL.md §8".to_string()),
            outcome: ExecutionOutcome::NotExecuted,
        }
    }

    /// Create an error for scope violation.
    pub fn scope_violation(feature: &str) -> Self {
        Self {
            domain: ControlPlaneErrorDomain::ValidationError,
            code: "PHASE7_SCOPE_VIOLATION".to_string(),
            message: format!("Feature '{}' is out of scope for Phase 7", feature),
            invariant: Some("P7-S3".to_string()),
            outcome: ExecutionOutcome::NotExecuted,
        }
    }

    // =========================================================================
    // KERNEL REJECTION ERRORS (§6)
    // =========================================================================

    /// Create an error from a kernel rejection.
    ///
    /// Per PHASE7_ERROR_MODEL.md §6.2:
    /// Kernel rejections MUST be surfaced verbatim.
    pub fn from_kernel_rejection(kernel_code: &str, kernel_message: &str) -> Self {
        Self {
            domain: ControlPlaneErrorDomain::KernelRejection,
            code: kernel_code.to_string(),
            message: kernel_message.to_string(),
            invariant: None,
            outcome: ExecutionOutcome::NotExecuted,
        }
    }

    // =========================================================================
    // TRANSPORT ERRORS (§7)
    // =========================================================================

    /// Create an error for transport failure.
    ///
    /// Per PHASE7_ERROR_MODEL.md §7.2:
    /// MUST be treated as unknown execution outcome.
    pub fn transport_failure(reason: &str) -> Self {
        Self {
            domain: ControlPlaneErrorDomain::TransportError,
            code: "PHASE7_TRANSPORT_FAILURE".to_string(),
            message: format!(
                "Transport failure: {}. Execution outcome unknown; verify kernel state.",
                reason
            ),
            invariant: None,
            outcome: ExecutionOutcome::Unknown,
        }
    }

    /// Create an error for timeout.
    pub fn timeout() -> Self {
        Self {
            domain: ControlPlaneErrorDomain::TransportError,
            code: "PHASE7_TIMEOUT".to_string(),
            message: "Request timed out. Execution outcome unknown; verify kernel state."
                .to_string(),
            invariant: None,
            outcome: ExecutionOutcome::Unknown,
        }
    }

    // =========================================================================
    // ACCESSORS
    // =========================================================================

    /// Get the error domain.
    pub fn domain(&self) -> ControlPlaneErrorDomain {
        self.domain
    }

    /// Get the error code.
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Get the error message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Get the referenced invariant, if any.
    pub fn invariant(&self) -> Option<&str> {
        self.invariant.as_deref()
    }

    /// Get the execution outcome.
    pub fn outcome(&self) -> ExecutionOutcome {
        self.outcome
    }

    /// Check if execution definitely did not occur.
    pub fn definitely_not_executed(&self) -> bool {
        matches!(self.outcome, ExecutionOutcome::NotExecuted)
    }
}

impl fmt::Display for ControlPlaneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} ({}): {}",
            self.domain, self.code, self.outcome, self.message
        )?;
        if let Some(ref inv) = self.invariant {
            write!(f, " [invariant: {}]", inv)?;
        }
        Ok(())
    }
}

impl std::error::Error for ControlPlaneError {}

/// Result type for control plane operations.
pub type ControlPlaneResult<T> = Result<T, ControlPlaneError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operator_input_error() {
        let err = ControlPlaneError::missing_argument("replica_id");
        assert_eq!(err.domain(), ControlPlaneErrorDomain::OperatorInput);
        assert!(err.definitely_not_executed());
    }

    #[test]
    fn test_validation_error_with_invariant() {
        let err = ControlPlaneError::missing_confirmation("request_promotion");
        assert_eq!(err.domain(), ControlPlaneErrorDomain::ValidationError);
        assert_eq!(err.invariant(), Some("P7-A2"));
    }

    #[test]
    fn test_kernel_rejection_passthrough() {
        let err = ControlPlaneError::from_kernel_rejection(
            "AERO_PROMOTION_ALREADY_IN_PROGRESS",
            "Another promotion is already in progress",
        );
        assert_eq!(err.domain(), ControlPlaneErrorDomain::KernelRejection);
        assert_eq!(err.code(), "AERO_PROMOTION_ALREADY_IN_PROGRESS");
    }

    #[test]
    fn test_transport_error_unknown_outcome() {
        let err = ControlPlaneError::transport_failure("connection reset");
        assert_eq!(err.domain(), ControlPlaneErrorDomain::TransportError);
        assert!(!err.definitely_not_executed());
        assert_eq!(err.outcome(), ExecutionOutcome::Unknown);
    }

    #[test]
    fn test_error_display() {
        let err = ControlPlaneError::missing_confirmation("request_promotion");
        let display = format!("{}", err);
        assert!(display.contains("VALIDATION_ERROR"));
        assert!(display.contains("PHASE7_MISSING_CONFIRMATION"));
        assert!(display.contains("P7-A2"));
    }
}
