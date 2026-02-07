//! Observability API
//!
//! Per DX_OBSERVABILITY_API.md:
//! - Read-only endpoints for state inspection
//! - Snapshot-explicit responses
//! - Deterministic output
//!
//! Read-only, Phase 4, no semantic authority.
//!
//! # Phase 7 Control Plane
//!
//! Per PHASE7_CONTROL_PLANE_ARCHITECTURE.md:
//! - Control plane API for operator control surfaces
//! - Confirmation-gated mutating commands
//! - Audit logging and observability

pub mod control_plane;
pub mod handlers;
pub mod response;
pub mod server;

pub use response::{ApiResponse, ObservedAt, SnapshotType};
pub use server::ObservabilityServer;
