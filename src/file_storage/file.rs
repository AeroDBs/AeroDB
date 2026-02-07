//! # File Operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

use super::backend::StorageBackend;
use super::bucket::{Bucket, BucketRegistry};
use super::errors::{StorageError, StorageResult};
use super::permissions::StoragePermissions;
use crate::auth::rls::RlsContext;

/// A storage object (file metadata)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageObject {
    pub id: Uuid,
    pub bucket_id: Uuid,
    pub path: String,
    pub size: u64,
    pub content_type: String,
    pub checksum: String,
    pub owner_id: Option<Uuid>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl StorageObject {
    /// Create a new storage object
    pub fn new(
        bucket_id: Uuid,
        path: String,
        size: u64,
        content_type: String,
        owner_id: Option<Uuid>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            bucket_id,
            path,
            size,
            content_type,
            checksum: String::new(),
            owner_id,
            metadata: Value::Object(Default::default()),
            created_at: now,
            updated_at: now,
        }
    }

    /// Calculate checksum for data
    pub fn calculate_checksum(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}

/// File service for CRUD operations
#[derive(Debug)]
pub struct FileService<B: StorageBackend> {
    backend: B,
    buckets: BucketRegistry,
    objects: RwLock<HashMap<String, StorageObject>>, // key: bucket_id/path
    permissions: StoragePermissions,
}

impl<B: StorageBackend> FileService<B> {
    /// Create a new file service
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            buckets: BucketRegistry::new(),
            objects: RwLock::new(HashMap::new()),
            permissions: StoragePermissions::new(),
        }
    }

    /// Get bucket registry
    pub fn buckets(&self) -> &BucketRegistry {
        &self.buckets
    }

    fn object_key(bucket_id: &Uuid, path: &str) -> String {
        format!("{}/{}", bucket_id, path)
    }

    /// Upload a file
    pub fn upload(
        &self,
        bucket_name: &str,
        path: &str,
        data: &[u8],
        content_type: &str,
        context: &RlsContext,
    ) -> StorageResult<StorageObject> {
        let bucket = self.buckets.get(bucket_name)?;

        // Check permissions
        self.permissions.check_write(&bucket, context)?;

        // Validate
        bucket.check_size(data.len() as u64)?;
        if !bucket.is_mime_allowed(content_type) {
            return Err(StorageError::InvalidMimeType(content_type.to_string()));
        }

        // Write to backend
        let storage_path = format!("{}/{}", bucket.id, path);
        self.backend.write(&storage_path, data)?;

        // Create metadata
        let mut object = StorageObject::new(
            bucket.id,
            path.to_string(),
            data.len() as u64,
            content_type.to_string(),
            context.user_id,
        );
        object.checksum = StorageObject::calculate_checksum(data);

        // Store metadata
        let key = Self::object_key(&bucket.id, path);
        if let Ok(mut objects) = self.objects.write() {
            objects.insert(key, object.clone());
        }

        Ok(object)
    }

    /// Download a file
    pub fn download(
        &self,
        bucket_name: &str,
        path: &str,
        context: &RlsContext,
    ) -> StorageResult<(StorageObject, Vec<u8>)> {
        let bucket = self.buckets.get(bucket_name)?;

        // Check permissions
        self.permissions.check_read(&bucket, context)?;

        // Get metadata
        let key = Self::object_key(&bucket.id, path);
        let object = {
            let objects = self
                .objects
                .read()
                .map_err(|_| StorageError::Internal("Lock poisoned".into()))?;
            objects
                .get(&key)
                .cloned()
                .ok_or_else(|| StorageError::ObjectNotFound(path.to_string()))?
        };

        // Read from backend
        let storage_path = format!("{}/{}", bucket.id, path);
        let data = self.backend.read(&storage_path)?;

        Ok((object, data))
    }

    /// Delete a file
    pub fn delete(&self, bucket_name: &str, path: &str, context: &RlsContext) -> StorageResult<()> {
        let bucket = self.buckets.get(bucket_name)?;

        // Check permissions
        self.permissions.check_delete(&bucket, context)?;

        let key = Self::object_key(&bucket.id, path);

        // Remove metadata
        let object = {
            let mut objects = self
                .objects
                .write()
                .map_err(|_| StorageError::Internal("Lock poisoned".into()))?;
            objects
                .remove(&key)
                .ok_or_else(|| StorageError::ObjectNotFound(path.to_string()))?
        };

        // Delete from backend
        let storage_path = format!("{}/{}", bucket.id, &object.path);
        self.backend.delete(&storage_path)?;

        Ok(())
    }

    /// List objects in a bucket
    pub fn list(
        &self,
        bucket_name: &str,
        prefix: &str,
        context: &RlsContext,
    ) -> StorageResult<Vec<StorageObject>> {
        let bucket = self.buckets.get(bucket_name)?;

        // Check permissions
        self.permissions.check_read(&bucket, context)?;

        let objects = self
            .objects
            .read()
            .map_err(|_| StorageError::Internal("Lock poisoned".into()))?;

        let prefix_key = format!("{}/", bucket.id);
        let results: Vec<StorageObject> = objects
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix_key))
            .filter(|(_, obj)| obj.path.starts_with(prefix))
            .map(|(_, obj)| obj.clone())
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_storage::bucket::{BucketConfig, BucketPolicy};
    use crate::file_storage::local::LocalBackend;
    use tempfile::TempDir;

    fn create_test_service() -> (FileService<LocalBackend>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let backend = LocalBackend::new(temp_dir.path().to_path_buf());
        let service = FileService::new(backend);
        (service, temp_dir)
    }

    fn public_bucket_config() -> BucketConfig {
        let mut config = BucketConfig::default();
        config.policy = BucketPolicy::Public;
        config
    }

    #[test]
    fn test_upload_download() {
        let (service, _temp) = create_test_service();
        let user_id = Uuid::new_v4();
        let context = RlsContext::authenticated(user_id);

        // Create public bucket
        service
            .buckets()
            .create("test".to_string(), None, public_bucket_config())
            .unwrap();

        // Upload
        let data = b"Hello, World!";
        let obj = service
            .upload("test", "hello.txt", data, "text/plain", &context)
            .unwrap();
        assert_eq!(obj.size, 13);

        // Download
        let (obj2, downloaded) = service.download("test", "hello.txt", &context).unwrap();
        assert_eq!(downloaded, data);
        assert_eq!(obj.id, obj2.id);
    }

    #[test]
    fn test_delete() {
        let (service, _temp) = create_test_service();
        let user_id = Uuid::new_v4();
        let context = RlsContext::authenticated(user_id);

        service
            .buckets()
            .create("test".to_string(), None, public_bucket_config())
            .unwrap();
        service
            .upload("test", "file.txt", b"data", "text/plain", &context)
            .unwrap();

        service.delete("test", "file.txt", &context).unwrap();

        assert!(service.download("test", "file.txt", &context).is_err());
    }

    #[test]
    fn test_checksum() {
        let checksum = StorageObject::calculate_checksum(b"test");
        assert!(!checksum.is_empty());
        assert_eq!(checksum.len(), 64); // SHA-256 hex
    }
}
