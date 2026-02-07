//! # Function Errors

use thiserror::Error;

/// Result type for function operations
pub type FunctionResult<T> = Result<T, FunctionError>;

/// Function errors
///
/// MANIFESTO ALIGNMENT: All error paths are explicit. No silent failures.
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

    /// MANIFESTO ALIGNMENT: WASM modules MUST export required functions.
    /// Silent success for missing exports violates fail-fast principle.
    /// Per Design Manifesto: "fail loudly, execute predictably, leave no surprises."
    #[error("WASM module must export '{export_name}' function. No silent success allowed.")]
    MissingExport { export_name: &'static str },

    /// MANIFESTO ALIGNMENT: Host functions that are declared but not bound
    /// must fail explicitly. No stub behavior permitted.
    /// Per Design Manifesto: "Determinism over magic"
    #[error("Host function '{function_name}' is not implemented. WASM modules cannot call unbound host functions.")]
    HostFunctionNotBound { function_name: &'static str },
}

impl FunctionError {
    /// Get HTTP status code
    ///
    /// MANIFESTO ALIGNMENT: Error codes are explicit and documented.
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
            // MANIFESTO ALIGNMENT: Missing exports are client errors (bad WASM)
            FunctionError::MissingExport { .. } => 400,
            // MANIFESTO ALIGNMENT: Unbound host functions are server limitations (501 Not Implemented)
            FunctionError::HostFunctionNotBound { .. } => 501,
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
