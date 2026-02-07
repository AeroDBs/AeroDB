//! # REST API HTTP Server
//!
//! Axum-based HTTP server for REST endpoints.

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde_json::Value;

use crate::auth::jwt::{JwtConfig, JwtManager};
use crate::auth::rls::RlsContext;

use super::errors::{RestError, RestResult};
use super::handler::RestHandler;
use super::parser::QueryParams;
use super::response::{
    DeleteResponse, InsertResponse, ListResponse, SingleResponse, UpdateResponse,
};

/// REST API server state
pub struct RestServer<H: RestHandler> {
    handler: Arc<H>,
    jwt_manager: JwtManager,
}

impl<H: RestHandler + 'static> RestServer<H> {
    pub fn new(handler: H, jwt_config: JwtConfig) -> Self {
        Self {
            handler: Arc::new(handler),
            jwt_manager: JwtManager::new(jwt_config),
        }
    }

    /// Build the Axum router
    pub fn router(self) -> Router {
        let state = Arc::new(self);

        Router::new()
            .route("/rest/v1/{collection}", get(list_handler))
            .route("/rest/v1/{collection}", post(insert_handler))
            .route("/rest/v1/{collection}/{id}", get(get_handler))
            .route("/rest/v1/{collection}/{id}", patch(update_handler))
            .route("/rest/v1/{collection}/{id}", delete(delete_handler))
            .with_state(state)
    }
}

/// Shared state type
type ServerState<H> = Arc<RestServer<H>>;

/// Extract RLS context from headers
fn extract_context<H: RestHandler>(
    server: &RestServer<H>,
    headers: &HeaderMap,
) -> RestResult<RlsContext> {
    // Check for service role (apikey header)
    if let Some(apikey) = headers.get("apikey").and_then(|v| v.to_str().ok()) {
        // In production, validate the API key
        if apikey.starts_with("service_") {
            return Ok(RlsContext::service_role());
        }
    }

    // Check for bearer token
    if let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        if let Some(token) = auth.strip_prefix("Bearer ") {
            let claims = server
                .jwt_manager
                .validate_token(token)
                .map_err(|e| RestError::Auth(e))?;
            let user_id = JwtManager::get_user_id(&claims).map_err(|e| RestError::Auth(e))?;
            return Ok(RlsContext::authenticated(user_id));
        }
    }

    // Anonymous access
    Ok(RlsContext::anonymous())
}

/// List records handler
async fn list_handler<H: RestHandler + 'static>(
    State(server): State<ServerState<H>>,
    Path(collection): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Json<ListResponse<Value>>, RestError> {
    let ctx = extract_context(&server, &headers)?;
    let params = QueryParams::parse(&query)?;

    let result = server.handler.list(&collection, params, &ctx)?;
    Ok(Json(result))
}

/// Get single record handler
async fn get_handler<H: RestHandler + 'static>(
    State(server): State<ServerState<H>>,
    Path((collection, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<SingleResponse<Value>>, RestError> {
    let ctx = extract_context(&server, &headers)?;

    let result = server.handler.get(&collection, &id, &ctx)?;
    Ok(Json(result))
}

/// Insert record handler
async fn insert_handler<H: RestHandler + 'static>(
    State(server): State<ServerState<H>>,
    Path(collection): Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<(StatusCode, Json<InsertResponse<Value>>), RestError> {
    let ctx = extract_context(&server, &headers)?;

    let result = server.handler.insert(&collection, body, &ctx)?;
    Ok((StatusCode::CREATED, Json(result)))
}

/// Update record handler
async fn update_handler<H: RestHandler + 'static>(
    State(server): State<ServerState<H>>,
    Path((collection, id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<UpdateResponse<Value>>, RestError> {
    let ctx = extract_context(&server, &headers)?;

    let result = server.handler.update(&collection, &id, body, &ctx)?;
    Ok(Json(result))
}

/// Delete record handler
async fn delete_handler<H: RestHandler + 'static>(
    State(server): State<ServerState<H>>,
    Path((collection, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<DeleteResponse>, RestError> {
    let ctx = extract_context(&server, &headers)?;

    let result = server.handler.delete(&collection, &id, &ctx)?;
    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use super::super::handler::InMemoryRestHandler;
    use super::*;
    use crate::auth::rls::DefaultRlsEnforcer;

    fn create_test_server() -> RestServer<InMemoryRestHandler<DefaultRlsEnforcer>> {
        let handler = InMemoryRestHandler::new(DefaultRlsEnforcer::new());
        RestServer::new(handler, JwtConfig::default())
    }

    #[test]
    fn test_server_creation() {
        let server = create_test_server();
        let _router = server.router();
        // Server creates successfully
    }
}
