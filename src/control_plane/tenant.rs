//! # Tenant Model
//!
//! Core types for multi-tenant management.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Tenant isolation model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IsolationModel {
    /// Schema-per-tenant: RLS-based isolation, all tenants in one DB
    Schema,
    /// Database-per-tenant: Separate database processes
    Database,
    /// Cluster-per-tenant: Dedicated cluster with replicas (future)
    Cluster,
}

impl Default for IsolationModel {
    fn default() -> Self {
        Self::Schema
    }
}

impl std::fmt::Display for IsolationModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Schema => write!(f, "schema"),
            Self::Database => write!(f, "database"),
            Self::Cluster => write!(f, "cluster"),
        }
    }
}

/// Tenant pricing plan
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Plan {
    /// Free tier: Limited resources
    Free,
    /// Pro tier: Production workloads
    Pro,
    /// Enterprise tier: Unlimited resources
    Enterprise,
}

impl Default for Plan {
    fn default() -> Self {
        Self::Free
    }
}

impl std::fmt::Display for Plan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Free => write!(f, "free"),
            Self::Pro => write!(f, "pro"),
            Self::Enterprise => write!(f, "enterprise"),
        }
    }
}

/// Tenant status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TenantStatus {
    /// Tenant is being provisioned
    Provisioning,
    /// Tenant is active and serving requests
    Active,
    /// Tenant is suspended (quota exceeded or billing issue)
    Suspended,
    /// Tenant is being deleted
    Deleting,
    /// Tenant has been deleted (soft delete)
    Deleted,
}

impl Default for TenantStatus {
    fn default() -> Self {
        Self::Provisioning
    }
}

/// Tenant configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantConfig {
    /// Database connection URL
    pub database_url: String,
    /// Port for database-per-tenant mode
    pub port: Option<u16>,
    /// Data directory for database-per-tenant mode
    pub data_dir: Option<String>,
    /// Process ID for database-per-tenant mode
    pub process_id: Option<u32>,
}

/// Core tenant model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    /// Unique tenant identifier
    pub tenant_id: Uuid,
    /// Tenant name (subdomain, e.g., "acme-corp")
    pub name: String,
    /// Pricing plan
    pub plan: Plan,
    /// Deployment region
    pub region: String,
    /// Isolation model
    pub isolation: IsolationModel,
    /// Current status
    pub status: TenantStatus,
    /// Database connection URL
    pub database_url: String,
    /// API key for authentication
    pub api_key: String,
    /// Tenant configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<TenantConfig>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Deletion timestamp (soft delete)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Tenant {
    /// Create a new tenant with default values
    pub fn new(name: String, plan: Plan, region: String, isolation: IsolationModel) -> Self {
        let tenant_id = Uuid::new_v4();
        let api_key = format!("ak_{}", Uuid::new_v4().to_string().replace("-", ""));
        let now = Utc::now();

        Self {
            tenant_id,
            name: name.clone(),
            plan,
            region: region.clone(),
            isolation,
            status: TenantStatus::Provisioning,
            database_url: format!("https://{}.aerodb.com", name),
            api_key,
            config: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    /// Check if tenant is active
    pub fn is_active(&self) -> bool {
        self.status == TenantStatus::Active
    }

    /// Check if tenant is deleted
    pub fn is_deleted(&self) -> bool {
        self.status == TenantStatus::Deleted || self.deleted_at.is_some()
    }

    /// Mark tenant as active
    pub fn activate(&mut self) {
        self.status = TenantStatus::Active;
        self.updated_at = Utc::now();
    }

    /// Mark tenant as suspended
    pub fn suspend(&mut self) {
        self.status = TenantStatus::Suspended;
        self.updated_at = Utc::now();
    }

    /// Mark tenant for deletion
    pub fn mark_deleted(&mut self) {
        self.status = TenantStatus::Deleted;
        self.deleted_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Update tenant configuration
    pub fn set_config(&mut self, config: TenantConfig) {
        self.database_url = config.database_url.clone();
        self.config = Some(config);
        self.updated_at = Utc::now();
    }
}

/// Request to create a new tenant
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTenantRequest {
    /// Tenant name (subdomain)
    pub name: String,
    /// Pricing plan
    #[serde(default)]
    pub plan: Plan,
    /// Deployment region
    #[serde(default = "default_region")]
    pub region: String,
    /// Isolation model
    #[serde(default)]
    pub isolation: IsolationModel,
}

fn default_region() -> String {
    "us-east-1".to_string()
}

/// Response after creating a tenant
#[derive(Debug, Clone, Serialize)]
pub struct CreateTenantResponse {
    /// Tenant ID
    pub tenant_id: Uuid,
    /// Database URL
    pub database_url: String,
    /// API key
    pub api_key: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl From<&Tenant> for CreateTenantResponse {
    fn from(tenant: &Tenant) -> Self {
        Self {
            tenant_id: tenant.tenant_id,
            database_url: tenant.database_url.clone(),
            api_key: tenant.api_key.clone(),
            created_at: tenant.created_at,
        }
    }
}

/// Tenant list item (summary view)
#[derive(Debug, Clone, Serialize)]
pub struct TenantListItem {
    /// Tenant ID
    pub tenant_id: Uuid,
    /// Tenant name
    pub name: String,
    /// Pricing plan
    pub plan: Plan,
    /// Current status
    pub status: TenantStatus,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Storage used (bytes)
    pub storage_used: u64,
    /// API requests this month
    pub api_requests_month: u64,
}

/// Tenant details (full view)
#[derive(Debug, Clone, Serialize)]
pub struct TenantDetails {
    /// Tenant ID
    pub tenant_id: Uuid,
    /// Tenant name
    pub name: String,
    /// Pricing plan
    pub plan: Plan,
    /// Deployment region
    pub region: String,
    /// Isolation model
    pub isolation: IsolationModel,
    /// Current status
    pub status: TenantStatus,
    /// Database URL
    pub database_url: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Quota limits
    pub quotas: super::quota::Quotas,
    /// Current usage
    pub usage: super::metering::UsageMetrics,
}

/// Request to update a tenant
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateTenantRequest {
    /// New pricing plan (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<Plan>,
    /// New status (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<TenantStatus>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tenant() {
        let tenant = Tenant::new(
            "acme-corp".to_string(),
            Plan::Pro,
            "us-east-1".to_string(),
            IsolationModel::Schema,
        );

        assert_eq!(tenant.name, "acme-corp");
        assert_eq!(tenant.plan, Plan::Pro);
        assert_eq!(tenant.isolation, IsolationModel::Schema);
        assert_eq!(tenant.status, TenantStatus::Provisioning);
        assert!(tenant.api_key.starts_with("ak_"));
        assert!(tenant.database_url.contains("acme-corp"));
    }

    #[test]
    fn test_tenant_lifecycle() {
        let mut tenant = Tenant::new(
            "test".to_string(),
            Plan::Free,
            "local".to_string(),
            IsolationModel::Schema,
        );

        assert!(!tenant.is_active());
        assert!(!tenant.is_deleted());

        tenant.activate();
        assert!(tenant.is_active());

        tenant.suspend();
        assert!(!tenant.is_active());
        assert_eq!(tenant.status, TenantStatus::Suspended);

        tenant.mark_deleted();
        assert!(tenant.is_deleted());
    }

    #[test]
    fn test_isolation_model_serialization() {
        let model = IsolationModel::Database;
        let json = serde_json::to_string(&model).unwrap();
        assert_eq!(json, "\"database\"");

        let parsed: IsolationModel = serde_json::from_str("\"schema\"").unwrap();
        assert_eq!(parsed, IsolationModel::Schema);
    }
}
