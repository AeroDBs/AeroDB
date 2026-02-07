//! # Database Facade for REST API
//!
//! Provides a high-level database interface for the REST API layer.
//! Acts as a bridge between HTTP handlers and the underlying executor.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde_json::Value;
use uuid::Uuid;

use super::errors::{RestError, RestResult};
use super::handler::RestHandler;
use super::parser::QueryParams;
use super::response::{
    DeleteResponse, InsertResponse, ListResponse, SingleResponse, UpdateResponse,
};
use crate::auth::rls::{DefaultRlsEnforcer, RlsContext, RlsEnforcer};
use crate::auth::AuthError;

/// Database operations for a collection
pub struct DatabaseFacade<E: RlsEnforcer = DefaultRlsEnforcer> {
    /// Collection data (in-memory for now, replace with storage)
    collections: Arc<RwLock<HashMap<String, CollectionData>>>,

    /// RLS enforcer
    rls: E,
}

/// Collection data store
#[derive(Debug, Default, Clone)]
struct CollectionData {
    /// Documents by ID
    documents: HashMap<String, Value>,
}

impl<E: RlsEnforcer> DatabaseFacade<E> {
    /// Create a new database facade with the given RLS enforcer
    pub fn new(rls: E) -> Self {
        Self {
            collections: Arc::new(RwLock::new(HashMap::new())),
            rls,
        }
    }

    /// Get or create a collection
    fn collection(&self, name: &str) -> CollectionData {
        let collections = self.collections.read().unwrap();
        collections.get(name).cloned().unwrap_or_default()
    }

    /// Update a collection
    fn update_collection<F>(&self, name: &str, f: F)
    where
        F: FnOnce(&mut CollectionData),
    {
        let mut collections = self.collections.write().unwrap();
        let collection = collections.entry(name.to_string()).or_default();
        f(collection);
    }

    /// Apply RLS filter to records
    fn apply_rls_filter(
        &self,
        collection: &str,
        records: &[Value],
        ctx: &RlsContext,
    ) -> RestResult<Vec<Value>> {
        // Get RLS filter for this collection
        let filter = self
            .rls
            .get_read_filter(collection, ctx)
            .map_err(RestError::Auth)?;

        Ok(match filter {
            Some(f) => {
                // Apply ownership filter
                records
                    .iter()
                    .filter(|doc| {
                        if let Some(owner_id) = doc.get(&f.field).and_then(|v| v.as_str()) {
                            if let Some(user_id) = &ctx.user_id {
                                return owner_id == user_id.to_string();
                            }
                        }
                        false
                    })
                    .cloned()
                    .collect()
            }
            None => records.to_vec(),
        })
    }

    /// Apply query filters to records
    fn apply_filters(records: &[Value], params: &QueryParams) -> Vec<Value> {
        records
            .iter()
            .filter(|doc| {
                for filter in &params.filters {
                    if !filter.matches(doc) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }

    /// Apply ordering
    fn apply_ordering(records: &mut [Value], params: &QueryParams) {
        if let Some(order_by) = params.order.first() {
            records.sort_by(|a, b| {
                let va = a.get(&order_by.field);
                let vb = b.get(&order_by.field);
                let cmp = compare_json_values(va, vb);
                if order_by.ascending {
                    cmp
                } else {
                    cmp.reverse()
                }
            });
        }
    }

    /// Apply pagination
    fn apply_pagination(records: Vec<Value>, params: &QueryParams) -> Vec<Value> {
        records
            .into_iter()
            .skip(params.offset)
            .take(params.limit)
            .collect()
    }

    /// Select fields from records
    fn select_fields(records: Vec<Value>, params: &QueryParams) -> Vec<Value> {
        match &params.select {
            None => records,
            Some(fields) if fields.is_empty() => records,
            Some(fields) => records
                .into_iter()
                .map(|doc| {
                    if let Value::Object(obj) = doc {
                        let filtered: serde_json::Map<String, Value> = obj
                            .into_iter()
                            .filter(|(k, _)| fields.contains(k))
                            .collect();
                        Value::Object(filtered)
                    } else {
                        doc
                    }
                })
                .collect(),
        }
    }
}

impl<E: RlsEnforcer + Send + Sync> RestHandler for DatabaseFacade<E> {
    fn list(
        &self,
        collection: &str,
        params: QueryParams,
        ctx: &RlsContext,
    ) -> RestResult<ListResponse<Value>> {
        let coll = self.collection(collection);
        let records: Vec<Value> = coll.documents.values().cloned().collect();

        // Apply RLS
        let filtered = self.apply_rls_filter(collection, &records, ctx)?;

        // Apply query filters
        let filtered = Self::apply_filters(&filtered, &params);

        // Get total count before pagination
        let total = filtered.len();

        // Apply ordering
        let mut sorted = filtered;
        Self::apply_ordering(&mut sorted, &params);

        // Apply pagination
        let paginated = Self::apply_pagination(sorted, &params);

        // Select fields
        let selected = Self::select_fields(paginated, &params);

        Ok(ListResponse {
            data: selected,
            count: total,
            limit: params.limit,
            offset: params.offset,
        })
    }

    fn get(
        &self,
        collection: &str,
        id: &str,
        ctx: &RlsContext,
    ) -> RestResult<SingleResponse<Value>> {
        let coll = self.collection(collection);

        let doc = coll.documents.get(id).ok_or(RestError::NotFound)?;

        // Apply RLS
        let allowed = self.apply_rls_filter(collection, &[doc.clone()], ctx)?;
        if allowed.is_empty() {
            return Err(RestError::NotFound);
        }

        Ok(SingleResponse {
            data: allowed.into_iter().next().unwrap(),
        })
    }

    fn insert(
        &self,
        collection: &str,
        mut data: Value,
        ctx: &RlsContext,
    ) -> RestResult<InsertResponse<Value>> {
        // Validate RLS for write
        self.rls
            .validate_write(collection, &data, ctx)
            .map_err(RestError::Auth)?;

        // Prepare document (inject owner_id if needed)
        self.rls
            .prepare_insert(collection, &mut data, ctx)
            .map_err(|e| RestError::InvalidBody(e.to_string()))?;

        // Generate ID if not present
        let id = if let Some(id) = data.get("_id").and_then(|v| v.as_str()) {
            id.to_string()
        } else {
            let id = Uuid::new_v4().to_string();
            data.as_object_mut()
                .unwrap()
                .insert("_id".to_string(), Value::String(id.clone()));
            id
        };

        // Insert document
        let result = data.clone();
        self.update_collection(collection, |coll| {
            coll.documents.insert(id, data);
        });

        Ok(InsertResponse {
            data: vec![result],
            count: 1,
        })
    }

    fn update(
        &self,
        collection: &str,
        id: &str,
        updates: Value,
        ctx: &RlsContext,
    ) -> RestResult<UpdateResponse<Value>> {
        let coll = self.collection(collection);

        // Get existing document
        let existing = coll.documents.get(id).ok_or(RestError::NotFound)?;

        // Check RLS
        let allowed = self.apply_rls_filter(collection, &[existing.clone()], ctx)?;
        if allowed.is_empty() {
            return Err(RestError::NotFound);
        }

        // Merge updates
        let mut updated = existing.clone();
        if let (Value::Object(base), Value::Object(patches)) = (&mut updated, updates) {
            for (k, v) in patches {
                base.insert(k, v);
            }
        }

        // Validate updated document
        self.rls
            .validate_write(collection, &updated, ctx)
            .map_err(RestError::Auth)?;

        // Store updated document
        let result = updated.clone();
        self.update_collection(collection, |coll| {
            coll.documents.insert(id.to_string(), updated);
        });

        Ok(UpdateResponse { data: result })
    }

    fn delete(&self, collection: &str, id: &str, ctx: &RlsContext) -> RestResult<DeleteResponse> {
        let coll = self.collection(collection);

        // Check if exists
        let existing = coll.documents.get(id).ok_or(RestError::NotFound)?;

        // Check RLS
        let allowed = self.apply_rls_filter(collection, &[existing.clone()], ctx)?;
        if allowed.is_empty() {
            return Err(RestError::NotFound);
        }

        // Delete
        self.update_collection(collection, |coll| {
            coll.documents.remove(id);
        });

        Ok(DeleteResponse { deleted: true })
    }
}

/// Compare JSON values for sorting
fn compare_json_values(a: Option<&Value>, b: Option<&Value>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(Value::Number(a)), Some(Value::Number(b))) => a
            .as_f64()
            .unwrap_or(0.0)
            .partial_cmp(&b.as_f64().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal),
        (Some(Value::String(a)), Some(Value::String(b))) => a.cmp(b),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        _ => std::cmp::Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::rls::{DefaultRlsEnforcer, RlsPolicy};
    use serde_json::json;

    fn create_facade() -> DatabaseFacade<DefaultRlsEnforcer> {
        DatabaseFacade::new(DefaultRlsEnforcer::new())
    }

    #[test]
    fn test_insert_and_list() {
        let db = create_facade();
        // Use service_role to bypass RLS for testing
        let ctx = RlsContext::service_role();

        // Insert
        let doc = json!({"name": "Alice", "age": 30});
        let result = db.insert("users", doc, &ctx).unwrap();
        assert!(result.data[0].get("_id").is_some());

        // List
        let params = QueryParams::default();
        let list = db.list("users", params, &ctx).unwrap();
        assert_eq!(list.count, 1);
    }

    #[test]
    fn test_rls_filtering() {
        let rls = DefaultRlsEnforcer::new().with_policy(
            "posts",
            RlsPolicy::Ownership {
                owner_field: "owner_id".to_string(),
            },
        );
        let db = DatabaseFacade::new(rls);

        // Insert as user1 (must include owner_id for ownership policy)
        let user1 = Uuid::new_v4();
        let ctx1 = RlsContext::authenticated(user1);
        db.insert(
            "posts",
            json!({"title": "User1 Post", "owner_id": user1.to_string()}),
            &ctx1,
        )
        .unwrap();

        // Insert as user2 (must include owner_id for ownership policy)
        let user2 = Uuid::new_v4();
        let ctx2 = RlsContext::authenticated(user2);
        db.insert(
            "posts",
            json!({"title": "User2 Post", "owner_id": user2.to_string()}),
            &ctx2,
        )
        .unwrap();

        // List as user1 - should only see user1's post
        let params = QueryParams::default();
        let list = db.list("posts", params, &ctx1).unwrap();
        assert_eq!(list.count, 1);
        assert_eq!(list.data[0].get("title").unwrap(), "User1 Post");
    }

    #[test]
    fn test_get_by_id() {
        let db = create_facade();
        // Use service_role to bypass RLS for testing
        let ctx = RlsContext::service_role();

        let doc = json!({"_id": "test-id", "name": "Test"});
        db.insert("items", doc, &ctx).unwrap();

        let result = db.get("items", "test-id", &ctx).unwrap();
        assert_eq!(result.data.get("name").unwrap(), "Test");
    }

    #[test]
    fn test_update() {
        let db = create_facade();
        // Use service_role to bypass RLS for testing
        let ctx = RlsContext::service_role();

        let doc = json!({"_id": "test-id", "name": "Old", "count": 1});
        db.insert("items", doc, &ctx).unwrap();

        let updates = json!({"name": "New", "count": 2});
        let result = db.update("items", "test-id", updates, &ctx).unwrap();
        assert_eq!(result.data.get("name").unwrap(), "New");
        assert_eq!(result.data.get("count").unwrap(), 2);
    }

    #[test]
    fn test_delete() {
        let db = create_facade();
        // Use service_role to bypass RLS for testing
        let ctx = RlsContext::service_role();

        let doc = json!({"_id": "delete-id", "name": "ToDelete"});
        db.insert("items", doc, &ctx).unwrap();

        let result = db.delete("items", "delete-id", &ctx).unwrap();
        assert!(result.deleted);

        // Verify deleted
        assert!(db.get("items", "delete-id", &ctx).is_err());
    }

    #[test]
    fn test_pagination() {
        let db = create_facade();
        // Use service_role to bypass RLS for testing
        let ctx = RlsContext::service_role();

        for i in 0..10 {
            db.insert("items", json!({"_id": format!("id-{}", i), "idx": i}), &ctx)
                .unwrap();
        }

        let params = QueryParams {
            limit: 3,
            offset: 2,
            ..Default::default()
        };

        let list = db.list("items", params, &ctx).unwrap();
        assert_eq!(list.data.len(), 3);
    }
}
