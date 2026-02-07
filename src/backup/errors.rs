//! Backup error types for AeroDB.
//!
//! Per ERRORS.md, backup errors follow the AERO_CATEGORY_NAME format.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// Backup error code following ERRORS.md format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupErrorCode {
    /// Snapshot creation failed
    AeroBackupSnapshotFailed,
    /// Archive creation failed
    AeroBackupArchiveFailed,
    /// Backup not found
    AeroBackupNotFound,
    /// Retention policy enforcement failed
    AeroBackupRetentionFailed,
    /// I/O error during backup
    AeroBackupIoError,
    /// Invalid backup configuration
    AeroBackupInvalidConfig,
    /// Backup directory not accessible
    AeroBackupDirNotAccessible,
}

impl BackupErrorCode {
    /// Returns the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            BackupErrorCode::AeroBackupSnapshotFailed => "AERO_BACKUP_SNAPSHOT_FAILED",
            BackupErrorCode::AeroBackupArchiveFailed => "AERO_BACKUP_ARCHIVE_FAILED",
            BackupErrorCode::AeroBackupNotFound => "AERO_BACKUP_NOT_FOUND",
            BackupErrorCode::AeroBackupRetentionFailed => "AERO_BACKUP_RETENTION_FAILED",
            BackupErrorCode::AeroBackupIoError => "AERO_BACKUP_IO_ERROR",
            BackupErrorCode::AeroBackupInvalidConfig => "AERO_BACKUP_INVALID_CONFIG",
            BackupErrorCode::AeroBackupDirNotAccessible => "AERO_BACKUP_DIR_NOT_ACCESSIBLE",
        }
    }

    /// Returns the severity level
    pub fn severity(&self) -> Severity {
        match self {
            BackupErrorCode::AeroBackupNotFound => Severity::Warning,
            _ => Severity::Error,
        }
    }
}

impl fmt::Display for BackupErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Severity level for backup errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
    Fatal,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Warning => write!(f, "WARNING"),
            Severity::Error => write!(f, "ERROR"),
            Severity::Fatal => write!(f, "FATAL"),
        }
    }
}

/// Backup error with context
#[derive(Debug)]
pub struct BackupError {
    code: BackupErrorCode,
    message: String,
    path: Option<PathBuf>,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl BackupError {
    /// Create a new backup error
    pub fn new(code: BackupErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            path: None,
            source: None,
        }
    }

    /// Create error with path context
    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Create error with source
    pub fn with_source(mut self, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(source));
        self
    }

    /// Snapshot creation failed
    pub fn snapshot_failed(message: impl Into<String>) -> Self {
        Self::new(BackupErrorCode::AeroBackupSnapshotFailed, message)
    }

    /// Archive creation failed
    pub fn archive_failed(message: impl Into<String>) -> Self {
        Self::new(BackupErrorCode::AeroBackupArchiveFailed, message)
    }

    /// Backup not found
    pub fn not_found(backup_id: impl Into<String>) -> Self {
        Self::new(
            BackupErrorCode::AeroBackupNotFound,
            format!("Backup not found: {}", backup_id.into()),
        )
    }

    /// Retention policy failed
    pub fn retention_failed(message: impl Into<String>) -> Self {
        Self::new(BackupErrorCode::AeroBackupRetentionFailed, message)
    }

    /// I/O error
    pub fn io_error(err: io::Error, context: impl Into<String>) -> Self {
        Self::new(BackupErrorCode::AeroBackupIoError, context).with_source(err)
    }

    /// Invalid configuration
    pub fn invalid_config(message: impl Into<String>) -> Self {
        Self::new(BackupErrorCode::AeroBackupInvalidConfig, message)
    }

    /// Directory not accessible
    pub fn dir_not_accessible(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        Self::new(
            BackupErrorCode::AeroBackupDirNotAccessible,
            format!("Backup directory not accessible: {}", path.display()),
        )
        .with_path(path)
    }

    /// Get the error code
    pub fn code(&self) -> BackupErrorCode {
        self.code
    }

    /// Get the message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Get the path if present
    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    /// Check if error is fatal
    pub fn is_fatal(&self) -> bool {
        matches!(self.code.severity(), Severity::Fatal)
    }
}

impl fmt::Display for BackupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.code.severity(), self.code, self.message)?;
        if let Some(ref path) = self.path {
            write!(f, " (path: {})", path.display())?;
        }
        if let Some(ref source) = self.source {
            write!(f, " (caused by: {})", source)?;
        }
        Ok(())
    }
}

impl std::error::Error for BackupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
    }
}

impl From<io::Error> for BackupError {
    fn from(err: io::Error) -> Self {
        Self::io_error(err, "I/O operation failed")
    }
}

/// Result type for backup operations
pub type BackupResult<T> = Result<T, BackupError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_display() {
        assert_eq!(
            BackupErrorCode::AeroBackupSnapshotFailed.as_str(),
            "AERO_BACKUP_SNAPSHOT_FAILED"
        );
        assert_eq!(
            BackupErrorCode::AeroBackupNotFound.as_str(),
            "AERO_BACKUP_NOT_FOUND"
        );
    }

    #[test]
    fn test_error_with_path() {
        let err = BackupError::dir_not_accessible("/tmp/backups");
        assert!(err.path().is_some());
        assert_eq!(err.code(), BackupErrorCode::AeroBackupDirNotAccessible);
    }

    #[test]
    fn test_error_display() {
        let err = BackupError::not_found("backup-123");
        let display = format!("{}", err);
        assert!(display.contains("AERO_BACKUP_NOT_FOUND"));
        assert!(display.contains("backup-123"));
    }

    #[test]
    fn test_severity_levels() {
        assert_eq!(
            BackupErrorCode::AeroBackupNotFound.severity(),
            Severity::Warning
        );
        assert_eq!(
            BackupErrorCode::AeroBackupSnapshotFailed.severity(),
            Severity::Error
        );
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let backup_err: BackupError = io_err.into();
        assert_eq!(backup_err.code(), BackupErrorCode::AeroBackupIoError);
    }
}
