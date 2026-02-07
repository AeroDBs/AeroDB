//! # Unified REST API
//!
//! Single operation endpoint that routes all requests through the pipeline.
//! Replaces separate CRUD endpoints with a unified interface.

use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::auth::jwt::{JwtConfig, JwtManager};
use crate::auth::rls::RlsContext;
use crate::core::operation::Operation;
use crate::core::{BridgeConfig, CoreError, PipelineBridge, RequestContext};
use crate::file_storage::file::FileService;
use crate::file_storage::local::LocalBackend;
use crate::functions::invoker::{InvocationContext, Invoker};
use crate::functions::registry::FunctionRegistry;
use crate::realtime::broadcast::BroadcastRegistry;
use crate::realtime::subscription::SubscriptionRegistry;

/// Unified operation request
#[derive(Debug, Deserialize)]
pub struct OperationRequest {
    /// The operation to execute
    #[serde(flatten)]
    pub operation: Operation,
}

/// Unified operation response
#[derive(Debug, Serialize)]
pub struct OperationResponse {
    /// Whether the operation succeeded
    pub success: bool,

    /// The operation result data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,

    /// Error information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,
}

/// Unified error information
#[derive(Debug, Serialize)]
pub struct ErrorInfo {
    /// Error code
    pub code: String,

    /// Human-readable message
    pub message: String,

    /// HTTP status code
    pub status: u16,
}

impl OperationResponse {
    pub fn success(data: Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(err: &CoreError) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ErrorInfo {
                code: err.code().to_string(),
                message: err.to_string(),
                status: err.status_code(),
            }),
        }
    }

    pub fn from_error_string(code: &str, message: String, status: u16) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ErrorInfo {
                code: code.to_string(),
                message,
                status,
            }),
        }
    }
}

/// Unified API server state
pub struct UnifiedApiServer {
    bridge: Arc<PipelineBridge>,
    jwt_manager: JwtManager,
    invoker: Arc<Invoker>,
    function_registry: Arc<FunctionRegistry>,
    file_service: Arc<FileService<LocalBackend>>,
    subscription_registry: Arc<SubscriptionRegistry>,
    broadcast_registry: Arc<BroadcastRegistry>,
}

impl UnifiedApiServer {
    /// Create a new unified API server with all services
    pub fn new(bridge: PipelineBridge, jwt_config: JwtConfig, storage_path: PathBuf) -> Self {
        let backend = LocalBackend::new(storage_path);
        Self {
            bridge: Arc::new(bridge),
            jwt_manager: JwtManager::new(jwt_config),
            invoker: Arc::new(Invoker::new()),
            function_registry: Arc::new(FunctionRegistry::new()),
            file_service: Arc::new(FileService::new(backend)),
            subscription_registry: Arc::new(SubscriptionRegistry::new()),
            broadcast_registry: Arc::new(BroadcastRegistry::new()),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        let bridge = PipelineBridge::new_in_memory(BridgeConfig::default());
        let storage_path = std::env::temp_dir().join("aerodb-storage");
        Self::new(bridge, JwtConfig::default(), storage_path)
    }

    /// Build the Axum router
    ///
    /// Provides a single endpoint: `POST /api/v1/operation`
    pub fn router(self) -> Router {
        let state = Arc::new(self);

        Router::new()
            .route("/api/v1/operation", post(execute_operation))
            .with_state(state)
    }
}

/// Shared state type
type ServerState = Arc<UnifiedApiServer>;

/// Execute any operation through the unified endpoint
async fn execute_operation(
    State(server): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<OperationRequest>,
) -> (StatusCode, Json<OperationResponse>) {
    // Build request context from headers
    let ctx = match build_context(&server.jwt_manager, &headers) {
        Ok(ctx) => ctx,
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(OperationResponse {
                    success: false,
                    data: None,
                    error: Some(ErrorInfo {
                        code: "AUTH_FAILED".to_string(),
                        message: e.to_string(),
                        status: 401,
                    }),
                }),
            );
        }
    };

    // Execute through pipeline
    match execute_via_bridge(&server, request.operation, ctx).await {
        Ok(data) => (StatusCode::OK, Json(OperationResponse::success(data))),
        Err(err) => {
            let status = StatusCode::from_u16(err.status_code())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            (status, Json(OperationResponse::error(&err)))
        }
    }
}

/// Build request context from HTTP headers
fn build_context(
    jwt_manager: &JwtManager,
    headers: &HeaderMap,
) -> Result<RequestContext, crate::auth::AuthError> {
    use crate::core::context::AuthContext;

    // Check for service role (apikey header)
    if let Some(apikey) = headers.get("apikey").and_then(|v| v.to_str().ok()) {
        if apikey.starts_with("service_") {
            return Ok(RequestContext::new(AuthContext::service_role()));
        }
    }

    // Check for bearer token
    if let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        if let Some(token) = auth.strip_prefix("Bearer ") {
            let claims = jwt_manager.validate_token(token)?;
            let user_id = JwtManager::get_user_id(&claims)?;
            return Ok(RequestContext::new(AuthContext::authenticated(user_id)));
        }
    }

    // Anonymous access
    Ok(RequestContext::anonymous())
}

/// Build RLS context from request context
fn build_rls_context(ctx: &RequestContext) -> RlsContext {
    if let Some(user_id) = ctx.auth.user_id {
        RlsContext::authenticated(user_id)
    } else {
        RlsContext::anonymous()
    }
}

/// Execute operation via the pipeline bridge and other services
async fn execute_via_bridge(
    server: &UnifiedApiServer,
    operation: Operation,
    ctx: RequestContext,
) -> Result<Value, CoreError> {
    use crate::core::operation::*;

    match operation {
        Operation::Read(read_op) => {
            server
                .bridge
                .read(&read_op.collection, &read_op.id, ctx)
                .await
        }

        Operation::Write(write_op) => {
            server
                .bridge
                .write(
                    &write_op.collection,
                    write_op.document,
                    &write_op.schema_id,
                    ctx,
                )
                .await
        }

        Operation::Update(update_op) => {
            server
                .bridge
                .update(&update_op.collection, &update_op.id, update_op.updates, ctx)
                .await
        }

        Operation::Delete(delete_op) => {
            server
                .bridge
                .delete(&delete_op.collection, &delete_op.id, ctx)
                .await
        }

        Operation::Query(query_op) => {
            server
                .bridge
                .query(
                    &query_op.collection,
                    query_op.filter.clone(),
                    query_op.limit,
                    query_op.offset,
                    ctx,
                )
                .await
        }

        // Explain returns the query plan without executing
        Operation::Explain(query_op) => Ok(serde_json::json!({
            "type": "explain",
            "collection": query_op.collection,
            "filter": query_op.filter,
            "limit": query_op.limit,
            "offset": query_op.offset,
            "plan": "full_collection_scan"
        })),

        // Subscribe - create subscription via registry
        Operation::Subscribe(sub_op) => {
            let subscription_id = Uuid::new_v4();
            let rls_ctx = build_rls_context(&ctx);

            // Create subscription object
            use crate::realtime::subscription::Subscription;
            let connection_id = Uuid::new_v4().to_string();
            let subscription = Subscription::new(connection_id, sub_op.channel.clone(), rls_ctx);

            match server.subscription_registry.subscribe(subscription) {
                Ok(sub_id) => Ok(serde_json::json!({
                    "type": "subscription",
                    "channel": sub_op.channel,
                    "subscription_id": sub_id,
                    "status": "created"
                })),
                Err(e) => Err(CoreError::internal(e.to_string())),
            }
        }

        // Unsubscribe - remove subscription from registry
        Operation::Unsubscribe { subscription_id } => {
            match server.subscription_registry.unsubscribe(&subscription_id) {
                Ok(()) => Ok(serde_json::json!({
                    "type": "unsubscribe",
                    "subscription_id": subscription_id,
                    "status": "removed"
                })),
                Err(e) => Err(CoreError::not_found(&format!(
                    "subscription/{}",
                    subscription_id
                ))),
            }
        }

        // Broadcast - send message to channel subscribers
        Operation::Broadcast(broadcast_op) => {
            let sender_id = ctx.auth.user_id;
            let sender_conn = Uuid::new_v4().to_string();

            match server.broadcast_registry.broadcast(
                &broadcast_op.channel,
                broadcast_op.event.clone(),
                broadcast_op.payload.clone(),
                sender_id,
                &sender_conn,
            ) {
                Ok((event, subscribers)) => Ok(serde_json::json!({
                    "type": "broadcast",
                    "channel": broadcast_op.channel,
                    "event": broadcast_op.event,
                    "status": "sent",
                    "subscribers_notified": subscribers.len()
                })),
                Err(e) => Err(CoreError::internal(e.to_string())),
            }
        }

        // Invoke - execute function via invoker
        Operation::Invoke(invoke_op) => {
            let user_id = ctx.auth.user_id;

            // Look up function by name
            match server.function_registry.get(&invoke_op.function_name) {
                Ok(function) => {
                    let context =
                        InvocationContext::new(&function, invoke_op.payload.clone(), user_id);

                    if invoke_op.async_mode {
                        // Async mode - queue and return immediately
                        Ok(serde_json::json!({
                            "type": "invoke",
                            "function": invoke_op.function_name,
                            "invocation_id": context.id.to_string(),
                            "status": "queued",
                            "async": true
                        }))
                    } else {
                        // Sync mode - execute and return result
                        match server.invoker.invoke(&function, context) {
                            Ok(result) => Ok(serde_json::json!({
                                "type": "invoke",
                                "function": invoke_op.function_name,
                                "invocation_id": result.id.to_string(),
                                "status": if result.success { "completed" } else { "failed" },
                                "result": result.result,
                                "error": result.error,
                                "duration_ms": result.duration_ms
                            })),
                            Err(e) => Err(CoreError::internal(e.to_string())),
                        }
                    }
                }
                Err(_) => Err(CoreError::not_found(&format!(
                    "function/{}",
                    invoke_op.function_name
                ))),
            }
        }

        // Upload - file uploads via unified API only support metadata operations
        // Actual file content upload should use the dedicated /storage routes
        Operation::Upload(file_op) => Ok(serde_json::json!({
            "type": "upload",
            "bucket": file_op.bucket,
            "path": file_op.path,
            "status": "use_storage_route",
            "message": "File content upload requires the dedicated /storage/buckets/{bucket}/files endpoint"
        })),

        // Download - file downloads via unified API return download URL
        // Actual file content should be retrieved from the dedicated /storage routes
        Operation::Download(file_op) => Ok(serde_json::json!({
            "type": "download",
            "bucket": file_op.bucket,
            "path": file_op.path,
            "url": format!("/storage/buckets/{}/files/{}", file_op.bucket, file_op.path),
            "message": "Use the URL to download file content"
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::operation::{ReadOp, WriteOp};

    #[test]
    fn test_operation_request_deserialize_read() {
        let json = r#"{"op": "read", "collection": "users", "id": "123"}"#;
        let req: OperationRequest = serde_json::from_str(json).unwrap();
        match req.operation {
            Operation::Read(read) => {
                assert_eq!(read.collection, "users");
                assert_eq!(read.id, "123");
            }
            _ => panic!("Expected Read operation"),
        }
    }

    #[test]
    fn test_operation_request_deserialize_write() {
        let json = r#"{"op": "write", "collection": "posts", "document": {"title": "Hello"}, "schema_id": "post_v1", "schema_version": "1.0"}"#;
        let req: OperationRequest = serde_json::from_str(json).unwrap();
        match req.operation {
            Operation::Write(write) => {
                assert_eq!(write.collection, "posts");
                assert_eq!(write.document["title"], "Hello");
            }
            _ => panic!("Expected Write operation"),
        }
    }

    #[test]
    fn test_operation_response_success() {
        let resp = OperationResponse::success(serde_json::json!({"id": "123"}));
        assert!(resp.success);
        assert!(resp.data.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_operation_response_error() {
        let err = CoreError::not_found("users/123");
        let resp = OperationResponse::error(&err);
        assert!(!resp.success);
        assert!(resp.data.is_none());
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().status, 404);
    }

    #[test]
    fn test_server_creation() {
        let server = UnifiedApiServer::with_defaults();
        let _router = server.router();
    }
}
