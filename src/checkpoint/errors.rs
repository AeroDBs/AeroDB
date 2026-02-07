//! Checkpoint-specific error types
//!
//! Per ERRORS.md, checkpoint errors follow the standard error model:
//! - Structured error codes in AERO_CATEGORY_NAME format
//! - Clear severity levels
//! - No silent failures
//!
//! All checkpoint errors are ERROR severity (not FATAL) per spec.
//! Checkpoint failure does NOT corrupt serving state.

use std::fmt;
use std::io;

/// Error severity levels per ERRORS.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Operation failed but system is healthy
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "ERROR"),
        }
    }
}

/// Checkpoint error codes per ERRORS.md format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointErrorCode {
    /// General checkpoint creation failure
    AeroCheckpointFailed,
    /// Marker file write failure
    AeroCheckpointMarkerFailed,
    /// WAL truncation failure
    AeroCheckpointWalTruncateFailed,
}

impl CheckpointErrorCode {
    /// Returns the string representation per ERRORS.md format
    pub fn as_str(&self) -> &'static str {
        match self {
            CheckpointErrorCode::AeroCheckpointFailed => "AERO_CHECKPOINT_FAILED",
            CheckpointErrorCode::AeroCheckpointMarkerFailed => "AERO_CHECKPOINT_MARKER_FAILED",
            CheckpointErrorCode::AeroCheckpointWalTruncateFailed => {
                "AERO_CHECKPOINT_WAL_TRUNCATE_FAILED"
            }
        }
    }

    /// Returns the severity level for this error code
    pub fn severity(&self) -> Severity {
        // All checkpoint errors are ERROR severity per spec
        Severity::Error
    }
}

impl fmt::Display for CheckpointErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Checkpoint error with full context
#[derive(Debug)]
pub struct CheckpointError {
    /// Error code following AERO_CATEGORY_NAME format
    code: CheckpointErrorCode,
    /// Human-readable error message
    message: String,
    /// Optional underlying IO error
    source: Option<io::Error>,
}

impl CheckpointError {
    /// Creates a new checkpoint error
    fn new(
        code: CheckpointErrorCode,
        message: impl Into<String>,
        source: Option<io::Error>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            source,
        }
    }

    /// Creates a general checkpoint failure error
    pub fn failed(message: impl Into<String>) -> Self {
        Self::new(CheckpointErrorCode::AeroCheckpointFailed, message, None)
    }

    /// Creates a checkpoint failure with source error
    pub fn failed_with_source(message: impl Into<String>, source: io::Error) -> Self {
        Self::new(
            CheckpointErrorCode::AeroCheckpointFailed,
            message,
            Some(source),
        )
    }

    /// Creates a marker write failure error
    pub fn marker_failed(message: impl Into<String>, source: io::Error) -> Self {
        Self::new(
            CheckpointErrorCode::AeroCheckpointMarkerFailed,
            message,
            Some(source),
        )
    }

    /// Creates a WAL truncation failure error
    pub fn wal_truncate_failed(message: impl Into<String>) -> Self {
        Self::new(
            CheckpointErrorCode::AeroCheckpointWalTruncateFailed,
            message,
            None,
        )
    }

    /// Creates a WAL truncation failure with source error
    pub fn wal_truncate_failed_with_source(message: impl Into<String>, source: io::Error) -> Self {
        Self::new(
            CheckpointErrorCode::AeroCheckpointWalTruncateFailed,
            message,
            Some(source),
        )
    }

    /// Returns the error code
    pub fn code(&self) -> CheckpointErrorCode {
        self.code
    }

    /// Returns the error message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the severity of this error
    pub fn severity(&self) -> Severity {
        self.code.severity()
    }

    /// Returns whether this error is fatal (requires process termination)
    /// Checkpoint errors are never fatal - they are ERROR severity
    pub fn is_fatal(&self) -> bool {
        false
    }
}

impl fmt::Display for CheckpointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {}: {}",
            self.code.severity(),
            self.code,
            self.message
        )?;
        if let Some(ref source) = self.source {
            write!(f, " (caused by: {})", source)?;
        }
        Ok(())
    }
}

impl std::error::Error for CheckpointError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

/// Result type for checkpoint operations
pub type CheckpointResult<T> = Result<T, CheckpointError>;

/// Convert from snapshot errors to checkpoint errors
impl From<crate::snapshot::SnapshotError> for CheckpointError {
    fn from(err: crate::snapshot::SnapshotError) -> Self {
        CheckpointError::failed(format!("Snapshot creation failed: {}", err))
    }
}

/// Convert from WAL errors to checkpoint errors
impl From<crate::wal::WalError> for CheckpointError {
    fn from(err: crate::wal::WalError) -> Self {
        CheckpointError::wal_truncate_failed(format!("WAL operation failed: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes_match_spec() {
        assert_eq!(
            CheckpointErrorCode::AeroCheckpointFailed.as_str(),
            "AERO_CHECKPOINT_FAILED"
        );
        assert_eq!(
            CheckpointErrorCode::AeroCheckpointMarkerFailed.as_str(),
            "AERO_CHECKPOINT_MARKER_FAILED"
        );
        assert_eq!(
            CheckpointErrorCode::AeroCheckpointWalTruncateFailed.as_str(),
            "AERO_CHECKPOINT_WAL_TRUNCATE_FAILED"
        );
    }

    #[test]
    fn test_all_errors_are_error_severity() {
        let codes = [
            CheckpointErrorCode::AeroCheckpointFailed,
            CheckpointErrorCode::AeroCheckpointMarkerFailed,
            CheckpointErrorCode::AeroCheckpointWalTruncateFailed,
        ];

        for code in codes {
            assert_eq!(code.severity(), Severity::Error);
        }
    }

    #[test]
    fn test_checkpoint_errors_not_fatal() {
        let err = CheckpointError::failed("test error");
        assert!(!err.is_fatal());
        assert_eq!(err.severity(), Severity::Error);
    }

    #[test]
    fn test_error_display_contains_required_fields() {
        let err = CheckpointError::failed("test message");
        let display = format!("{}", err);

        assert!(display.contains("ERROR"));
        assert!(display.contains("AERO_CHECKPOINT_FAILED"));
        assert!(display.contains("test message"));
    }

    #[test]
    fn test_marker_error() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        let err = CheckpointError::marker_failed("could not write marker", io_err);

        assert_eq!(err.code(), CheckpointErrorCode::AeroCheckpointMarkerFailed);
        assert!(!err.is_fatal());
    }

    #[test]
    fn test_wal_truncate_error() {
        let err = CheckpointError::wal_truncate_failed("truncation failed");

        assert_eq!(
            err.code(),
            CheckpointErrorCode::AeroCheckpointWalTruncateFailed
        );
        assert!(!err.is_fatal());
    }

    #[test]
    fn test_error_with_source() {
        let io_err = io::Error::new(io::ErrorKind::Other, "disk full");
        let err = CheckpointError::failed_with_source("checkpoint failed", io_err);

        let display = format!("{}", err);
        assert!(display.contains("caused by"));
        assert!(display.contains("disk full"));
    }
}
