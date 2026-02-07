//! Execution Pipeline
//!
//! Deterministic middleware pipeline for all operations.
//! Enforces: Auth → RLS → Plan → Execute → Observe

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value;

use super::context::RequestContext;
use super::error::{CoreError, CoreResult};
use super::middleware::Middleware;
use super::operation::Operation;

/// Result of an operation
pub type OperationResult = CoreResult<Value>;

/// Next middleware in chain
pub struct Next<'a> {
    middleware: &'a [Arc<dyn Middleware>],
    executor: &'a dyn OperationExecutor,
}

impl<'a> Next<'a> {
    /// Run the next middleware or executor
    pub fn run(
        self,
        op: &'a Operation,
        ctx: &'a mut RequestContext,
    ) -> Pin<Box<dyn Future<Output = OperationResult> + Send + 'a>> {
        Box::pin(async move {
            if let Some((first, rest)) = self.middleware.split_first() {
                let next = Next {
                    middleware: rest,
                    executor: self.executor,
                };
                first.process(op, ctx, next).await
            } else {
                // End of middleware chain, execute operation
                self.executor.execute(op, ctx).await
            }
        })
    }
}

/// Operation executor (final stage of pipeline)
pub trait OperationExecutor: Send + Sync {
    /// Execute the operation
    fn execute(
        &self,
        op: &Operation,
        ctx: &RequestContext,
    ) -> Pin<Box<dyn Future<Output = OperationResult> + Send + '_>>;
}

/// The unified execution pipeline
pub struct Pipeline {
    middleware: Vec<Arc<dyn Middleware>>,
    executor: Arc<dyn OperationExecutor>,
}

impl Pipeline {
    /// Create a new pipeline with the given executor
    pub fn new(executor: impl OperationExecutor + 'static) -> Self {
        Self {
            middleware: Vec::new(),
            executor: Arc::new(executor),
        }
    }

    /// Add middleware to the pipeline
    pub fn with_middleware(mut self, m: impl Middleware + 'static) -> Self {
        self.middleware.push(Arc::new(m));
        self
    }

    /// Execute an operation through the pipeline
    pub async fn execute(&self, op: Operation, mut ctx: RequestContext) -> OperationResult {
        let next = Next {
            middleware: &self.middleware,
            executor: self.executor.as_ref(),
        };
        next.run(&op, &mut ctx).await
    }

    /// Get the number of middleware stages
    pub fn middleware_count(&self) -> usize {
        self.middleware.len()
    }
}

/// Builder for pipeline construction
pub struct PipelineBuilder {
    middleware: Vec<Arc<dyn Middleware>>,
}

impl PipelineBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            middleware: Vec::new(),
        }
    }

    /// Add middleware
    pub fn with(mut self, m: impl Middleware + 'static) -> Self {
        self.middleware.push(Arc::new(m));
        self
    }

    /// Build the pipeline with the given executor
    pub fn build(self, executor: impl OperationExecutor + 'static) -> Pipeline {
        Pipeline {
            middleware: self.middleware,
            executor: Arc::new(executor),
        }
    }
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// No-op executor for testing
pub struct NoOpExecutor;

impl OperationExecutor for NoOpExecutor {
    fn execute(
        &self,
        op: &Operation,
        _ctx: &RequestContext,
    ) -> Pin<Box<dyn Future<Output = OperationResult> + Send + '_>> {
        let op_name = op.name().to_string();
        Box::pin(async move {
            Ok(serde_json::json!({
                "status": "ok",
                "operation": op_name
            }))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::context::AuthContext;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_pipeline_with_no_middleware() {
        let pipeline = Pipeline::new(NoOpExecutor);
        let ctx = RequestContext::new(AuthContext::authenticated(Uuid::new_v4()));
        let op = Operation::Read(crate::core::operation::ReadOp {
            collection: "users".to_string(),
            id: "user_1".to_string(),
            select: None,
        });

        let result = pipeline.execute(op, ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pipeline_builder() {
        let pipeline = PipelineBuilder::new().build(NoOpExecutor);

        assert_eq!(pipeline.middleware_count(), 0);
    }
}
