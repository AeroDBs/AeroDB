//! Developer Experience & Visibility (Phase 4)
//!
//! Per DX_VISION.md:
//! - Makes AeroDB observable, explorable, and trustworthy
//! - Introduces NO new database semantics
//! - All components are read-only
//!
//! Per DX_INVARIANTS.md §P4-1:
//! - Phase 4 components have zero semantic authority
//! - Cannot create, modify, or influence database state
//!
//! Per DX_INVARIANTS.md §P4-16:
//! - Phase 4 MUST be fully disableable
//! - Disabling requires no migration, data changes, or behavior changes

pub mod api;
pub mod config;
pub mod explain;

pub use config::DxConfig;
