//! Storage error types following ERRORS.md specification
//!
//! Error codes:
//! - AERO_STORAGE_IO_ERROR (ERROR severity)
//! - AERO_STORAGE_WRITE_FAILED (ERROR severity)
//! - AERO_STORAGE_READ_FAILED (ERROR severity)
//! - AERO_DATA_CORRUPTION (FATAL severity) - from CORRUPTION category

use std::fmt;
use std::io;

/// Severity levels for storage errors as defined in ERRORS.md
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

/// Storage-specific error codes as defined in ERRORS.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageErrorCode {
    /// Disk I/O failure
    AeroStorageIoError,
    /// Document write failed
    AeroStorageWriteFailed,
    /// Document read failed
    AeroStorageReadFailed,
    /// Data checksum failure (from CORRUPTION category)
    AeroDataCorruption,
}

impl StorageErrorCode {
    /// Returns the string code as defined in ERRORS.md
    pub fn code(&self) -> &'static str {
        match self {
            StorageErrorCode::AeroStorageIoError => "AERO_STORAGE_IO_ERROR",
            StorageErrorCode::AeroStorageWriteFailed => "AERO_STORAGE_WRITE_FAILED",
            StorageErrorCode::AeroStorageReadFailed => "AERO_STORAGE_READ_FAILED",
            StorageErrorCode::AeroDataCorruption => "AERO_DATA_CORRUPTION",
        }
    }

    /// Returns the severity level for this error
    pub fn severity(&self) -> Severity {
        match self {
            StorageErrorCode::AeroStorageIoError => Severity::Error,
            StorageErrorCode::AeroStorageWriteFailed => Severity::Error,
            StorageErrorCode::AeroStorageReadFailed => Severity::Error,
            StorageErrorCode::AeroDataCorruption => Severity::Fatal,
        }
    }

    /// Returns the invariant violated by this error, if applicable
    pub fn invariant(&self) -> Option<&'static str> {
        match self {
            StorageErrorCode::AeroStorageIoError => None,
            StorageErrorCode::AeroStorageWriteFailed => Some("D1"),
            StorageErrorCode::AeroStorageReadFailed => None,
            StorageErrorCode::AeroDataCorruption => Some("D2"),
        }
    }
}

impl fmt::Display for StorageErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// Storage error type with full context as required by ERRORS.md
#[derive(Debug)]
pub struct StorageError {
    /// Error code
    code: StorageErrorCode,
    /// Human-readable message
    message: String,
    /// Optional details about the error context
    details: Option<String>,
    /// Underlying IO error if applicable
    source: Option<io::Error>,
}

impl StorageError {
    /// Create a new storage I/O error
    pub fn io_error(message: impl Into<String>, source: io::Error) -> Self {
        Self {
            code: StorageErrorCode::AeroStorageIoError,
            message: message.into(),
            details: None,
            source: Some(source),
        }
    }

    /// Create a new storage write failed error
    pub fn write_failed(message: impl Into<String>, source: io::Error) -> Self {
        Self {
            code: StorageErrorCode::AeroStorageWriteFailed,
            message: message.into(),
            details: None,
            source: Some(source),
        }
    }

    /// Create a storage write failed error without IO source
    pub fn write_failed_no_source(message: impl Into<String>) -> Self {
        Self {
            code: StorageErrorCode::AeroStorageWriteFailed,
            message: message.into(),
            details: None,
            source: None,
        }
    }

    /// Create a new storage read failed error
    pub fn read_failed(message: impl Into<String>, source: io::Error) -> Self {
        Self {
            code: StorageErrorCode::AeroStorageReadFailed,
            message: message.into(),
            details: None,
            source: Some(source),
        }
    }

    /// Create a new data corruption error (FATAL)
    pub fn data_corruption(message: impl Into<String>) -> Self {
        Self {
            code: StorageErrorCode::AeroDataCorruption,
            message: message.into(),
            details: None,
            source: None,
        }
    }

    /// Create a data corruption error with byte offset context
    pub fn corruption_at_offset(offset: u64, reason: impl Into<String>) -> Self {
        Self {
            code: StorageErrorCode::AeroDataCorruption,
            message: reason.into(),
            details: Some(format!("byte_offset: {}", offset)),
            source: None,
        }
    }

    /// Create a data corruption error with document ID context
    pub fn corruption_for_document(document_id: &str, reason: impl Into<String>) -> Self {
        Self {
            code: StorageErrorCode::AeroDataCorruption,
            message: reason.into(),
            details: Some(format!("document_id: {}", document_id)),
            source: None,
        }
    }

    /// Returns the error code
    pub fn code(&self) -> StorageErrorCode {
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

impl fmt::Display for StorageError {
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

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

/// Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes_match_spec() {
        assert_eq!(
            StorageErrorCode::AeroStorageIoError.code(),
            "AERO_STORAGE_IO_ERROR"
        );
        assert_eq!(
            StorageErrorCode::AeroStorageWriteFailed.code(),
            "AERO_STORAGE_WRITE_FAILED"
        );
        assert_eq!(
            StorageErrorCode::AeroStorageReadFailed.code(),
            "AERO_STORAGE_READ_FAILED"
        );
        assert_eq!(
            StorageErrorCode::AeroDataCorruption.code(),
            "AERO_DATA_CORRUPTION"
        );
    }

    #[test]
    fn test_severity_levels_match_spec() {
        assert_eq!(
            StorageErrorCode::AeroStorageIoError.severity(),
            Severity::Error
        );
        assert_eq!(
            StorageErrorCode::AeroStorageWriteFailed.severity(),
            Severity::Error
        );
        assert_eq!(
            StorageErrorCode::AeroStorageReadFailed.severity(),
            Severity::Error
        );
        assert_eq!(
            StorageErrorCode::AeroDataCorruption.severity(),
            Severity::Fatal
        );
    }

    #[test]
    fn test_data_corruption_is_fatal() {
        let err = StorageError::data_corruption("checksum mismatch");
        assert!(err.is_fatal());
        assert_eq!(err.code().code(), "AERO_DATA_CORRUPTION");
    }

    #[test]
    fn test_write_failed_not_fatal() {
        let err = StorageError::write_failed(
            "disk full",
            io::Error::new(io::ErrorKind::Other, "disk full"),
        );
        assert!(!err.is_fatal());
    }

    #[test]
    fn test_error_display_contains_required_fields() {
        let err = StorageError::corruption_at_offset(1024, "checksum mismatch");
        let display = format!("{}", err);
        assert!(display.contains("AERO_DATA_CORRUPTION"));
        assert!(display.contains("FATAL"));
        assert!(display.contains("checksum mismatch"));
        assert!(display.contains("byte_offset: 1024"));
        assert!(display.contains("D2"));
    }
}
