//! Observability HTTP Routes
//!
//! HTTP endpoints for system observability including health checks and metrics.

use axum::{http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use serde::Serialize;
use serde_json::Value;

use crate::observability::MetricsRegistry;

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Create observability routes
pub fn observability_routes() -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
}

/// Health check route (also available at root /health)
pub fn health_routes() -> Router {
    Router::new().route("/health", get(health_handler))
}

/// Health check handler
async fn health_handler() -> impl IntoResponse {
    let response = HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    (StatusCode::OK, Json(response))
}

/// Metrics handler - returns metrics as JSON
async fn metrics_handler() -> impl IntoResponse {
    let registry = MetricsRegistry::new();
    let json_str = registry.to_json();

    // Parse the JSON string to a Value for proper JSON response
    let metrics: Value = serde_json::from_str(&json_str)
        .unwrap_or_else(|_| serde_json::json!({"error": "Failed to serialize metrics"}));

    (StatusCode::OK, Json(metrics))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "ok".to_string(),
            version: "0.1.0".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("ok"));
    }
}
