//! # Database-per-Tenant Provisioner
//!
//! Separate database processes for complete isolation.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

use super::errors::{ControlPlaneError, ControlPlaneResult};
use super::tenant::{Tenant, TenantConfig};

/// Port range for tenant databases
const PORT_RANGE_START: u16 = 50000;
const PORT_RANGE_END: u16 = 60000;

/// Database-per-tenant provisioner
#[derive(Debug)]
pub struct DatabaseProvisioner {
    /// Base data directory
    base_data_dir: PathBuf,
    /// Allocated ports
    allocated_ports: Arc<RwLock<HashMap<Uuid, u16>>>,
    /// Next available port
    next_port: Arc<RwLock<u16>>,
    /// Whether to actually start processes
    start_processes: bool,
}

impl DatabaseProvisioner {
    /// Create a new database provisioner
    pub fn new() -> Self {
        Self {
            base_data_dir: PathBuf::from("/data/tenants"),
            allocated_ports: Arc::new(RwLock::new(HashMap::new())),
            next_port: Arc::new(RwLock::new(PORT_RANGE_START)),
            start_processes: false, // Default to false for safety
        }
    }

    /// Create with custom data directory
    pub fn with_data_dir(data_dir: PathBuf) -> Self {
        Self {
            base_data_dir: data_dir,
            allocated_ports: Arc::new(RwLock::new(HashMap::new())),
            next_port: Arc::new(RwLock::new(PORT_RANGE_START)),
            start_processes: false,
        }
    }

    /// Enable actual process spawning
    pub fn with_process_spawning(mut self) -> Self {
        self.start_processes = true;
        self
    }

    /// Allocate a port for a tenant
    fn allocate_port(&self, tenant_id: Uuid) -> ControlPlaneResult<u16> {
        let mut next = self.next_port.write().unwrap();
        let mut allocated = self.allocated_ports.write().unwrap();

        // Check if already allocated
        if let Some(&port) = allocated.get(&tenant_id) {
            return Ok(port);
        }

        // Find next available port
        let port = *next;
        if port >= PORT_RANGE_END {
            return Err(ControlPlaneError::ProvisioningFailed {
                tenant_id: tenant_id.to_string(),
                reason: "No available ports".to_string(),
            });
        }

        *next += 1;
        allocated.insert(tenant_id, port);

        Ok(port)
    }

    /// Release a port
    fn release_port(&self, tenant_id: Uuid) {
        let mut allocated = self.allocated_ports.write().unwrap();
        allocated.remove(&tenant_id);
    }

    /// Get data directory for a tenant
    pub fn tenant_data_dir(&self, tenant_id: Uuid) -> PathBuf {
        self.base_data_dir.join(tenant_id.to_string())
    }

    /// Provision database-per-tenant resources
    pub async fn provision(&self, tenant: &Tenant) -> ControlPlaneResult<TenantConfig> {
        let port = self.allocate_port(tenant.tenant_id)?;
        let data_dir = self.tenant_data_dir(tenant.tenant_id);

        if self.start_processes {
            // Create data directory
            tokio::fs::create_dir_all(&data_dir).await.map_err(|e| {
                ControlPlaneError::ProvisioningFailed {
                    tenant_id: tenant.tenant_id.to_string(),
                    reason: format!("Failed to create data directory: {}", e),
                }
            })?;

            // Initialize database
            let init_output = tokio::process::Command::new("aerodb")
                .args(&["init", "--data-dir", data_dir.to_str().unwrap()])
                .output()
                .await
                .map_err(|e| ControlPlaneError::ProcessError {
                    message: format!("Failed to initialize database: {}", e),
                })?;

            if !init_output.status.success() {
                return Err(ControlPlaneError::ProvisioningFailed {
                    tenant_id: tenant.tenant_id.to_string(),
                    reason: format!(
                        "Database init failed: {}",
                        String::from_utf8_lossy(&init_output.stderr)
                    ),
                });
            }

            // Start database process
            let _child = tokio::process::Command::new("aerodb")
                .args(&[
                    "serve",
                    "--port",
                    &port.to_string(),
                    "--data-dir",
                    data_dir.to_str().unwrap(),
                ])
                .spawn()
                .map_err(|e| ControlPlaneError::ProcessError {
                    message: format!("Failed to start database: {}", e),
                })?;

            // Wait for health check
            self.wait_for_health(port, 30).await?;
        }

        let database_url = format!("http://localhost:{}", port);

        Ok(TenantConfig {
            database_url,
            port: Some(port),
            data_dir: Some(data_dir.to_string_lossy().to_string()),
            process_id: None, // Would be set from child.id() in production
        })
    }

    /// Wait for database to be ready
    async fn wait_for_health(&self, port: u16, timeout_secs: u64) -> ControlPlaneResult<()> {
        let url = format!("http://localhost:{}/health", port);
        let client = reqwest::Client::new();

        let start = std::time::Instant::now();
        loop {
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => return Ok(()),
                _ => {
                    if start.elapsed().as_secs() > timeout_secs {
                        return Err(ControlPlaneError::ProvisioningFailed {
                            tenant_id: "unknown".to_string(),
                            reason: "Health check timeout".to_string(),
                        });
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
            }
        }
    }

    /// Deprovision database-per-tenant resources
    pub async fn deprovision(&self, tenant: &Tenant) -> ControlPlaneResult<()> {
        if self.start_processes {
            // Stop database process (would use stored PID in production)
            // For now, we just clean up
        }

        // Release port
        self.release_port(tenant.tenant_id);

        if self.start_processes {
            // Remove data directory
            let data_dir = self.tenant_data_dir(tenant.tenant_id);
            if data_dir.exists() {
                tokio::fs::remove_dir_all(&data_dir).await.map_err(|e| {
                    ControlPlaneError::DeprovisioningFailed {
                        tenant_id: tenant.tenant_id.to_string(),
                        reason: format!("Failed to remove data directory: {}", e),
                    }
                })?;
            }
        }

        Ok(())
    }

    /// Get allocated port for a tenant
    pub fn get_port(&self, tenant_id: Uuid) -> Option<u16> {
        let allocated = self.allocated_ports.read().unwrap();
        allocated.get(&tenant_id).copied()
    }

    /// List all allocated tenants and ports
    pub fn list_allocated(&self) -> Vec<(Uuid, u16)> {
        let allocated = self.allocated_ports.read().unwrap();
        allocated.iter().map(|(id, port)| (*id, *port)).collect()
    }
}

impl Default for DatabaseProvisioner {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for DatabaseProvisioner {
    fn clone(&self) -> Self {
        Self {
            base_data_dir: self.base_data_dir.clone(),
            allocated_ports: self.allocated_ports.clone(),
            next_port: self.next_port.clone(),
            start_processes: self.start_processes,
        }
    }
}

/// Database instance information
#[derive(Debug, Clone)]
pub struct TenantDatabase {
    /// Tenant ID
    pub tenant_id: Uuid,
    /// Port
    pub port: u16,
    /// Data directory
    pub data_dir: PathBuf,
    /// Process ID (if running)
    pub process_id: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control_plane::tenant::{IsolationModel, Plan};

    #[test]
    fn test_port_allocation() {
        let provisioner = DatabaseProvisioner::new();
        let tenant_id = Uuid::new_v4();

        let port1 = provisioner.allocate_port(tenant_id).unwrap();
        assert!(port1 >= PORT_RANGE_START && port1 < PORT_RANGE_END);

        // Same tenant should get same port
        let port2 = provisioner.allocate_port(tenant_id).unwrap();
        assert_eq!(port1, port2);

        // Different tenant should get different port
        let tenant_id2 = Uuid::new_v4();
        let port3 = provisioner.allocate_port(tenant_id2).unwrap();
        assert_ne!(port1, port3);
    }

    #[test]
    fn test_data_dir() {
        let provisioner = DatabaseProvisioner::with_data_dir(PathBuf::from("/tmp/test"));
        let tenant_id = Uuid::new_v4();

        let dir = provisioner.tenant_data_dir(tenant_id);
        assert!(dir.starts_with("/tmp/test"));
        assert!(dir.to_str().unwrap().contains(&tenant_id.to_string()));
    }

    #[tokio::test]
    async fn test_provision() {
        let provisioner = DatabaseProvisioner::new();
        let tenant = Tenant::new(
            "db-tenant".to_string(),
            Plan::Pro,
            "local".to_string(),
            IsolationModel::Database,
        );

        let config = provisioner.provision(&tenant).await.unwrap();
        assert!(config.port.is_some());
        assert!(config.data_dir.is_some());
        assert!(config.database_url.contains("localhost"));
    }

    #[tokio::test]
    async fn test_deprovision() {
        let provisioner = DatabaseProvisioner::new();
        let tenant = Tenant::new(
            "deprov-tenant".to_string(),
            Plan::Free,
            "local".to_string(),
            IsolationModel::Database,
        );

        // Provision first
        let _config = provisioner.provision(&tenant).await.unwrap();
        assert!(provisioner.get_port(tenant.tenant_id).is_some());

        // Deprovision
        provisioner.deprovision(&tenant).await.unwrap();
        assert!(provisioner.get_port(tenant.tenant_id).is_none());
    }
}
