//! API Layer for aerodb
//!
//! The API Layer orchestrates all subsystems behind a single global lock.
//!
//! # Design Principles
//!
//! - Single global mutex for all operations
//! - Strict request handling flow
//! - Error codes passed through unchanged
//! - No timestamps, no generated IDs, no metadata injection
//!
//! # Supported Operations
//!
//! - insert
//! - update
//! - delete
//! - query
//! - explain

mod errors;
mod handler;
mod request;
mod response;

pub use errors::{ApiError, ApiErrorCode, ApiResult};
pub use handler::{ApiHandler, Subsystems};
pub use request::{DeleteRequest, InsertRequest, QueryRequest, Request, UpdateRequest};
pub use response::{ErrorResponse, Response, SuccessResponse};
