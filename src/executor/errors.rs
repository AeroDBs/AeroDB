//! Executor error types following ERRORS.md specification
//!
//! Error codes:
//! - AERO_EXECUTION_FAILED (ERROR)
//! - AERO_DATA_CORRUPTION (FATAL)
//! - AERO_EXECUTION_LIMIT (ERROR)

use std::fmt;

/// Severity levels for executor errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Operation failed but system is healthy
    Error,
    /// System must halt immediately
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

/// Executor-specific error codes as defined in ERRORS.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutorErrorCode {
    /// General execution failure
    AeroExecutionFailed,
    /// Data checksum mismatch (FATAL)
    AeroDataCorruption,
    /// Limit exceeded during execution
    AeroExecutionLimit,
}

impl ExecutorErrorCode {
    /// Returns the string code as defined in ERRORS.md
    pub fn code(&self) -> &'static str {
        match self {
            ExecutorErrorCode::AeroExecutionFailed => "AERO_EXECUTION_FAILED",
            ExecutorErrorCode::AeroDataCorruption => "AERO_DATA_CORRUPTION",
            ExecutorErrorCode::AeroExecutionLimit => "AERO_EXECUTION_LIMIT",
        }
    }

    /// Returns the severity level for this error
    pub fn severity(&self) -> Severity {
        match self {
            ExecutorErrorCode::AeroDataCorruption => Severity::Fatal,
            _ => Severity::Error,
        }
    }

    /// Returns the invariant violated by this error
    pub fn invariant(&self) -> &'static str {
        match self {
            ExecutorErrorCode::AeroExecutionFailed => "T2",
            ExecutorErrorCode::AeroDataCorruption => "D2",
            ExecutorErrorCode::AeroExecutionLimit => "Q1",
        }
    }
}

impl fmt::Display for ExecutorErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// Executor error type with full context
#[derive(Debug)]
pub struct ExecutorError {
    /// Error code
    code: ExecutorErrorCode,
    /// Human-readable message
    message: String,
    /// File offset if applicable
    offset: Option<u64>,
}

impl ExecutorError {
    /// Create an execution failed error
    pub fn execution_failed(reason: impl Into<String>) -> Self {
        Self {
            code: ExecutorErrorCode::AeroExecutionFailed,
            message: reason.into(),
            offset: None,
        }
    }

    /// Create a data corruption error (FATAL)
    pub fn data_corruption(offset: u64, reason: impl Into<String>) -> Self {
        Self {
            code: ExecutorErrorCode::AeroDataCorruption,
            message: format!("Data corruption at offset {}: {}", offset, reason.into()),
            offset: Some(offset),
        }
    }

    /// Create an execution limit error
    pub fn execution_limit(reason: impl Into<String>) -> Self {
        Self {
            code: ExecutorErrorCode::AeroExecutionLimit,
            message: reason.into(),
            offset: None,
        }
    }

    /// Returns the error code
    pub fn code(&self) -> ExecutorErrorCode {
        self.code
    }

    /// Returns the severity level
    pub fn severity(&self) -> Severity {
        self.code.severity()
    }

    /// Returns the invariant violated
    pub fn invariant(&self) -> &'static str {
        self.code.invariant()
    }

    /// Returns the error message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the offset if applicable
    pub fn offset(&self) -> Option<u64> {
        self.offset
    }

    /// Returns whether this is a fatal error
    pub fn is_fatal(&self) -> bool {
        self.severity() == Severity::Fatal
    }
}

impl fmt::Display for ExecutorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {}: {}",
            self.code.severity(),
            self.code.code(),
            self.message
        )?;
        write!(f, " [violates {}]", self.code.invariant())?;
        Ok(())
    }
}

impl std::error::Error for ExecutorError {}

/// Result type for executor operations
pub type ExecutorResult<T> = Result<T, ExecutorError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes_match_spec() {
        assert_eq!(
            ExecutorErrorCode::AeroExecutionFailed.code(),
            "AERO_EXECUTION_FAILED"
        );
        assert_eq!(
            ExecutorErrorCode::AeroDataCorruption.code(),
            "AERO_DATA_CORRUPTION"
        );
        assert_eq!(
            ExecutorErrorCode::AeroExecutionLimit.code(),
            "AERO_EXECUTION_LIMIT"
        );
    }

    #[test]
    fn test_corruption_is_fatal() {
        let err = ExecutorError::data_corruption(1234, "checksum mismatch");
        assert!(err.is_fatal());
        assert_eq!(err.code().severity(), Severity::Fatal);
    }

    #[test]
    fn test_execution_failed_not_fatal() {
        let err = ExecutorError::execution_failed("test error");
        assert!(!err.is_fatal());
        assert_eq!(err.code().severity(), Severity::Error);
    }

    #[test]
    fn test_error_display() {
        let err = ExecutorError::data_corruption(100, "bad checksum");
        let display = format!("{}", err);
        assert!(display.contains("AERO_DATA_CORRUPTION"));
        assert!(display.contains("FATAL"));
        assert!(display.contains("D2"));
    }
}
