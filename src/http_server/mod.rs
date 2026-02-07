//! # AeroDB HTTP Server Module
//!
//! Phase 13.5: Dashboard HTTP Server Integration
//!
//! This module provides an HTTP-based API server for the AeroDB dashboard.
//! It combines all endpoint routers into a unified Axum server.
//!
//! # Endpoints
//!
//! - `/health` - Health check
//! - `/setup/*` - First-run setup wizard (locked after complete)
//! - `/rest/v1/*` - REST API for database operations
//! - `/auth/*` - Authentication endpoints
//! - `/observability/*` - Metrics and monitoring
//! - `/storage/*` - File storage endpoints
//! - `/functions/*` - Serverless functions endpoints
//! - `/realtime/*` - Real-time subscriptions and WebSocket
//! - `/backup/*` - Backup and restore endpoints
//! - `/cluster/*` - Cluster management endpoints

pub mod auth_management_routes;
pub mod auth_routes;
pub mod backup_routes;
pub mod cluster_routes;
pub mod config;
pub mod database_routes;
pub mod functions_routes;
pub mod observability_routes;
pub mod realtime_routes;
pub mod server;
pub mod setup_routes;
pub mod storage_routes;

pub use config::HttpServerConfig;
pub use server::HttpServer;
