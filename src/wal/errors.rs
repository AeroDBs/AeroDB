//! WAL error types following ERRORS.md specification
//!
//! Error codes:
//! - AERO_WAL_APPEND_FAILED (ERROR severity)
//! - AERO_WAL_FSYNC_FAILED (FATAL severity)
//! - AERO_WAL_CORRUPTION (FATAL severity)

use std::fmt;
use std::io;

/// Severity levels for WAL errors as defined in ERRORS.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Operation fails, server continues
    Error,
    /// aerodb must terminate
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

/// WAL-specific error codes as defined in ERRORS.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalErrorCode {
    /// WAL write failed
    AeroWalAppendFailed,
    /// WAL fsync failed
    AeroWalFsyncFailed,
    /// WAL checksum failure
    AeroWalCorruption,
}

impl WalErrorCode {
    /// Returns the string code as defined in ERRORS.md
    pub fn code(&self) -> &'static str {
        match self {
            WalErrorCode::AeroWalAppendFailed => "AERO_WAL_APPEND_FAILED",
            WalErrorCode::AeroWalFsyncFailed => "AERO_WAL_FSYNC_FAILED",
            WalErrorCode::AeroWalCorruption => "AERO_WAL_CORRUPTION",
        }
    }

    /// Returns the severity level for this error
    pub fn severity(&self) -> Severity {
        match self {
            WalErrorCode::AeroWalAppendFailed => Severity::Error,
            WalErrorCode::AeroWalFsyncFailed => Severity::Fatal,
            WalErrorCode::AeroWalCorruption => Severity::Fatal,
        }
    }

    /// Returns the invariant violated by this error, if applicable
    pub fn invariant(&self) -> Option<&'static str> {
        match self {
            WalErrorCode::AeroWalAppendFailed => Some("D1"),
            WalErrorCode::AeroWalFsyncFailed => Some("D1"),
            WalErrorCode::AeroWalCorruption => Some("K2"),
        }
    }
}

impl fmt::Display for WalErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// WAL error type with full context as required by ERRORS.md
#[derive(Debug)]
pub struct WalError {
    /// Error code
    code: WalErrorCode,
    /// Human-readable message
    message: String,
    /// Optional details about the error context
    details: Option<String>,
    /// Underlying IO error if applicable
    source: Option<io::Error>,
}

impl WalError {
    /// Create a new WAL append failed error
    pub fn append_failed(message: impl Into<String>, source: io::Error) -> Self {
        Self {
            code: WalErrorCode::AeroWalAppendFailed,
            message: message.into(),
            details: None,
            source: Some(source),
        }
    }

    /// Create a new WAL fsync failed error
    pub fn fsync_failed(message: impl Into<String>, source: io::Error) -> Self {
        Self {
            code: WalErrorCode::AeroWalFsyncFailed,
            message: message.into(),
            details: None,
            source: Some(source),
        }
    }

    /// Create a new WAL corruption error
    pub fn corruption(message: impl Into<String>) -> Self {
        Self {
            code: WalErrorCode::AeroWalCorruption,
            message: message.into(),
            details: None,
            source: None,
        }
    }

    /// Create a WAL corruption error with sequence number context
    pub fn corruption_at_sequence(sequence: u64, reason: impl Into<String>) -> Self {
        Self {
            code: WalErrorCode::AeroWalCorruption,
            message: reason.into(),
            details: Some(format!("sequence_number: {}", sequence)),
            source: None,
        }
    }

    /// Create a WAL corruption error with byte offset context
    pub fn corruption_at_offset(offset: u64, reason: impl Into<String>) -> Self {
        Self {
            code: WalErrorCode::AeroWalCorruption,
            message: reason.into(),
            details: Some(format!("byte_offset: {}", offset)),
            source: None,
        }
    }

    /// Returns the error code
    pub fn code(&self) -> WalErrorCode {
        self.code
    }

    /// Returns the severity level
    pub fn severity(&self) -> Severity {
        self.code.severity()
    }

    /// Returns the invariant violated, if applicable
    pub fn invariant(&self) -> Option<&'static str> {
        self.code.invariant()
    }

    /// Returns the error message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns additional error details
    pub fn details(&self) -> Option<&str> {
        self.details.as_deref()
    }

    /// Returns whether this error is fatal (requires process termination)
    pub fn is_fatal(&self) -> bool {
        self.severity() == Severity::Fatal
    }
}

impl fmt::Display for WalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {}: {}",
            self.code.severity(),
            self.code.code(),
            self.message
        )?;
        if let Some(ref details) = self.details {
            write!(f, " ({})", details)?;
        }
        if let Some(ref invariant) = self.code.invariant() {
            write!(f, " [violates {}]", invariant)?;
        }
        Ok(())
    }
}

impl std::error::Error for WalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

/// Result type for WAL operations
pub type WalResult<T> = Result<T, WalError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes_match_spec() {
        assert_eq!(
            WalErrorCode::AeroWalAppendFailed.code(),
            "AERO_WAL_APPEND_FAILED"
        );
        assert_eq!(
            WalErrorCode::AeroWalFsyncFailed.code(),
            "AERO_WAL_FSYNC_FAILED"
        );
        assert_eq!(
            WalErrorCode::AeroWalCorruption.code(),
            "AERO_WAL_CORRUPTION"
        );
    }

    #[test]
    fn test_severity_levels_match_spec() {
        assert_eq!(
            WalErrorCode::AeroWalAppendFailed.severity(),
            Severity::Error
        );
        assert_eq!(WalErrorCode::AeroWalFsyncFailed.severity(), Severity::Fatal);
        assert_eq!(WalErrorCode::AeroWalCorruption.severity(), Severity::Fatal);
    }

    #[test]
    fn test_fsync_failed_is_fatal() {
        let err = WalError::fsync_failed(
            "fsync failed",
            io::Error::new(io::ErrorKind::Other, "disk error"),
        );
        assert!(err.is_fatal());
    }

    #[test]
    fn test_corruption_is_fatal() {
        let err = WalError::corruption("checksum mismatch");
        assert!(err.is_fatal());
    }

    #[test]
    fn test_append_failed_is_not_fatal() {
        let err = WalError::append_failed(
            "write failed",
            io::Error::new(io::ErrorKind::Other, "disk full"),
        );
        assert!(!err.is_fatal());
    }

    #[test]
    fn test_error_display_contains_required_fields() {
        let err = WalError::corruption_at_sequence(42, "checksum mismatch");
        let display = format!("{}", err);
        assert!(display.contains("AERO_WAL_CORRUPTION"));
        assert!(display.contains("FATAL"));
        assert!(display.contains("checksum mismatch"));
        assert!(display.contains("sequence_number: 42"));
        assert!(display.contains("K2"));
    }
}
