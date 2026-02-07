//! # Local Filesystem Backend

use std::fs;
use std::path::PathBuf;

use super::backend::StorageBackend;
use super::errors::{StorageError, StorageResult};

/// Local filesystem storage backend
#[derive(Debug)]
pub struct LocalBackend {
    root: PathBuf,
}

impl LocalBackend {
    /// Create a new local backend
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn full_path(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }
}

impl StorageBackend for LocalBackend {
    fn write(&self, path: &str, data: &[u8]) -> StorageResult<()> {
        let full_path = self.full_path(path);

        // Create parent directories
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).map_err(|e| StorageError::IoError(e.to_string()))?;
        }

        fs::write(&full_path, data).map_err(|e| StorageError::IoError(e.to_string()))
    }

    fn read(&self, path: &str) -> StorageResult<Vec<u8>> {
        let full_path = self.full_path(path);

        fs::read(&full_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::ObjectNotFound(path.to_string())
            } else {
                StorageError::IoError(e.to_string())
            }
        })
    }

    fn delete(&self, path: &str) -> StorageResult<()> {
        let full_path = self.full_path(path);

        fs::remove_file(&full_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::ObjectNotFound(path.to_string())
            } else {
                StorageError::IoError(e.to_string())
            }
        })
    }

    fn exists(&self, path: &str) -> StorageResult<bool> {
        Ok(self.full_path(path).exists())
    }

    fn list(&self, prefix: &str) -> StorageResult<Vec<String>> {
        let full_path = self.full_path(prefix);
        let mut results = Vec::new();

        if full_path.is_dir() {
            for entry in
                fs::read_dir(&full_path).map_err(|e| StorageError::IoError(e.to_string()))?
            {
                if let Ok(entry) = entry {
                    if let Some(name) = entry.file_name().to_str() {
                        results.push(format!("{}/{}", prefix, name));
                    }
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_read() {
        let temp = TempDir::new().unwrap();
        let backend = LocalBackend::new(temp.path().to_path_buf());

        backend.write("test.txt", b"hello").unwrap();
        let data = backend.read("test.txt").unwrap();
        assert_eq!(data, b"hello");
    }

    #[test]
    fn test_nested_path() {
        let temp = TempDir::new().unwrap();
        let backend = LocalBackend::new(temp.path().to_path_buf());

        backend.write("a/b/c/file.txt", b"nested").unwrap();
        let data = backend.read("a/b/c/file.txt").unwrap();
        assert_eq!(data, b"nested");
    }

    #[test]
    fn test_delete() {
        let temp = TempDir::new().unwrap();
        let backend = LocalBackend::new(temp.path().to_path_buf());

        backend.write("delete-me.txt", b"bye").unwrap();
        assert!(backend.exists("delete-me.txt").unwrap());

        backend.delete("delete-me.txt").unwrap();
        assert!(!backend.exists("delete-me.txt").unwrap());
    }

    #[test]
    fn test_not_found() {
        let temp = TempDir::new().unwrap();
        let backend = LocalBackend::new(temp.path().to_path_buf());

        let result = backend.read("nonexistent.txt");
        assert!(matches!(result, Err(StorageError::ObjectNotFound(_))));
    }
}
