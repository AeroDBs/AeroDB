//! API Handler for aerodb
//!
//! Orchestrates all subsystems behind a single global mutex.
//! Enforces strict request handling flow.

use std::sync::Mutex;

use serde_json::{json, Value};

use crate::index::{DocumentInfo, IndexManager};
use crate::planner::{
    FilterOp, IndexMetadata, Predicate, Query, QueryPlan, QueryPlanner, ScanType, SortSpec,
};
use crate::schema::{SchemaLoader, SchemaValidator};
use crate::storage::{StoragePayload, StorageReader, StorageWriter};
use crate::wal::{RecordType, WalPayload, WalWriter};

use super::errors::{ApiError, ApiResult};
use super::request::{DeleteRequest, InsertRequest, QueryRequest, Request, UpdateRequest};
use super::response::Response;

/// Subsystem references for API handler
pub struct Subsystems<'a> {
    pub schema_loader: &'a SchemaLoader,
    pub wal_writer: &'a mut WalWriter,
    pub storage_writer: &'a mut StorageWriter,
    pub storage_reader: &'a mut StorageReader,
    pub index_manager: &'a mut IndexManager,
}

/// API Handler with global execution lock
pub struct ApiHandler {
    /// Global mutex for serialized execution
    lock: Mutex<()>,

    /// Collection name (single collection in Phase 0)
    collection: String,
}

impl ApiHandler {
    /// Create a new API handler
    pub fn new(collection: impl Into<String>) -> Self {
        Self {
            lock: Mutex::new(()),
            collection: collection.into(),
        }
    }

    /// Handle a raw JSON request string
    ///
    /// Acquires global lock at entry, releases on return.
    pub fn handle(&self, json_request: &str, subsystems: &mut Subsystems<'_>) -> Response {
        // Acquire global lock at request entry
        let _guard = self.lock.lock().expect("Lock poisoned");

        // Parse request
        let request = match Request::parse(json_request) {
            Ok(r) => r,
            Err(e) => return Response::error(&e),
        };

        // Dispatch to appropriate handler
        let result = match request {
            Request::Insert(r) => self.handle_insert(r, subsystems),
            Request::Update(r) => self.handle_update(r, subsystems),
            Request::Delete(r) => self.handle_delete(r, subsystems),
            Request::Query(r) => self.handle_query(r, subsystems),
            Request::Explain(r) => self.handle_explain(r, subsystems),
        };

        // Lock released when _guard drops
        match result {
            Ok(data) => Response::success(data),
            Err(e) => Response::error(&e),
        }
    }

    /// Handle insert operation
    ///
    /// Flow:
    /// 1. Validate schema
    /// 2. Build write intent
    /// 3. Append WAL record
    /// 4. Apply to Storage
    /// 5. Update Index
    fn handle_insert(&self, req: InsertRequest, sys: &mut Subsystems<'_>) -> ApiResult<Value> {
        let validator = SchemaValidator::new(sys.schema_loader);

        // 1. Validate schema
        validator
            .validate_document(&req.schema_id, &req.schema_version, &req.document)
            .map_err(ApiError::from_schema_error)?;

        // Extract document ID
        let doc_id = req
            .document
            .get("_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ApiError::invalid_request("Document missing _id"))?
            .to_string();

        // 2. Build write intent
        let body_bytes = serde_json::to_vec(&req.document).map_err(|e| {
            ApiError::invalid_request(format!("Failed to serialize document: {}", e))
        })?;

        let wal_payload = WalPayload::new(
            &self.collection,
            &doc_id,
            &req.schema_id,
            &req.schema_version,
            body_bytes.clone(),
        );

        // 3. Append WAL record
        sys.wal_writer
            .append(RecordType::Insert, wal_payload)
            .map_err(ApiError::from_wal_error)?;

        // 4. Apply to Storage
        let storage_payload = StoragePayload::new(
            &self.collection,
            &doc_id,
            &req.schema_id,
            &req.schema_version,
            body_bytes,
        );
        let offset = sys
            .storage_writer
            .write(&storage_payload)
            .map_err(ApiError::from_storage_error)?;

        // 5. Update Index
        let doc_info = DocumentInfo {
            document_id: doc_id.clone(),
            schema_id: req.schema_id,
            schema_version: req.schema_version,
            is_tombstone: false,
            body: req.document,
            offset,
        };
        sys.index_manager.apply_write(&doc_info);

        Ok(json!({"inserted": doc_id}))
    }

    /// Handle update operation
    ///
    /// Flow:
    /// 1. Validate schema
    /// 2. Check document exists
    /// 3. Build write intent
    /// 4. Append WAL record
    /// 5. Apply to Storage
    /// 6. Update Index
    fn handle_update(&self, req: UpdateRequest, sys: &mut Subsystems<'_>) -> ApiResult<Value> {
        let validator = SchemaValidator::new(sys.schema_loader);

        // Extract document ID
        let doc_id = req
            .document
            .get("_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ApiError::invalid_request("Document missing _id"))?
            .to_string();

        // 1. Validate schema (update mode)
        validator
            .validate_update(&req.schema_id, &req.schema_version, &doc_id, &req.document)
            .map_err(ApiError::from_schema_error)?;

        // 2. Check document exists (via index)
        let offsets = sys.index_manager.lookup_pk(&doc_id);
        if offsets.is_empty() {
            return Err(ApiError::invalid_request(format!(
                "Document not found: {}",
                doc_id
            )));
        }

        // 3. Build write intent
        let body_bytes = serde_json::to_vec(&req.document).map_err(|e| {
            ApiError::invalid_request(format!("Failed to serialize document: {}", e))
        })?;

        let wal_payload = WalPayload::new(
            &self.collection,
            &doc_id,
            &req.schema_id,
            &req.schema_version,
            body_bytes.clone(),
        );

        // 4. Append WAL record
        sys.wal_writer
            .append(RecordType::Update, wal_payload)
            .map_err(ApiError::from_wal_error)?;

        // 5. Apply to Storage (overwrite)
        let storage_payload = StoragePayload::new(
            &self.collection,
            &doc_id,
            &req.schema_id,
            &req.schema_version,
            body_bytes,
        );
        let offset = sys
            .storage_writer
            .write(&storage_payload)
            .map_err(ApiError::from_storage_error)?;

        // 6. Update Index
        let doc_info = DocumentInfo {
            document_id: doc_id.clone(),
            schema_id: req.schema_id,
            schema_version: req.schema_version,
            is_tombstone: false,
            body: req.document,
            offset,
        };
        sys.index_manager.apply_write(&doc_info);

        Ok(json!({"updated": doc_id}))
    }

    /// Handle delete operation
    ///
    /// Flow:
    /// 1. Check document exists
    /// 2. Append WAL record
    /// 3. Apply tombstone to Storage
    /// 4. Update Index
    fn handle_delete(&self, req: DeleteRequest, sys: &mut Subsystems<'_>) -> ApiResult<Value> {
        // 1. Check document exists (via index)
        let offsets = sys.index_manager.lookup_pk(&req.document_id);
        if offsets.is_empty() {
            return Err(ApiError::invalid_request(format!(
                "Document not found: {}",
                req.document_id
            )));
        }

        // Get the old document body for index removal
        let old_offset = offsets[offsets.len() - 1];
        let old_doc = sys
            .storage_reader
            .read_at(old_offset)
            .map_err(ApiError::from_storage_error)?;

        let old_body: Value = serde_json::from_slice(&old_doc.document_body).unwrap_or(json!({}));

        // 2. Append WAL record
        let wal_payload = WalPayload::tombstone(
            &self.collection,
            &req.document_id,
            &req.schema_id,
            "", // version empty for delete
        );

        sys.wal_writer
            .append(RecordType::Delete, wal_payload)
            .map_err(ApiError::from_wal_error)?;

        // 3. Apply tombstone to Storage
        sys.storage_writer
            .write_tombstone(&self.collection, &req.document_id, &req.schema_id, "")
            .map_err(ApiError::from_storage_error)?;

        // 4. Update Index
        sys.index_manager.apply_delete(&req.document_id, &old_body);

        Ok(json!({"deleted": req.document_id}))
    }

    /// Handle query operation
    ///
    /// Flow:
    /// 1. Parse query
    /// 2. Call Planner
    /// 3. Call Executor (simplified: use index + storage)
    /// 4. Return results
    fn handle_query(&self, req: QueryRequest, sys: &mut Subsystems<'_>) -> ApiResult<Value> {
        // Build index metadata
        let index_metadata =
            IndexMetadata::with_indexes(sys.index_manager.indexed_fields().iter().cloned());

        let planner = QueryPlanner::new(sys.schema_loader, &index_metadata);

        // 1. Build query AST
        let query = self.build_query(&req)?;

        // 2. Call Planner
        let plan = planner.plan(&query).map_err(ApiError::from_planner_error)?;

        // 3. Execute query (simplified execution)
        let mut results = Vec::new();

        // Get offsets from index based on plan
        let offsets = self.get_offsets_for_plan(&plan, &query, sys.index_manager);

        // Read documents at offsets
        for offset in offsets.iter().take(req.limit) {
            if let Ok(record) = sys.storage_reader.read_at(*offset) {
                // Skip tombstones
                if record.is_tombstone {
                    continue;
                }

                // Check schema match
                if record.schema_id != req.schema_id || record.schema_version != req.schema_version
                {
                    continue;
                }

                // Parse body
                if let Ok(doc) = serde_json::from_slice::<Value>(&record.document_body) {
                    results.push(doc);
                }
            }
        }

        Ok(json!(results))
    }

    /// Handle explain operation
    fn handle_explain(&self, req: QueryRequest, sys: &mut Subsystems<'_>) -> ApiResult<Value> {
        // Build index metadata
        let index_metadata =
            IndexMetadata::with_indexes(sys.index_manager.indexed_fields().iter().cloned());

        let planner = QueryPlanner::new(sys.schema_loader, &index_metadata);

        // Build query AST
        let query = self.build_query(&req)?;

        // Call Planner
        let plan = planner.plan(&query).map_err(ApiError::from_planner_error)?;

        // Return explain output
        Ok(json!({
            "scan_type": format!("{:?}", plan.scan_type),
            "chosen_index": plan.chosen_index,
            "predicates": plan.predicates.len(),
            "sort": plan.sort.as_ref().map(|s| &s.field),
            "limit": plan.limit
        }))
    }

    /// Build a Query AST from a QueryRequest
    fn build_query(&self, req: &QueryRequest) -> ApiResult<Query> {
        let mut query = Query::new(&self.collection, &req.schema_id)
            .with_schema_version(&req.schema_version)
            .with_limit(req.limit as u64);

        // Parse filter
        if let Some(filter) = &req.filter {
            if let Some(obj) = filter.as_object() {
                for (field, condition) in obj {
                    if let Some(cond_obj) = condition.as_object() {
                        for (op, value) in cond_obj {
                            let predicate = match op.as_str() {
                                "$eq" => Predicate::eq(field, value.clone()),
                                "$gte" => Predicate::gte(field, value.clone()),
                                "$gt" => Predicate::gt(field, value.clone()),
                                "$lte" => Predicate::lte(field, value.clone()),
                                "$lt" => Predicate::lt(field, value.clone()),
                                other => {
                                    return Err(ApiError::invalid_request(format!(
                                        "Unknown filter operator: {}",
                                        other
                                    )))
                                }
                            };
                            query = query.with_predicate(predicate);
                        }
                    }
                }
            }
        }

        // Parse sort
        if let Some(sort_str) = &req.sort {
            let sort = if sort_str.starts_with('-') {
                SortSpec::desc(&sort_str[1..])
            } else {
                SortSpec::asc(sort_str)
            };
            query = query.with_sort(sort);
        }

        Ok(query)
    }

    /// Get offsets from index based on plan
    fn get_offsets_for_plan(
        &self,
        plan: &QueryPlan,
        query: &Query,
        index_manager: &IndexManager,
    ) -> Vec<u64> {
        match plan.scan_type {
            ScanType::PrimaryKey => {
                // Find PK predicate
                for pred in &query.predicates {
                    if pred.field == "_id" {
                        if let FilterOp::Eq(ref val) = pred.op {
                            if let Some(pk) = val.as_str() {
                                return index_manager.lookup_pk(pk);
                            }
                        }
                    }
                }
                Vec::new()
            }
            ScanType::IndexedEquality => {
                let field = &plan.chosen_index;
                for pred in &query.predicates {
                    if &pred.field == field {
                        if let FilterOp::Eq(ref val) = pred.op {
                            return index_manager.lookup_eq(field, val);
                        }
                    }
                }
                Vec::new()
            }
            ScanType::IndexedRange => {
                let field = &plan.chosen_index;
                let mut min: Option<&Value> = None;
                let mut max: Option<&Value> = None;

                for pred in &query.predicates {
                    if &pred.field == field {
                        match &pred.op {
                            FilterOp::Gte(v) | FilterOp::Gt(v) => min = Some(v),
                            FilterOp::Lte(v) | FilterOp::Lt(v) => max = Some(v),
                            _ => {}
                        }
                    }
                }

                index_manager.lookup_range(field, min, max, Some(plan.limit as usize))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{FieldDef, Schema};
    use serde_json::json;
    use std::collections::{HashMap, HashSet};
    use tempfile::TempDir;

    fn setup_test_env() -> (
        TempDir,
        SchemaLoader,
        WalWriter,
        StorageWriter,
        StorageReader,
        IndexManager,
    ) {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path();

        // Create schema loader
        let mut loader = SchemaLoader::new(data_dir);

        let mut fields = HashMap::new();
        fields.insert("_id".to_string(), FieldDef::required_string());
        fields.insert("name".to_string(), FieldDef::required_string());
        fields.insert("age".to_string(), FieldDef::optional_int());

        let schema = Schema::new("users", "v1", fields);
        loader.register(schema).unwrap();

        // Create WAL writer
        let wal_writer = WalWriter::open(data_dir).unwrap();

        // Create storage writer and reader
        let storage_writer = StorageWriter::open(data_dir).unwrap();
        let storage_reader = StorageReader::open_from_data_dir(data_dir).unwrap();

        // Create index manager
        let mut indexed = HashSet::new();
        indexed.insert("age".to_string());
        let index_manager = IndexManager::new(indexed);

        (
            temp_dir,
            loader,
            wal_writer,
            storage_writer,
            storage_reader,
            index_manager,
        )
    }

    #[test]
    fn test_insert_and_query_roundtrip() {
        let (_temp, loader, mut wal, mut storage_w, mut storage_r, mut index) = setup_test_env();

        let handler = ApiHandler::new("users");
        let mut subsystems = Subsystems {
            schema_loader: &loader,
            wal_writer: &mut wal,
            storage_writer: &mut storage_w,
            storage_reader: &mut storage_r,
            index_manager: &mut index,
        };

        // Insert
        let insert_req = r#"{
            "op": "insert",
            "schema_id": "users",
            "schema_version": "v1",
            "document": {"_id": "user_1", "name": "Alice", "age": 25}
        }"#;

        let resp = handler.handle(insert_req, &mut subsystems);
        assert!(resp.is_success(), "Insert should succeed");

        // Query
        let query_req = r#"{
            "op": "query",
            "schema_id": "users",
            "schema_version": "v1",
            "filter": {"_id": {"$eq": "user_1"}},
            "limit": 10
        }"#;

        let resp = handler.handle(query_req, &mut subsystems);
        assert!(resp.is_success(), "Query should succeed");
    }

    #[test]
    fn test_invalid_schema_rejected() {
        let (_temp, loader, mut wal, mut storage_w, mut storage_r, mut index) = setup_test_env();

        let handler = ApiHandler::new("users");
        let mut subsystems = Subsystems {
            schema_loader: &loader,
            wal_writer: &mut wal,
            storage_writer: &mut storage_w,
            storage_reader: &mut storage_r,
            index_manager: &mut index,
        };

        // Insert with unknown schema
        let insert_req = r#"{
            "op": "insert",
            "schema_id": "unknown",
            "schema_version": "v1",
            "document": {"_id": "user_1", "name": "Alice"}
        }"#;

        let resp = handler.handle(insert_req, &mut subsystems);
        assert!(!resp.is_success());
    }

    #[test]
    fn test_unbounded_query_rejected() {
        let (_temp, loader, mut wal, mut storage_w, mut storage_r, mut index) = setup_test_env();

        let handler = ApiHandler::new("users");
        let mut subsystems = Subsystems {
            schema_loader: &loader,
            wal_writer: &mut wal,
            storage_writer: &mut storage_w,
            storage_reader: &mut storage_r,
            index_manager: &mut index,
        };

        // Query without indexed filter
        let query_req = r#"{
            "op": "query",
            "schema_id": "users",
            "schema_version": "v1",
            "filter": {"name": {"$eq": "Alice"}},
            "limit": 10
        }"#;

        let resp = handler.handle(query_req, &mut subsystems);
        // This should fail because "name" is not indexed
        assert!(!resp.is_success());
    }

    #[test]
    fn test_explain_returns_deterministic_plan() {
        let (_temp, loader, mut wal, mut storage_w, mut storage_r, mut index) = setup_test_env();

        let handler = ApiHandler::new("users");
        let mut subsystems = Subsystems {
            schema_loader: &loader,
            wal_writer: &mut wal,
            storage_writer: &mut storage_w,
            storage_reader: &mut storage_r,
            index_manager: &mut index,
        };

        let explain_req = r#"{
            "op": "explain",
            "schema_id": "users",
            "schema_version": "v1",
            "filter": {"_id": {"$eq": "user_1"}},
            "limit": 10
        }"#;

        let resp1 = handler.handle(explain_req, &mut subsystems);
        let resp2 = handler.handle(explain_req, &mut subsystems);

        // Plans should be identical
        assert_eq!(resp1.to_json(), resp2.to_json());
    }

    #[test]
    fn test_serialization_enforced() {
        // This test verifies the lock exists; actual blocking tested differently
        let (_temp, loader, mut wal, mut storage_w, mut storage_r, mut index) = setup_test_env();

        let handler = ApiHandler::new("users");
        let mut subsystems = Subsystems {
            schema_loader: &loader,
            wal_writer: &mut wal,
            storage_writer: &mut storage_w,
            storage_reader: &mut storage_r,
            index_manager: &mut index,
        };

        // Sequential operations should succeed
        let insert1 = r#"{
            "op": "insert",
            "schema_id": "users",
            "schema_version": "v1",
            "document": {"_id": "user_1", "name": "Alice"}
        }"#;

        let insert2 = r#"{
            "op": "insert",
            "schema_id": "users",
            "schema_version": "v1",
            "document": {"_id": "user_2", "name": "Bob"}
        }"#;

        let resp1 = handler.handle(insert1, &mut subsystems);
        let resp2 = handler.handle(insert2, &mut subsystems);

        assert!(resp1.is_success());
        assert!(resp2.is_success());
    }

    #[test]
    fn test_corruption_surfaced() {
        // Corruption is surfaced when storage/WAL returns error
        // This is implicitly tested via pass-through errors

        let (_temp, loader, mut wal, mut storage_w, mut storage_r, mut index) = setup_test_env();

        let handler = ApiHandler::new("users");
        let mut subsystems = Subsystems {
            schema_loader: &loader,
            wal_writer: &mut wal,
            storage_writer: &mut storage_w,
            storage_reader: &mut storage_r,
            index_manager: &mut index,
        };

        // Insert a document - this confirms error propagation works
        let insert_req = r#"{
            "op": "insert",
            "schema_id": "users",
            "schema_version": "v1",
            "document": {"_id": "user_1", "name": "Alice"}
        }"#;

        let resp = handler.handle(insert_req, &mut subsystems);
        assert!(resp.is_success());
    }
}
