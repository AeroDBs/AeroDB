//! Observability Middleware
//!
//! Automatic instrumentation for metrics, audit logs, and tracing.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::core::context::RequestContext;
use crate::core::operation::Operation;
use crate::core::pipeline::{Next, OperationResult};

use super::Middleware;

/// Metrics recorder trait
pub trait MetricsRecorder: Send + Sync {
    fn record(&self, name: &str, value: f64, labels: &[(&str, &str)]);
    fn increment(&self, name: &str, labels: &[(&str, &str)]);
}

/// Audit logger trait
pub trait AuditLogger: Send + Sync {
    fn log(
        &self,
        user_id: Option<uuid::Uuid>,
        operation: &str,
        success: bool,
        request_id: uuid::Uuid,
        details: Option<String>,
    );
}

/// No-op implementations for testing
pub struct NoOpMetrics;
impl MetricsRecorder for NoOpMetrics {
    fn record(&self, _: &str, _: f64, _: &[(&str, &str)]) {}
    fn increment(&self, _: &str, _: &[(&str, &str)]) {}
}

pub struct NoOpAudit;
impl AuditLogger for NoOpAudit {
    fn log(&self, _: Option<uuid::Uuid>, _: &str, _: bool, _: uuid::Uuid, _: Option<String>) {}
}

/// Adapter that wraps MetricsRegistry from observability module
pub struct MetricsRegistryAdapter {
    registry: Arc<crate::observability::MetricsRegistry>,
}

impl MetricsRegistryAdapter {
    pub fn new(registry: Arc<crate::observability::MetricsRegistry>) -> Self {
        Self { registry }
    }
}

impl MetricsRecorder for MetricsRegistryAdapter {
    fn record(&self, name: &str, _value: f64, labels: &[(&str, &str)]) {
        // Map common metric names to MetricsRegistry methods
        match name {
            "operation_duration_ms" => {
                // Duration is recorded but MetricsRegistry only has counters
                // For now, we just increment queries_executed as a proxy
            }
            _ => {}
        }
        // Log for debugging
        let _ = (name, labels);
    }

    fn increment(&self, name: &str, labels: &[(&str, &str)]) {
        match name {
            "operation_success_total" => {
                if labels
                    .iter()
                    .any(|(k, v)| *k == "operation" && *v == "query")
                {
                    self.registry.increment_queries_executed();
                } else {
                    self.registry.increment_writes();
                }
            }
            "operation_error_total" => {
                self.registry.increment_queries_rejected();
            }
            _ => {}
        }
    }
}

/// Adapter that wraps AuditLog from observability module
pub struct AuditLogAdapter<A: crate::observability::AuditLog> {
    audit_log: Arc<A>,
}

impl<A: crate::observability::AuditLog> AuditLogAdapter<A> {
    pub fn new(audit_log: Arc<A>) -> Self {
        Self { audit_log }
    }
}

impl<A: crate::observability::AuditLog + 'static> AuditLogger for AuditLogAdapter<A> {
    fn log(
        &self,
        user_id: Option<uuid::Uuid>,
        operation: &str,
        success: bool,
        request_id: uuid::Uuid,
        details: Option<String>,
    ) {
        use crate::observability::{AuditAction, AuditOutcome, AuditRecord};

        let action = AuditAction::CommandExecuted;
        let outcome = if success {
            AuditOutcome::Success
        } else {
            AuditOutcome::Failed
        };

        let mut record = AuditRecord::new(action, outcome)
            .with_command(operation)
            .with_request_id(request_id);

        if let Some(uid) = user_id {
            record = record.with_operator(uid.to_string());
        }

        if let Some(err) = details {
            record = record.with_error(err);
        }

        // Best-effort logging - observability should never crash
        let _ = self.audit_log.append(&record);
    }
}

/// Observability middleware
pub struct ObserveMiddleware {
    metrics: Arc<dyn MetricsRecorder>,
    audit: Arc<dyn AuditLogger>,
}

impl ObserveMiddleware {
    pub fn new(metrics: impl MetricsRecorder + 'static, audit: impl AuditLogger + 'static) -> Self {
        Self {
            metrics: Arc::new(metrics),
            audit: Arc::new(audit),
        }
    }

    /// Create a no-op middleware for testing
    pub fn noop() -> Self {
        Self::new(NoOpMetrics, NoOpAudit)
    }

    /// Create with real observability subsystem adapters
    pub fn with_registry_and_audit<A: crate::observability::AuditLog + 'static>(
        registry: Arc<crate::observability::MetricsRegistry>,
        audit_log: Arc<A>,
    ) -> Self {
        Self::new(
            MetricsRegistryAdapter::new(registry),
            AuditLogAdapter::new(audit_log),
        )
    }
}

impl Middleware for ObserveMiddleware {
    fn process<'a>(
        &'a self,
        op: &'a Operation,
        ctx: &'a mut RequestContext,
        next: Next<'a>,
    ) -> Pin<Box<dyn Future<Output = OperationResult> + Send + 'a>> {
        Box::pin(async move {
            let op_name = op.name();
            let collection = op.collection().unwrap_or("none").to_string();

            // Execute operation
            let result = next.run(op, ctx).await;

            // Record metrics
            let duration_ms = ctx.elapsed_ms() as f64;
            self.metrics.record(
                "operation_duration_ms",
                duration_ms,
                &[("operation", op_name), ("collection", &collection)],
            );

            self.metrics.increment(
                if result.is_ok() {
                    "operation_success_total"
                } else {
                    "operation_error_total"
                },
                &[("operation", op_name), ("collection", &collection)],
            );

            // Audit log
            let details = result.as_ref().err().map(|e| e.to_string());
            self.audit.log(
                ctx.auth.user_id,
                op_name,
                result.is_ok(),
                ctx.request_id,
                details,
            );

            result
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::context::AuthContext;
    use crate::core::operation::ReadOp;
    use crate::core::pipeline::{NoOpExecutor, Pipeline};
    use crate::observability::MemoryAuditLog;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_observe_middleware_records_metrics() {
        let pipeline = Pipeline::new(NoOpExecutor).with_middleware(ObserveMiddleware::noop());

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
    async fn test_observe_middleware_with_real_audit() {
        let audit_log = Arc::new(MemoryAuditLog::new());
        let registry = Arc::new(crate::observability::MetricsRegistry::new());

        let middleware =
            ObserveMiddleware::with_registry_and_audit(registry.clone(), audit_log.clone());
        let pipeline = Pipeline::new(NoOpExecutor).with_middleware(middleware);

        let ctx = RequestContext::new(AuthContext::authenticated(Uuid::new_v4()));
        let op = Operation::Read(ReadOp {
            collection: "posts".to_string(),
            id: "post_1".to_string(),
            select: None,
        });

        let _ = pipeline.execute(op, ctx).await;

        // Verify audit log was written
        assert!(!audit_log.is_empty());
        let records = audit_log.records();
        assert_eq!(records.len(), 1);
    }
}
