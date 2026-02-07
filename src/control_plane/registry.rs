//! # Tenant Registry
//!
//! Storage and retrieval of tenant metadata.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

use super::errors::{ControlPlaneError, ControlPlaneResult};
use super::tenant::{Tenant, TenantListItem, TenantStatus, UpdateTenantRequest};
use super::metering::UsageTracker;

/// In-memory tenant registry
/// In production, use a persistent database
#[derive(Debug, Clone)]
pub struct TenantRegistry {
    /// Tenants by ID
    tenants: Arc<RwLock<HashMap<Uuid, Tenant>>>,
    /// Tenant ID by name (for uniqueness check)
    names: Arc<RwLock<HashMap<String, Uuid>>>,
    /// Tenant ID by API key
    api_keys: Arc<RwLock<HashMap<String, Uuid>>>,
    /// Usage tracker
    usage_tracker: Arc<UsageTracker>,
}

impl TenantRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            tenants: Arc::new(RwLock::new(HashMap::new())),
            names: Arc::new(RwLock::new(HashMap::new())),
            api_keys: Arc::new(RwLock::new(HashMap::new())),
            usage_tracker: Arc::new(UsageTracker::new()),
        }
    }

    /// Create with shared usage tracker
    pub fn with_usage_tracker(usage_tracker: Arc<UsageTracker>) -> Self {
        Self {
            tenants: Arc::new(RwLock::new(HashMap::new())),
            names: Arc::new(RwLock::new(HashMap::new())),
            api_keys: Arc::new(RwLock::new(HashMap::new())),
            usage_tracker,
        }
    }

    /// Get the usage tracker
    pub fn usage_tracker(&self) -> Arc<UsageTracker> {
        self.usage_tracker.clone()
    }

    /// Validate tenant name
    fn validate_name(name: &str) -> ControlPlaneResult<()> {
        // Must be 3-63 characters
        if name.len() < 3 || name.len() > 63 {
            return Err(ControlPlaneError::InvalidTenantName {
                name: name.to_string(),
                reason: "Name must be 3-63 characters".to_string(),
            });
        }

        // Must start with a letter
        if !name.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false) {
            return Err(ControlPlaneError::InvalidTenantName {
                name: name.to_string(),
                reason: "Name must start with a lowercase letter".to_string(),
            });
        }

        // Must contain only lowercase letters, numbers, and hyphens
        if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err(ControlPlaneError::InvalidTenantName {
                name: name.to_string(),
                reason: "Name must contain only lowercase letters, numbers, and hyphens".to_string(),
            });
        }

        // Must not end with a hyphen
        if name.ends_with('-') {
            return Err(ControlPlaneError::InvalidTenantName {
                name: name.to_string(),
                reason: "Name must not end with a hyphen".to_string(),
            });
        }

        // Must not contain consecutive hyphens
        if name.contains("--") {
            return Err(ControlPlaneError::InvalidTenantName {
                name: name.to_string(),
                reason: "Name must not contain consecutive hyphens".to_string(),
            });
        }

        Ok(())
    }

    /// Check if name is available
    pub fn is_name_available(&self, name: &str) -> bool {
        let names = self.names.read().unwrap();
        !names.contains_key(name)
    }

    /// Insert a new tenant
    pub fn insert(&self, tenant: Tenant) -> ControlPlaneResult<()> {
        // Validate name
        Self::validate_name(&tenant.name)?;

        // Check name uniqueness
        let mut names = self.names.write().unwrap();
        if names.contains_key(&tenant.name) {
            return Err(ControlPlaneError::TenantNameExists {
                name: tenant.name.clone(),
            });
        }

        // Insert into all indexes
        let mut tenants = self.tenants.write().unwrap();
        let mut api_keys = self.api_keys.write().unwrap();

        names.insert(tenant.name.clone(), tenant.tenant_id);
        api_keys.insert(tenant.api_key.clone(), tenant.tenant_id);
        tenants.insert(tenant.tenant_id, tenant);

        Ok(())
    }

    /// Get tenant by ID
    pub fn get(&self, tenant_id: Uuid) -> ControlPlaneResult<Tenant> {
        let tenants = self.tenants.read().unwrap();
        tenants
            .get(&tenant_id)
            .cloned()
            .ok_or_else(|| ControlPlaneError::TenantNotFound {
                tenant_id: tenant_id.to_string(),
            })
    }

    /// Get tenant by name
    pub fn get_by_name(&self, name: &str) -> ControlPlaneResult<Tenant> {
        let names = self.names.read().unwrap();
        let tenant_id = names
            .get(name)
            .ok_or_else(|| ControlPlaneError::TenantNotFound {
                tenant_id: name.to_string(),
            })?;

        self.get(*tenant_id)
    }

    /// Get tenant by API key
    pub fn get_by_api_key(&self, api_key: &str) -> ControlPlaneResult<Tenant> {
        let api_keys = self.api_keys.read().unwrap();
        let tenant_id = api_keys
            .get(api_key)
            .ok_or_else(|| ControlPlaneError::TenantNotFound {
                tenant_id: "invalid api key".to_string(),
            })?;

        self.get(*tenant_id)
    }

    /// List all tenants (not deleted)
    pub fn list(&self) -> Vec<TenantListItem> {
        let tenants = self.tenants.read().unwrap();
        tenants
            .values()
            .filter(|t| !t.is_deleted())
            .map(|t| {
                let usage = self.usage_tracker.get_current_usage(t.tenant_id);
                TenantListItem {
                    tenant_id: t.tenant_id,
                    name: t.name.clone(),
                    plan: t.plan,
                    status: t.status,
                    created_at: t.created_at,
                    storage_used: usage.storage_bytes,
                    api_requests_month: usage.api_requests,
                }
            })
            .collect()
    }

    /// Update tenant
    pub fn update(&self, tenant_id: Uuid, update: UpdateTenantRequest) -> ControlPlaneResult<Tenant> {
        let mut tenants = self.tenants.write().unwrap();
        let tenant = tenants
            .get_mut(&tenant_id)
            .ok_or_else(|| ControlPlaneError::TenantNotFound {
                tenant_id: tenant_id.to_string(),
            })?;

        if let Some(plan) = update.plan {
            tenant.plan = plan;
        }
        if let Some(status) = update.status {
            tenant.status = status;
        }
        tenant.updated_at = chrono::Utc::now();

        Ok(tenant.clone())
    }

    /// Delete tenant (soft delete)
    pub fn delete(&self, tenant_id: Uuid) -> ControlPlaneResult<()> {
        let mut tenants = self.tenants.write().unwrap();
        let tenant = tenants
            .get_mut(&tenant_id)
            .ok_or_else(|| ControlPlaneError::TenantNotFound {
                tenant_id: tenant_id.to_string(),
            })?;

        tenant.mark_deleted();

        // Remove from name index (allow reuse)
        let mut names = self.names.write().unwrap();
        names.remove(&tenant.name);

        // Remove from API key index
        let mut api_keys = self.api_keys.write().unwrap();
        api_keys.remove(&tenant.api_key);

        Ok(())
    }

    /// Activate tenant
    pub fn activate(&self, tenant_id: Uuid) -> ControlPlaneResult<()> {
        let mut tenants = self.tenants.write().unwrap();
        let tenant = tenants
            .get_mut(&tenant_id)
            .ok_or_else(|| ControlPlaneError::TenantNotFound {
                tenant_id: tenant_id.to_string(),
            })?;

        tenant.activate();
        Ok(())
    }

    /// Suspend tenant
    pub fn suspend(&self, tenant_id: Uuid) -> ControlPlaneResult<()> {
        let mut tenants = self.tenants.write().unwrap();
        let tenant = tenants
            .get_mut(&tenant_id)
            .ok_or_else(|| ControlPlaneError::TenantNotFound {
                tenant_id: tenant_id.to_string(),
            })?;

        tenant.suspend();
        Ok(())
    }

    /// Update tenant configuration
    pub fn set_config(&self, tenant_id: Uuid, config: super::tenant::TenantConfig) -> ControlPlaneResult<()> {
        let mut tenants = self.tenants.write().unwrap();
        let tenant = tenants
            .get_mut(&tenant_id)
            .ok_or_else(|| ControlPlaneError::TenantNotFound {
                tenant_id: tenant_id.to_string(),
            })?;

        tenant.set_config(config);
        Ok(())
    }

    /// Count total tenants (including deleted)
    pub fn count(&self) -> usize {
        let tenants = self.tenants.read().unwrap();
        tenants.len()
    }

    /// Count active tenants
    pub fn count_active(&self) -> usize {
        let tenants = self.tenants.read().unwrap();
        tenants.values().filter(|t| t.status == TenantStatus::Active).count()
    }
}

impl Default for TenantRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control_plane::tenant::{IsolationModel, Plan};

    #[test]
    fn test_tenant_crud() {
        let registry = TenantRegistry::new();

        // Create tenant
        let tenant = Tenant::new(
            "acme-corp".to_string(),
            Plan::Pro,
            "us-east-1".to_string(),
            IsolationModel::Schema,
        );
        let tenant_id = tenant.tenant_id;
        registry.insert(tenant).unwrap();

        // Get by ID
        let retrieved = registry.get(tenant_id).unwrap();
        assert_eq!(retrieved.name, "acme-corp");

        // Get by name
        let by_name = registry.get_by_name("acme-corp").unwrap();
        assert_eq!(by_name.tenant_id, tenant_id);

        // List
        let list = registry.list();
        assert_eq!(list.len(), 1);

        // Delete
        registry.delete(tenant_id).unwrap();
        let deleted = registry.get(tenant_id).unwrap();
        assert!(deleted.is_deleted());

        // List should be empty after delete
        let list_after = registry.list();
        assert_eq!(list_after.len(), 0);
    }

    #[test]
    fn test_name_validation() {
        let registry = TenantRegistry::new();

        // Valid names
        assert!(TenantRegistry::validate_name("acme").is_ok());
        assert!(TenantRegistry::validate_name("acme-corp").is_ok());
        assert!(TenantRegistry::validate_name("acme123").is_ok());
        assert!(TenantRegistry::validate_name("a-b-c-123").is_ok());

        // Invalid names
        assert!(TenantRegistry::validate_name("ab").is_err()); // Too short
        assert!(TenantRegistry::validate_name("123abc").is_err()); // Starts with number
        assert!(TenantRegistry::validate_name("-acme").is_err()); // Starts with hyphen
        assert!(TenantRegistry::validate_name("acme-").is_err()); // Ends with hyphen
        assert!(TenantRegistry::validate_name("acme--corp").is_err()); // Consecutive hyphens
        assert!(TenantRegistry::validate_name("ACME").is_err()); // Uppercase
    }

    #[test]
    fn test_duplicate_name() {
        let registry = TenantRegistry::new();

        let tenant1 = Tenant::new(
            "acme".to_string(),
            Plan::Free,
            "local".to_string(),
            IsolationModel::Schema,
        );
        registry.insert(tenant1).unwrap();

        let tenant2 = Tenant::new(
            "acme".to_string(),
            Plan::Pro,
            "local".to_string(),
            IsolationModel::Schema,
        );
        let result = registry.insert(tenant2);
        assert!(result.is_err());
    }
}
