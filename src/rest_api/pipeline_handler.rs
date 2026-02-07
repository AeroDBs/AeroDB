//! # Pipeline REST Handler
//!
//! REST handler implementation that delegates to the core execution pipeline.
//! RLS/auth are handled by the pipeline middleware, not inline.

use std::sync::Arc;

use serde_json::Value;
use tokio::runtime::Handle;
use uuid::Uuid;

use crate::auth::rls::RlsContext;
use crate::core::{AuthContext, BridgeConfig, PipelineBridge, RequestContext};

use super::errors::{RestError, RestResult};
use super::filter::FilterSet;
use super::parser::QueryParams;
use super::response::{
    DeleteResponse, InsertResponse, ListResponse, SingleResponse, UpdateResponse,
};
use super::RestHandler;

/// REST handler that delegates to the core execution pipeline
///
/// This eliminates inline RLS checks - they're handled by pipeline middleware.
pub struct PipelineRestHandler {
    bridge: Arc<PipelineBridge>,
    runtime: Handle,
}

impl PipelineRestHandler {
    /// Create a new pipeline handler with an existing bridge
    pub fn new(bridge: Arc<PipelineBridge>, runtime: Handle) -> Self {
        Self { bridge, runtime }
    }

    /// Create with a new in-memory bridge (for testing)
    pub fn new_in_memory(runtime: Handle) -> Self {
        let bridge = PipelineBridge::new_in_memory(BridgeConfig::default());
        Self {
            bridge: Arc::new(bridge),
            runtime,
        }
    }

    /// Convert RlsContext to core AuthContext
    fn to_auth_context(ctx: &RlsContext) -> AuthContext {
        if ctx.is_service_role {
            AuthContext::service_role()
        } else if let Some(user_id) = ctx.user_id {
            AuthContext::authenticated(user_id)
        } else {
            AuthContext::anonymous()
        }
    }

    /// Create RequestContext from RlsContext
    fn to_request_context(ctx: &RlsContext) -> RequestContext {
        RequestContext::new(Self::to_auth_context(ctx))
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
                    (Some(Value::Number(av)), Some(Value::Number(bv))) => {
                        let a_f = av.as_f64().unwrap_or(0.0);
                        let b_f = bv.as_f64().unwrap_or(0.0);
                        a_f.partial_cmp(&b_f).unwrap_or(std::cmp::Ordering::Equal)
                    }
                    (Some(Value::String(av)), Some(Value::String(bv))) => av.cmp(bv),
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

impl RestHandler for PipelineRestHandler {
    fn list(
        &self,
        collection: &str,
        params: QueryParams,
        ctx: &RlsContext,
    ) -> RestResult<ListResponse<Value>> {
        let context = Self::to_request_context(ctx);
        let collection = collection.to_string();
        let limit = params.limit;
        let offset = params.offset;

        // Execute query through pipeline
        let result = self
            .runtime
            .block_on(
                self.bridge
                    .query(&collection, None, limit + offset, 0, context),
            )
            .map_err(|e| RestError::Internal(e.to_string()))?;

        // Convert query result to Vec<Value>
        let mut records: Vec<Value> =
            if let Some(arr) = result.get("results").and_then(|v| v.as_array()) {
                arr.clone()
            } else {
                vec![]
            };

        // Apply query filters (already RLS filtered by pipeline)
        records = Self::apply_query_filters(&records, &params);

        // Apply ordering
        Self::apply_ordering(&mut records, &params);

        // Apply pagination
        let records: Vec<Value> = records.into_iter().skip(offset).take(limit).collect();

        // Select fields
        let records = Self::select_fields(records, &params);

        Ok(ListResponse::new(records, limit, offset))
    }

    fn get(
        &self,
        collection: &str,
        id: &str,
        ctx: &RlsContext,
    ) -> RestResult<SingleResponse<Value>> {
        let context = Self::to_request_context(ctx);

        // Execute read through pipeline (RLS handled by middleware)
        let result = self
            .runtime
            .block_on(self.bridge.read(collection, id, context))
            .map_err(|e| {
                // Check if it's a not-found error
                if e.to_string().contains("not found") {
                    RestError::NotFound
                } else {
                    RestError::Internal(e.to_string())
                }
            })?;

        // Check if it's a "not found" result (document with no id)
        if result.is_null() || (result.get("_id").is_none() && result.get("id").is_none()) {
            return Err(RestError::NotFound);
        }

        Ok(SingleResponse::new(result))
    }

    fn insert(
        &self,
        collection: &str,
        data: Value,
        ctx: &RlsContext,
    ) -> RestResult<InsertResponse<Value>> {
        let context = Self::to_request_context(ctx);

        // Execute write through pipeline (RLS/auth handled by middleware)
        let result = self
            .runtime
            .block_on(self.bridge.write(collection, data, "default", context))
            .map_err(|e| RestError::Internal(e.to_string()))?;

        Ok(InsertResponse::new(vec![result]))
    }

    fn update(
        &self,
        collection: &str,
        id: &str,
        data: Value,
        ctx: &RlsContext,
    ) -> RestResult<UpdateResponse<Value>> {
        let context = Self::to_request_context(ctx);

        // Execute update through pipeline
        let result = self
            .runtime
            .block_on(self.bridge.update(collection, id, data, context))
            .map_err(|e| RestError::Internal(e.to_string()))?;

        Ok(UpdateResponse::new(result))
    }

    fn delete(&self, collection: &str, id: &str, ctx: &RlsContext) -> RestResult<DeleteResponse> {
        let context = Self::to_request_context(ctx);

        // Execute delete through pipeline
        let result = self
            .runtime
            .block_on(self.bridge.delete(collection, id, context))
            .map_err(|e| RestError::Internal(e.to_string()))?;

        // Check if delete was successful
        if result
            .get("deleted")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            Ok(DeleteResponse::success())
        } else {
            Err(RestError::NotFound)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    fn setup_handler() -> (PipelineRestHandler, Runtime) {
        let runtime = Runtime::new().unwrap();
        // Create bridge without auth/RLS for testing
        let bridge = PipelineBridge::new_in_memory(BridgeConfig {
            enable_auth: false,
            enable_rls: false,
            enable_observe: false,
            ..Default::default()
        });
        let handler = PipelineRestHandler::new(Arc::new(bridge), runtime.handle().clone());
        (handler, runtime)
    }

    #[test]
    fn test_pipeline_handler_insert_and_get() {
        let (handler, _rt) = setup_handler();
        let ctx = RlsContext::service_role();

        // Insert
        let data = serde_json::json!({"name": "Alice", "age": 30});
        let result = handler.insert("users", data, &ctx).unwrap();
        let id = result.data[0].get("id").unwrap().as_str().unwrap();

        // Get
        let result = handler.get("users", id, &ctx).unwrap();
        assert_eq!(result.data["name"], "Alice");
    }

    #[test]
    fn test_pipeline_handler_update() {
        let (handler, _rt) = setup_handler();
        let ctx = RlsContext::service_role();

        // Insert
        let data = serde_json::json!({"name": "Bob"});
        let result = handler.insert("users", data, &ctx).unwrap();
        let id = result.data[0].get("id").unwrap().as_str().unwrap();

        // Update
        let updates = serde_json::json!({"name": "Robert"});
        let result = handler.update("users", id, updates, &ctx).unwrap();
        assert_eq!(result.data["name"], "Robert");
    }

    #[test]
    fn test_pipeline_handler_delete() {
        let (handler, _rt) = setup_handler();
        let ctx = RlsContext::service_role();

        // Insert
        let data = serde_json::json!({"name": "Charlie"});
        let result = handler.insert("users", data, &ctx).unwrap();
        let id = result.data[0].get("id").unwrap().as_str().unwrap();

        // Delete
        let result = handler.delete("users", id, &ctx);
        assert!(result.is_ok());
    }
}
