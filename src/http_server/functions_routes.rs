//! Functions HTTP Routes
//!
//! Endpoints for serverless function management and invocation.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::functions::function::Function;
use crate::functions::invoker::{InvocationContext, InvocationResult, Invoker};
use crate::functions::registry::FunctionRegistry;
use crate::functions::trigger::TriggerType;

// ==================
// Shared State
// ==================

/// Functions state shared across handlers
pub struct FunctionsState {
    pub registry: Arc<FunctionRegistry>,
    pub invoker: Invoker,
}

impl FunctionsState {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(FunctionRegistry::new()),
            invoker: Invoker::new(),
        }
    }
}

impl Default for FunctionsState {
    fn default() -> Self {
        Self::new()
    }
}

// ==================
// Request/Response Types
// ==================

#[derive(Debug, Serialize)]
pub struct FunctionResponse {
    pub id: String,
    pub name: String,
    pub trigger_type: String,
    pub trigger_config: Value,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&Function> for FunctionResponse {
    fn from(func: &Function) -> Self {
        let (trigger_type, trigger_config) = match &func.trigger {
            TriggerType::Http { path, method } => (
                "http".to_string(),
                serde_json::json!({ "path": path, "method": format!("{:?}", method) }),
            ),
            TriggerType::Database { collection, event } => (
                "database".to_string(),
                serde_json::json!({ "collection": collection, "event": format!("{:?}", event) }),
            ),
            TriggerType::Schedule { cron } => {
                ("schedule".to_string(), serde_json::json!({ "cron": cron }))
            }
            TriggerType::Webhook { .. } => ("webhook".to_string(), serde_json::json!({})),
        };

        Self {
            id: func.id.to_string(),
            name: func.name.clone(),
            trigger_type,
            trigger_config,
            enabled: func.enabled,
            created_at: func.created_at.to_rfc3339(),
            updated_at: func.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FunctionsListResponse {
    pub functions: Vec<FunctionResponse>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct CreateFunctionRequest {
    pub name: String,
    pub trigger_type: String,
    #[serde(default)]
    pub trigger_config: Value,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub wasm_bytes: Option<Vec<u8>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFunctionRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InvokeFunctionRequest {
    #[serde(default)]
    pub payload: Value,
    #[serde(default)]
    pub async_mode: bool,
}

#[derive(Debug, Serialize)]
pub struct InvokeResponse {
    pub id: String,
    pub success: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

impl From<InvocationResult> for InvokeResponse {
    fn from(result: InvocationResult) -> Self {
        Self {
            id: result.id.to_string(),
            success: result.success,
            result: result.result,
            error: result.error,
            duration_ms: result.duration_ms,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FunctionLogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct FunctionLogsResponse {
    pub logs: Vec<FunctionLogEntry>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct InvocationHistoryEntry {
    pub id: String,
    pub function_id: String,
    pub timestamp: String,
    pub duration_ms: u64,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InvocationsResponse {
    pub invocations: Vec<InvocationHistoryEntry>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct FunctionStatsResponse {
    pub invocation_count: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub avg_duration_ms: f64,
    pub last_invoked_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FunctionVersion {
    pub version: String,
    pub created_at: String,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct VersionsResponse {
    pub versions: Vec<FunctionVersion>,
}

#[derive(Debug, Serialize)]
pub struct FunctionTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub trigger_type: String,
}

#[derive(Debug, Serialize)]
pub struct TemplatesResponse {
    pub templates: Vec<FunctionTemplate>,
}

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub since: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

// ==================
// Functions Routes
// ==================

/// Create functions routes
pub fn functions_routes(state: Arc<FunctionsState>) -> Router {
    Router::new()
        // CRUD operations
        .route("/", get(list_functions_handler))
        .route("/", post(create_function_handler))
        .route("/{id}", get(get_function_handler))
        .route("/{id}", patch(update_function_handler))
        .route("/{id}", delete(delete_function_handler))
        // Invocation
        .route("/{id}/invoke", post(invoke_function_handler))
        // Logs and history
        .route("/{id}/logs", get(get_function_logs_handler))
        .route("/{id}/invocations", get(get_invocations_handler))
        .route("/{id}/stats", get(get_function_stats_handler))
        // Versioning
        .route("/{id}/versions", get(list_versions_handler))
        // Templates
        .route("/templates", get(list_templates_handler))
        .with_state(state)
}

// ==================
// Helper Functions
// ==================

fn parse_trigger(trigger_type: &str, config: &Value) -> Option<TriggerType> {
    match trigger_type.to_lowercase().as_str() {
        "http" => {
            let path = config.get("path").and_then(|v| v.as_str()).unwrap_or("/");
            Some(TriggerType::http(path.to_string()))
        }
        "database" => {
            let collection = config.get("collection").and_then(|v| v.as_str())?;
            Some(TriggerType::database(
                collection.to_string(),
                crate::functions::trigger::DbEventType::Insert,
            ))
        }
        "schedule" => {
            let cron = config
                .get("cron")
                .and_then(|v| v.as_str())
                .unwrap_or("0 * * * *");
            Some(TriggerType::schedule(cron.to_string()))
        }
        _ => None,
    }
}

fn get_user_id_from_headers(headers: &HeaderMap) -> Option<Uuid> {
    // Would extract from JWT token
    None
}

// ==================
// CRUD Handlers
// ==================

async fn list_functions_handler(
    State(state): State<Arc<FunctionsState>>,
    headers: HeaderMap,
) -> Result<Json<FunctionsListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let functions = state.registry.list();
    let response: Vec<FunctionResponse> = functions.iter().map(FunctionResponse::from).collect();

    Ok(Json(FunctionsListResponse {
        total: response.len(),
        functions: response,
    }))
}

async fn get_function_handler(
    State(state): State<Arc<FunctionsState>>,
    Path(id): Path<String>,
) -> Result<Json<FunctionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let function = state.registry.get(&id).map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
                code: 404,
            }),
        )
    })?;

    Ok(Json(FunctionResponse::from(&function)))
}

async fn create_function_handler(
    State(state): State<Arc<FunctionsState>>,
    headers: HeaderMap,
    Json(request): Json<CreateFunctionRequest>,
) -> Result<(StatusCode, Json<FunctionResponse>), (StatusCode, Json<ErrorResponse>)> {
    let trigger =
        parse_trigger(&request.trigger_type, &request.trigger_config).ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("Invalid trigger type: {}", request.trigger_type),
                    code: 400,
                }),
            )
        })?;

    // Use provided WASM bytes or empty placeholder
    let wasm_bytes = request.wasm_bytes.unwrap_or_else(|| {
        vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00] // Empty WASM module
    });

    let function = Function::new(request.name, trigger, wasm_bytes);

    state.registry.register(function.clone()).map_err(|e| {
        (
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: e.to_string(),
                code: 409,
            }),
        )
    })?;

    Ok((StatusCode::CREATED, Json(FunctionResponse::from(&function))))
}

async fn update_function_handler(
    State(_state): State<Arc<FunctionsState>>,
    Path(id): Path<String>,
    Json(_request): Json<UpdateFunctionRequest>,
) -> Result<Json<FunctionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Would need to add update capability to registry
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "Function update not yet implemented".to_string(),
            code: 501,
        }),
    ))
}

async fn delete_function_handler(
    State(state): State<Arc<FunctionsState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    state.registry.unregister(&id).map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
                code: 404,
            }),
        )
    })?;

    Ok(StatusCode::NO_CONTENT)
}

// ==================
// Invocation Handler
// ==================

async fn invoke_function_handler(
    State(state): State<Arc<FunctionsState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<InvokeFunctionRequest>,
) -> Result<Json<InvokeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let function = state.registry.get(&id).map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
                code: 404,
            }),
        )
    })?;

    let user_id = get_user_id_from_headers(&headers);
    let context = InvocationContext::new(&function, request.payload, user_id);

    let result = state.invoker.invoke(&function, context).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: 500,
            }),
        )
    })?;

    Ok(Json(InvokeResponse::from(result)))
}

// ==================
// Logs and History Handlers
// ==================

async fn get_function_logs_handler(
    State(_state): State<Arc<FunctionsState>>,
    Path(id): Path<String>,
    Query(query): Query<LogsQuery>,
) -> Result<Json<FunctionLogsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Would retrieve from log storage
    Ok(Json(FunctionLogsResponse {
        logs: vec![],
        total: 0,
    }))
}

async fn get_invocations_handler(
    State(_state): State<Arc<FunctionsState>>,
    Path(id): Path<String>,
) -> Result<Json<InvocationsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Would retrieve from invocation history store
    Ok(Json(InvocationsResponse {
        invocations: vec![],
        total: 0,
    }))
}

async fn get_function_stats_handler(
    State(_state): State<Arc<FunctionsState>>,
    Path(id): Path<String>,
) -> Result<Json<FunctionStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(FunctionStatsResponse {
        invocation_count: 0,
        success_count: 0,
        error_count: 0,
        avg_duration_ms: 0.0,
        last_invoked_at: None,
    }))
}

// ==================
// Versioning Handlers
// ==================

async fn list_versions_handler(
    State(_state): State<Arc<FunctionsState>>,
    Path(id): Path<String>,
) -> Result<Json<VersionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(VersionsResponse { versions: vec![] }))
}

// ==================
// Template Handlers
// ==================

async fn list_templates_handler(
    State(_state): State<Arc<FunctionsState>>,
) -> Result<Json<TemplatesResponse>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(TemplatesResponse {
        templates: vec![
            FunctionTemplate {
                id: "http-handler".to_string(),
                name: "HTTP Handler".to_string(),
                description: "Basic HTTP request handler".to_string(),
                trigger_type: "http".to_string(),
            },
            FunctionTemplate {
                id: "database-trigger".to_string(),
                name: "Database Trigger".to_string(),
                description: "Trigger on database changes".to_string(),
                trigger_type: "database".to_string(),
            },
            FunctionTemplate {
                id: "scheduled-job".to_string(),
                name: "Scheduled Job".to_string(),
                description: "Run on a schedule (cron)".to_string(),
                trigger_type: "schedule".to_string(),
            },
        ],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_functions_state_creation() {
        let state = FunctionsState::new();
        assert!(state.registry.is_empty());
    }

    #[test]
    fn test_parse_http_trigger() {
        let config = serde_json::json!({ "path": "/api/test" });
        let trigger = parse_trigger("http", &config);
        assert!(trigger.is_some());
    }
}
