//! Crash testing framework for AeroDB
//!
//! Per CRASH_TESTING.md, this module provides:
//! - Crash injection at deterministic points
//! - Subprocess management
//! - Post-crash validation

pub mod harness;
pub mod scenarios;
pub mod utils;

pub use harness::*;
pub use utils::*;

// Integration tests run via `cargo test --test crash_tests`
