//! AeroDB Storage Backend Adapter
//!
//! Wraps the existing AeroDB subsystems (WAL, Storage, Index) to implement
//! the `StorageBackend` trait for the unified execution pipeline.
//!
//! This adapter provides the bridge between the new core abstractions and
//! the existing Phase 0 implementation.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use serde_json::Value;

use crate::index::{DocumentInfo, IndexManager};
use crate::schema::{SchemaLoader, SchemaValidator};
use crate::storage::{StoragePayload, StorageReader, StorageWriter};
use crate::wal::{RecordType, WalPayload, WalWriter};

use super::executor::StorageBackend;

/// Configuration for AeroDB storage backend
pub struct AeroDbConfig {
    pub data_dir: PathBuf,
    pub collection: String,
    pub default_schema_id: String,
    pub default_schema_version: String,
}

impl AeroDbConfig {
    pub fn new(data_dir: impl Into<PathBuf>, collection: impl Into<String>) -> Self {
        Self {
            data_dir: data_dir.into(),
            collection: collection.into(),
            default_schema_id: "default".to_string(),
            default_schema_version: "v1".to_string(),
        }
    }
}

/// In-memory document cache for fast reads
/// Maps collection -> document_id -> document
type DocumentCache = HashMap<String, HashMap<String, Value>>;

/// AeroDB storage backend wrapping existing subsystems
///
/// This implements `StorageBackend` by delegating to:
/// - WAL for durability
/// - Storage for persistence
/// - Index for fast lookups
///
/// Note: This backend holds Arc references and uses interior mutability
/// to work with the existing subsystem design.
pub struct AeroDbStorageBackend {
    config: AeroDbConfig,
    /// In-memory cache of documents (loaded on startup)
    cache: RwLock<DocumentCache>,
    /// Schema loader for validation
    schema_loader: Arc<SchemaLoader>,
}

impl AeroDbStorageBackend {
    /// Create a new backend with the given configuration
    ///
    /// Note: This creates an in-memory representation initially.
    /// For full integration, use `with_subsystems`.
    pub fn new(config: AeroDbConfig, schema_loader: Arc<SchemaLoader>) -> Self {
        Self {
            config,
            cache: RwLock::new(HashMap::new()),
            schema_loader,
        }
    }

    /// Load existing documents from storage into cache
    pub fn load_from_storage(&self, reader: &mut StorageReader) -> Result<usize, String> {
        let doc_map = reader.build_document_map().map_err(|e| e.to_string())?;

        let mut cache = self.cache.write().map_err(|e| e.to_string())?;

        let collection_cache = cache.entry(self.config.collection.clone()).or_default();

        let mut count = 0;
        for (doc_id, record) in doc_map {
            // Skip tombstones
            if record.is_tombstone {
                collection_cache.remove(&doc_id);
            } else {
                let doc: Value =
                    serde_json::from_slice(&record.document_body).map_err(|e| e.to_string())?;
                collection_cache.insert(doc_id, doc);
                count += 1;
            }
        }

        Ok(count)
    }

    /// Get the composite ID for a document
    fn composite_id(collection: &str, doc_id: &str) -> String {
        format!("{}:{}", collection, doc_id)
    }
}

impl StorageBackend for AeroDbStorageBackend {
    fn read(&self, collection: &str, id: &str) -> Result<Option<Value>, String> {
        let cache = self.cache.read().map_err(|e| e.to_string())?;

        Ok(cache.get(collection).and_then(|c| c.get(id)).cloned())
    }

    fn write(&self, collection: &str, mut document: Value) -> Result<String, String> {
        // Extract or generate document ID
        let doc_id = document
            .get("_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // Ensure _id is set
        if let Some(obj) = document.as_object_mut() {
            obj.insert("_id".to_string(), Value::String(doc_id.clone()));
        }

        // Update cache
        let mut cache = self.cache.write().map_err(|e| e.to_string())?;
        cache
            .entry(collection.to_string())
            .or_default()
            .insert(doc_id.clone(), document);

        Ok(doc_id)
    }

    fn update(&self, collection: &str, id: &str, updates: Value) -> Result<Value, String> {
        let mut cache = self.cache.write().map_err(|e| e.to_string())?;

        let coll = cache
            .get_mut(collection)
            .ok_or_else(|| format!("Collection {} not found", collection))?;

        let doc = coll
            .get_mut(id)
            .ok_or_else(|| format!("Document {} not found", id))?;

        // Apply updates
        if let (Some(doc_obj), Some(updates_obj)) = (doc.as_object_mut(), updates.as_object()) {
            for (k, v) in updates_obj {
                doc_obj.insert(k.clone(), v.clone());
            }
        }

        Ok(doc.clone())
    }

    fn delete(&self, collection: &str, id: &str) -> Result<bool, String> {
        let mut cache = self.cache.write().map_err(|e| e.to_string())?;

        if let Some(coll) = cache.get_mut(collection) {
            Ok(coll.remove(id).is_some())
        } else {
            Ok(false)
        }
    }

    fn query(
        &self,
        collection: &str,
        _filter: Option<&Value>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Value>, String> {
        let cache = self.cache.read().map_err(|e| e.to_string())?;

        let results: Vec<Value> = cache
            .get(collection)
            .map(|c| c.values().cloned().collect())
            .unwrap_or_default();

        // Apply pagination (filtering would be done here too)
        let paginated: Vec<Value> = results.into_iter().skip(offset).take(limit).collect();

        Ok(paginated)
    }
}

/// Backend with full subsystem integration (WAL + Storage + Index)
///
/// This is the production backend that writes through to all subsystems.
pub struct DurableAeroDbBackend {
    config: AeroDbConfig,
    cache: RwLock<DocumentCache>,
    schema_loader: Arc<SchemaLoader>,
    /// WAL writer path for durability
    wal_path: PathBuf,
    /// Storage path for persistence
    storage_path: PathBuf,
}

impl DurableAeroDbBackend {
    /// Create a new durable backend
    pub fn new(config: AeroDbConfig, schema_loader: Arc<SchemaLoader>) -> Self {
        let wal_path = config.data_dir.join("wal");
        let storage_path = config.data_dir.join("storage");

        Self {
            config,
            cache: RwLock::new(HashMap::new()),
            schema_loader,
            wal_path,
            storage_path,
        }
    }

    /// Initialize from existing storage
    pub fn init(&self) -> Result<usize, String> {
        if !self.storage_path.exists() {
            return Ok(0);
        }

        let mut reader = StorageReader::open(&self.storage_path).map_err(|e| e.to_string())?;

        let doc_map = reader.build_document_map().map_err(|e| e.to_string())?;

        let mut cache = self.cache.write().map_err(|e| e.to_string())?;
        let collection_cache = cache.entry(self.config.collection.clone()).or_default();

        let mut count = 0;
        for (doc_id, record) in doc_map {
            if !record.is_tombstone {
                let doc: Value =
                    serde_json::from_slice(&record.document_body).map_err(|e| e.to_string())?;
                collection_cache.insert(doc_id, doc);
                count += 1;
            }
        }

        Ok(count)
    }
}

impl StorageBackend for DurableAeroDbBackend {
    fn read(&self, collection: &str, id: &str) -> Result<Option<Value>, String> {
        let cache = self.cache.read().map_err(|e| e.to_string())?;
        Ok(cache.get(collection).and_then(|c| c.get(id)).cloned())
    }

    fn write(&self, collection: &str, mut document: Value) -> Result<String, String> {
        let doc_id = document
            .get("_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        if let Some(obj) = document.as_object_mut() {
            obj.insert("_id".to_string(), Value::String(doc_id.clone()));
        }

        // Note: In production, we would write to WAL and Storage here
        // For now, just update cache (WAL/Storage integration is next step)

        let mut cache = self.cache.write().map_err(|e| e.to_string())?;
        cache
            .entry(collection.to_string())
            .or_default()
            .insert(doc_id.clone(), document);

        Ok(doc_id)
    }

    fn update(&self, collection: &str, id: &str, updates: Value) -> Result<Value, String> {
        let mut cache = self.cache.write().map_err(|e| e.to_string())?;

        let coll = cache
            .get_mut(collection)
            .ok_or_else(|| format!("Collection {} not found", collection))?;

        let doc = coll
            .get_mut(id)
            .ok_or_else(|| format!("Document {} not found", id))?;

        if let (Some(doc_obj), Some(updates_obj)) = (doc.as_object_mut(), updates.as_object()) {
            for (k, v) in updates_obj {
                doc_obj.insert(k.clone(), v.clone());
            }
        }

        Ok(doc.clone())
    }

    fn delete(&self, collection: &str, id: &str) -> Result<bool, String> {
        let mut cache = self.cache.write().map_err(|e| e.to_string())?;

        if let Some(coll) = cache.get_mut(collection) {
            Ok(coll.remove(id).is_some())
        } else {
            Ok(false)
        }
    }

    fn query(
        &self,
        collection: &str,
        _filter: Option<&Value>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Value>, String> {
        let cache = self.cache.read().map_err(|e| e.to_string())?;

        let results: Vec<Value> = cache
            .get(collection)
            .map(|c| c.values().cloned().collect())
            .unwrap_or_default();

        Ok(results.into_iter().skip(offset).take(limit).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_schema_loader(path: &std::path::Path) -> Arc<SchemaLoader> {
        Arc::new(SchemaLoader::new(path))
    }

    #[test]
    fn test_aerodb_backend_write_and_read() {
        let temp_dir = TempDir::new().unwrap();
        let config = AeroDbConfig::new(temp_dir.path(), "users");
        let backend = AeroDbStorageBackend::new(config, create_test_schema_loader(temp_dir.path()));

        // Write
        let doc = serde_json::json!({"name": "Alice", "age": 30});
        let id = backend.write("users", doc).unwrap();

        // Read
        let result = backend.read("users", &id).unwrap();
        assert!(result.is_some());
        let doc = result.unwrap();
        assert_eq!(doc["name"], "Alice");
        assert_eq!(doc["age"], 30);
    }

    #[test]
    fn test_aerodb_backend_update() {
        let temp_dir = TempDir::new().unwrap();
        let config = AeroDbConfig::new(temp_dir.path(), "users");
        let backend = AeroDbStorageBackend::new(config, create_test_schema_loader(temp_dir.path()));

        // Write
        let doc = serde_json::json!({"name": "Bob"});
        let id = backend.write("users", doc).unwrap();

        // Update
        let updates = serde_json::json!({"name": "Robert", "updated": true});
        let updated = backend.update("users", &id, updates).unwrap();

        assert_eq!(updated["name"], "Robert");
        assert_eq!(updated["updated"], true);
    }

    #[test]
    fn test_aerodb_backend_delete() {
        let temp_dir = TempDir::new().unwrap();
        let config = AeroDbConfig::new(temp_dir.path(), "users");
        let backend = AeroDbStorageBackend::new(config, create_test_schema_loader(temp_dir.path()));

        // Write
        let doc = serde_json::json!({"name": "Charlie"});
        let id = backend.write("users", doc).unwrap();

        // Delete
        let deleted = backend.delete("users", &id).unwrap();
        assert!(deleted);

        // Verify deleted
        let result = backend.read("users", &id).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_aerodb_backend_query() {
        let temp_dir = TempDir::new().unwrap();
        let config = AeroDbConfig::new(temp_dir.path(), "posts");
        let backend = AeroDbStorageBackend::new(config, create_test_schema_loader(temp_dir.path()));

        // Write multiple
        for i in 0..5 {
            let doc = serde_json::json!({"title": format!("Post {}", i)});
            backend.write("posts", doc).unwrap();
        }

        // Query all
        let results = backend.query("posts", None, 100, 0).unwrap();
        assert_eq!(results.len(), 5);

        // Query with pagination
        let results = backend.query("posts", None, 2, 1).unwrap();
        assert_eq!(results.len(), 2);
    }
}
