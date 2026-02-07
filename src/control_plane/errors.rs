//! # Control Plane Errors
//!
//! Error types for multi-tenant operations.

use serde::Serialize;
use std::fmt;

/// Control plane error types
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "details")]
pub enum ControlPlaneError {
    /// Tenant not found
    TenantNotFound {
        tenant_id: String,
    },

    /// Tenant name already exists
    TenantNameExists {
        name: String,
    },

    /// Invalid tenant name
    InvalidTenantName {
        name: String,
        reason: String,
    },

    /// Provisioning failed
    ProvisioningFailed {
        tenant_id: String,
        reason: String,
    },

    /// Deprovisioning failed
    DeprovisioningFailed {
        tenant_id: String,
        reason: String,
    },

    /// Quota exceeded
    QuotaExceeded {
        tenant_id: String,
        resource: String,
        used: u64,
        limit: u64,
    },

    /// Tenant is suspended
    TenantSuspended {
        tenant_id: String,
    },

    /// Tenant is deleted
    TenantDeleted {
        tenant_id: String,
    },

    /// Invalid isolation model for operation
    InvalidIsolationModel {
        model: String,
        reason: String,
    },

    /// Database connection error
    DatabaseError {
        message: String,
    },

    /// Process management error
    ProcessError {
        message: String,
    },

    /// Configuration error
    ConfigError {
        message: String,
    },

    /// Internal error
    Internal {
        message: String,
    },
}

impl fmt::Display for ControlPlaneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TenantNotFound { tenant_id } => {
                write!(f, "Tenant not found: {}", tenant_id)
            }
            Self::TenantNameExists { name } => {
                write!(f, "Tenant name already exists: {}", name)
            }
            Self::InvalidTenantName { name, reason } => {
                write!(f, "Invalid tenant name '{}': {}", name, reason)
            }
            Self::ProvisioningFailed { tenant_id, reason } => {
                write!(f, "Provisioning failed for {}: {}", tenant_id, reason)
            }
            Self::DeprovisioningFailed { tenant_id, reason } => {
                write!(f, "Deprovisioning failed for {}: {}", tenant_id, reason)
            }
            Self::QuotaExceeded {
                tenant_id,
                resource,
                used,
                limit,
            } => {
                write!(
                    f,
                    "Quota exceeded for {} on {}: {} / {}",
                    tenant_id, resource, used, limit
                )
            }
            Self::TenantSuspended { tenant_id } => {
                write!(f, "Tenant is suspended: {}", tenant_id)
            }
            Self::TenantDeleted { tenant_id } => {
                write!(f, "Tenant is deleted: {}", tenant_id)
            }
            Self::InvalidIsolationModel { model, reason } => {
                write!(f, "Invalid isolation model '{}': {}", model, reason)
            }
            Self::DatabaseError { message } => {
                write!(f, "Database error: {}", message)
            }
            Self::ProcessError { message } => {
                write!(f, "Process error: {}", message)
            }
            Self::ConfigError { message } => {
                write!(f, "Configuration error: {}", message)
            }
            Self::Internal { message } => {
                write!(f, "Internal error: {}", message)
            }
        }
    }
}

impl std::error::Error for ControlPlaneError {}

/// Result type for control plane operations
pub type ControlPlaneResult<T> = Result<T, ControlPlaneError>;

impl ControlPlaneError {
    /// Get HTTP status code for this error
    pub fn status_code(&self) -> u16 {
        match self {
            Self::TenantNotFound { .. } => 404,
            Self::TenantNameExists { .. } => 409,
            Self::InvalidTenantName { .. } => 400,
            Self::ProvisioningFailed { .. } => 500,
            Self::DeprovisioningFailed { .. } => 500,
            Self::QuotaExceeded { .. } => 429,
            Self::TenantSuspended { .. } => 403,
            Self::TenantDeleted { .. } => 410,
            Self::InvalidIsolationModel { .. } => 400,
            Self::DatabaseError { .. } => 500,
            Self::ProcessError { .. } => 500,
            Self::ConfigError { .. } => 400,
            Self::Internal { .. } => 500,
        }
    }

    /// Get error code for API responses
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::TenantNotFound { .. } => "TENANT_NOT_FOUND",
            Self::TenantNameExists { .. } => "TENANT_NAME_EXISTS",
            Self::InvalidTenantName { .. } => "INVALID_TENANT_NAME",
            Self::ProvisioningFailed { .. } => "PROVISIONING_FAILED",
            Self::DeprovisioningFailed { .. } => "DEPROVISIONING_FAILED",
            Self::QuotaExceeded { .. } => "QUOTA_EXCEEDED",
            Self::TenantSuspended { .. } => "TENANT_SUSPENDED",
            Self::TenantDeleted { .. } => "TENANT_DELETED",
            Self::InvalidIsolationModel { .. } => "INVALID_ISOLATION_MODEL",
            Self::DatabaseError { .. } => "DATABASE_ERROR",
            Self::ProcessError { .. } => "PROCESS_ERROR",
            Self::ConfigError { .. } => "CONFIG_ERROR",
            Self::Internal { .. } => "INTERNAL_ERROR",
        }
    }
}

/// API error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: &'static str,
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl From<ControlPlaneError> for ErrorResponse {
    fn from(err: ControlPlaneError) -> Self {
        Self {
            error: err.to_string(),
            code: err.error_code(),
            status: err.status_code(),
            details: serde_json::to_value(&err).ok(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        let err = ControlPlaneError::TenantNotFound {
            tenant_id: "test".to_string(),
        };
        assert_eq!(err.status_code(), 404);
        assert_eq!(err.error_code(), "TENANT_NOT_FOUND");

        let err = ControlPlaneError::QuotaExceeded {
            tenant_id: "test".to_string(),
            resource: "storage".to_string(),
            used: 600,
            limit: 500,
        };
        assert_eq!(err.status_code(), 429);
        assert_eq!(err.error_code(), "QUOTA_EXCEEDED");
    }

    #[test]
    fn test_error_display() {
        let err = ControlPlaneError::TenantNameExists {
            name: "acme".to_string(),
        };
        assert!(err.to_string().contains("acme"));
    }
}
