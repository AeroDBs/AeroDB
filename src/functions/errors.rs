//! # Function Errors

use thiserror::Error;

/// Result type for function operations
pub type FunctionResult<T> = Result<T, FunctionError>;

/// Function errors
#[derive(Debug, Clone, Error)]
pub enum FunctionError {
    #[error("Function not found: {0}")]
    NotFound(String),

    #[error("Function already exists: {0}")]
    AlreadyExists(String),

    #[error("Compilation error: {0}")]
    CompilationError(String),

    #[error("Function timeout after {0}ms")]
    Timeout(u64),

    #[error("Memory limit exceeded: {0}MB")]
    MemoryExceeded(u32),

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Invalid trigger: {0}")]
    InvalidTrigger(String),

    #[error("Invalid cron expression: {0}")]
    InvalidCron(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl FunctionError {
    /// Get HTTP status code
    pub fn status_code(&self) -> u16 {
        match self {
            FunctionError::NotFound(_) => 404,
            FunctionError::AlreadyExists(_) => 409,
            FunctionError::CompilationError(_) => 400,
            FunctionError::Timeout(_) => 504,
            FunctionError::MemoryExceeded(_) => 500,
            FunctionError::RuntimeError(_) => 500,
            FunctionError::InvalidTrigger(_) => 400,
            FunctionError::InvalidCron(_) => 400,
            FunctionError::Internal(_) => 500,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_codes() {
        assert_eq!(FunctionError::NotFound("test".into()).status_code(), 404);
        assert_eq!(FunctionError::Timeout(1000).status_code(), 504);
    }
}
