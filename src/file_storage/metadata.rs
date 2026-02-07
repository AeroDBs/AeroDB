//! # Metadata Storage
//!
//! Abstraction for file storage metadata persistence.
//! Supports both in-memory (testing) and database-backed (production) storage.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use uuid::Uuid;

use super::errors::{StorageError, StorageResult};
use super::file::StorageObject;

/// Trait for metadata storage operations
pub trait MetadataStore: Send + Sync {
    /// Get an object by bucket and path
    fn get(&self, bucket_id: &Uuid, path: &str) -> StorageResult<Option<StorageObject>>;

    /// Put (insert/update) an object
    fn put(&self, object: &StorageObject) -> StorageResult<()>;

    /// Delete an object
    fn delete(&self, bucket_id: &Uuid, path: &str) -> StorageResult<()>;

    /// List objects in a bucket with optional prefix
    fn list(
        &self,
        bucket_id: &Uuid,
        prefix: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> StorageResult<Vec<StorageObject>>;

    /// Move an object to a new path
    fn move_object(&self, bucket_id: &Uuid, from_path: &str, to_path: &str) -> StorageResult<()>;

    /// Copy an object to a new path
    fn copy(
        &self,
        bucket_id: &Uuid,
        from_path: &str,
        to_bucket_id: &Uuid,
        to_path: &str,
    ) -> StorageResult<StorageObject>;
}

/// In-memory metadata store for testing
#[derive(Debug, Default)]
pub struct InMemoryMetadataStore {
    objects: RwLock<HashMap<String, StorageObject>>,
}

impl InMemoryMetadataStore {
    pub fn new() -> Self {
        Self {
            objects: RwLock::new(HashMap::new()),
        }
    }

    fn key(bucket_id: &Uuid, path: &str) -> String {
        format!("{}/{}", bucket_id, path)
    }
}

impl MetadataStore for InMemoryMetadataStore {
    fn get(&self, bucket_id: &Uuid, path: &str) -> StorageResult<Option<StorageObject>> {
        let key = Self::key(bucket_id, path);
        let objects = self
            .objects
            .read()
            .map_err(|_| StorageError::Internal("Lock poisoned".to_string()))?;
        Ok(objects.get(&key).cloned())
    }

    fn put(&self, object: &StorageObject) -> StorageResult<()> {
        let key = Self::key(&object.bucket_id, &object.path);
        let mut objects = self
            .objects
            .write()
            .map_err(|_| StorageError::Internal("Lock poisoned".to_string()))?;
        objects.insert(key, object.clone());
        Ok(())
    }

    fn delete(&self, bucket_id: &Uuid, path: &str) -> StorageResult<()> {
        let key = Self::key(bucket_id, path);
        let mut objects = self
            .objects
            .write()
            .map_err(|_| StorageError::Internal("Lock poisoned".to_string()))?;
        objects.remove(&key);
        Ok(())
    }

    fn list(
        &self,
        bucket_id: &Uuid,
        prefix: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> StorageResult<Vec<StorageObject>> {
        let objects = self
            .objects
            .read()
            .map_err(|_| StorageError::Internal("Lock poisoned".to_string()))?;

        let bucket_prefix = format!("{}/", bucket_id);
        let full_prefix = match prefix {
            Some(p) => format!("{}{}", bucket_prefix, p),
            None => bucket_prefix,
        };

        let result: Vec<StorageObject> = objects
            .iter()
            .filter(|(k, _)| k.starts_with(&full_prefix))
            .map(|(_, v)| v.clone())
            .skip(offset)
            .take(limit)
            .collect();

        Ok(result)
    }

    fn move_object(&self, bucket_id: &Uuid, from_path: &str, to_path: &str) -> StorageResult<()> {
        let from_key = Self::key(bucket_id, from_path);
        let to_key = Self::key(bucket_id, to_path);

        let mut objects = self
            .objects
            .write()
            .map_err(|_| StorageError::Internal("Lock poisoned".to_string()))?;

        let mut object = objects
            .remove(&from_key)
            .ok_or_else(|| StorageError::ObjectNotFound(from_path.to_string()))?;

        object.path = to_path.to_string();
        object.updated_at = Utc::now();
        objects.insert(to_key, object);
        Ok(())
    }

    fn copy(
        &self,
        bucket_id: &Uuid,
        from_path: &str,
        to_bucket_id: &Uuid,
        to_path: &str,
    ) -> StorageResult<StorageObject> {
        let from_key = Self::key(bucket_id, from_path);

        let objects = self
            .objects
            .read()
            .map_err(|_| StorageError::Internal("Lock poisoned".to_string()))?;

        let source = objects
            .get(&from_key)
            .ok_or_else(|| StorageError::ObjectNotFound(from_path.to_string()))?;

        // Create a copy with new ID and path
        let now = Utc::now();
        let mut copied = source.clone();
        copied.id = Uuid::new_v4();
        copied.bucket_id = *to_bucket_id;
        copied.path = to_path.to_string();
        copied.created_at = now;
        copied.updated_at = now;

        drop(objects);

        // Insert the copy
        self.put(&copied)?;
        Ok(copied)
    }
}

/// Database-backed metadata store using the REST API DatabaseFacade
/// This is the production implementation that persists metadata to AeroDB.
pub struct DatabaseMetadataStore<H> {
    handler: Arc<H>,
}

impl<H> DatabaseMetadataStore<H> {
    pub fn new(handler: Arc<H>) -> Self {
        Self { handler }
    }
}

impl<H: crate::rest_api::RestHandler + Send + Sync> MetadataStore for DatabaseMetadataStore<H> {
    fn get(&self, bucket_id: &Uuid, path: &str) -> StorageResult<Option<StorageObject>> {
        use crate::auth::rls::RlsContext;
        use crate::rest_api::filter::{FilterExpr, FilterOperator};
        use crate::rest_api::parser::QueryParams;

        let ctx = RlsContext::service_role();
        let mut params = QueryParams::default();
        params.filters = vec![
            FilterExpr::new(
                "bucket_id".to_string(),
                FilterOperator::Eq,
                json!(bucket_id.to_string()),
            ),
            FilterExpr::new("path".to_string(), FilterOperator::Eq, json!(path)),
        ];
        params.limit = 1;

        let result = self
            .handler
            .list("storage_objects", params, &ctx)
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        if result.data.is_empty() {
            return Ok(None);
        }

        serde_json::from_value(result.data[0].clone())
            .map(Some)
            .map_err(|e| StorageError::Internal(e.to_string()))
    }

    fn put(&self, object: &StorageObject) -> StorageResult<()> {
        use crate::auth::rls::RlsContext;

        let ctx = RlsContext::service_role();
        let data =
            serde_json::to_value(object).map_err(|e| StorageError::Internal(e.to_string()))?;

        // Check if exists, update or insert
        if self.get(&object.bucket_id, &object.path)?.is_some() {
            // Update existing
            self.handler
                .update("storage_objects", &object.id.to_string(), data, &ctx)
                .map_err(|e| StorageError::Internal(e.to_string()))?;
        } else {
            // Insert new
            self.handler
                .insert("storage_objects", data, &ctx)
                .map_err(|e| StorageError::Internal(e.to_string()))?;
        }

        Ok(())
    }

    fn delete(&self, bucket_id: &Uuid, path: &str) -> StorageResult<()> {
        use crate::auth::rls::RlsContext;

        // Get the object to find its ID
        let object = self
            .get(bucket_id, path)?
            .ok_or_else(|| StorageError::ObjectNotFound(path.to_string()))?;

        let ctx = RlsContext::service_role();
        self.handler
            .delete("storage_objects", &object.id.to_string(), &ctx)
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        Ok(())
    }

    fn list(
        &self,
        bucket_id: &Uuid,
        prefix: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> StorageResult<Vec<StorageObject>> {
        use crate::auth::rls::RlsContext;
        use crate::rest_api::filter::{FilterExpr, FilterOperator};
        use crate::rest_api::parser::QueryParams;

        let ctx = RlsContext::service_role();
        let mut params = QueryParams::default();
        params.filters = vec![FilterExpr::new(
            "bucket_id".to_string(),
            FilterOperator::Eq,
            json!(bucket_id.to_string()),
        )];

        if let Some(p) = prefix {
            params.filters.push(FilterExpr::new(
                "path".to_string(),
                FilterOperator::Like,
                json!(format!("{}%", p)),
            ));
        }

        params.limit = limit;
        params.offset = offset;

        let result = self
            .handler
            .list("storage_objects", params, &ctx)
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        result
            .data
            .into_iter()
            .map(|v| serde_json::from_value(v).map_err(|e| StorageError::Internal(e.to_string())))
            .collect()
    }

    fn move_object(&self, bucket_id: &Uuid, from_path: &str, to_path: &str) -> StorageResult<()> {
        use crate::auth::rls::RlsContext;

        let mut object = self
            .get(bucket_id, from_path)?
            .ok_or_else(|| StorageError::ObjectNotFound(from_path.to_string()))?;

        object.path = to_path.to_string();
        object.updated_at = Utc::now();

        let ctx = RlsContext::service_role();
        let data =
            serde_json::to_value(&object).map_err(|e| StorageError::Internal(e.to_string()))?;

        self.handler
            .update("storage_objects", &object.id.to_string(), data, &ctx)
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        Ok(())
    }

    fn copy(
        &self,
        bucket_id: &Uuid,
        from_path: &str,
        to_bucket_id: &Uuid,
        to_path: &str,
    ) -> StorageResult<StorageObject> {
        let source = self
            .get(bucket_id, from_path)?
            .ok_or_else(|| StorageError::ObjectNotFound(from_path.to_string()))?;

        let now = Utc::now();
        let mut copied = source;
        copied.id = Uuid::new_v4();
        copied.bucket_id = *to_bucket_id;
        copied.path = to_path.to_string();
        copied.created_at = now;
        copied.updated_at = now;

        self.put(&copied)?;
        Ok(copied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_object(bucket_id: Uuid, path: &str) -> StorageObject {
        StorageObject::new(
            bucket_id,
            path.to_string(),
            1024,
            "text/plain".to_string(),
            None,
        )
    }

    #[test]
    fn test_inmemory_put_get() {
        let store = InMemoryMetadataStore::new();
        let bucket_id = Uuid::new_v4();
        let object = create_test_object(bucket_id, "test/file.txt");

        store.put(&object).unwrap();

        let retrieved = store.get(&bucket_id, "test/file.txt").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().path, "test/file.txt");
    }

    #[test]
    fn test_inmemory_delete() {
        let store = InMemoryMetadataStore::new();
        let bucket_id = Uuid::new_v4();
        let object = create_test_object(bucket_id, "test/file.txt");

        store.put(&object).unwrap();
        store.delete(&bucket_id, "test/file.txt").unwrap();

        let retrieved = store.get(&bucket_id, "test/file.txt").unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_inmemory_list() {
        let store = InMemoryMetadataStore::new();
        let bucket_id = Uuid::new_v4();

        store
            .put(&create_test_object(bucket_id, "folder/a.txt"))
            .unwrap();
        store
            .put(&create_test_object(bucket_id, "folder/b.txt"))
            .unwrap();
        store
            .put(&create_test_object(bucket_id, "other/c.txt"))
            .unwrap();

        let result = store.list(&bucket_id, Some("folder/"), 10, 0).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_inmemory_move() {
        let store = InMemoryMetadataStore::new();
        let bucket_id = Uuid::new_v4();
        let object = create_test_object(bucket_id, "old/path.txt");

        store.put(&object).unwrap();
        store
            .move_object(&bucket_id, "old/path.txt", "new/path.txt")
            .unwrap();

        assert!(store.get(&bucket_id, "old/path.txt").unwrap().is_none());
        assert!(store.get(&bucket_id, "new/path.txt").unwrap().is_some());
    }

    #[test]
    fn test_inmemory_copy() {
        let store = InMemoryMetadataStore::new();
        let bucket_id = Uuid::new_v4();
        let object = create_test_object(bucket_id, "source.txt");

        store.put(&object).unwrap();
        let copied = store
            .copy(&bucket_id, "source.txt", &bucket_id, "dest.txt")
            .unwrap();

        assert_eq!(copied.path, "dest.txt");
        assert_ne!(copied.id, object.id);

        // Both should exist
        assert!(store.get(&bucket_id, "source.txt").unwrap().is_some());
        assert!(store.get(&bucket_id, "dest.txt").unwrap().is_some());
    }
}
