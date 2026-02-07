//! # Database Migrations Module
//!
//! MANIFESTO ALIGNMENT: Deterministic, checksummed, reversible schema evolution.
//!
//! Per Design Manifesto: "fail loudly, execute predictably, leave no surprises."
//!
//! # Design Principles
//!
//! 1. **Deterministic**: Same migrations always produce same schema state
//! 2. **Checksummed**: CRC32 verification detects manual edits to migration files
//! 3. **Reversible**: Every migration has explicit `up` and `down` operations
//! 4. **Fail-fast**: Migration failures halt execution immediately
//! 5. **Transactional**: All operations in a migration succeed or all fail
//! 6. **Tracked**: `_system.migrations` collection records applied migrations
//!
//! # Migration File Format
//!
//! Migrations are YAML files with explicit structure:
//!
//! ```yaml
//! version: 1
//! checksum: crc32:ABC12345
//! timestamp: 2026-02-08T00:00:00Z
//! up:
//!   - create_collection:
//!       name: users
//!       schema:
//!         properties:
//!           email: { type: string, required: true }
//! down:
//!   - drop_collection:
//!       name: users
//! ```
//!
//! # Usage
//!
//! ```bash
//! aerodb migrate create "add_users"  # Create new migration
//! aerodb migrate up                   # Apply pending migrations
//! aerodb migrate down                 # Rollback last migration
//! aerodb migrate status               # Show migration status
//! ```

pub mod checksum;
pub mod errors;
pub mod generator;
pub mod operations;
pub mod runner;
pub mod state;

pub use errors::{MigrationError, MigrationResult};
pub use runner::MigrationRunner;
pub use state::{MigrationState, MigrationStatus};

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Migration version number
pub type MigrationVersion = u64;

/// A single migration file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Migration {
    /// Migration version (sequential, starts at 1)
    pub version: MigrationVersion,

    /// Human-readable name (from filename)
    pub name: String,

    /// CRC32 checksum of migration content
    pub checksum: String,

    /// Timestamp when migration was created
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// File path on disk
    #[serde(skip)]
    pub file_path: Option<PathBuf>,

    /// Operations to apply (up migration)
    pub up: Vec<MigrationOperation>,

    /// Operations to revert (down migration)
    pub down: Vec<MigrationOperation>,
}

/// A single migration operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationOperation {
    /// Create a new collection with schema
    CreateCollection {
        name: String,
        schema: serde_json::Value,
    },

    /// Drop an existing collection
    DropCollection { name: String },

    /// Add a field to a collection
    AddField {
        collection: String,
        field: String,
        field_type: String,
        #[serde(default)]
        required: bool,
        #[serde(default)]
        default: Option<serde_json::Value>,
    },

    /// Remove a field from a collection
    RemoveField { collection: String, field: String },

    /// Rename a field
    RenameField {
        collection: String,
        from: String,
        to: String,
    },

    /// Add an index
    CreateIndex {
        collection: String,
        fields: Vec<String>,
        #[serde(default)]
        unique: bool,
        #[serde(default)]
        name: Option<String>,
    },

    /// Drop an index
    DropIndex { collection: String, name: String },

    /// Rename a collection
    RenameCollection { from: String, to: String },

    /// Execute raw operation (escape hatch - use sparingly)
    ///
    /// MANIFESTO ALIGNMENT: This is an explicit escape hatch.
    /// Use only when no other operation type fits.
    Raw { operation: serde_json::Value },
}

impl Migration {
    /// Validate migration structure
    pub fn validate(&self) -> MigrationResult<()> {
        // Version must be positive
        if self.version == 0 {
            return Err(MigrationError::InvalidMigration {
                reason: "Version must be >= 1".to_string(),
            });
        }

        // Must have at least one up operation
        if self.up.is_empty() {
            return Err(MigrationError::InvalidMigration {
                reason: "Migration must have at least one 'up' operation".to_string(),
            });
        }

        // Checksum must be present
        if self.checksum.is_empty() {
            return Err(MigrationError::InvalidMigration {
                reason: "Checksum is required".to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_operation_serialization() {
        let op = MigrationOperation::CreateCollection {
            name: "users".to_string(),
            schema: serde_json::json!({
                "properties": {
                    "email": { "type": "string", "required": true }
                }
            }),
        };

        let yaml = serde_yaml::to_string(&op).unwrap();
        assert!(yaml.contains("create_collection"));
        assert!(yaml.contains("users"));
    }

    #[test]
    fn test_migration_validation_empty_up() {
        let migration = Migration {
            version: 1,
            name: "test".to_string(),
            checksum: "crc32:ABC12345".to_string(),
            timestamp: chrono::Utc::now(),
            file_path: None,
            up: vec![],
            down: vec![],
        };

        let result = migration.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("up"));
    }

    #[test]
    fn test_migration_validation_zero_version() {
        let migration = Migration {
            version: 0,
            name: "test".to_string(),
            checksum: "crc32:ABC12345".to_string(),
            timestamp: chrono::Utc::now(),
            file_path: None,
            up: vec![MigrationOperation::CreateCollection {
                name: "test".to_string(),
                schema: serde_json::json!({}),
            }],
            down: vec![],
        };

        let result = migration.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Version"));
    }
}
