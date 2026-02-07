//! # Control Plane Module
//!
//! Multi-tenant hosting infrastructure for AeroDB.
//!
//! ## Components
//!
//! - `tenant`: Tenant model and types
//! - `registry`: Tenant storage and retrieval
//! - `provisioning`: Tenant provisioning orchestration
//! - `schema_provisioner`: Schema-per-tenant (RLS-based)
//! - `database_provisioner`: Database-per-tenant (separate processes)
//! - `quota`: Quota definitions and enforcement
//! - `metering`: Usage tracking
//! - `billing`: Invoice generation
//! - `errors`: Control plane errors
//!
//! ## Isolation Models
//!
//! 1. **Schema-per-Tenant**: All tenants in one DB, RLS-based isolation
//! 2. **Database-per-Tenant**: Separate database processes per tenant
//! 3. **Cluster-per-Tenant**: Dedicated cluster (future)

pub mod billing;
pub mod database_provisioner;
pub mod errors;
pub mod metering;
pub mod provisioning;
pub mod quota;
pub mod registry;
pub mod schema_provisioner;
pub mod tenant;

pub use billing::*;
pub use errors::*;
pub use metering::*;
pub use provisioning::*;
pub use quota::*;
pub use registry::*;
pub use tenant::*;
