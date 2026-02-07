//! Schema Validator subsystem for aerodb
//!
//! Per SCHEMA.md, schemas are mandatory first-class artifacts enforced at write time.
//!
//! # Design Principles
//!
//! - Mandatory on all writes (S1)
//! - Validation before WAL (S2)
//! - Explicit version binding (S3)
//! - Violations abort writes (S4)
//! - No nulls, defaults, or coercion
//! - Deterministic validation

mod errors;
mod loader;
mod types;
mod validator;

pub use errors::{SchemaError, SchemaErrorCode, SchemaResult};
pub use loader::SchemaLoader;
pub use types::{FieldDef, FieldType, Schema};
pub use validator::SchemaValidator;
