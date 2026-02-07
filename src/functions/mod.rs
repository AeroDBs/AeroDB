//! # AeroDB Serverless Functions Module
//!
//! Phase 12: Serverless Functions
//!
//! WebAssembly-based serverless functions with HTTP, database,
//! and scheduled triggers.

pub mod errors;
pub mod function;
pub mod invoker;
pub mod registry;
pub mod runtime;
pub mod scheduler;
pub mod store;
pub mod trigger;

pub use errors::{FunctionError, FunctionResult};
pub use function::{Function, FunctionConfig};
pub use invoker::{InvocationContext, InvocationResult, Invoker};
pub use registry::FunctionRegistry;
pub use runtime::{ExecutionContext, ExecutionResult, RuntimeConfig, WasmRuntime, WasmtimeRuntime};
pub use scheduler::Scheduler;
pub use trigger::TriggerType;
