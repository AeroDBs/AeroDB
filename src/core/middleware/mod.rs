//! Middleware Traits and Implementations
//!
//! Middleware stages for the execution pipeline.

use std::future::Future;
use std::pin::Pin;

use super::context::RequestContext;
use super::operation::Operation;
use super::pipeline::{Next, OperationResult};

/// Middleware trait for pipeline stages
pub trait Middleware: Send + Sync {
    /// Process the operation, optionally modifying context
    fn process<'a>(
        &'a self,
        op: &'a Operation,
        ctx: &'a mut RequestContext,
        next: Next<'a>,
    ) -> Pin<Box<dyn Future<Output = OperationResult> + Send + 'a>>;
}

/// Composable middleware implementations
pub mod auth;
pub mod observe;
pub mod rls;
