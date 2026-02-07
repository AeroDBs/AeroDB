//! # File Storage Errors

use thiserror::Error;

/// Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;

/// File storage errors
#[derive(Debug, Clone, Error)]
pub enum StorageError {
    // Bucket errors
    #[error("Bucket not found: {0}")]
    BucketNotFound(String),

    #[error("Bucket already exists: {0}")]
    BucketAlreadyExists(String),

    #[error("Bucket not empty")]
    BucketNotEmpty,

    // Object errors
    #[error("Object not found: {0}")]
    ObjectNotFound(String),

    #[error("Object already exists: {0}")]
    ObjectAlreadyExists(String),

    // Validation errors
    #[error("File too large: {0} bytes (max: {1})")]
    FileTooLarge(u64, u64),

    #[error("Invalid MIME type: {0}")]
    InvalidMimeType(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    // Permission errors
    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    // Signed URL errors
    #[error("URL expired")]
    UrlExpired,

    #[error("Invalid signature")]
    InvalidSignature,

    // I/O errors
    #[error("Storage full")]
    StorageFull,

    #[error("I/O error: {0}")]
    IoError(String),

    #[error("Checksum mismatch")]
    ChecksumMismatch,

    // Internal
    #[error("Internal error: {0}")]
    Internal(String),
}

impl StorageError {
    /// Get HTTP status code
    pub fn status_code(&self) -> u16 {
        match self {
            StorageError::BucketNotFound(_) => 404,
            StorageError::BucketAlreadyExists(_) => 409,
            StorageError::BucketNotEmpty => 409,
            StorageError::ObjectNotFound(_) => 404,
            StorageError::ObjectAlreadyExists(_) => 409,
            StorageError::FileTooLarge(_, _) => 413,
            StorageError::InvalidMimeType(_) => 415,
            StorageError::InvalidPath(_) => 400,
            StorageError::Unauthorized => 401,
            StorageError::Forbidden => 403,
            StorageError::UrlExpired => 403,
            StorageError::InvalidSignature => 403,
            StorageError::StorageFull => 507,
            StorageError::IoError(_) => 500,
            StorageError::ChecksumMismatch => 500,
            StorageError::Internal(_) => 500,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_codes() {
        assert_eq!(
            StorageError::BucketNotFound("test".into()).status_code(),
            404
        );
        assert_eq!(StorageError::FileTooLarge(100, 50).status_code(), 413);
        assert_eq!(StorageError::Unauthorized.status_code(), 401);
    }
}
