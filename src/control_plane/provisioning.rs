//! # Provisioning Orchestration
//!
//! Coordinates tenant provisioning across isolation models.

use std::sync::Arc;
use uuid::Uuid;

use super::database_provisioner::DatabaseProvisioner;
use super::errors::{ControlPlaneError, ControlPlaneResult};
use super::registry::TenantRegistry;
use super::schema_provisioner::SchemaProvisioner;
use super::tenant::{CreateTenantRequest, CreateTenantResponse, IsolationModel, Tenant, TenantConfig};

/// Provisioning orchestrator
#[derive(Clone)]
pub struct ProvisioningService {
    /// Tenant registry
    registry: Arc<TenantRegistry>,
    /// Schema provisioner
    schema_provisioner: Arc<SchemaProvisioner>,
    /// Database provisioner
    database_provisioner: Arc<DatabaseProvisioner>,
}

impl ProvisioningService {
    /// Create a new provisioning service
    pub fn new(registry: Arc<TenantRegistry>) -> Self {
        Self {
            registry,
            schema_provisioner: Arc::new(SchemaProvisioner::new()),
            database_provisioner: Arc::new(DatabaseProvisioner::new()),
        }
    }

    /// Create with custom provisioners
    pub fn with_provisioners(
        registry: Arc<TenantRegistry>,
        schema_provisioner: Arc<SchemaProvisioner>,
        database_provisioner: Arc<DatabaseProvisioner>,
    ) -> Self {
        Self {
            registry,
            schema_provisioner,
            database_provisioner,
        }
    }

    /// Get the registry
    pub fn registry(&self) -> Arc<TenantRegistry> {
        self.registry.clone()
    }

    /// Create and provision a new tenant
    pub async fn create_tenant(
        &self,
        request: CreateTenantRequest,
    ) -> ControlPlaneResult<CreateTenantResponse> {
        // Create tenant object
        let mut tenant = Tenant::new(
            request.name.clone(),
            request.plan,
            request.region.clone(),
            request.isolation,
        );

        // Insert into registry (validates name, checks uniqueness)
        self.registry.insert(tenant.clone())?;

        // Provision based on isolation model
        let config = match request.isolation {
            IsolationModel::Schema => {
                self.schema_provisioner.provision(&tenant).await?
            }
            IsolationModel::Database => {
                self.database_provisioner.provision(&tenant).await?
            }
            IsolationModel::Cluster => {
                // Cluster provisioning not yet implemented
                return Err(ControlPlaneError::InvalidIsolationModel {
                    model: "cluster".to_string(),
                    reason: "Cluster isolation is not yet implemented".to_string(),
                });
            }
        };

        // Update tenant with config and activate
        self.registry.set_config(tenant.tenant_id, config)?;
        self.registry.activate(tenant.tenant_id)?;

        // Return response
        Ok(CreateTenantResponse::from(&tenant))
    }

    /// Delete a tenant
    pub async fn delete_tenant(&self, tenant_id: Uuid) -> ControlPlaneResult<()> {
        // Get tenant
        let tenant = self.registry.get(tenant_id)?;

        // Check if already deleted
        if tenant.is_deleted() {
            return Err(ControlPlaneError::TenantDeleted {
                tenant_id: tenant_id.to_string(),
            });
        }

        // Deprovision based on isolation model
        match tenant.isolation {
            IsolationModel::Schema => {
                self.schema_provisioner.deprovision(&tenant).await?;
            }
            IsolationModel::Database => {
                self.database_provisioner.deprovision(&tenant).await?;
            }
            IsolationModel::Cluster => {
                // Cluster deprovisioning not yet implemented
            }
        }

        // Mark as deleted in registry
        self.registry.delete(tenant_id)?;

        Ok(())
    }

    /// Get tenant details
    pub fn get_tenant(&self, tenant_id: Uuid) -> ControlPlaneResult<Tenant> {
        self.registry.get(tenant_id)
    }

    /// Get tenant by name
    pub fn get_tenant_by_name(&self, name: &str) -> ControlPlaneResult<Tenant> {
        self.registry.get_by_name(name)
    }

    /// Get tenant by API key
    pub fn get_tenant_by_api_key(&self, api_key: &str) -> ControlPlaneResult<Tenant> {
        self.registry.get_by_api_key(api_key)
    }

    /// List all tenants
    pub fn list_tenants(&self) -> Vec<super::tenant::TenantListItem> {
        self.registry.list()
    }

    /// Check if tenant is active and not suspended
    pub fn verify_tenant_access(&self, tenant_id: Uuid) -> ControlPlaneResult<Tenant> {
        let tenant = self.registry.get(tenant_id)?;

        if tenant.is_deleted() {
            return Err(ControlPlaneError::TenantDeleted {
                tenant_id: tenant_id.to_string(),
            });
        }

        if tenant.status == super::tenant::TenantStatus::Suspended {
            return Err(ControlPlaneError::TenantSuspended {
                tenant_id: tenant_id.to_string(),
            });
        }

        Ok(tenant)
    }

    /// Suspend a tenant (e.g., for quota exceeded)
    pub fn suspend_tenant(&self, tenant_id: Uuid) -> ControlPlaneResult<()> {
        self.registry.suspend(tenant_id)
    }

    /// Reactivate a suspended tenant
    pub fn activate_tenant(&self, tenant_id: Uuid) -> ControlPlaneResult<()> {
        self.registry.activate(tenant_id)
    }
}

/// Trait for provisioners
pub trait Provisioner: Send + Sync {
    /// Provision resources for a tenant
    fn provision(
        &self,
        tenant: &Tenant,
    ) -> impl std::future::Future<Output = ControlPlaneResult<TenantConfig>> + Send;

    /// Deprovision resources for a tenant
    fn deprovision(
        &self,
        tenant: &Tenant,
    ) -> impl std::future::Future<Output = ControlPlaneResult<()>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control_plane::tenant::Plan;

    #[tokio::test]
    async fn test_create_schema_tenant() {
        let registry = Arc::new(TenantRegistry::new());
        let service = ProvisioningService::new(registry);

        let request = CreateTenantRequest {
            name: "test-tenant".to_string(),
            plan: Plan::Free,
            region: "local".to_string(),
            isolation: IsolationModel::Schema,
        };

        let response = service.create_tenant(request).await.unwrap();
        assert!(!response.api_key.is_empty());
        assert!(response.database_url.contains("test-tenant"));

        // Verify tenant is active
        let tenant = service.get_tenant(response.tenant_id).unwrap();
        assert!(tenant.is_active());
    }

    #[tokio::test]
    async fn test_delete_tenant() {
        let registry = Arc::new(TenantRegistry::new());
        let service = ProvisioningService::new(registry);

        let request = CreateTenantRequest {
            name: "delete-me".to_string(),
            plan: Plan::Free,
            region: "local".to_string(),
            isolation: IsolationModel::Schema,
        };

        let response = service.create_tenant(request).await.unwrap();
        service.delete_tenant(response.tenant_id).await.unwrap();

        // Tenant should be marked as deleted
        let tenant = service.get_tenant(response.tenant_id).unwrap();
        assert!(tenant.is_deleted());
    }

    #[tokio::test]
    async fn test_cluster_not_implemented() {
        let registry = Arc::new(TenantRegistry::new());
        let service = ProvisioningService::new(registry);

        let request = CreateTenantRequest {
            name: "cluster-tenant".to_string(),
            plan: Plan::Enterprise,
            region: "local".to_string(),
            isolation: IsolationModel::Cluster,
        };

        let result = service.create_tenant(request).await;
        assert!(result.is_err());
    }
}
