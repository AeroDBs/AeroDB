//! Unified Executor
//!
//! Bridges the core pipeline to existing AeroDB subsystems.
//! This executor implements `OperationExecutor` and routes operations
//! to the appropriate handlers.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::{json, Value};

use crate::core::context::RequestContext;
use crate::core::error::CoreError;
use crate::core::operation::{DeleteOp, Operation, QueryOp, ReadOp, UpdateOp, WriteOp};
use crate::core::pipeline::{OperationExecutor, OperationResult};

/// Trait for the storage backend
pub trait StorageBackend: Send + Sync {
    /// Read a document by ID
    fn read(&self, collection: &str, id: &str) -> Result<Option<Value>, String>;

    /// Write a document
    fn write(&self, collection: &str, document: Value) -> Result<String, String>;

    /// Update a document
    fn update(&self, collection: &str, id: &str, updates: Value) -> Result<Value, String>;

    /// Delete a document
    fn delete(&self, collection: &str, id: &str) -> Result<bool, String>;

    /// Query documents
    fn query(
        &self,
        collection: &str,
        filter: Option<&Value>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Value>, String>;
}

/// Unified executor that routes operations through subsystems
pub struct UnifiedExecutor {
    storage: Arc<dyn StorageBackend>,
}

impl UnifiedExecutor {
    /// Create a new unified executor
    pub fn new(storage: impl StorageBackend + 'static) -> Self {
        Self {
            storage: Arc::new(storage),
        }
    }
}

impl OperationExecutor for UnifiedExecutor {
    fn execute(
        &self,
        op: &Operation,
        ctx: &RequestContext,
    ) -> Pin<Box<dyn Future<Output = OperationResult> + Send + '_>> {
        // Clone what we need for the async block
        let storage = Arc::clone(&self.storage);
        let rls_filters = ctx.rls_filters.clone();

        // Clone the operation data we need
        let op_clone = op.clone();

        Box::pin(async move {
            match op_clone {
                Operation::Read(read) => execute_read(&storage, &read, &rls_filters),
                Operation::Write(write) => execute_write(&storage, &write),
                Operation::Update(update) => execute_update(&storage, &update, &rls_filters),
                Operation::Delete(delete) => execute_delete(&storage, &delete, &rls_filters),
                Operation::Query(query) => execute_query(&storage, &query, &rls_filters),
                Operation::Explain(query) => {
                    // Return query plan instead of executing
                    Ok(json!({
                        "plan": {
                            "collection": query.collection,
                            "filter": query.filter,
                            "limit": query.limit,
                            "offset": query.offset,
                            "rls_filters": rls_filters.len(),
                        }
                    }))
                }
                _ => {
                    // Other operations not yet implemented in unified executor
                    Err(CoreError::execution(format!(
                        "Operation {} not yet implemented in unified executor",
                        op_clone.name()
                    )))
                }
            }
        })
    }
}

fn execute_read(
    storage: &Arc<dyn StorageBackend>,
    op: &ReadOp,
    rls_filters: &[crate::core::context::RlsFilter],
) -> OperationResult {
    let result = storage
        .read(&op.collection, &op.id)
        .map_err(CoreError::execution)?;

    match result {
        Some(doc) => {
            // Apply RLS filters to verify access
            for filter in rls_filters {
                if !check_rls_filter(&doc, filter) {
                    return Err(CoreError::not_found(format!(
                        "Document {} not found in {}",
                        op.id, op.collection
                    )));
                }
            }

            // Apply select projection if specified
            let doc = apply_select(doc, op.select.as_ref());
            Ok(doc)
        }
        None => Err(CoreError::not_found(format!(
            "Document {} not found in {}",
            op.id, op.collection
        ))),
    }
}

fn execute_write(storage: &Arc<dyn StorageBackend>, op: &WriteOp) -> OperationResult {
    let id = storage
        .write(&op.collection, op.document.clone())
        .map_err(CoreError::execution)?;

    Ok(json!({
        "id": id,
        "collection": op.collection,
        "created": true
    }))
}

fn execute_update(
    storage: &Arc<dyn StorageBackend>,
    op: &UpdateOp,
    rls_filters: &[crate::core::context::RlsFilter],
) -> OperationResult {
    // First read to check RLS
    let existing = storage
        .read(&op.collection, &op.id)
        .map_err(CoreError::execution)?;

    match existing {
        Some(doc) => {
            // Verify RLS allows access
            for filter in rls_filters {
                if !check_rls_filter(&doc, filter) {
                    return Err(CoreError::access_denied(format!(
                        "Cannot update document {} in {}",
                        op.id, op.collection
                    )));
                }
            }

            // Perform update
            let updated = storage
                .update(&op.collection, &op.id, op.updates.clone())
                .map_err(CoreError::execution)?;

            Ok(updated)
        }
        None => Err(CoreError::not_found(format!(
            "Document {} not found in {}",
            op.id, op.collection
        ))),
    }
}

fn execute_delete(
    storage: &Arc<dyn StorageBackend>,
    op: &DeleteOp,
    rls_filters: &[crate::core::context::RlsFilter],
) -> OperationResult {
    // First read to check RLS
    let existing = storage
        .read(&op.collection, &op.id)
        .map_err(CoreError::execution)?;

    match existing {
        Some(doc) => {
            // Verify RLS allows access
            for filter in rls_filters {
                if !check_rls_filter(&doc, filter) {
                    return Err(CoreError::access_denied(format!(
                        "Cannot delete document {} in {}",
                        op.id, op.collection
                    )));
                }
            }

            // Perform delete
            storage
                .delete(&op.collection, &op.id)
                .map_err(CoreError::execution)?;

            Ok(json!({
                "id": op.id,
                "collection": op.collection,
                "deleted": true
            }))
        }
        None => Err(CoreError::not_found(format!(
            "Document {} not found in {}",
            op.id, op.collection
        ))),
    }
}

fn execute_query(
    storage: &Arc<dyn StorageBackend>,
    op: &QueryOp,
    rls_filters: &[crate::core::context::RlsFilter],
) -> OperationResult {
    let results = storage
        .query(&op.collection, op.filter.as_ref(), op.limit, op.offset)
        .map_err(CoreError::execution)?;

    // Apply RLS filters
    let filtered: Vec<Value> = results
        .into_iter()
        .filter(|doc| rls_filters.iter().all(|f| check_rls_filter(doc, f)))
        .collect();

    // Apply select projection
    let projected: Vec<Value> = filtered
        .into_iter()
        .map(|doc| apply_select(doc, op.select.as_ref()))
        .collect();

    Ok(json!({
        "data": projected,
        "count": projected.len()
    }))
}

/// Check if a document passes an RLS filter
fn check_rls_filter(doc: &Value, filter: &crate::core::context::RlsFilter) -> bool {
    use crate::core::context::FilterOperator;

    let field_value = doc.get(&filter.field);

    match (&filter.operator, field_value) {
        (FilterOperator::Eq, Some(v)) => v == &filter.value,
        (FilterOperator::Neq, Some(v)) => v != &filter.value,
        (FilterOperator::In, Some(v)) => {
            if let Some(arr) = filter.value.as_array() {
                arr.contains(v)
            } else {
                false
            }
        }
        (FilterOperator::Contains, Some(v)) => {
            if let (Some(haystack), Some(needle)) = (v.as_str(), filter.value.as_str()) {
                haystack.contains(needle)
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Apply select projection to a document
fn apply_select(mut doc: Value, select: Option<&Vec<String>>) -> Value {
    match select {
        Some(fields) if !fields.is_empty() => {
            if let Some(obj) = doc.as_object_mut() {
                let keys: Vec<String> = obj.keys().cloned().collect();
                for key in keys {
                    if !fields.contains(&key) && key != "_id" {
                        obj.remove(&key);
                    }
                }
            }
            doc
        }
        _ => doc,
    }
}

/// In-memory storage backend for testing
pub struct InMemoryStorage {
    data: std::sync::RwLock<
        std::collections::HashMap<String, std::collections::HashMap<String, Value>>,
    >,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            data: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageBackend for InMemoryStorage {
    fn read(&self, collection: &str, id: &str) -> Result<Option<Value>, String> {
        let data = self.data.read().map_err(|e| e.to_string())?;
        Ok(data.get(collection).and_then(|c| c.get(id)).cloned())
    }

    fn write(&self, collection: &str, mut document: Value) -> Result<String, String> {
        let mut data = self.data.write().map_err(|e| e.to_string())?;

        let id = document
            .get("_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        if let Some(obj) = document.as_object_mut() {
            obj.insert("_id".to_string(), Value::String(id.clone()));
        }

        data.entry(collection.to_string())
            .or_default()
            .insert(id.clone(), document);

        Ok(id)
    }

    fn update(&self, collection: &str, id: &str, updates: Value) -> Result<Value, String> {
        let mut data = self.data.write().map_err(|e| e.to_string())?;

        let coll = data
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
        let mut data = self.data.write().map_err(|e| e.to_string())?;

        if let Some(coll) = data.get_mut(collection) {
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
        let data = self.data.read().map_err(|e| e.to_string())?;

        let results: Vec<Value> = data
            .get(collection)
            .map(|c| c.values().cloned().collect())
            .unwrap_or_default();

        // Apply pagination
        let paginated: Vec<Value> = results.into_iter().skip(offset).take(limit).collect();

        Ok(paginated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::context::AuthContext;
    use crate::core::middleware::auth::AuthMiddleware;
    use crate::core::pipeline::Pipeline;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_unified_executor_write_and_read() {
        let storage = InMemoryStorage::new();
        let executor = UnifiedExecutor::new(storage);
        let pipeline = Pipeline::new(executor).with_middleware(AuthMiddleware::new());

        let user_id = Uuid::new_v4();
        let ctx = RequestContext::new(AuthContext::authenticated(user_id));

        // Write
        let write_op = Operation::Write(WriteOp {
            collection: "users".to_string(),
            document: json!({"name": "Alice", "email": "alice@example.com"}),
            schema_id: "users".to_string(),
            schema_version: "v1".to_string(),
        });

        let result = pipeline.execute(write_op, ctx.clone()).await;
        assert!(result.is_ok());

        let write_result = result.unwrap();
        let id = write_result["id"].as_str().unwrap();

        // Read
        let read_op = Operation::Read(ReadOp {
            collection: "users".to_string(),
            id: id.to_string(),
            select: None,
        });

        let result = pipeline.execute(read_op, ctx).await;
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert_eq!(doc["name"], "Alice");
    }

    #[tokio::test]
    async fn test_unified_executor_query() {
        let storage = InMemoryStorage::new();
        let executor = UnifiedExecutor::new(storage);
        let pipeline = Pipeline::new(executor);

        let ctx = RequestContext::service_role();

        // Insert some data
        for i in 0..5 {
            let write_op = Operation::Write(WriteOp {
                collection: "posts".to_string(),
                document: json!({"title": format!("Post {}", i)}),
                schema_id: "posts".to_string(),
                schema_version: "v1".to_string(),
            });
            pipeline.execute(write_op, ctx.clone()).await.unwrap();
        }

        // Query
        let query_op = Operation::Query(QueryOp {
            collection: "posts".to_string(),
            filter: None,
            select: None,
            order: None,
            limit: 10,
            offset: 0,
            schema_id: None,
            schema_version: None,
        });

        let result = pipeline.execute(query_op, ctx).await;
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data["count"], 5);
    }
}
