//! # REST API Handler
//!
//! Handles REST requests and translates to AeroDB operations.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use uuid::Uuid;

use crate::auth::rls::{RlsContext, RlsEnforcer};

use super::errors::{RestError, RestResult};
use super::filter::{FilterExpr, FilterSet};
use super::parser::QueryParams;
use super::response::{
    DeleteResponse, InsertResponse, ListResponse, SingleResponse, UpdateResponse,
};

/// REST handler trait for collection operations
pub trait RestHandler: Send + Sync {
    /// List records in a collection
    fn list(
        &self,
        collection: &str,
        params: QueryParams,
        ctx: &RlsContext,
    ) -> RestResult<ListResponse<Value>>;

    /// Get a single record by ID
    fn get(
        &self,
        collection: &str,
        id: &str,
        ctx: &RlsContext,
    ) -> RestResult<SingleResponse<Value>>;

    /// Insert a record
    fn insert(
        &self,
        collection: &str,
        data: Value,
        ctx: &RlsContext,
    ) -> RestResult<InsertResponse<Value>>;

    /// Update a record
    fn update(
        &self,
        collection: &str,
        id: &str,
        data: Value,
        ctx: &RlsContext,
    ) -> RestResult<UpdateResponse<Value>>;

    /// Delete a record
    fn delete(&self, collection: &str, id: &str, ctx: &RlsContext) -> RestResult<DeleteResponse>;
}

/// In-memory REST handler for testing
///
/// In production, this would delegate to the actual AeroDB executor.
pub struct InMemoryRestHandler<E: RlsEnforcer> {
    /// Data store: collection -> records
    data: std::sync::RwLock<HashMap<String, Vec<Value>>>,

    /// RLS enforcer
    rls: Arc<E>,
}

impl<E: RlsEnforcer> InMemoryRestHandler<E> {
    pub fn new(rls: E) -> Self {
        Self {
            data: std::sync::RwLock::new(HashMap::new()),
            rls: Arc::new(rls),
        }
    }

    /// Apply RLS filter if needed
    fn apply_rls_filter(
        &self,
        collection: &str,
        records: &[Value],
        ctx: &RlsContext,
    ) -> RestResult<Vec<Value>> {
        let filter = self.rls.get_read_filter(collection, ctx)?;

        match filter {
            Some(rls_filter) => {
                let filter_expr = FilterExpr::new(
                    rls_filter.field,
                    super::filter::FilterOperator::Eq,
                    rls_filter.value,
                );
                Ok(records
                    .iter()
                    .filter(|r| filter_expr.matches(r))
                    .cloned()
                    .collect())
            }
            None => Ok(records.to_vec()),
        }
    }

    /// Apply query filters
    fn apply_query_filters(records: &[Value], params: &QueryParams) -> Vec<Value> {
        let filter_set = FilterSet {
            filters: params.filters.clone(),
        };

        records
            .iter()
            .filter(|r| filter_set.matches(r))
            .cloned()
            .collect()
    }

    /// Apply ordering
    fn apply_ordering(records: &mut [Value], params: &QueryParams) {
        if params.order.is_empty() {
            return;
        }

        records.sort_by(|a, b| {
            for order in &params.order {
                let a_val = a.get(&order.field);
                let b_val = b.get(&order.field);

                let cmp = match (a_val, b_val) {
                    (Some(Value::Number(a)), Some(Value::Number(b))) => {
                        let a = a.as_f64().unwrap_or(0.0);
                        let b = b.as_f64().unwrap_or(0.0);
                        a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
                    }
                    (Some(Value::String(a)), Some(Value::String(b))) => a.cmp(b),
                    _ => std::cmp::Ordering::Equal,
                };

                let cmp = if order.ascending { cmp } else { cmp.reverse() };
                if cmp != std::cmp::Ordering::Equal {
                    return cmp;
                }
            }
            std::cmp::Ordering::Equal
        });
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
            Some(fields) if fields.len() == 1 && fields[0] == "*" => records,
            Some(fields) => records
                .into_iter()
                .map(|r| {
                    if let Value::Object(obj) = r {
                        let filtered: serde_json::Map<String, Value> = obj
                            .into_iter()
                            .filter(|(k, _)| fields.contains(k))
                            .collect();
                        Value::Object(filtered)
                    } else {
                        r
                    }
                })
                .collect(),
        }
    }
}

impl<E: RlsEnforcer> RestHandler for InMemoryRestHandler<E> {
    fn list(
        &self,
        collection: &str,
        params: QueryParams,
        ctx: &RlsContext,
    ) -> RestResult<ListResponse<Value>> {
        let data = self
            .data
            .read()
            .map_err(|_| RestError::Internal("Lock poisoned".to_string()))?;

        let records = data.get(collection).cloned().unwrap_or_default();

        // Apply RLS filter
        let records = self.apply_rls_filter(collection, &records, ctx)?;

        // Apply query filters
        let records = Self::apply_query_filters(&records, &params);

        // Apply ordering
        let mut records = records;
        Self::apply_ordering(&mut records, &params);

        // Apply pagination
        let records = Self::apply_pagination(records, &params);

        // Select fields
        let records = Self::select_fields(records, &params);

        Ok(ListResponse::new(records, params.limit, params.offset))
    }

    fn get(
        &self,
        collection: &str,
        id: &str,
        ctx: &RlsContext,
    ) -> RestResult<SingleResponse<Value>> {
        let data = self
            .data
            .read()
            .map_err(|_| RestError::Internal("Lock poisoned".to_string()))?;

        let records = data.get(collection).cloned().unwrap_or_default();

        // Apply RLS filter
        let records = self.apply_rls_filter(collection, &records, ctx)?;

        // Find by ID
        let record = records
            .into_iter()
            .find(|r| r.get("id").and_then(|v| v.as_str()) == Some(id))
            .ok_or(RestError::NotFound)?;

        Ok(SingleResponse::new(record))
    }

    fn insert(
        &self,
        collection: &str,
        mut data: Value,
        ctx: &RlsContext,
    ) -> RestResult<InsertResponse<Value>> {
        // Prepare insert (add owner field)
        self.rls.prepare_insert(collection, &mut data, ctx)?;

        // Add ID if not present
        if data.get("id").is_none() {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("id".to_string(), Value::String(Uuid::new_v4().to_string()));
            }
        }

        // Validate write
        self.rls.validate_write(collection, &data, ctx)?;

        let mut store = self
            .data
            .write()
            .map_err(|_| RestError::Internal("Lock poisoned".to_string()))?;

        store
            .entry(collection.to_string())
            .or_default()
            .push(data.clone());

        Ok(InsertResponse::single(data))
    }

    fn update(
        &self,
        collection: &str,
        id: &str,
        updates: Value,
        ctx: &RlsContext,
    ) -> RestResult<UpdateResponse<Value>> {
        let mut store = self
            .data
            .write()
            .map_err(|_| RestError::Internal("Lock poisoned".to_string()))?;

        let records = store
            .get_mut(collection)
            .ok_or(RestError::CollectionNotFound(collection.to_string()))?;

        let record = records
            .iter_mut()
            .find(|r| r.get("id").and_then(|v| v.as_str()) == Some(id))
            .ok_or(RestError::NotFound)?;

        // Validate RLS
        self.rls.validate_write(collection, record, ctx)?;

        // Apply updates
        if let (Some(record_obj), Some(updates_obj)) = (record.as_object_mut(), updates.as_object())
        {
            for (key, value) in updates_obj {
                record_obj.insert(key.clone(), value.clone());
            }
        }

        Ok(UpdateResponse::new(record.clone()))
    }

    fn delete(&self, collection: &str, id: &str, ctx: &RlsContext) -> RestResult<DeleteResponse> {
        let mut store = self
            .data
            .write()
            .map_err(|_| RestError::Internal("Lock poisoned".to_string()))?;

        let records = store
            .get_mut(collection)
            .ok_or(RestError::CollectionNotFound(collection.to_string()))?;

        // Find record and validate RLS
        let idx = records
            .iter()
            .position(|r| r.get("id").and_then(|v| v.as_str()) == Some(id))
            .ok_or(RestError::NotFound)?;

        let record = &records[idx];
        self.rls.validate_write(collection, record, ctx)?;

        records.remove(idx);

        Ok(DeleteResponse::success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::rls::DefaultRlsEnforcer;

    fn create_test_handler() -> InMemoryRestHandler<DefaultRlsEnforcer> {
        InMemoryRestHandler::new(DefaultRlsEnforcer::new())
    }

    #[test]
    fn test_insert_and_list() {
        let handler = create_test_handler();
        let user_id = Uuid::new_v4();
        let ctx = RlsContext::authenticated(user_id);

        // Insert
        let data = serde_json::json!({"title": "Test Post"});
        let result = handler.insert("posts", data, &ctx).unwrap();
        assert_eq!(result.count, 1);

        // List
        let params = QueryParams::default();
        let list = handler.list("posts", params, &ctx).unwrap();
        assert_eq!(list.count, 1);
    }

    #[test]
    fn test_rls_filtering() {
        let handler = create_test_handler();
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();

        // User 1 creates a post
        let ctx1 = RlsContext::authenticated(user1);
        handler
            .insert("posts", serde_json::json!({"title": "User 1 Post"}), &ctx1)
            .unwrap();

        // User 2 creates a post
        let ctx2 = RlsContext::authenticated(user2);
        handler
            .insert("posts", serde_json::json!({"title": "User 2 Post"}), &ctx2)
            .unwrap();

        // User 1 should only see their post
        let list1 = handler
            .list("posts", QueryParams::default(), &ctx1)
            .unwrap();
        assert_eq!(list1.count, 1);

        // User 2 should only see their post
        let list2 = handler
            .list("posts", QueryParams::default(), &ctx2)
            .unwrap();
        assert_eq!(list2.count, 1);

        // Service role sees all
        let service_ctx = RlsContext::service_role();
        let all = handler
            .list("posts", QueryParams::default(), &service_ctx)
            .unwrap();
        assert_eq!(all.count, 2);
    }

    #[test]
    fn test_get_by_id() {
        let handler = create_test_handler();
        let user_id = Uuid::new_v4();
        let ctx = RlsContext::authenticated(user_id);

        // Insert
        let data = serde_json::json!({"title": "Test"});
        let insert_result = handler.insert("posts", data, &ctx).unwrap();
        let id = insert_result.data[0]["id"].as_str().unwrap();

        // Get
        let result = handler.get("posts", id, &ctx).unwrap();
        assert_eq!(result.data["title"], "Test");
    }

    #[test]
    fn test_update() {
        let handler = create_test_handler();
        let user_id = Uuid::new_v4();
        let ctx = RlsContext::authenticated(user_id);

        // Insert
        let data = serde_json::json!({"title": "Original"});
        let insert_result = handler.insert("posts", data, &ctx).unwrap();
        let id = insert_result.data[0]["id"].as_str().unwrap();

        // Update
        let updates = serde_json::json!({"title": "Updated"});
        let result = handler.update("posts", id, updates, &ctx).unwrap();
        assert_eq!(result.data["title"], "Updated");
    }

    #[test]
    fn test_delete() {
        let handler = create_test_handler();
        let user_id = Uuid::new_v4();
        let ctx = RlsContext::authenticated(user_id);

        // Insert
        let data = serde_json::json!({"title": "To Delete"});
        let insert_result = handler.insert("posts", data, &ctx).unwrap();
        let id = insert_result.data[0]["id"].as_str().unwrap();

        // Delete
        let result = handler.delete("posts", id, &ctx).unwrap();
        assert!(result.deleted);

        // Verify deleted
        let get_result = handler.get("posts", id, &ctx);
        assert!(matches!(get_result, Err(RestError::NotFound)));
    }
}
