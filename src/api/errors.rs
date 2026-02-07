//! API error types following ERRORS.md specification
//!
//! API errors are pass-through: they preserve the original error codes
//! from lower subsystems (Schema, Planner, Executor, etc.)

use std::fmt;

/// API error severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Recoverable error
    Error,
    /// System must halt
    Fatal,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "ERROR"),
            Severity::Fatal => write!(f, "FATAL"),
        }
    }
}

/// API-specific error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiErrorCode {
    /// Invalid request format
    AeroInvalidRequest,
    /// Unknown operation
    AeroUnknownOperation,
    /// Pass-through error from subsystem
    PassThrough,
}

impl ApiErrorCode {
    /// Returns the string code
    pub fn code(&self) -> &'static str {
        match self {
            ApiErrorCode::AeroInvalidRequest => "AERO_INVALID_REQUEST",
            ApiErrorCode::AeroUnknownOperation => "AERO_UNKNOWN_OPERATION",
            ApiErrorCode::PassThrough => "PASS_THROUGH",
        }
    }

    /// Returns the severity level
    pub fn severity(&self) -> Severity {
        match self {
            ApiErrorCode::AeroInvalidRequest => Severity::Error,
            ApiErrorCode::AeroUnknownOperation => Severity::Error,
            ApiErrorCode::PassThrough => Severity::Error, // Can be overridden
        }
    }
}

impl fmt::Display for ApiErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// API error with preserved subsystem error information
#[derive(Debug)]
pub struct ApiError {
    /// Original error code string (from subsystem or API)
    code: String,
    /// Error message
    message: String,
    /// Severity
    severity: Severity,
}

impl ApiError {
    /// Create an invalid request error
    pub fn invalid_request(reason: impl Into<String>) -> Self {
        Self {
            code: ApiErrorCode::AeroInvalidRequest.code().to_string(),
            message: reason.into(),
            severity: Severity::Error,
        }
    }

    /// Create an unknown operation error
    pub fn unknown_operation(op: impl Into<String>) -> Self {
        Self {
            code: ApiErrorCode::AeroUnknownOperation.code().to_string(),
            message: format!("Unknown operation: {}", op.into()),
            severity: Severity::Error,
        }
    }

    /// Create from a schema error (pass-through)
    pub fn from_schema_error(err: crate::schema::SchemaError) -> Self {
        Self {
            code: err.code().code().to_string(),
            message: err.message().to_string(),
            severity: if err.is_fatal() {
                Severity::Fatal
            } else {
                Severity::Error
            },
        }
    }

    /// Create from a planner error (pass-through)
    pub fn from_planner_error(err: crate::planner::PlannerError) -> Self {
        Self {
            code: err.code().code().to_string(),
            message: err.message().to_string(),
            severity: Severity::Error, // Planner errors are always recoverable
        }
    }

    /// Create from an executor error (pass-through)
    pub fn from_executor_error(err: crate::executor::ExecutorError) -> Self {
        Self {
            code: err.code().code().to_string(),
            message: err.message().to_string(),
            severity: if err.is_fatal() {
                Severity::Fatal
            } else {
                Severity::Error
            },
        }
    }

    /// Create from a WAL error (pass-through)
    pub fn from_wal_error(err: crate::wal::WalError) -> Self {
        Self {
            code: err.code().code().to_string(),
            message: err.message().to_string(),
            severity: if err.is_fatal() {
                Severity::Fatal
            } else {
                Severity::Error
            },
        }
    }

    /// Create from a storage error (pass-through)
    pub fn from_storage_error(err: crate::storage::StorageError) -> Self {
        Self {
            code: err.code().code().to_string(),
            message: err.message().to_string(),
            severity: if err.is_fatal() {
                Severity::Fatal
            } else {
                Severity::Error
            },
        }
    }

    /// Returns the error code
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Returns the error message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the severity
    pub fn severity(&self) -> Severity {
        self.severity
    }

    /// Returns whether this is a fatal error
    pub fn is_fatal(&self) -> bool {
        matches!(self.severity, Severity::Fatal)
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.severity, self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

/// Result type for API operations
pub type ApiResult<T> = Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_request_error() {
        let err = ApiError::invalid_request("missing field");
        assert_eq!(err.code(), "AERO_INVALID_REQUEST");
        assert!(!err.is_fatal());
    }

    #[test]
    fn test_unknown_operation_error() {
        let err = ApiError::unknown_operation("foo");
        assert_eq!(err.code(), "AERO_UNKNOWN_OPERATION");
        assert!(err.message().contains("foo"));
    }
}
