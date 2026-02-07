//! Query Executor subsystem for aerodb
//!
//! Per QUERY.md, the executor consumes plans and produces deterministic results.
//!
//! # Execution Flow (strict order)
//!
//! 1. Use chosen_index to obtain candidate document offsets
//! 2. Read documents from storage
//! 3. Validate checksum on every read
//! 4. Filter documents strictly according to predicates
//! 5. Apply schema version filtering
//! 6. Apply sort (if specified)
//! 7. Apply limit
//! 8. Return ordered results
//!
//! # Invariants
//!
//! - T2: Deterministic execution
//! - D2: Checksum validation on every read
//! - F1: Fail loudly on corruption

mod errors;
mod executor;
mod filters;
mod result;
mod sorter;

pub use errors::{ExecutorError, ExecutorErrorCode, ExecutorResult};
pub use executor::{IndexLookup, QueryExecutor};
pub use filters::PredicateFilter;
pub use result::{ExecutionResult, ResultDocument};
pub use sorter::ResultSorter;
