//! Restore-specific error types
//!
//! Per ERRORS.md, restore errors follow the standard error model:
//! - Structured error codes in AERO_CATEGORY_NAME format
//! - Clear severity levels
//! - No silent failures
//!
//! All restore errors are FATAL severity per spec.
//! Restore failure requires operator intervention.

use std::fmt;
use std::io;

/// Error severity levels per ERRORS.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Unrecoverable error requiring operator intervention
    Fatal,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Fatal => write!(f, "FATAL"),
        }
    }
}

/// Restore error codes per ERRORS.md format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreErrorCode {
    /// General restore failure
    AeroRestoreFailed,
    /// I/O failure during restore
    AeroRestoreIo,
    /// Data corruption detected
    AeroRestoreCorruption,
    /// Invalid backup format
    AeroRestoreInvalidBackup,
}

impl RestoreErrorCode {
    /// Returns the string representation per ERRORS.md format
    pub fn as_str(&self) -> &'static str {
        match self {
            RestoreErrorCode::AeroRestoreFailed => "AERO_RESTORE_FAILED",
            RestoreErrorCode::AeroRestoreIo => "AERO_RESTORE_IO",
            RestoreErrorCode::AeroRestoreCorruption => "AERO_RESTORE_CORRUPTION",
            RestoreErrorCode::AeroRestoreInvalidBackup => "AERO_RESTORE_INVALID_BACKUP",
        }
    }

    /// Returns the severity level for this error code
    pub fn severity(&self) -> Severity {
        // All restore errors are FATAL severity per spec
        Severity::Fatal
    }
}

impl fmt::Display for RestoreErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Restore error with full context
#[derive(Debug)]
pub struct RestoreError {
    /// Error code following AERO_CATEGORY_NAME format
    code: RestoreErrorCode,
    /// Human-readable error message
    message: String,
    /// Optional underlying IO error
    source: Option<io::Error>,
}

impl RestoreError {
    /// Creates a new restore error
    fn new(code: RestoreErrorCode, message: impl Into<String>, source: Option<io::Error>) -> Self {
        Self {
            code,
            message: message.into(),
            source,
        }
    }

    /// Creates a general restore failure error
    pub fn failed(message: impl Into<String>) -> Self {
        Self::new(RestoreErrorCode::AeroRestoreFailed, message, None)
    }

    /// Creates a restore failure with source error
    pub fn failed_with_source(message: impl Into<String>, source: io::Error) -> Self {
        Self::new(RestoreErrorCode::AeroRestoreFailed, message, Some(source))
    }

    /// Creates an I/O error during restore
    pub fn io_error(message: impl Into<String>, source: io::Error) -> Self {
        Self::new(RestoreErrorCode::AeroRestoreIo, message, Some(source))
    }

    /// Creates an I/O error at a specific path
    pub fn io_error_at_path(path: &std::path::Path, source: io::Error) -> Self {
        Self::io_error(format!("I/O error at {}", path.display()), source)
    }

    /// Creates a corruption error
    pub fn corruption(message: impl Into<String>) -> Self {
        Self::new(RestoreErrorCode::AeroRestoreCorruption, message, None)
    }

    /// Creates an invalid backup error
    pub fn invalid_backup(message: impl Into<String>) -> Self {
        Self::new(RestoreErrorCode::AeroRestoreInvalidBackup, message, None)
    }

    /// Creates an invalid backup error with source
    pub fn invalid_backup_with_source(message: impl Into<String>, source: io::Error) -> Self {
        Self::new(
            RestoreErrorCode::AeroRestoreInvalidBackup,
            message,
            Some(source),
        )
    }

    /// Returns the error code
    pub fn code(&self) -> RestoreErrorCode {
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
    /// Restore errors are always fatal per spec
    pub fn is_fatal(&self) -> bool {
        true
    }
}

impl fmt::Display for RestoreError {
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

impl std::error::Error for RestoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

/// Result type for restore operations
pub type RestoreResult<T> = Result<T, RestoreError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes_match_spec() {
        assert_eq!(
            RestoreErrorCode::AeroRestoreFailed.as_str(),
            "AERO_RESTORE_FAILED"
        );
        assert_eq!(RestoreErrorCode::AeroRestoreIo.as_str(), "AERO_RESTORE_IO");
        assert_eq!(
            RestoreErrorCode::AeroRestoreCorruption.as_str(),
            "AERO_RESTORE_CORRUPTION"
        );
        assert_eq!(
            RestoreErrorCode::AeroRestoreInvalidBackup.as_str(),
            "AERO_RESTORE_INVALID_BACKUP"
        );
    }

    #[test]
    fn test_all_errors_are_fatal_severity() {
        let codes = [
            RestoreErrorCode::AeroRestoreFailed,
            RestoreErrorCode::AeroRestoreIo,
            RestoreErrorCode::AeroRestoreCorruption,
            RestoreErrorCode::AeroRestoreInvalidBackup,
        ];

        for code in codes {
            assert_eq!(code.severity(), Severity::Fatal);
        }
    }

    #[test]
    fn test_restore_errors_are_fatal() {
        let err = RestoreError::failed("test error");
        assert!(err.is_fatal());
        assert_eq!(err.severity(), Severity::Fatal);
    }

    #[test]
    fn test_error_display_contains_required_fields() {
        let err = RestoreError::failed("test message");
        let display = format!("{}", err);

        assert!(display.contains("FATAL"));
        assert!(display.contains("AERO_RESTORE_FAILED"));
        assert!(display.contains("test message"));
    }

    #[test]
    fn test_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let err = RestoreError::io_error("could not read file", io_err);

        assert_eq!(err.code(), RestoreErrorCode::AeroRestoreIo);
        assert!(err.is_fatal());
    }

    #[test]
    fn test_corruption_error() {
        let err = RestoreError::corruption("checksum mismatch");

        assert_eq!(err.code(), RestoreErrorCode::AeroRestoreCorruption);
        assert!(err.is_fatal());
    }

    #[test]
    fn test_invalid_backup_error() {
        let err = RestoreError::invalid_backup("missing snapshot directory");

        assert_eq!(err.code(), RestoreErrorCode::AeroRestoreInvalidBackup);
        assert!(err.is_fatal());
    }

    #[test]
    fn test_error_with_source() {
        let io_err = io::Error::new(io::ErrorKind::Other, "disk full");
        let err = RestoreError::failed_with_source("restore failed", io_err);

        let display = format!("{}", err);
        assert!(display.contains("caused by"));
        assert!(display.contains("disk full"));
    }
}
