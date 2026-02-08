//! Resource Errors
//!
//! Error types for resource exhaustion and limits.
//! Maps to HTTP 507 (Insufficient Storage) and 503 (Service Unavailable).

use std::fmt;

/// Resource type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Memory,
    Disk,
    FileDescriptors,
    Connections,
    ResultSet,
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceType::Memory => write!(f, "memory"),
            ResourceType::Disk => write!(f, "disk"),
            ResourceType::FileDescriptors => write!(f, "file_descriptors"),
            ResourceType::Connections => write!(f, "connections"),
            ResourceType::ResultSet => write!(f, "result_set"),
        }
    }
}

/// Resource error types
#[derive(Debug)]
pub enum ResourceError {
    /// Disk is full or below minimum threshold
    DiskFull {
        available: u64,
        required: u64,
    },

    /// Memory limit exceeded
    MemoryExhausted {
        current: u64,
        requested: u64,
        limit: u64,
    },

    /// File descriptor limit reached
    FileDescriptorLimit {
        current: usize,
        limit: usize,
    },

    /// Connection limit reached
    ConnectionLimit {
        current: usize,
        limit: usize,
    },

    /// Result set too large
    ResultSetTooLarge {
        requested: usize,
        limit: usize,
    },

    /// Write refused due to read-only mode
    ReadOnlyMode,

    /// IO error during resource check
    IoError(String),
}

impl fmt::Display for ResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceError::DiskFull { available, required } => {
                write!(
                    f,
                    "Disk space exhausted: {} bytes available, {} bytes required. \
                     Free disk space or increase storage to continue writes.",
                    available, required
                )
            }
            ResourceError::MemoryExhausted { current, requested, limit } => {
                write!(
                    f,
                    "Memory limit exceeded: {} bytes in use, {} bytes requested, {} bytes limit. \
                     Reduce query scope or increase max_memory_bytes configuration.",
                    current, requested, limit
                )
            }
            ResourceError::FileDescriptorLimit { current, limit } => {
                write!(
                    f,
                    "File descriptor limit reached: {} open, {} max. \
                     Close unused connections or increase max_file_descriptors configuration.",
                    current, limit
                )
            }
            ResourceError::ConnectionLimit { current, limit } => {
                write!(
                    f,
                    "Connection limit reached: {} active, {} max. \
                     Close unused connections or try again later.",
                    current, limit
                )
            }
            ResourceError::ResultSetTooLarge { requested, limit } => {
                write!(
                    f,
                    "Result set too large: {} documents requested, {} max allowed. \
                     Add filters or pagination to reduce result size.",
                    requested, limit
                )
            }
            ResourceError::ReadOnlyMode => {
                write!(
                    f,
                    "System is in read-only mode due to resource exhaustion. \
                     Free resources and restart to resume writes."
                )
            }
            ResourceError::IoError(msg) => {
                write!(f, "Resource check failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for ResourceError {}

impl ResourceError {
    /// Get the resource type for this error
    pub fn resource_type(&self) -> ResourceType {
        match self {
            ResourceError::DiskFull { .. } => ResourceType::Disk,
            ResourceError::MemoryExhausted { .. } => ResourceType::Memory,
            ResourceError::FileDescriptorLimit { .. } => ResourceType::FileDescriptors,
            ResourceError::ConnectionLimit { .. } => ResourceType::Connections,
            ResourceError::ResultSetTooLarge { .. } => ResourceType::ResultSet,
            ResourceError::ReadOnlyMode => ResourceType::Disk,
            ResourceError::IoError(_) => ResourceType::Disk,
        }
    }

    /// Get HTTP status code for this error
    pub fn http_status_code(&self) -> u16 {
        match self {
            ResourceError::DiskFull { .. } => 507,           // Insufficient Storage
            ResourceError::MemoryExhausted { .. } => 503,    // Service Unavailable
            ResourceError::FileDescriptorLimit { .. } => 503,
            ResourceError::ConnectionLimit { .. } => 503,
            ResourceError::ResultSetTooLarge { .. } => 400,  // Bad Request (client can fix)
            ResourceError::ReadOnlyMode => 503,
            ResourceError::IoError(_) => 500,
        }
    }

    /// Is this error recoverable without restart?
    pub fn is_recoverable(&self) -> bool {
        match self {
            ResourceError::DiskFull { .. } => false, // Need manual intervention
            ResourceError::MemoryExhausted { .. } => true, // Wait for queries to complete
            ResourceError::FileDescriptorLimit { .. } => true,
            ResourceError::ConnectionLimit { .. } => true,
            ResourceError::ResultSetTooLarge { .. } => true,
            ResourceError::ReadOnlyMode => false,
            ResourceError::IoError(_) => false,
        }
    }
}

/// Result type for resource operations
pub type ResourceResult<T> = Result<T, ResourceError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ResourceError::DiskFull {
            available: 1000,
            required: 2000,
        };
        let msg = err.to_string();
        assert!(msg.contains("1000"));
        assert!(msg.contains("2000"));
    }

    #[test]
    fn test_http_status_codes() {
        assert_eq!(
            ResourceError::DiskFull { available: 0, required: 1 }.http_status_code(),
            507
        );
        assert_eq!(
            ResourceError::MemoryExhausted { current: 0, requested: 1, limit: 0 }.http_status_code(),
            503
        );
        assert_eq!(
            ResourceError::ResultSetTooLarge { requested: 10, limit: 5 }.http_status_code(),
            400
        );
    }
}
