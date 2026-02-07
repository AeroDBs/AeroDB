//! # Migration Errors
//!
//! MANIFESTO ALIGNMENT: All error paths are explicit.
//!
//! Per Design Manifesto: "fail loudly, execute predictably, leave no surprises."

use std::fmt;
use std::io;
use std::path::PathBuf;

/// Result type for migration operations
pub type MigrationResult<T> = Result<T, MigrationError>;

/// Migration error types
///
/// MANIFESTO ALIGNMENT: Every error is explicit with actionable context.
#[derive(Debug)]
pub enum MigrationError {
    /// Migration file could not be read
    FileRead {
        path: PathBuf,
        source: io::Error,
    },

    /// Migration file could not be written
    FileWrite {
        path: PathBuf,
        source: io::Error,
    },

    /// Migration file failed YAML parsing
    ParseError {
        path: PathBuf,
        message: String,
    },

    /// Checksum mismatch - migration file was modified
    ///
    /// MANIFESTO ALIGNMENT: Fail loudly on detected tampering.
    ChecksumMismatch {
        migration: String,
        expected: String,
        actual: String,
    },

    /// Migration is structurally invalid
    InvalidMigration {
        reason: String,
    },

    /// Migration version already exists
    DuplicateVersion {
        version: u64,
    },

    /// Migration version not found
    MigrationNotFound {
        version: u64,
    },

    /// Cannot apply migration (prerequisite not met)
    CannotApply {
        version: u64,
        reason: String,
    },

    /// Cannot rollback migration
    CannotRollback {
        version: u64,
        reason: String,
    },

    /// Migration execution failed
    ExecutionFailed {
        version: u64,
        operation: String,
        reason: String,
    },

    /// Migration directory does not exist
    DirectoryNotFound {
        path: PathBuf,
    },

    /// State tracking error
    StateError {
        message: String,
    },

    /// Lock contention - another migration is in progress
    ///
    /// MANIFESTO ALIGNMENT: Explicit concurrency control.
    MigrationLocked {
        holder: String,
        since: chrono::DateTime<chrono::Utc>,
    },

    /// Generic internal error
    Internal {
        message: String,
    },
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileRead { path, source } => {
                write!(f, "Failed to read migration file {:?}: {}", path, source)
            }
            Self::FileWrite { path, source } => {
                write!(f, "Failed to write migration file {:?}: {}", path, source)
            }
            Self::ParseError { path, message } => {
                write!(f, "Failed to parse migration {:?}: {}", path, message)
            }
            Self::ChecksumMismatch {
                migration,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Checksum mismatch for migration '{}': expected {}, got {}. \
                     Migration file may have been manually modified.",
                    migration, expected, actual
                )
            }
            Self::InvalidMigration { reason } => {
                write!(f, "Invalid migration: {}", reason)
            }
            Self::DuplicateVersion { version } => {
                write!(f, "Migration version {} already exists", version)
            }
            Self::MigrationNotFound { version } => {
                write!(f, "Migration version {} not found", version)
            }
            Self::CannotApply { version, reason } => {
                write!(f, "Cannot apply migration {}: {}", version, reason)
            }
            Self::CannotRollback { version, reason } => {
                write!(f, "Cannot rollback migration {}: {}", version, reason)
            }
            Self::ExecutionFailed {
                version,
                operation,
                reason,
            } => {
                write!(
                    f,
                    "Migration {} failed at operation '{}': {}",
                    version, operation, reason
                )
            }
            Self::DirectoryNotFound { path } => {
                write!(f, "Migration directory not found: {:?}", path)
            }
            Self::StateError { message } => {
                write!(f, "Migration state error: {}", message)
            }
            Self::MigrationLocked { holder, since } => {
                write!(
                    f,
                    "Migration locked by '{}' since {}. Another migration may be in progress.",
                    holder, since
                )
            }
            Self::Internal { message } => {
                write!(f, "Internal migration error: {}", message)
            }
        }
    }
}

impl std::error::Error for MigrationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::FileRead { source, .. } | Self::FileWrite { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<io::Error> for MigrationError {
    fn from(err: io::Error) -> Self {
        Self::Internal {
            message: err.to_string(),
        }
    }
}

impl From<serde_yaml::Error> for MigrationError {
    fn from(err: serde_yaml::Error) -> Self {
        Self::ParseError {
            path: PathBuf::new(),
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_mismatch_message() {
        let err = MigrationError::ChecksumMismatch {
            migration: "001_create_users".to_string(),
            expected: "crc32:ABC12345".to_string(),
            actual: "crc32:DEF67890".to_string(),
        };

        let msg = err.to_string();
        assert!(msg.contains("001_create_users"));
        assert!(msg.contains("ABC12345"));
        assert!(msg.contains("DEF67890"));
        assert!(msg.contains("manually modified"));
    }

    #[test]
    fn test_migration_locked_message() {
        let err = MigrationError::MigrationLocked {
            holder: "process-1234".to_string(),
            since: chrono::Utc::now(),
        };

        let msg = err.to_string();
        assert!(msg.contains("process-1234"));
        assert!(msg.contains("locked"));
    }
}
