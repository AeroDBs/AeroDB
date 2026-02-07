//! Replication Configuration
//!
//! Per PHASE5_IMPLEMENTATION_ORDER.md §Stage 1:
//! - Node role: Primary or Replica
//! - Immutable role at startup
//! - Replica identity (UUID)
//! - Replication disabled path (default-safe per P5-I16)
//!
//! Per PHASE5_INVARIANTS.md §P5-I16:
//! - Replication MUST be disableable at startup
//! - Disabling MUST NOT affect primary behavior

use super::errors::{ReplicationError, ReplicationResult};
use super::role::ReplicationRole;
use uuid::Uuid;

/// Replication configuration per PHASE5_IMPLEMENTATION_ORDER.md §Stage 1
///
/// Configured externally (file, env, CLI), immutable after startup.
/// Per REPLICATION_MODEL.md §6: Authority is determined externally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicationConfig {
    /// Whether replication is enabled.
    ///
    /// Per P5-I16: Default is `false` (replication disabled).
    /// When disabled, system runs as standalone primary with
    /// identical behavior to Phase 0-4.
    pub enabled: bool,

    /// Node role when replication is enabled.
    ///
    /// Per REPLICATION_MODEL.md §2:
    /// - Primary: sole write authority
    /// - Replica: consumes history, never creates it
    pub role: ReplicationRole,

    /// Unique replica identifier.
    ///
    /// Per PHASE5_IMPLEMENTATION_ORDER.md §Stage 1:
    /// - Required for replicas (auto-generated if None)
    /// - Ignored for primaries
    pub replica_id: Option<Uuid>,

    /// Primary node address for replicas to connect to.
    ///
    /// Per Stage 2+ requirements:
    /// - Required for replicas
    /// - Forbidden for primaries
    /// - Not used in Stage 1 (networking deferred)
    pub primary_address: Option<String>,
}

impl ReplicationConfig {
    /// Create a new replication configuration.
    pub fn new(
        enabled: bool,
        role: ReplicationRole,
        replica_id: Option<Uuid>,
        primary_address: Option<String>,
    ) -> Self {
        Self {
            enabled,
            role,
            replica_id,
            primary_address,
        }
    }

    /// Create a disabled configuration (default).
    ///
    /// Per P5-I16: This is the default-safe path.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            role: ReplicationRole::Primary,
            replica_id: None,
            primary_address: None,
        }
    }

    /// Create a primary configuration.
    pub fn primary() -> Self {
        Self {
            enabled: true,
            role: ReplicationRole::Primary,
            replica_id: None,
            primary_address: None,
        }
    }

    /// Create a replica configuration.
    ///
    /// Generates a new UUID if replica_id is None.
    pub fn replica(primary_address: String, replica_id: Option<Uuid>) -> Self {
        Self {
            enabled: true,
            role: ReplicationRole::Replica,
            replica_id: Some(replica_id.unwrap_or_else(Uuid::new_v4)),
            primary_address: Some(primary_address),
        }
    }

    /// Validate the configuration.
    ///
    /// Per PHASE5_IMPLEMENTATION_ORDER.md §Stage 1:
    /// - Replica requires primary_address
    /// - Primary forbids primary_address
    pub fn validate(&self) -> ReplicationResult<()> {
        if !self.enabled {
            // Disabled config is always valid
            return Ok(());
        }

        match self.role {
            ReplicationRole::Primary => {
                if self.primary_address.is_some() {
                    return Err(ReplicationError::configuration_error(
                        "Primary must not have primary_address configured",
                    ));
                }
                if self.replica_id.is_some() {
                    return Err(ReplicationError::configuration_error(
                        "Primary must not have replica_id configured",
                    ));
                }
            }
            ReplicationRole::Replica => {
                if self.primary_address.is_none() {
                    return Err(ReplicationError::configuration_error(
                        "Replica requires primary_address to be configured",
                    ));
                }
                if self.replica_id.is_none() {
                    return Err(ReplicationError::configuration_error(
                        "Replica requires replica_id (should be auto-generated)",
                    ));
                }
            }
        }

        Ok(())
    }

    /// Check if replication is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Check if this is a primary configuration.
    pub fn is_primary(&self) -> bool {
        self.enabled && self.role == ReplicationRole::Primary
    }

    /// Check if this is a replica configuration.
    pub fn is_replica(&self) -> bool {
        self.enabled && self.role == ReplicationRole::Replica
    }

    /// Get the replica ID if this is a replica.
    pub fn get_replica_id(&self) -> Option<Uuid> {
        if self.is_replica() {
            self.replica_id
        } else {
            None
        }
    }
}

impl Default for ReplicationConfig {
    /// Default is disabled per P5-I16.
    fn default() -> Self {
        Self::disabled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_disabled() {
        // Per P5-I16: Replication MUST be disableable
        let config = ReplicationConfig::default();
        assert!(!config.enabled);
        assert!(!config.is_enabled());
    }

    #[test]
    fn test_disabled_config_always_valid() {
        let config = ReplicationConfig::disabled();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_primary_config_valid() {
        let config = ReplicationConfig::primary();
        assert!(config.validate().is_ok());
        assert!(config.is_primary());
        assert!(!config.is_replica());
    }

    #[test]
    fn test_replica_config_valid() {
        let config = ReplicationConfig::replica("primary:5432".to_string(), None);
        assert!(config.validate().is_ok());
        assert!(config.is_replica());
        assert!(!config.is_primary());
        assert!(config.replica_id.is_some());
    }

    #[test]
    fn test_replica_requires_primary_address() {
        // Per PHASE5_IMPLEMENTATION_ORDER.md §Stage 1
        let config = ReplicationConfig {
            enabled: true,
            role: ReplicationRole::Replica,
            replica_id: Some(Uuid::new_v4()),
            primary_address: None,
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("primary_address"));
    }

    #[test]
    fn test_primary_forbids_primary_address() {
        // Per PHASE5_IMPLEMENTATION_ORDER.md §Stage 1
        let config = ReplicationConfig {
            enabled: true,
            role: ReplicationRole::Primary,
            replica_id: None,
            primary_address: Some("other:5432".to_string()),
        };
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_primary_forbids_replica_id() {
        let config = ReplicationConfig {
            enabled: true,
            role: ReplicationRole::Primary,
            replica_id: Some(Uuid::new_v4()),
            primary_address: None,
        };
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_replica_auto_generates_uuid() {
        let config1 = ReplicationConfig::replica("primary:5432".to_string(), None);
        let config2 = ReplicationConfig::replica("primary:5432".to_string(), None);

        // Each replica gets a unique ID
        assert_ne!(config1.replica_id, config2.replica_id);
    }

    #[test]
    fn test_replica_preserves_provided_uuid() {
        let id = Uuid::new_v4();
        let config = ReplicationConfig::replica("primary:5432".to_string(), Some(id));
        assert_eq!(config.replica_id, Some(id));
    }

    #[test]
    fn test_get_replica_id_only_for_replicas() {
        let primary = ReplicationConfig::primary();
        assert!(primary.get_replica_id().is_none());

        let replica = ReplicationConfig::replica("primary:5432".to_string(), None);
        assert!(replica.get_replica_id().is_some());
    }
}
