//! Core Error Types
//!
//! Unified error handling for the execution pipeline.

use std::fmt;

/// Core module result type
pub type CoreResult<T> = Result<T, CoreError>;

/// Core error type
#[derive(Debug)]
pub enum CoreError {
    /// Authentication required
    AuthRequired,

    /// Access denied by policy
    AccessDenied(String),

    /// Resource not found
    NotFound(String),

    /// Validation error
    Validation(String),

    /// Execution error
    Execution(String),

    /// Internal error
    Internal(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuthRequired => write!(f, "Authentication required"),
            Self::AccessDenied(msg) => write!(f, "Access denied: {}", msg),
            Self::NotFound(msg) => write!(f, "Not found: {}", msg),
            Self::Validation(msg) => write!(f, "Validation error: {}", msg),
            Self::Execution(msg) => write!(f, "Execution error: {}", msg),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for CoreError {}

impl CoreError {
    /// Create a not found error
    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound(resource.into())
    }

    /// Create a validation error
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    /// Create an access denied error
    pub fn access_denied(msg: impl Into<String>) -> Self {
        Self::AccessDenied(msg.into())
    }

    /// Create an execution error
    pub fn execution(msg: impl Into<String>) -> Self {
        Self::Execution(msg.into())
    }

    /// Create an internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Get error code for API responses
    pub fn code(&self) -> &'static str {
        match self {
            Self::AuthRequired => "AUTH_REQUIRED",
            Self::AccessDenied(_) => "ACCESS_DENIED",
            Self::NotFound(_) => "NOT_FOUND",
            Self::Validation(_) => "VALIDATION_ERROR",
            Self::Execution(_) => "EXECUTION_ERROR",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }

    /// Get HTTP status code
    pub fn status_code(&self) -> u16 {
        match self {
            Self::AuthRequired => 401,
            Self::AccessDenied(_) => 403,
            Self::NotFound(_) => 404,
            Self::Validation(_) => 400,
            Self::Execution(_) => 500,
            Self::Internal(_) => 500,
        }
    }
}

impl From<serde_json::Error> for CoreError {
    fn from(e: serde_json::Error) -> Self {
        Self::Validation(e.to_string())
    }
}
