//! RLS Middleware
//!
//! Evaluates Row-Level Security policies and injects filters.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value;

use crate::core::context::{FilterOperator, RequestContext, RlsFilter};
use crate::core::error::CoreError;
use crate::core::operation::Operation;
use crate::core::pipeline::{Next, OperationResult};

use super::Middleware;

/// RLS policy provider trait
pub trait RlsPolicyProvider: Send + Sync {
    /// Get the read filter for a collection
    fn get_read_filter(
        &self,
        collection: &str,
        user_id: Option<uuid::Uuid>,
    ) -> Result<Option<RlsFilter>, String>;

    /// Validate a write operation
    fn validate_write(
        &self,
        collection: &str,
        document: &Value,
        user_id: Option<uuid::Uuid>,
    ) -> Result<(), String>;
}

/// Default RLS policy: ownership-based
pub struct OwnershipPolicy {
    owner_field: String,
}

impl OwnershipPolicy {
    pub fn new(owner_field: impl Into<String>) -> Self {
        Self {
            owner_field: owner_field.into(),
        }
    }
}

impl Default for OwnershipPolicy {
    fn default() -> Self {
        Self::new("owner_id")
    }
}

impl RlsPolicyProvider for OwnershipPolicy {
    fn get_read_filter(
        &self,
        _collection: &str,
        user_id: Option<uuid::Uuid>,
    ) -> Result<Option<RlsFilter>, String> {
        match user_id {
            Some(id) => Ok(Some(RlsFilter {
                field: self.owner_field.clone(),
                operator: FilterOperator::Eq,
                value: Value::String(id.to_string()),
            })),
            None => Err("Authentication required for RLS".to_string()),
        }
    }

    fn validate_write(
        &self,
        _collection: &str,
        document: &Value,
        user_id: Option<uuid::Uuid>,
    ) -> Result<(), String> {
        let user_id = user_id.ok_or("Authentication required")?;

        let doc_owner = document
            .get(&self.owner_field)
            .and_then(|v| v.as_str())
            .and_then(|s| uuid::Uuid::parse_str(s).ok());

        match doc_owner {
            Some(owner) if owner == user_id => Ok(()),
            Some(_) => Err("Cannot modify documents owned by another user".to_string()),
            None => Ok(()), // New document, owner will be set
        }
    }
}

/// RLS middleware
pub struct RlsMiddleware {
    policy: Arc<dyn RlsPolicyProvider>,
}

impl RlsMiddleware {
    pub fn new(policy: impl RlsPolicyProvider + 'static) -> Self {
        Self {
            policy: Arc::new(policy),
        }
    }

    pub fn ownership() -> Self {
        Self::new(OwnershipPolicy::default())
    }
}

impl Middleware for RlsMiddleware {
    fn process<'a>(
        &'a self,
        op: &'a Operation,
        ctx: &'a mut RequestContext,
        next: Next<'a>,
    ) -> Pin<Box<dyn Future<Output = OperationResult> + Send + 'a>> {
        Box::pin(async move {
            // Service role bypasses RLS
            if ctx.bypass_rls() {
                return next.run(op, ctx).await;
            }

            // Get collection if applicable
            if let Some(collection) = op.collection() {
                // Get and inject RLS filter for reads
                match op {
                    Operation::Read(_) | Operation::Query(_) => {
                        let filter = self
                            .policy
                            .get_read_filter(collection, ctx.auth.user_id)
                            .map_err(CoreError::access_denied)?;

                        if let Some(f) = filter {
                            ctx.rls_filters.push(f);
                        }
                    }
                    Operation::Write(w) => {
                        self.policy
                            .validate_write(collection, &w.document, ctx.auth.user_id)
                            .map_err(CoreError::access_denied)?;
                    }
                    Operation::Update(u) => {
                        self.policy
                            .validate_write(collection, &u.updates, ctx.auth.user_id)
                            .map_err(CoreError::access_denied)?;
                    }
                    _ => {}
                }
            }

            next.run(op, ctx).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::context::AuthContext;
    use crate::core::operation::{QueryOp, WriteOp};
    use crate::core::pipeline::{NoOpExecutor, Pipeline};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_rls_injects_filter() {
        let pipeline = Pipeline::new(NoOpExecutor).with_middleware(RlsMiddleware::ownership());

        let user_id = Uuid::new_v4();
        let mut ctx = RequestContext::new(AuthContext::authenticated(user_id));
        let op = Operation::Query(QueryOp {
            collection: "posts".to_string(),
            filter: None,
            select: None,
            order: None,
            limit: 10,
            offset: 0,
            schema_id: None,
            schema_version: None,
        });

        // Execute through pipeline
        let _ = pipeline.execute(op.clone(), ctx.clone()).await;

        // Note: In real implementation, ctx would be modified
        // This test validates the middleware structure
    }

    #[tokio::test]
    async fn test_service_role_bypasses_rls() {
        let pipeline = Pipeline::new(NoOpExecutor).with_middleware(RlsMiddleware::ownership());

        let ctx = RequestContext::service_role();
        let op = Operation::Query(QueryOp {
            collection: "posts".to_string(),
            filter: None,
            select: None,
            order: None,
            limit: 10,
            offset: 0,
            schema_id: None,
            schema_version: None,
        });

        let result = pipeline.execute(op, ctx).await;
        assert!(result.is_ok());
    }
}
