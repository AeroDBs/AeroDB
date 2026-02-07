//! Index Manager for aerodb
//!
//! Manages in-memory indexes rebuilt from storage on startup.
//!
//! # API
//!
//! - `rebuild_from_storage(reader)` - Rebuild all indexes
//! - `apply_write(doc, offset)` - Update index after storage write
//! - `apply_delete(doc_id)` - Update index after delete
//! - `lookup_eq(field, value)` - Exact match lookup
//! - `lookup_range(field, min, max, limit)` - Range lookup

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use super::btree::{IndexKey, IndexTree, StorageOffset};
use super::errors::{IndexError, IndexResult};

/// Document info extracted from storage for indexing
#[derive(Debug, Clone)]
pub struct DocumentInfo {
    /// Document ID
    pub document_id: String,
    /// Schema ID
    pub schema_id: String,
    /// Schema version
    pub schema_version: String,
    /// Is tombstone
    pub is_tombstone: bool,
    /// Document body (parsed JSON)
    pub body: Value,
    /// Storage offset
    pub offset: StorageOffset,
}

/// Trait for scanning storage during rebuild
pub trait StorageScan {
    /// Read the next document record for indexing
    /// Returns None if at end
    /// Returns Err if checksum fails (corruption)
    fn scan_next(&mut self) -> IndexResult<Option<DocumentInfo>>;

    /// Reset to beginning of storage
    fn reset(&mut self) -> IndexResult<()>;

    /// Get current offset
    fn current_offset(&self) -> u64;
}

/// Index Manager that maintains in-memory indexes
pub struct IndexManager {
    /// Primary key index (_id -> offset)
    pk_index: IndexTree,

    /// Secondary indexes (field -> IndexTree)
    field_indexes: HashMap<String, IndexTree>,

    /// Indexed field names
    indexed_fields: HashSet<String>,

    /// Document ID to offset mapping (for delete)
    doc_offsets: HashMap<String, StorageOffset>,
}

impl IndexManager {
    /// Creates a new empty index manager
    pub fn new(indexed_fields: HashSet<String>) -> Self {
        let mut field_indexes = HashMap::new();
        for field in &indexed_fields {
            field_indexes.insert(field.clone(), IndexTree::new());
        }

        Self {
            pk_index: IndexTree::new(),
            field_indexes,
            indexed_fields,
            doc_offsets: HashMap::new(),
        }
    }

    /// Create with no secondary indexes (PK only)
    pub fn pk_only() -> Self {
        Self::new(HashSet::new())
    }

    /// Rebuild all indexes from storage.
    ///
    /// Behavior:
    /// - Sequentially scan storage
    /// - Ignore tombstones
    /// - For each live document: extract indexed fields, insert offset
    /// - Deterministic traversal order
    ///
    /// On checksum failure: returns AERO_DATA_CORRUPTION (FATAL)
    pub fn rebuild_from_storage<S: StorageScan>(&mut self, storage: &mut S) -> IndexResult<()> {
        // Clear existing indexes
        self.pk_index.clear();
        for tree in self.field_indexes.values_mut() {
            tree.clear();
        }
        self.doc_offsets.clear();

        // Reset storage to beginning
        storage.reset()?;

        loop {
            let doc = match storage.scan_next() {
                Ok(Some(d)) => d,
                Ok(None) => break, // End of storage
                Err(e) => {
                    // Corruption detected - FATAL
                    return Err(IndexError::data_corruption(
                        storage.current_offset(),
                        e.message(),
                    ));
                }
            };

            // Skip tombstones
            if doc.is_tombstone {
                continue;
            }

            // Index this document
            self.index_document(&doc);
        }

        Ok(())
    }

    /// Index a single document
    fn index_document(&mut self, doc: &DocumentInfo) {
        // Primary key index
        let pk_key = IndexKey::from_string(&doc.document_id);
        self.pk_index.insert(pk_key, doc.offset);

        // Track doc -> offset
        self.doc_offsets.insert(doc.document_id.clone(), doc.offset);

        // Secondary indexes
        for field in &self.indexed_fields {
            if let Some(value) = doc.body.get(field) {
                if let Some(key) = IndexKey::from_json(value) {
                    if let Some(tree) = self.field_indexes.get_mut(field) {
                        tree.insert(key, doc.offset);
                    }
                }
            }
        }
    }

    /// Remove a document from indexes
    fn unindex_document(&mut self, doc_id: &str, offset: StorageOffset, body: &Value) {
        // Remove from PK index
        let pk_key = IndexKey::from_string(doc_id);
        self.pk_index.remove(&pk_key, offset);

        // Remove from doc_offsets
        self.doc_offsets.remove(doc_id);

        // Remove from secondary indexes
        for field in &self.indexed_fields {
            if let Some(value) = body.get(field) {
                if let Some(key) = IndexKey::from_json(value) {
                    if let Some(tree) = self.field_indexes.get_mut(field) {
                        tree.remove(&key, offset);
                    }
                }
            }
        }
    }

    /// Apply a write (insert or update) to indexes.
    ///
    /// Called AFTER storage write.
    /// Updates in-memory index only.
    pub fn apply_write(&mut self, doc: &DocumentInfo) {
        // If document already exists, remove old index entry
        if let Some(&old_offset) = self.doc_offsets.get(&doc.document_id) {
            // Note: For proper update, we'd need the old body.
            // In Phase 0, we just overwrite the PK index.
            let pk_key = IndexKey::from_string(&doc.document_id);
            self.pk_index.remove(&pk_key, old_offset);
        }

        // Add new index entry
        self.index_document(doc);
    }

    /// Apply a delete to indexes.
    ///
    /// Called AFTER storage write (tombstone).
    /// Removes document from all indexes.
    pub fn apply_delete(&mut self, doc_id: &str, body: &Value) {
        if let Some(offset) = self.doc_offsets.get(doc_id).copied() {
            self.unindex_document(doc_id, offset, body);
        }
    }

    /// Lookup all offsets for an exact primary key match.
    ///
    /// Returns offsets sorted ascending.
    pub fn lookup_pk(&self, pk: &str) -> Vec<StorageOffset> {
        let key = IndexKey::from_string(pk);
        self.pk_index.lookup_eq(&key)
    }

    /// Lookup all offsets for an exact field match.
    ///
    /// Returns offsets sorted ascending.
    pub fn lookup_eq(&self, field: &str, value: &Value) -> Vec<StorageOffset> {
        if field == "_id" {
            if let Some(s) = value.as_str() {
                return self.lookup_pk(s);
            }
            return Vec::new();
        }

        let Some(tree) = self.field_indexes.get(field) else {
            return Vec::new();
        };

        let Some(key) = IndexKey::from_json(value) else {
            return Vec::new();
        };

        tree.lookup_eq(&key)
    }

    /// Lookup offsets in a range.
    ///
    /// Returns offsets sorted ascending.
    /// Limit is applied after collecting offsets.
    pub fn lookup_range(
        &self,
        field: &str,
        min: Option<&Value>,
        max: Option<&Value>,
        limit: Option<usize>,
    ) -> Vec<StorageOffset> {
        let Some(tree) = self.field_indexes.get(field) else {
            return Vec::new();
        };

        let min_key = min.and_then(IndexKey::from_json);
        let max_key = max.and_then(IndexKey::from_json);

        let mut offsets = tree.lookup_range(min_key.as_ref(), max_key.as_ref());

        if let Some(lim) = limit {
            offsets.truncate(lim);
        }

        offsets
    }

    /// Get all offsets in primary key order.
    ///
    /// Returns offsets sorted ascending.
    pub fn all_offsets_pk_order(&self) -> Vec<StorageOffset> {
        self.pk_index.lookup_range(None, None)
    }

    /// Returns the set of indexed fields
    pub fn indexed_fields(&self) -> &HashSet<String> {
        &self.indexed_fields
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct MockStorage {
        documents: Vec<DocumentInfo>,
        position: usize,
        corrupt_at: Option<usize>,
    }

    impl MockStorage {
        fn new(documents: Vec<DocumentInfo>) -> Self {
            Self {
                documents,
                position: 0,
                corrupt_at: None,
            }
        }

        fn with_corruption_at(mut self, index: usize) -> Self {
            self.corrupt_at = Some(index);
            self
        }
    }

    impl StorageScan for MockStorage {
        fn scan_next(&mut self) -> IndexResult<Option<DocumentInfo>> {
            if self.position >= self.documents.len() {
                return Ok(None);
            }

            if self.corrupt_at == Some(self.position) {
                return Err(IndexError::data_corruption(
                    self.documents[self.position].offset,
                    "checksum mismatch",
                ));
            }

            let doc = self.documents[self.position].clone();
            self.position += 1;
            Ok(Some(doc))
        }

        fn reset(&mut self) -> IndexResult<()> {
            self.position = 0;
            Ok(())
        }

        fn current_offset(&self) -> u64 {
            if self.position < self.documents.len() {
                self.documents[self.position].offset
            } else {
                0
            }
        }
    }

    fn make_doc(id: &str, age: i64, offset: u64) -> DocumentInfo {
        DocumentInfo {
            document_id: id.to_string(),
            schema_id: "users".to_string(),
            schema_version: "v1".to_string(),
            is_tombstone: false,
            body: json!({"_id": id, "age": age, "name": format!("User_{}", id)}),
            offset,
        }
    }

    fn make_tombstone(id: &str, offset: u64) -> DocumentInfo {
        DocumentInfo {
            document_id: id.to_string(),
            schema_id: "".to_string(),
            schema_version: "".to_string(),
            is_tombstone: true,
            body: json!({}),
            offset,
        }
    }

    #[test]
    fn test_rebuild_from_storage() {
        let docs = vec![
            make_doc("user_1", 25, 100),
            make_doc("user_2", 30, 200),
            make_doc("user_3", 25, 300),
        ];

        let mut storage = MockStorage::new(docs);
        let mut indexed = HashSet::new();
        indexed.insert("age".to_string());

        let mut manager = IndexManager::new(indexed);
        manager.rebuild_from_storage(&mut storage).unwrap();

        // Check PK index
        assert_eq!(manager.lookup_pk("user_1"), vec![100]);
        assert_eq!(manager.lookup_pk("user_2"), vec![200]);

        // Check secondary index
        let age_25 = manager.lookup_eq("age", &json!(25));
        assert_eq!(age_25, vec![100, 300]);
    }

    #[test]
    fn test_delete_removes_index_entry() {
        let mut manager = IndexManager::pk_only();

        let doc = make_doc("user_1", 25, 100);
        manager.apply_write(&doc);

        assert_eq!(manager.lookup_pk("user_1"), vec![100]);

        manager.apply_delete("user_1", &doc.body);

        assert!(manager.lookup_pk("user_1").is_empty());
    }

    #[test]
    fn test_overwrite_updates_index() {
        let mut manager = IndexManager::pk_only();

        // Insert at offset 100
        let doc1 = make_doc("user_1", 25, 100);
        manager.apply_write(&doc1);
        assert_eq!(manager.lookup_pk("user_1"), vec![100]);

        // Overwrite at offset 200
        let doc2 = make_doc("user_1", 30, 200);
        manager.apply_write(&doc2);
        assert_eq!(manager.lookup_pk("user_1"), vec![200]);
    }

    #[test]
    fn test_lookup_eq_deterministic() {
        let docs = vec![
            make_doc("user_3", 25, 300),
            make_doc("user_1", 25, 100),
            make_doc("user_2", 25, 200),
        ];

        let mut storage = MockStorage::new(docs);
        let mut indexed = HashSet::new();
        indexed.insert("age".to_string());

        let mut manager = IndexManager::new(indexed);
        manager.rebuild_from_storage(&mut storage).unwrap();

        // Same query twice should return same order
        let result1 = manager.lookup_eq("age", &json!(25));
        let result2 = manager.lookup_eq("age", &json!(25));

        assert_eq!(result1, result2);
        assert_eq!(result1, vec![100, 200, 300]); // Sorted ascending
    }

    #[test]
    fn test_lookup_range_deterministic() {
        let docs = vec![
            make_doc("user_1", 20, 100),
            make_doc("user_2", 25, 200),
            make_doc("user_3", 30, 300),
            make_doc("user_4", 35, 400),
        ];

        let mut storage = MockStorage::new(docs);
        let mut indexed = HashSet::new();
        indexed.insert("age".to_string());

        let mut manager = IndexManager::new(indexed);
        manager.rebuild_from_storage(&mut storage).unwrap();

        let result = manager.lookup_range("age", Some(&json!(25)), Some(&json!(35)), None);
        assert_eq!(result, vec![200, 300, 400]);

        // With limit
        let result_limited = manager.lookup_range("age", Some(&json!(25)), None, Some(2));
        assert_eq!(result_limited, vec![200, 300]);
    }

    #[test]
    fn test_corruption_during_rebuild_halts() {
        let docs = vec![make_doc("user_1", 25, 100), make_doc("user_2", 30, 200)];

        let mut storage = MockStorage::new(docs).with_corruption_at(1);
        let mut manager = IndexManager::pk_only();

        let result = manager.rebuild_from_storage(&mut storage);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code().code(), "AERO_DATA_CORRUPTION");
    }

    #[test]
    fn test_tombstones_ignored() {
        let docs = vec![
            make_doc("user_1", 25, 100),
            make_tombstone("user_2", 200),
            make_doc("user_3", 30, 300),
        ];

        let mut storage = MockStorage::new(docs);
        let mut manager = IndexManager::pk_only();
        manager.rebuild_from_storage(&mut storage).unwrap();

        // user_2 should not be in index
        assert!(manager.lookup_pk("user_2").is_empty());
        assert_eq!(manager.lookup_pk("user_1"), vec![100]);
        assert_eq!(manager.lookup_pk("user_3"), vec![300]);
    }
}
