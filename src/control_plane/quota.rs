//! # Quota Management
//!
//! Resource quota definitions and enforcement for multi-tenancy.

use serde::{Deserialize, Serialize};

use super::errors::{ControlPlaneError, ControlPlaneResult};
use super::tenant::Plan;

/// Resource quotas per tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quotas {
    /// Maximum storage in bytes (documents + indexes)
    pub storage_bytes: u64,
    /// Maximum API requests per month
    pub api_requests_month: u64,
    /// Maximum concurrent realtime connections
    pub realtime_connections: u64,
    /// Maximum file storage in bytes
    pub file_storage_bytes: u64,
    /// Maximum number of collections/tables
    pub max_collections: u32,
    /// Maximum document size in bytes
    pub max_document_size: u64,
}

impl Quotas {
    /// Get quotas for a specific plan
    pub fn for_plan(plan: &Plan) -> Self {
        match plan {
            Plan::Free => Self::free(),
            Plan::Pro => Self::pro(),
            Plan::Enterprise => Self::enterprise(),
        }
    }

    /// Free tier quotas
    pub fn free() -> Self {
        Self {
            storage_bytes: 500 * 1024 * 1024,           // 500 MB
            api_requests_month: 10_000,
            realtime_connections: 100,
            file_storage_bytes: 1 * 1024 * 1024 * 1024, // 1 GB
            max_collections: 20,
            max_document_size: 1 * 1024 * 1024,         // 1 MB
        }
    }

    /// Pro tier quotas
    pub fn pro() -> Self {
        Self {
            storage_bytes: 100 * 1024 * 1024 * 1024,     // 100 GB
            api_requests_month: 1_000_000,
            realtime_connections: 10_000,
            file_storage_bytes: 100 * 1024 * 1024 * 1024, // 100 GB
            max_collections: 500,
            max_document_size: 16 * 1024 * 1024,          // 16 MB
        }
    }

    /// Enterprise tier (unlimited)
    pub fn enterprise() -> Self {
        Self {
            storage_bytes: u64::MAX,
            api_requests_month: u64::MAX,
            realtime_connections: u64::MAX,
            file_storage_bytes: u64::MAX,
            max_collections: u32::MAX,
            max_document_size: u64::MAX,
        }
    }

    /// Unlimited quotas (for service role)
    pub fn unlimited() -> Self {
        Self::enterprise()
    }
}

impl Default for Quotas {
    fn default() -> Self {
        Self::free()
    }
}

/// Quota enforcement result
#[derive(Debug, Clone)]
pub struct QuotaCheck {
    /// Whether the quota check passed
    pub allowed: bool,
    /// Current usage
    pub used: u64,
    /// Limit
    pub limit: u64,
    /// Remaining
    pub remaining: u64,
}

impl QuotaCheck {
    /// Create a passing quota check
    pub fn allowed(used: u64, limit: u64) -> Self {
        Self {
            allowed: true,
            used,
            limit,
            remaining: limit.saturating_sub(used),
        }
    }

    /// Create a failing quota check
    pub fn denied(used: u64, limit: u64) -> Self {
        Self {
            allowed: false,
            used,
            limit,
            remaining: 0,
        }
    }
}

/// Quota enforcer for tenant operations
#[derive(Debug, Clone)]
pub struct QuotaEnforcer {
    quotas: Quotas,
    tenant_id: String,
}

impl QuotaEnforcer {
    /// Create a new quota enforcer
    pub fn new(tenant_id: impl Into<String>, quotas: Quotas) -> Self {
        Self {
            quotas,
            tenant_id: tenant_id.into(),
        }
    }

    /// Check if storage usage is within quota
    pub fn check_storage(&self, current_bytes: u64, additional_bytes: u64) -> QuotaCheck {
        let total = current_bytes.saturating_add(additional_bytes);
        if total <= self.quotas.storage_bytes {
            QuotaCheck::allowed(current_bytes, self.quotas.storage_bytes)
        } else {
            QuotaCheck::denied(current_bytes, self.quotas.storage_bytes)
        }
    }

    /// Enforce storage quota (returns error if exceeded)
    pub fn enforce_storage(
        &self,
        current_bytes: u64,
        additional_bytes: u64,
    ) -> ControlPlaneResult<()> {
        let check = self.check_storage(current_bytes, additional_bytes);
        if check.allowed {
            Ok(())
        } else {
            Err(ControlPlaneError::QuotaExceeded {
                tenant_id: self.tenant_id.clone(),
                resource: "storage".to_string(),
                used: check.used,
                limit: check.limit,
            })
        }
    }

    /// Check if API request count is within quota
    pub fn check_api_requests(&self, current_count: u64) -> QuotaCheck {
        if current_count < self.quotas.api_requests_month {
            QuotaCheck::allowed(current_count, self.quotas.api_requests_month)
        } else {
            QuotaCheck::denied(current_count, self.quotas.api_requests_month)
        }
    }

    /// Enforce API request quota
    pub fn enforce_api_requests(&self, current_count: u64) -> ControlPlaneResult<()> {
        let check = self.check_api_requests(current_count);
        if check.allowed {
            Ok(())
        } else {
            Err(ControlPlaneError::QuotaExceeded {
                tenant_id: self.tenant_id.clone(),
                resource: "api_requests".to_string(),
                used: check.used,
                limit: check.limit,
            })
        }
    }

    /// Check if realtime connection count is within quota
    pub fn check_realtime_connections(&self, current_count: u64) -> QuotaCheck {
        if current_count < self.quotas.realtime_connections {
            QuotaCheck::allowed(current_count, self.quotas.realtime_connections)
        } else {
            QuotaCheck::denied(current_count, self.quotas.realtime_connections)
        }
    }

    /// Enforce realtime connection quota
    pub fn enforce_realtime_connections(&self, current_count: u64) -> ControlPlaneResult<()> {
        let check = self.check_realtime_connections(current_count);
        if check.allowed {
            Ok(())
        } else {
            Err(ControlPlaneError::QuotaExceeded {
                tenant_id: self.tenant_id.clone(),
                resource: "realtime_connections".to_string(),
                used: check.used,
                limit: check.limit,
            })
        }
    }

    /// Check if file storage usage is within quota
    pub fn check_file_storage(&self, current_bytes: u64, additional_bytes: u64) -> QuotaCheck {
        let total = current_bytes.saturating_add(additional_bytes);
        if total <= self.quotas.file_storage_bytes {
            QuotaCheck::allowed(current_bytes, self.quotas.file_storage_bytes)
        } else {
            QuotaCheck::denied(current_bytes, self.quotas.file_storage_bytes)
        }
    }

    /// Enforce file storage quota
    pub fn enforce_file_storage(
        &self,
        current_bytes: u64,
        additional_bytes: u64,
    ) -> ControlPlaneResult<()> {
        let check = self.check_file_storage(current_bytes, additional_bytes);
        if check.allowed {
            Ok(())
        } else {
            Err(ControlPlaneError::QuotaExceeded {
                tenant_id: self.tenant_id.clone(),
                resource: "file_storage".to_string(),
                used: check.used,
                limit: check.limit,
            })
        }
    }

    /// Check document size
    pub fn check_document_size(&self, size_bytes: u64) -> QuotaCheck {
        if size_bytes <= self.quotas.max_document_size {
            QuotaCheck::allowed(size_bytes, self.quotas.max_document_size)
        } else {
            QuotaCheck::denied(size_bytes, self.quotas.max_document_size)
        }
    }

    /// Get the quotas
    pub fn quotas(&self) -> &Quotas {
        &self.quotas
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_free_tier_quotas() {
        let quotas = Quotas::free();
        assert_eq!(quotas.storage_bytes, 500 * 1024 * 1024);
        assert_eq!(quotas.api_requests_month, 10_000);
        assert_eq!(quotas.realtime_connections, 100);
    }

    #[test]
    fn test_quota_enforcement() {
        let enforcer = QuotaEnforcer::new("test-tenant", Quotas::free());

        // Under quota
        assert!(enforcer.enforce_storage(100 * 1024 * 1024, 1024).is_ok());

        // Over quota
        let result = enforcer.enforce_storage(500 * 1024 * 1024, 1024);
        assert!(result.is_err());
        if let Err(ControlPlaneError::QuotaExceeded { resource, .. }) = result {
            assert_eq!(resource, "storage");
        }
    }

    #[test]
    fn test_quota_check() {
        let enforcer = QuotaEnforcer::new("test", Quotas::free());
        let check = enforcer.check_api_requests(5000);
        assert!(check.allowed);
        assert_eq!(check.used, 5000);
        assert_eq!(check.remaining, 5000);

        let check = enforcer.check_api_requests(10_000);
        assert!(!check.allowed);
    }
}
