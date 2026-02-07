//! Query Planner subsystem for aerodb
//!
//! Per QUERY.md, the planner produces deterministic, bounded query plans.
//!
//! # Design Principles
//!
//! - Deterministic: Same inputs → same plan (T1)
//! - Bounded: All queries must have provable limits (Q1)
//! - Indexed: Filters only on indexed fields (Q2)
//! - Explicit: No guessing or implicit behavior (Q3)
//!
//! # Index Selection Priority (strict order)
//!
//! 1. Primary key equality (_id)
//! 2. Indexed equality predicate
//! 3. Indexed range predicate with limit
//!
//! Ties broken lexicographically by field name.

mod ast;
mod bounds;
mod errors;
mod explain;
mod planner;

pub use ast::{FilterOp, Predicate, Query, SortDirection, SortSpec};
pub use bounds::BoundednessProof;
pub use errors::{PlannerError, PlannerErrorCode, PlannerResult};
pub use explain::ExplainPlan;
pub use planner::{IndexMetadata, QueryPlan, QueryPlanner, ScanType, SchemaRegistry};
