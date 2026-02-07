//! Index error types following ERRORS.md specification
//!
//! Error codes:
//! - AERO_INDEX_BUILD_FAILED (FATAL)
//! - AERO_DATA_CORRUPTION (FATAL)

use std::fmt;

/// Severity levels for index errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// System must halt immediately
    Fatal,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Fatal => write!(f, "FATAL"),
        }
    }
}

/// Index-specific error codes as defined in ERRORS.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexErrorCode {
    /// Index build failed
    AeroIndexBuildFailed,
    /// Data corruption detected during rebuild
    AeroDataCorruption,
}

impl IndexErrorCode {
    /// Returns the string code as defined in ERRORS.md
    pub fn code(&self) -> &'static str {
        match self {
            IndexErrorCode::AeroIndexBuildFailed => "AERO_INDEX_BUILD_FAILED",
            IndexErrorCode::AeroDataCorruption => "AERO_DATA_CORRUPTION",
        }
    }

    /// Returns the severity level for this error
    pub fn severity(&self) -> Severity {
        Severity::Fatal // All index errors are FATAL
    }

    /// Returns the invariant violated by this error
    pub fn invariant(&self) -> &'static str {
        match self {
            IndexErrorCode::AeroIndexBuildFailed => "R1",
            IndexErrorCode::AeroDataCorruption => "K2",
        }
    }
}

impl fmt::Display for IndexErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// Index error type with full context
#[derive(Debug)]
pub struct IndexError {
    /// Error code
    code: IndexErrorCode,
    /// Human-readable message
    message: String,
    /// Offset if applicable
    offset: Option<u64>,
}

impl IndexError {
    /// Create an index build failed error
    pub fn build_failed(reason: impl Into<String>) -> Self {
        Self {
            code: IndexErrorCode::AeroIndexBuildFailed,
            message: reason.into(),
            offset: None,
        }
    }

    /// Create a data corruption error
    pub fn data_corruption(offset: u64, reason: impl Into<String>) -> Self {
        Self {
            code: IndexErrorCode::AeroDataCorruption,
            message: format!("Data corruption at offset {}: {}", offset, reason.into()),
            offset: Some(offset),
        }
    }

    /// Returns the error code
    pub fn code(&self) -> IndexErrorCode {
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
        true // All index errors are FATAL
    }
}

impl fmt::Display for IndexError {
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

impl std::error::Error for IndexError {}

/// Result type for index operations
pub type IndexResult<T> = Result<T, IndexError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes_match_spec() {
        assert_eq!(
            IndexErrorCode::AeroIndexBuildFailed.code(),
            "AERO_INDEX_BUILD_FAILED"
        );
        assert_eq!(
            IndexErrorCode::AeroDataCorruption.code(),
            "AERO_DATA_CORRUPTION"
        );
    }

    #[test]
    fn test_all_errors_are_fatal() {
        let codes = [
            IndexErrorCode::AeroIndexBuildFailed,
            IndexErrorCode::AeroDataCorruption,
        ];

        for code in codes {
            assert_eq!(code.severity(), Severity::Fatal);
        }
    }

    #[test]
    fn test_error_display() {
        let err = IndexError::data_corruption(1234, "checksum mismatch");
        let display = format!("{}", err);
        assert!(display.contains("AERO_DATA_CORRUPTION"));
        assert!(display.contains("FATAL"));
        assert!(display.contains("K2"));
    }
}
