//! Pipeline Bridge
//!
//! Provides a bridge between the new unified pipeline and
//! existing API handlers for backward compatibility.

use std::sync::Arc;

use serde_json::Value;

use crate::core::context::{AuthContext, RequestContext};
use crate::core::error::CoreError;
use crate::core::middleware::auth::AuthMiddleware;
use crate::core::middleware::observe::{AuditLogger, MetricsRecorder, ObserveMiddleware};
use crate::core::middleware::rls::{OwnershipPolicy, RlsMiddleware};
use crate::core::operation::{DeleteOp, Operation, QueryOp, ReadOp, UpdateOp, WriteOp};
use crate::core::pipeline::Pipeline;
use crate::core::StorageBackend;

use super::executor::UnifiedExecutor;

/// Configuration for the pipeline bridge
#[derive(Clone)]
pub struct BridgeConfig {
    /// Enable RLS middleware
    pub enable_rls: bool,
    /// Enable auth middleware
    pub enable_auth: bool,
    /// Enable observability middleware
    pub enable_observe: bool,
    /// Allow anonymous reads
    pub allow_anonymous_reads: bool,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            enable_rls: true,
            enable_auth: true,
            enable_observe: true,
            allow_anonymous_reads: false,
        }
    }
}

/// Pipeline bridge for backward-compatible API integration
pub struct PipelineBridge {
    pipeline: Pipeline,
}

impl PipelineBridge {
    /// Create a new pipeline bridge with in-memory storage (for testing)
    pub fn new_in_memory(config: BridgeConfig) -> Self {
        let storage = super::executor::InMemoryStorage::new();
        Self::with_storage(storage, config)
    }

    /// Create a pipeline bridge with custom storage
    pub fn with_storage(storage: impl StorageBackend + 'static, config: BridgeConfig) -> Self {
        let executor = UnifiedExecutor::new(storage);
        let mut pipeline = Pipeline::new(executor);

        if config.enable_auth {
            let auth = if config.allow_anonymous_reads {
                AuthMiddleware::new().with_anonymous_reads()
            } else {
                AuthMiddleware::new()
            };
            pipeline = pipeline.with_middleware(auth);
        }

        if config.enable_rls {
            pipeline = pipeline.with_middleware(RlsMiddleware::ownership());
        }

        if config.enable_observe {
            pipeline = pipeline.with_middleware(ObserveMiddleware::noop());
        }

        Self { pipeline }
    }

    /// Execute a read operation
    pub async fn read(
        &self,
        collection: &str,
        id: &str,
        ctx: RequestContext,
    ) -> Result<Value, CoreError> {
        let op = Operation::Read(ReadOp {
            collection: collection.to_string(),
            id: id.to_string(),
            select: None,
        });
        self.pipeline.execute(op, ctx).await
    }

    /// Execute a write operation
    pub async fn write(
        &self,
        collection: &str,
        document: Value,
        schema_id: &str,
        ctx: RequestContext,
    ) -> Result<Value, CoreError> {
        let op = Operation::Write(WriteOp {
            collection: collection.to_string(),
            document,
            schema_id: schema_id.to_string(),
            schema_version: "v1".to_string(),
        });
        self.pipeline.execute(op, ctx).await
    }

    /// Execute an update operation
    pub async fn update(
        &self,
        collection: &str,
        id: &str,
        updates: Value,
        ctx: RequestContext,
    ) -> Result<Value, CoreError> {
        let op = Operation::Update(UpdateOp {
            collection: collection.to_string(),
            id: id.to_string(),
            updates,
            schema_id: None,
            schema_version: None,
        });
        self.pipeline.execute(op, ctx).await
    }

    /// Execute a delete operation
    pub async fn delete(
        &self,
        collection: &str,
        id: &str,
        ctx: RequestContext,
    ) -> Result<Value, CoreError> {
        let op = Operation::Delete(DeleteOp {
            collection: collection.to_string(),
            id: id.to_string(),
            schema_id: None,
        });
        self.pipeline.execute(op, ctx).await
    }

    /// Execute a query operation
    pub async fn query(
        &self,
        collection: &str,
        filter: Option<Value>,
        limit: usize,
        offset: usize,
        ctx: RequestContext,
    ) -> Result<Value, CoreError> {
        let op = Operation::Query(QueryOp {
            collection: collection.to_string(),
            filter,
            select: None,
            order: None,
            limit,
            offset,
            schema_id: None,
            schema_version: None,
        });
        self.pipeline.execute(op, ctx).await
    }

    /// Execute any operation
    pub async fn execute(&self, op: Operation, ctx: RequestContext) -> Result<Value, CoreError> {
        self.pipeline.execute(op, ctx).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_bridge_crud_flow() {
        let bridge = PipelineBridge::new_in_memory(BridgeConfig {
            enable_auth: false,
            enable_rls: false,
            enable_observe: false,
            ..Default::default()
        });

        let ctx = RequestContext::anonymous();

        // Write
        let result = bridge
            .write(
                "users",
                serde_json::json!({"name": "Bob", "email": "bob@example.com"}),
                "users",
                ctx.clone(),
            )
            .await;
        assert!(result.is_ok());
        let id = result.unwrap()["id"].as_str().unwrap().to_string();

        // Read
        let result = bridge.read("users", &id, ctx.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["name"], "Bob");

        // Update
        let result = bridge
            .update(
                "users",
                &id,
                serde_json::json!({"name": "Robert"}),
                ctx.clone(),
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["name"], "Robert");

        // Query
        let result = bridge.query("users", None, 10, 0, ctx.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["count"], 1);

        // Delete
        let result = bridge.delete("users", &id, ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_bridge_with_auth() {
        let bridge = PipelineBridge::new_in_memory(BridgeConfig::default());

        // Anonymous should fail
        let ctx = RequestContext::anonymous();
        let result = bridge
            .write("users", serde_json::json!({"name": "Alice"}), "users", ctx)
            .await;
        assert!(result.is_err());

        // Authenticated should work
        let ctx = RequestContext::new(AuthContext::authenticated(Uuid::new_v4()));
        let result = bridge
            .write("users", serde_json::json!({"name": "Alice"}), "users", ctx)
            .await;
        assert!(result.is_ok());
    }
}
