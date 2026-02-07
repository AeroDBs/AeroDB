//! Write-Through Storage Backend
//!
//! Full durability backend that writes through to WAL and Storage.
//! This is the production backend for AeroDB.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};

use serde_json::Value;

use crate::index::{DocumentInfo, IndexManager};
use crate::schema::SchemaLoader;
use crate::storage::{StoragePayload, StorageReader, StorageWriter};
use crate::wal::{RecordType, WalPayload, WalWriter};

use super::executor::StorageBackend;

/// In-memory document cache
type DocumentCache = HashMap<String, HashMap<String, Value>>;

/// Write-through backend with full WAL + Storage durability
///
/// This backend provides:
/// - WAL append before storage write (crash safety)
/// - Storage write with fsync (durability)
/// - In-memory cache (performance)
/// - Optional index updates
///
/// Thread-safety is provided via `Mutex` on writers.
pub struct WriteThroughBackend {
    /// Collection name
    collection: String,
    /// Default schema ID for operations
    default_schema_id: String,
    /// Default schema version
    default_schema_version: String,
    /// In-memory cache of documents
    cache: RwLock<DocumentCache>,
    /// WAL writer (protected by mutex)
    wal_writer: Mutex<WalWriter>,
    /// Storage writer (protected by mutex)
    storage_writer: Mutex<StorageWriter>,
    /// Index manager (optional)
    index_manager: Option<Mutex<IndexManager>>,
}

impl WriteThroughBackend {
    /// Create a new write-through backend from existing subsystems
    pub fn new(
        collection: impl Into<String>,
        wal_writer: WalWriter,
        storage_writer: StorageWriter,
    ) -> Self {
        Self {
            collection: collection.into(),
            default_schema_id: "default".to_string(),
            default_schema_version: "v1".to_string(),
            cache: RwLock::new(HashMap::new()),
            wal_writer: Mutex::new(wal_writer),
            storage_writer: Mutex::new(storage_writer),
            index_manager: None,
        }
    }

    /// Open a new backend from a data directory
    pub fn open(data_dir: &Path, collection: impl Into<String>) -> Result<Self, String> {
        let wal_writer = WalWriter::open(data_dir).map_err(|e| e.to_string())?;
        let storage_writer = StorageWriter::open(data_dir).map_err(|e| e.to_string())?;

        let mut backend = Self::new(collection, wal_writer, storage_writer);
        backend.load_from_storage(data_dir)?;

        Ok(backend)
    }

    /// Set schema defaults
    pub fn with_schema(mut self, schema_id: &str, schema_version: &str) -> Self {
        self.default_schema_id = schema_id.to_string();
        self.default_schema_version = schema_version.to_string();
        self
    }

    /// Attach an index manager for automatic index updates
    pub fn with_index(mut self, index_manager: IndexManager) -> Self {
        self.index_manager = Some(Mutex::new(index_manager));
        self
    }

    /// Load existing documents from storage into cache
    fn load_from_storage(&mut self, data_dir: &Path) -> Result<usize, String> {
        let storage_path = data_dir.join("data").join("documents.dat");

        if !storage_path.exists() {
            return Ok(0);
        }

        let mut reader = StorageReader::open(&storage_path).map_err(|e| e.to_string())?;

        let doc_map = reader.build_document_map().map_err(|e| e.to_string())?;

        let mut cache = self.cache.write().map_err(|e| e.to_string())?;
        let collection_cache = cache.entry(self.collection.clone()).or_default();

        let mut count = 0;
        for (composite_id, record) in doc_map {
            // Parse composite ID (collection:doc_id)
            let doc_id = composite_id
                .split(':')
                .nth(1)
                .unwrap_or(&composite_id)
                .to_string();

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

impl StorageBackend for WriteThroughBackend {
    fn read(&self, collection: &str, id: &str) -> Result<Option<Value>, String> {
        let cache = self.cache.read().map_err(|e| e.to_string())?;
        Ok(cache.get(collection).and_then(|c| c.get(id)).cloned())
    }

    fn write(&self, collection: &str, mut document: Value) -> Result<String, String> {
        // Generate or extract document ID
        let doc_id = document
            .get("_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // Ensure _id is set
        if let Some(obj) = document.as_object_mut() {
            obj.insert("_id".to_string(), Value::String(doc_id.clone()));
        }

        // Serialize document
        let body_bytes = serde_json::to_vec(&document)
            .map_err(|e| format!("Failed to serialize document: {}", e))?;

        // 1. WAL append (durability first)
        let wal_payload = WalPayload::new(
            collection,
            &doc_id,
            &self.default_schema_id,
            &self.default_schema_version,
            body_bytes.clone(),
        );

        {
            let mut wal = self.wal_writer.lock().map_err(|e| e.to_string())?;
            wal.append(RecordType::Insert, wal_payload)
                .map_err(|e| format!("WAL append failed: {}", e))?;
        }

        // 2. Storage write (persistence)
        let storage_payload = StoragePayload::new(
            collection,
            &doc_id,
            &self.default_schema_id,
            &self.default_schema_version,
            body_bytes,
        );

        let offset = {
            let mut storage = self.storage_writer.lock().map_err(|e| e.to_string())?;
            storage
                .write(&storage_payload)
                .map_err(|e| format!("Storage write failed: {}", e))?
        };

        // 3. Index update (optional)
        if let Some(ref index_mutex) = self.index_manager {
            let doc_info = DocumentInfo {
                document_id: doc_id.clone(),
                schema_id: self.default_schema_id.clone(),
                schema_version: self.default_schema_version.clone(),
                is_tombstone: false,
                body: document.clone(),
                offset,
            };
            let mut index = index_mutex.lock().map_err(|e| e.to_string())?;
            index.apply_write(&doc_info);
        }

        // 4. Cache update (performance)
        {
            let mut cache = self.cache.write().map_err(|e| e.to_string())?;
            cache
                .entry(collection.to_string())
                .or_default()
                .insert(doc_id.clone(), document);
        }

        Ok(doc_id)
    }

    fn update(&self, collection: &str, id: &str, updates: Value) -> Result<Value, String> {
        // Read current document
        let current = self
            .read(collection, id)?
            .ok_or_else(|| format!("Document {} not found", id))?;

        // Merge updates
        let mut updated = current.clone();
        if let (Some(doc_obj), Some(updates_obj)) = (updated.as_object_mut(), updates.as_object()) {
            for (k, v) in updates_obj {
                doc_obj.insert(k.clone(), v.clone());
            }
        }

        // Serialize merged document
        let body_bytes = serde_json::to_vec(&updated)
            .map_err(|e| format!("Failed to serialize document: {}", e))?;

        // 1. WAL append
        let wal_payload = WalPayload::new(
            collection,
            id,
            &self.default_schema_id,
            &self.default_schema_version,
            body_bytes.clone(),
        );

        {
            let mut wal = self.wal_writer.lock().map_err(|e| e.to_string())?;
            wal.append(RecordType::Update, wal_payload)
                .map_err(|e| format!("WAL append failed: {}", e))?;
        }

        // 2. Storage write
        let storage_payload = StoragePayload::new(
            collection,
            id,
            &self.default_schema_id,
            &self.default_schema_version,
            body_bytes,
        );

        let offset = {
            let mut storage = self.storage_writer.lock().map_err(|e| e.to_string())?;
            storage
                .write(&storage_payload)
                .map_err(|e| format!("Storage write failed: {}", e))?
        };

        // 3. Index update
        if let Some(ref index_mutex) = self.index_manager {
            let doc_info = DocumentInfo {
                document_id: id.to_string(),
                schema_id: self.default_schema_id.clone(),
                schema_version: self.default_schema_version.clone(),
                is_tombstone: false,
                body: updated.clone(),
                offset,
            };
            let mut index = index_mutex.lock().map_err(|e| e.to_string())?;
            index.apply_write(&doc_info);
        }

        // 4. Cache update
        {
            let mut cache = self.cache.write().map_err(|e| e.to_string())?;
            cache
                .entry(collection.to_string())
                .or_default()
                .insert(id.to_string(), updated.clone());
        }

        Ok(updated)
    }

    fn delete(&self, collection: &str, id: &str) -> Result<bool, String> {
        // Check if exists
        let exists = self.read(collection, id)?.is_some();
        if !exists {
            return Ok(false);
        }

        // 1. WAL append tombstone
        let wal_payload = WalPayload::tombstone(
            collection,
            id,
            &self.default_schema_id,
            &self.default_schema_version,
        );

        {
            let mut wal = self.wal_writer.lock().map_err(|e| e.to_string())?;
            wal.append(RecordType::Delete, wal_payload)
                .map_err(|e| format!("WAL append failed: {}", e))?;
        }

        // 2. Storage write tombstone
        let offset = {
            let mut storage = self.storage_writer.lock().map_err(|e| e.to_string())?;
            storage
                .write_tombstone(
                    collection,
                    id,
                    &self.default_schema_id,
                    &self.default_schema_version,
                )
                .map_err(|e| format!("Storage write failed: {}", e))?
        };

        // 3. Index update
        if let Some(ref index_mutex) = self.index_manager {
            let doc_info = DocumentInfo {
                document_id: id.to_string(),
                schema_id: self.default_schema_id.clone(),
                schema_version: self.default_schema_version.clone(),
                is_tombstone: true,
                body: serde_json::json!({}),
                offset,
            };
            let mut index = index_mutex.lock().map_err(|e| e.to_string())?;
            index.apply_write(&doc_info);
        }

        // 4. Cache remove
        {
            let mut cache = self.cache.write().map_err(|e| e.to_string())?;
            if let Some(coll) = cache.get_mut(collection) {
                coll.remove(id);
            }
        }

        Ok(true)
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

        // Apply pagination
        Ok(results.into_iter().skip(offset).take(limit).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_backend(temp_dir: &TempDir) -> WriteThroughBackend {
        let wal_writer = WalWriter::open(temp_dir.path()).unwrap();
        let storage_writer = StorageWriter::open(temp_dir.path()).unwrap();
        WriteThroughBackend::new("users", wal_writer, storage_writer)
    }

    #[test]
    fn test_write_through_write_and_read() {
        let temp_dir = TempDir::new().unwrap();
        let backend = setup_backend(&temp_dir);

        // Write
        let doc = serde_json::json!({"name": "Alice", "age": 30});
        let id = backend.write("users", doc).unwrap();

        // Read
        let result = backend.read("users", &id).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap()["name"], "Alice");
    }

    #[test]
    fn test_write_through_update() {
        let temp_dir = TempDir::new().unwrap();
        let backend = setup_backend(&temp_dir);

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
    fn test_write_through_delete() {
        let temp_dir = TempDir::new().unwrap();
        let backend = setup_backend(&temp_dir);

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
    fn test_write_through_persists_to_storage() {
        let temp_dir = TempDir::new().unwrap();

        // Write with first backend instance
        let doc_id = {
            let backend = setup_backend(&temp_dir);
            let doc = serde_json::json!({"name": "Durable Dan"});
            backend.write("users", doc).unwrap()
        };

        // Create new backend instance and load from storage
        let backend2 = WriteThroughBackend::open(temp_dir.path(), "users").unwrap();
        let result = backend2.read("users", &doc_id).unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap()["name"], "Durable Dan");
    }

    #[test]
    fn test_write_through_delete_persists() {
        let temp_dir = TempDir::new().unwrap();

        // Write and delete with first backend
        let doc_id = {
            let backend = setup_backend(&temp_dir);
            let doc = serde_json::json!({"name": "Ephemeral Eve"});
            let id = backend.write("users", doc).unwrap();
            backend.delete("users", &id).unwrap();
            id
        };

        // Verify deletion persists
        let backend2 = WriteThroughBackend::open(temp_dir.path(), "users").unwrap();
        let result = backend2.read("users", &doc_id).unwrap();
        assert!(result.is_none());
    }
}
