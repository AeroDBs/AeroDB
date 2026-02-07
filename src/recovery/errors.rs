//! Recovery error types following ERRORS.md specification
//!
//! Error codes:
//! - AERO_WAL_CORRUPTION (FATAL)
//! - AERO_RECOVERY_SCHEMA_MISSING (FATAL)
//! - AERO_RECOVERY_VERIFICATION_FAILED (FATAL)
//! - AERO_RECOVERY_FAILED (FATAL)

use std::fmt;

/// Severity levels for recovery errors
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

/// Recovery-specific error codes as defined in ERRORS.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryErrorCode {
    /// WAL data is corrupted
    AeroWalCorruption,
    /// Schema referenced in WAL not found
    AeroRecoverySchemaMissing,
    /// Consistency verification failed
    AeroRecoveryVerificationFailed,
    /// General recovery failure
    AeroRecoveryFailed,
    /// Storage data is corrupted
    AeroStorageCorruption,
}

impl RecoveryErrorCode {
    /// Returns the string code as defined in ERRORS.md
    pub fn code(&self) -> &'static str {
        match self {
            RecoveryErrorCode::AeroWalCorruption => "AERO_WAL_CORRUPTION",
            RecoveryErrorCode::AeroRecoverySchemaMissing => "AERO_RECOVERY_SCHEMA_MISSING",
            RecoveryErrorCode::AeroRecoveryVerificationFailed => {
                "AERO_RECOVERY_VERIFICATION_FAILED"
            }
            RecoveryErrorCode::AeroRecoveryFailed => "AERO_RECOVERY_FAILED",
            RecoveryErrorCode::AeroStorageCorruption => "AERO_STORAGE_CORRUPTION",
        }
    }

    /// Returns the severity level for this error
    pub fn severity(&self) -> Severity {
        Severity::Fatal // All recovery errors are FATAL
    }

    /// Returns the invariant violated by this error
    pub fn invariant(&self) -> &'static str {
        match self {
            RecoveryErrorCode::AeroWalCorruption => "K2",
            RecoveryErrorCode::AeroRecoverySchemaMissing => "S3",
            RecoveryErrorCode::AeroRecoveryVerificationFailed => "D2",
            RecoveryErrorCode::AeroRecoveryFailed => "R1",
            RecoveryErrorCode::AeroStorageCorruption => "K2",
        }
    }
}

impl fmt::Display for RecoveryErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// Recovery error type with full context
#[derive(Debug)]
pub struct RecoveryError {
    /// Error code
    code: RecoveryErrorCode,
    /// Human-readable message
    message: String,
    /// Byte offset if applicable
    offset: Option<u64>,
    /// Sequence number if applicable
    sequence: Option<u64>,
}

impl RecoveryError {
    /// Create a WAL corruption error
    pub fn wal_corruption(offset: u64, reason: impl Into<String>) -> Self {
        Self {
            code: RecoveryErrorCode::AeroWalCorruption,
            message: format!("WAL corruption at offset {}: {}", offset, reason.into()),
            offset: Some(offset),
            sequence: None,
        }
    }

    /// Create a schema missing error
    pub fn schema_missing(schema_id: impl Into<String>, schema_version: impl Into<String>) -> Self {
        Self {
            code: RecoveryErrorCode::AeroRecoverySchemaMissing,
            message: format!(
                "Schema '{}' version '{}' not found during recovery",
                schema_id.into(),
                schema_version.into()
            ),
            offset: None,
            sequence: None,
        }
    }

    /// Create a verification failed error
    pub fn verification_failed(reason: impl Into<String>) -> Self {
        Self {
            code: RecoveryErrorCode::AeroRecoveryVerificationFailed,
            message: reason.into(),
            offset: None,
            sequence: None,
        }
    }

    /// Create a general recovery failed error
    pub fn recovery_failed(reason: impl Into<String>) -> Self {
        Self {
            code: RecoveryErrorCode::AeroRecoveryFailed,
            message: reason.into(),
            offset: None,
            sequence: None,
        }
    }

    /// Create a storage corruption error
    pub fn storage_corruption(offset: u64, reason: impl Into<String>) -> Self {
        Self {
            code: RecoveryErrorCode::AeroStorageCorruption,
            message: format!("Storage corruption at offset {}: {}", offset, reason.into()),
            offset: Some(offset),
            sequence: None,
        }
    }

    /// Returns the error code
    pub fn code(&self) -> RecoveryErrorCode {
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
        true // All recovery errors are FATAL
    }
}

impl fmt::Display for RecoveryError {
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

impl std::error::Error for RecoveryError {}

/// Result type for recovery operations
pub type RecoveryResult<T> = Result<T, RecoveryError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes_match_spec() {
        assert_eq!(
            RecoveryErrorCode::AeroWalCorruption.code(),
            "AERO_WAL_CORRUPTION"
        );
        assert_eq!(
            RecoveryErrorCode::AeroRecoverySchemaMissing.code(),
            "AERO_RECOVERY_SCHEMA_MISSING"
        );
        assert_eq!(
            RecoveryErrorCode::AeroRecoveryVerificationFailed.code(),
            "AERO_RECOVERY_VERIFICATION_FAILED"
        );
        assert_eq!(
            RecoveryErrorCode::AeroRecoveryFailed.code(),
            "AERO_RECOVERY_FAILED"
        );
    }

    #[test]
    fn test_all_errors_are_fatal() {
        let codes = [
            RecoveryErrorCode::AeroWalCorruption,
            RecoveryErrorCode::AeroRecoverySchemaMissing,
            RecoveryErrorCode::AeroRecoveryVerificationFailed,
            RecoveryErrorCode::AeroRecoveryFailed,
        ];

        for code in codes {
            assert_eq!(code.severity(), Severity::Fatal);
        }
    }

    #[test]
    fn test_error_display() {
        let err = RecoveryError::wal_corruption(1234, "checksum mismatch");
        let display = format!("{}", err);
        assert!(display.contains("AERO_WAL_CORRUPTION"));
        assert!(display.contains("FATAL"));
        assert!(display.contains("K2"));
    }
}
