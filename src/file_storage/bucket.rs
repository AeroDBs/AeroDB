//! # Bucket Management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

use super::errors::{StorageError, StorageResult};

/// Bucket access policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BucketPolicy {
    /// Anyone can read, owner can write
    Public,
    /// Only authenticated users can read/write
    Authenticated,
    /// Only owner can read/write
    Private,
}

impl Default for BucketPolicy {
    fn default() -> Self {
        Self::Private
    }
}

/// Bucket configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketConfig {
    /// Allowed MIME types (empty = all)
    #[serde(default)]
    pub allowed_mime_types: Vec<String>,

    /// Maximum file size in bytes (0 = unlimited)
    #[serde(default = "default_max_size")]
    pub max_file_size: u64,

    /// Access policy
    #[serde(default)]
    pub policy: BucketPolicy,
}

fn default_max_size() -> u64 {
    100 * 1024 * 1024 // 100MB
}

impl Default for BucketConfig {
    fn default() -> Self {
        Self {
            allowed_mime_types: Vec::new(),
            max_file_size: default_max_size(),
            policy: BucketPolicy::Private,
        }
    }
}

/// A storage bucket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bucket {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Option<Uuid>,
    pub config: BucketConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Bucket {
    /// Create a new bucket
    pub fn new(name: String, owner_id: Option<Uuid>, config: BucketConfig) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            owner_id,
            config,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if MIME type is allowed
    pub fn is_mime_allowed(&self, mime: &str) -> bool {
        if self.config.allowed_mime_types.is_empty() {
            return true;
        }

        for allowed in &self.config.allowed_mime_types {
            if allowed.ends_with("/*") {
                let prefix = &allowed[..allowed.len() - 1];
                if mime.starts_with(prefix) {
                    return true;
                }
            } else if allowed == mime {
                return true;
            }
        }

        false
    }

    /// Check file size limit
    pub fn check_size(&self, size: u64) -> StorageResult<()> {
        if self.config.max_file_size > 0 && size > self.config.max_file_size {
            Err(StorageError::FileTooLarge(size, self.config.max_file_size))
        } else {
            Ok(())
        }
    }
}

/// Bucket registry
#[derive(Debug, Default)]
pub struct BucketRegistry {
    buckets: RwLock<HashMap<String, Bucket>>,
}

impl BucketRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a bucket
    pub fn create(
        &self,
        name: String,
        owner_id: Option<Uuid>,
        config: BucketConfig,
    ) -> StorageResult<Bucket> {
        let mut buckets = self
            .buckets
            .write()
            .map_err(|_| StorageError::Internal("Lock poisoned".into()))?;

        if buckets.contains_key(&name) {
            return Err(StorageError::BucketAlreadyExists(name));
        }

        let bucket = Bucket::new(name.clone(), owner_id, config);
        buckets.insert(name, bucket.clone());
        Ok(bucket)
    }

    /// Get a bucket
    pub fn get(&self, name: &str) -> StorageResult<Bucket> {
        let buckets = self
            .buckets
            .read()
            .map_err(|_| StorageError::Internal("Lock poisoned".into()))?;

        buckets
            .get(name)
            .cloned()
            .ok_or_else(|| StorageError::BucketNotFound(name.to_string()))
    }

    /// Delete a bucket
    pub fn delete(&self, name: &str) -> StorageResult<()> {
        let mut buckets = self
            .buckets
            .write()
            .map_err(|_| StorageError::Internal("Lock poisoned".into()))?;

        buckets
            .remove(name)
            .ok_or_else(|| StorageError::BucketNotFound(name.to_string()))?;

        Ok(())
    }

    /// List all buckets
    pub fn list(&self) -> Vec<Bucket> {
        self.buckets
            .read()
            .map(|b| b.values().cloned().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bucket_creation() {
        let bucket = Bucket::new("avatars".to_string(), None, BucketConfig::default());
        assert_eq!(bucket.name, "avatars");
    }

    #[test]
    fn test_mime_type_validation() {
        let mut config = BucketConfig::default();
        config.allowed_mime_types = vec!["image/*".to_string(), "application/pdf".to_string()];

        let bucket = Bucket::new("docs".to_string(), None, config);

        assert!(bucket.is_mime_allowed("image/png"));
        assert!(bucket.is_mime_allowed("image/jpeg"));
        assert!(bucket.is_mime_allowed("application/pdf"));
        assert!(!bucket.is_mime_allowed("text/plain"));
    }

    #[test]
    fn test_size_validation() {
        let mut config = BucketConfig::default();
        config.max_file_size = 1024; // 1KB

        let bucket = Bucket::new("small".to_string(), None, config);

        assert!(bucket.check_size(500).is_ok());
        assert!(bucket.check_size(2048).is_err());
    }

    #[test]
    fn test_registry() {
        let registry = BucketRegistry::new();

        let bucket = registry
            .create("test".to_string(), None, BucketConfig::default())
            .unwrap();
        assert_eq!(bucket.name, "test");

        let fetched = registry.get("test").unwrap();
        assert_eq!(fetched.id, bucket.id);

        registry.delete("test").unwrap();
        assert!(registry.get("test").is_err());
    }
}
