//! Snapshot error types following ERRORS.md specification
//!
//! Error codes:
//! - AERO_SNAPSHOT_FAILED (ERROR severity)
//! - AERO_SNAPSHOT_IO (ERROR severity)
//! - AERO_SNAPSHOT_MANIFEST (ERROR severity)

use std::fmt;
use std::io;

/// Severity levels for snapshot errors as defined in ERRORS.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Operation fails, server continues
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "ERROR"),
        }
    }
}

/// Snapshot-specific error codes as defined in ERRORS.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotErrorCode {
    /// General snapshot creation failure
    AeroSnapshotFailed,
    /// I/O failure during snapshot
    AeroSnapshotIo,
    /// Manifest generation/write failure
    AeroSnapshotManifest,
}

impl SnapshotErrorCode {
    /// Returns the string code as defined in ERRORS.md
    pub fn code(&self) -> &'static str {
        match self {
            SnapshotErrorCode::AeroSnapshotFailed => "AERO_SNAPSHOT_FAILED",
            SnapshotErrorCode::AeroSnapshotIo => "AERO_SNAPSHOT_IO",
            SnapshotErrorCode::AeroSnapshotManifest => "AERO_SNAPSHOT_MANIFEST",
        }
    }

    /// Returns the severity level for this error
    pub fn severity(&self) -> Severity {
        // All snapshot errors are ERROR severity (not FATAL)
        // Snapshot failure does not require process termination
        Severity::Error
    }

    /// Returns the invariant violated by this error, if applicable
    pub fn invariant(&self) -> Option<&'static str> {
        // Snapshot errors don't directly violate core invariants
        None
    }
}

impl fmt::Display for SnapshotErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// Snapshot error type with full context as required by ERRORS.md
#[derive(Debug)]
pub struct SnapshotError {
    /// Error code
    code: SnapshotErrorCode,
    /// Human-readable message
    message: String,
    /// Optional details about the error context
    details: Option<String>,
    /// Underlying IO error if applicable
    source: Option<io::Error>,
}

impl SnapshotError {
    /// Create a new snapshot failed error
    pub fn snapshot_failed(message: impl Into<String>) -> Self {
        Self {
            code: SnapshotErrorCode::AeroSnapshotFailed,
            message: message.into(),
            details: None,
            source: None,
        }
    }

    /// Create a snapshot failed error with IO source
    pub fn snapshot_failed_io(message: impl Into<String>, source: io::Error) -> Self {
        Self {
            code: SnapshotErrorCode::AeroSnapshotFailed,
            message: message.into(),
            details: None,
            source: Some(source),
        }
    }

    /// Create a new snapshot I/O error
    pub fn io_error(message: impl Into<String>, source: io::Error) -> Self {
        Self {
            code: SnapshotErrorCode::AeroSnapshotIo,
            message: message.into(),
            details: None,
            source: Some(source),
        }
    }

    /// Create a snapshot I/O error with path context
    pub fn io_error_at_path(path: &std::path::Path, source: io::Error) -> Self {
        Self {
            code: SnapshotErrorCode::AeroSnapshotIo,
            message: format!("I/O error at path: {}", path.display()),
            details: None,
            source: Some(source),
        }
    }

    /// Create a new manifest error
    pub fn manifest_error(message: impl Into<String>) -> Self {
        Self {
            code: SnapshotErrorCode::AeroSnapshotManifest,
            message: message.into(),
            details: None,
            source: None,
        }
    }

    /// Create a manifest error with IO source
    pub fn manifest_io_error(message: impl Into<String>, source: io::Error) -> Self {
        Self {
            code: SnapshotErrorCode::AeroSnapshotManifest,
            message: message.into(),
            details: None,
            source: Some(source),
        }
    }

    /// Add details to an error
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    /// Returns the error code
    pub fn code(&self) -> SnapshotErrorCode {
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
    /// Snapshot errors are never fatal - they are ERROR severity
    pub fn is_fatal(&self) -> bool {
        false
    }
}

impl fmt::Display for SnapshotError {
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
        Ok(())
    }
}

impl std::error::Error for SnapshotError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

/// Result type for snapshot operations
pub type SnapshotResult<T> = Result<T, SnapshotError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes_match_spec() {
        assert_eq!(
            SnapshotErrorCode::AeroSnapshotFailed.code(),
            "AERO_SNAPSHOT_FAILED"
        );
        assert_eq!(SnapshotErrorCode::AeroSnapshotIo.code(), "AERO_SNAPSHOT_IO");
        assert_eq!(
            SnapshotErrorCode::AeroSnapshotManifest.code(),
            "AERO_SNAPSHOT_MANIFEST"
        );
    }

    #[test]
    fn test_all_errors_are_error_severity() {
        assert_eq!(
            SnapshotErrorCode::AeroSnapshotFailed.severity(),
            Severity::Error
        );
        assert_eq!(
            SnapshotErrorCode::AeroSnapshotIo.severity(),
            Severity::Error
        );
        assert_eq!(
            SnapshotErrorCode::AeroSnapshotManifest.severity(),
            Severity::Error
        );
    }

    #[test]
    fn test_snapshot_errors_not_fatal() {
        let err = SnapshotError::snapshot_failed("test failure");
        assert!(!err.is_fatal());

        let err =
            SnapshotError::io_error("io failed", io::Error::new(io::ErrorKind::Other, "test"));
        assert!(!err.is_fatal());

        let err = SnapshotError::manifest_error("manifest failed");
        assert!(!err.is_fatal());
    }

    #[test]
    fn test_error_display_contains_required_fields() {
        let err = SnapshotError::snapshot_failed("snapshot creation aborted")
            .with_details("storage copy failed");
        let display = format!("{}", err);
        assert!(display.contains("AERO_SNAPSHOT_FAILED"));
        assert!(display.contains("ERROR"));
        assert!(display.contains("snapshot creation aborted"));
        assert!(display.contains("storage copy failed"));
    }

    #[test]
    fn test_io_error_with_path() {
        let path = std::path::Path::new("/test/path/storage.dat");
        let err = SnapshotError::io_error_at_path(
            path,
            io::Error::new(io::ErrorKind::NotFound, "not found"),
        );
        assert!(err.message().contains("/test/path/storage.dat"));
        assert_eq!(err.code(), SnapshotErrorCode::AeroSnapshotIo);
    }
}
