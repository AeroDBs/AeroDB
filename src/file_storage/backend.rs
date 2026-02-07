//! # Storage Backend Trait

use super::errors::StorageResult;

/// Backend trait for file storage
pub trait StorageBackend: Send + Sync + std::fmt::Debug {
    /// Write data to path
    fn write(&self, path: &str, data: &[u8]) -> StorageResult<()>;

    /// Read data from path
    fn read(&self, path: &str) -> StorageResult<Vec<u8>>;

    /// Delete file at path
    fn delete(&self, path: &str) -> StorageResult<()>;

    /// Check if path exists
    fn exists(&self, path: &str) -> StorageResult<bool>;

    /// List files with prefix
    fn list(&self, prefix: &str) -> StorageResult<Vec<String>>;
}
