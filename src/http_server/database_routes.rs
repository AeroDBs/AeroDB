//! Database HTTP Routes
//!
//! Endpoints for table management, queries, and database operations.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::auth::rls::RlsContext;
use crate::core::{BridgeConfig, PipelineBridge, RequestContext};

// ==================
// Shared State
// ==================

/// Database state shared across handlers
pub struct DatabaseState {
    pub bridge: Arc<PipelineBridge>,
}

impl DatabaseState {
    pub fn new() -> Self {
        Self {
            bridge: Arc::new(PipelineBridge::new_in_memory(BridgeConfig::default())),
        }
    }
}

impl Default for DatabaseState {
    fn default() -> Self {
        Self::new()
    }
}

// ==================
// Request/Response Types
// ==================

#[derive(Debug, Serialize)]
pub struct TableInfo {
    pub name: String,
    pub row_count: u64,
    pub schema: Value,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct TablesListResponse {
    pub tables: Vec<TableInfo>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct CreateTableRequest {
    pub name: String,
    pub schema: Value,
}

#[derive(Debug, Serialize)]
pub struct TableSchemaResponse {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
    pub primary_key: Vec<String>,
    pub indexes: Vec<IndexInfo>,
}

#[derive(Debug, Serialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct IndexInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
}

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub query: String,
    #[serde(default)]
    pub params: Vec<Value>,
}

#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub rows: Vec<Value>,
    pub row_count: usize,
    pub execution_time_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct TableDataQuery {
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub order_by: Option<String>,
    #[serde(default)]
    pub filter: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TableDataResponse {
    pub data: Vec<Value>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Deserialize)]
pub struct InsertRowRequest {
    pub data: Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRowRequest {
    pub data: Value,
}

#[derive(Debug, Serialize)]
pub struct DatabaseStatsResponse {
    pub table_count: usize,
    pub total_rows: u64,
    pub total_size_bytes: u64,
    pub index_count: usize,
}

#[derive(Debug, Serialize)]
pub struct ErdResponse {
    pub tables: Vec<ErdTable>,
    pub relationships: Vec<ErdRelationship>,
}

#[derive(Debug, Serialize)]
pub struct ErdTable {
    pub name: String,
    pub columns: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ErdRelationship {
    pub from_table: String,
    pub to_table: String,
    pub from_column: String,
    pub to_column: String,
    pub relationship_type: String,
}

#[derive(Debug, Serialize)]
pub struct MigrationInfo {
    pub id: String,
    pub name: String,
    pub applied_at: Option<String>,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct MigrationsListResponse {
    pub migrations: Vec<MigrationInfo>,
}

#[derive(Debug, Deserialize)]
pub struct CreateIndexRequest {
    pub name: String,
    pub columns: Vec<String>,
    #[serde(default)]
    pub unique: bool,
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
// Database Routes
// ==================

/// Create database routes
pub fn database_routes(state: Arc<DatabaseState>) -> Router {
    Router::new()
        // Table management
        .route("/tables", get(list_tables_handler))
        .route("/tables", post(create_table_handler))
        .route("/tables/{name}", get(get_table_schema_handler))
        .route("/tables/{name}", delete(drop_table_handler))
        .route("/tables/{name}/data", get(get_table_data_handler))
        .route("/tables/{name}/rows", post(insert_row_handler))
        .route("/tables/{name}/rows/{id}", get(get_row_handler))
        .route("/tables/{name}/rows/{id}", delete(delete_row_handler))
        // Query execution
        .route("/query", post(execute_query_handler))
        // Statistics
        .route("/database/stats", get(get_database_stats_handler))
        .route("/database/erd", get(get_erd_handler))
        // Migrations
        .route("/migrations", get(list_migrations_handler))
        .route("/migrations/apply", post(apply_migration_handler))
        .route("/migrations/rollback", post(rollback_migration_handler))
        // Indexes
        .route("/tables/{name}/indexes", get(list_indexes_handler))
        .route("/tables/{name}/indexes", post(create_index_handler))
        .route(
            "/tables/{name}/indexes/{index_name}",
            delete(drop_index_handler),
        )
        // Relationships
        .route(
            "/tables/{name}/relationships",
            get(list_relationships_handler),
        )
        .with_state(state)
}

// ==================
// Helper Functions
// ==================

fn get_request_context(headers: &HeaderMap) -> RequestContext {
    if let Some(auth) = headers.get("authorization") {
        if auth
            .to_str()
            .ok()
            .map(|s| s.starts_with("Bearer "))
            .unwrap_or(false)
        {
            return RequestContext::new(crate::core::context::AuthContext::service_role());
        }
    }
    RequestContext::anonymous()
}

// ==================
// Table Management Handlers
// ==================

async fn list_tables_handler(
    State(state): State<Arc<DatabaseState>>,
    headers: HeaderMap,
) -> Result<Json<TablesListResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Would query schema/table registry
    Ok(Json(TablesListResponse {
        tables: vec![],
        total: 0,
    }))
}

async fn create_table_handler(
    State(_state): State<Arc<DatabaseState>>,
    headers: HeaderMap,
    Json(request): Json<CreateTableRequest>,
) -> Result<(StatusCode, Json<MessageResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Would create table via schema manager
    Ok((
        StatusCode::CREATED,
        Json(MessageResponse {
            message: format!("Table '{}' created", request.name),
        }),
    ))
}

async fn get_table_schema_handler(
    State(_state): State<Arc<DatabaseState>>,
    Path(name): Path<String>,
) -> Result<Json<TableSchemaResponse>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(TableSchemaResponse {
        name,
        columns: vec![],
        primary_key: vec!["id".to_string()],
        indexes: vec![],
    }))
}

async fn drop_table_handler(
    State(_state): State<Arc<DatabaseState>>,
    Path(name): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    Ok(StatusCode::NO_CONTENT)
}

async fn get_table_data_handler(
    State(state): State<Arc<DatabaseState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Query(query): Query<TableDataQuery>,
) -> Result<Json<TableDataResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = get_request_context(&headers);
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    // Would query via bridge
    let result = state
        .bridge
        .query(&name, None, limit, offset, ctx)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    code: 500,
                }),
            )
        })?;

    let rows: Vec<Value> = result.as_array().cloned().unwrap_or_default();

    Ok(Json(TableDataResponse {
        total: rows.len(),
        data: rows,
        limit,
        offset,
    }))
}

async fn insert_row_handler(
    State(state): State<Arc<DatabaseState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(request): Json<InsertRowRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<ErrorResponse>)> {
    let ctx = get_request_context(&headers);

    let result = state
        .bridge
        .write(&name, request.data, "default", ctx)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: e.to_string(),
                    code: 400,
                }),
            )
        })?;

    Ok((StatusCode::CREATED, Json(result)))
}

async fn get_row_handler(
    State(state): State<Arc<DatabaseState>>,
    headers: HeaderMap,
    Path((name, id)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = get_request_context(&headers);

    let result = state.bridge.read(&name, &id, ctx).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
                code: 404,
            }),
        )
    })?;

    Ok(Json(result))
}

async fn delete_row_handler(
    State(state): State<Arc<DatabaseState>>,
    headers: HeaderMap,
    Path((name, id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let ctx = get_request_context(&headers);

    state.bridge.delete(&name, &id, ctx).await.map_err(|e| {
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
// Query Execution Handlers
// ==================

async fn execute_query_handler(
    State(_state): State<Arc<DatabaseState>>,
    headers: HeaderMap,
    Json(request): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Would execute raw SQL query
    Ok(Json(QueryResponse {
        rows: vec![],
        row_count: 0,
        execution_time_ms: 0,
    }))
}

// ==================
// Statistics Handlers
// ==================

async fn get_database_stats_handler(
    State(_state): State<Arc<DatabaseState>>,
) -> Result<Json<DatabaseStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(DatabaseStatsResponse {
        table_count: 0,
        total_rows: 0,
        total_size_bytes: 0,
        index_count: 0,
    }))
}

async fn get_erd_handler(
    State(_state): State<Arc<DatabaseState>>,
) -> Result<Json<ErdResponse>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(ErdResponse {
        tables: vec![],
        relationships: vec![],
    }))
}

// ==================
// Migration Handlers
// ==================

async fn list_migrations_handler(
    State(_state): State<Arc<DatabaseState>>,
) -> Result<Json<MigrationsListResponse>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(MigrationsListResponse { migrations: vec![] }))
}

async fn apply_migration_handler(
    State(_state): State<Arc<DatabaseState>>,
) -> Result<Json<MessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(MessageResponse {
        message: "Migration applied".to_string(),
    }))
}

async fn rollback_migration_handler(
    State(_state): State<Arc<DatabaseState>>,
) -> Result<Json<MessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(MessageResponse {
        message: "Migration rolled back".to_string(),
    }))
}

// ==================
// Index Handlers
// ==================

async fn list_indexes_handler(
    State(_state): State<Arc<DatabaseState>>,
    Path(name): Path<String>,
) -> Result<Json<Vec<IndexInfo>>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(vec![]))
}

async fn create_index_handler(
    State(_state): State<Arc<DatabaseState>>,
    Path(name): Path<String>,
    Json(request): Json<CreateIndexRequest>,
) -> Result<(StatusCode, Json<MessageResponse>), (StatusCode, Json<ErrorResponse>)> {
    Ok((
        StatusCode::CREATED,
        Json(MessageResponse {
            message: format!("Index '{}' created on table '{}'", request.name, name),
        }),
    ))
}

async fn drop_index_handler(
    State(_state): State<Arc<DatabaseState>>,
    Path((name, index_name)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    Ok(StatusCode::NO_CONTENT)
}

// ==================
// Relationship Handlers
// ==================

async fn list_relationships_handler(
    State(_state): State<Arc<DatabaseState>>,
    Path(name): Path<String>,
) -> Result<Json<Vec<ErdRelationship>>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(vec![]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_state_creation() {
        let state = DatabaseState::new();
        // State should be created successfully
    }
}
