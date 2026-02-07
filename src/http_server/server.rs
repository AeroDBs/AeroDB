//! # HTTP Server
//!
//! Main HTTP server combining all endpoint routers.
//!
//! This is the unified entry point for the AeroDB dashboard API.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

use super::auth_management_routes::auth_management_routes;
use super::auth_routes::{auth_routes, AuthState};
use super::backup_routes::{backup_routes, BackupState};
use super::cluster_routes::{cluster_routes, ClusterState};
use super::config::HttpServerConfig;
use super::database_routes::{database_routes, DatabaseState};
use super::functions_routes::{functions_routes, FunctionsState};
use super::observability_routes::{health_routes, observability_routes};
use super::realtime_routes::{realtime_routes, RealtimeState};
use super::setup_routes::{setup_routes, SetupState};
use super::storage_routes::{storage_routes, StorageState};

/// HTTP Server for AeroDB Dashboard
pub struct HttpServer {
    config: HttpServerConfig,
    router: Router,
}

impl HttpServer {
    /// Create a new HTTP server with default configuration
    pub fn new() -> Self {
        Self::with_config(HttpServerConfig::default())
    }

    /// Create a new HTTP server with custom configuration
    pub fn with_config(config: HttpServerConfig) -> Self {
        let router = Self::build_router(&config);
        Self { config, router }
    }

    /// Build the combined router with all endpoints
    fn build_router(config: &HttpServerConfig) -> Router {
        // Create shared states for each module
        let setup_state = Arc::new(SetupState::new());
        let auth_state = Arc::new(AuthState::new());
        let storage_state = Arc::new(StorageState::with_default_path());
        let database_state = Arc::new(DatabaseState::new());
        let functions_state = Arc::new(FunctionsState::new());
        let realtime_state = Arc::new(RealtimeState::new());
        let backup_state = Arc::new(BackupState::new());
        let cluster_state = Arc::new(ClusterState::new());

        // Configure CORS from config
        let cors = if config.cors_origins.is_empty() {
            // If no origins configured, use permissive for development
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        } else {
            // Use configured origins for production
            use tower_http::cors::AllowOrigin;
            let origins: Vec<_> = config
                .cors_origins
                .iter()
                .filter_map(|s| s.parse().ok())
                .collect();

            CorsLayer::new()
                .allow_origin(AllowOrigin::list(origins))
                .allow_methods(Any)
                .allow_headers(Any)
        };

        // Combine all routes
        Router::new()
            // Health check at root level
            .merge(health_routes())
            // Setup routes under /setup (first-run wizard, locked after complete)
            .nest("/setup", setup_routes(setup_state))
            // Auth routes under /auth
            .nest("/auth", auth_routes(auth_state.clone()))
            // Auth management routes (extends /auth with user management, sessions, RLS, etc.)
            .nest("/auth", auth_management_routes(auth_state))
            // Observability routes under /observability
            .nest("/observability", observability_routes())
            // Storage routes under /storage
            .nest("/storage", storage_routes(storage_state))
            // Database routes under /api
            .nest("/api", database_routes(database_state))
            // Functions routes under /functions
            .nest("/functions", functions_routes(functions_state))
            // Realtime routes under /realtime (includes WebSocket endpoint)
            .nest("/realtime", realtime_routes(realtime_state))
            // Backup routes under /backup
            .nest("/backup", backup_routes(backup_state))
            // Cluster routes under /cluster
            .nest("/cluster", cluster_routes(cluster_state))
            // Apply CORS middleware
            .layer(cors)
    }

    /// Get the socket address
    pub fn socket_addr(&self) -> String {
        self.config.socket_addr()
    }

    /// Get the router (for testing)
    pub fn router(self) -> Router {
        self.router
    }

    /// Start the HTTP server (async)
    pub async fn start(self) -> Result<(), std::io::Error> {
        let addr: SocketAddr = self
            .config
            .socket_addr()
            .parse()
            .expect("Invalid socket address");

        println!("Starting AeroDB HTTP server on {}", addr);
        println!("Dashboard API available at http://{}", addr);
        println!("Health check: http://{}/health", addr);
        println!("API endpoints:");
        println!("  - /auth/* - Authentication & user management");
        println!("  - /api/* - Database operations");
        println!("  - /storage/* - File storage");
        println!("  - /functions/* - Serverless functions");
        println!("  - /realtime/* - Subscriptions & WebSocket");
        println!("  - /backup/* - Backup & restore");
        println!("  - /cluster/* - Cluster management");
        println!("  - /observability/* - Metrics & monitoring");

        let listener = TcpListener::bind(addr).await?;
        axum::serve(listener, self.router).await?;

        Ok(())
    }
}

impl Default for HttpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let server = HttpServer::new();
        assert_eq!(server.socket_addr(), "0.0.0.0:54321");
    }

    #[test]
    fn test_server_with_custom_port() {
        let config = HttpServerConfig::with_port(8080);
        let server = HttpServer::with_config(config);
        assert_eq!(server.socket_addr(), "0.0.0.0:8080");
    }

    #[test]
    fn test_router_builds() {
        let server = HttpServer::new();
        let _router = server.router();
        // If we get here, router construction succeeded
    }
}
