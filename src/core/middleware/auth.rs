//! Authentication Middleware
//!
//! Validates authentication and extracts user context.

use std::future::Future;
use std::pin::Pin;

use crate::core::context::RequestContext;
use crate::core::error::CoreError;
use crate::core::operation::Operation;
use crate::core::pipeline::{Next, OperationResult};

use super::Middleware;

/// Authentication middleware
pub struct AuthMiddleware {
    /// Allow anonymous access to certain operations
    allow_anonymous_reads: bool,
}

impl AuthMiddleware {
    /// Create a new auth middleware
    pub fn new() -> Self {
        Self {
            allow_anonymous_reads: false,
        }
    }

    /// Allow anonymous reads (for public collections)
    pub fn with_anonymous_reads(mut self) -> Self {
        self.allow_anonymous_reads = true;
        self
    }
}

impl Default for AuthMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for AuthMiddleware {
    fn process<'a>(
        &'a self,
        op: &'a Operation,
        ctx: &'a mut RequestContext,
        next: Next<'a>,
    ) -> Pin<Box<dyn Future<Output = OperationResult> + Send + 'a>> {
        Box::pin(async move {
            // Service role always passes
            if ctx.auth.is_service_role {
                return next.run(op, ctx).await;
            }

            // Check if auth is required
            let requires_auth = match op {
                Operation::Read(_) | Operation::Query(_) => !self.allow_anonymous_reads,
                _ => true,
            };

            if requires_auth && !ctx.auth.is_authenticated {
                return Err(CoreError::AuthRequired);
            }

            // Continue to next middleware
            next.run(op, ctx).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::context::AuthContext;
    use crate::core::operation::ReadOp;
    use crate::core::pipeline::{NoOpExecutor, Pipeline};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_authenticated_user_passes() {
        let pipeline = Pipeline::new(NoOpExecutor).with_middleware(AuthMiddleware::new());

        let ctx = RequestContext::new(AuthContext::authenticated(Uuid::new_v4()));
        let op = Operation::Read(ReadOp {
            collection: "users".to_string(),
            id: "user_1".to_string(),
            select: None,
        });

        let result = pipeline.execute(op, ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_anonymous_user_denied() {
        let pipeline = Pipeline::new(NoOpExecutor).with_middleware(AuthMiddleware::new());

        let ctx = RequestContext::anonymous();
        let op = Operation::Read(ReadOp {
            collection: "users".to_string(),
            id: "user_1".to_string(),
            select: None,
        });

        let result = pipeline.execute(op, ctx).await;
        assert!(matches!(result, Err(CoreError::AuthRequired)));
    }

    #[tokio::test]
    async fn test_anonymous_reads_allowed() {
        let pipeline = Pipeline::new(NoOpExecutor)
            .with_middleware(AuthMiddleware::new().with_anonymous_reads());

        let ctx = RequestContext::anonymous();
        let op = Operation::Read(ReadOp {
            collection: "public_data".to_string(),
            id: "item_1".to_string(),
            select: None,
        });

        let result = pipeline.execute(op, ctx).await;
        assert!(result.is_ok());
    }
}
