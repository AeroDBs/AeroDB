//! # AeroDB REST API Module
//!
//! Provides HTTP endpoints for CRUD operations on all collections,
//! with RLS enforcement through the core pipeline.

pub mod database;
pub mod errors;
pub mod filter;
pub mod generator;
pub mod handler;
pub mod parser;
pub mod pipeline_handler;
pub mod response;
pub mod server;
pub mod unified_api;

pub use database::DatabaseFacade;
pub use errors::{RestError, RestResult};
pub use filter::{FilterExpr, FilterOperator};
pub use handler::RestHandler;
pub use parser::QueryParams;
pub use pipeline_handler::PipelineRestHandler;
pub use server::RestServer;
pub use unified_api::{OperationRequest, OperationResponse, UnifiedApiServer};
