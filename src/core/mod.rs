//! # AeroDB Core Module
//!
//! Unified abstractions for operation execution, middleware pipeline,
//! and policy enforcement. All subsystems route through this module.
//!
//! ## Design Principles
//!
//! - Single unified operation model for all requests
//! - Middleware-based execution pipeline
//! - Centralized policy/RLS enforcement
//! - Automatic observability hooks

pub mod adapter;
pub mod bridge;
pub mod context;
pub mod error;
pub mod executor;
pub mod middleware;
pub mod operation;
pub mod pipeline;
pub mod write_through;

pub use adapter::{AeroDbConfig, AeroDbStorageBackend, DurableAeroDbBackend};
pub use bridge::{BridgeConfig, PipelineBridge};
pub use context::{AuthContext, RequestContext, RlsFilter};
pub use error::{CoreError, CoreResult};
pub use executor::{InMemoryStorage, StorageBackend, UnifiedExecutor};
pub use middleware::Middleware;
pub use operation::Operation;
pub use pipeline::{Next, OperationExecutor, Pipeline};
pub use write_through::WriteThroughBackend;
