//! # Migration Operations
//!
//! MANIFESTO ALIGNMENT: Explicit operation execution.
//!
//! This module provides the actual execution logic for migration operations,
//! translating declarative operations into schema changes.

use super::errors::{MigrationError, MigrationResult};
use super::MigrationOperation;
use std::sync::Arc;

/// Operation executor trait
///
/// MANIFESTO ALIGNMENT: Explicit interface for operation execution.
/// All operations are deterministic and reversible.
pub trait OperationExecutor: Send + Sync {
    /// Execute an operation
    fn execute(&self, operation: &MigrationOperation) -> MigrationResult<()>;

    /// Check if collection exists
    fn collection_exists(&self, name: &str) -> MigrationResult<bool>;

    /// Check if index exists
    fn index_exists(&self, collection: &str, name: &str) -> MigrationResult<bool>;
}

/// In-memory operation executor (for testing)
#[derive(Debug, Default)]
pub struct InMemoryExecutor {
    collections: std::sync::RwLock<std::collections::HashSet<String>>,
    indexes: std::sync::RwLock<std::collections::HashMap<String, Vec<String>>>,
}

impl InMemoryExecutor {
    pub fn new() -> Self {
        Self::default()
    }
}

impl OperationExecutor for InMemoryExecutor {
    fn execute(&self, operation: &MigrationOperation) -> MigrationResult<()> {
        match operation {
            MigrationOperation::CreateCollection { name, schema: _ } => {
                let mut collections = self.collections.write().unwrap();
                if collections.contains(name) {
                    return Err(MigrationError::ExecutionFailed {
                        version: 0,
                        operation: "create_collection".to_string(),
                        reason: format!("Collection '{}' already exists", name),
                    });
                }
                collections.insert(name.clone());
                Ok(())
            }
            MigrationOperation::DropCollection { name } => {
                let mut collections = self.collections.write().unwrap();
                if !collections.contains(name) {
                    return Err(MigrationError::ExecutionFailed {
                        version: 0,
                        operation: "drop_collection".to_string(),
                        reason: format!("Collection '{}' does not exist", name),
                    });
                }
                collections.remove(name);
                Ok(())
            }
            MigrationOperation::CreateIndex {
                collection,
                fields,
                unique: _,
                name,
            } => {
                let index_name = name.clone().unwrap_or_else(|| fields.join("_"));
                let mut indexes = self.indexes.write().unwrap();
                let key = format!("{}.{}", collection, index_name);
                if indexes.contains_key(&key) {
                    return Err(MigrationError::ExecutionFailed {
                        version: 0,
                        operation: "create_index".to_string(),
                        reason: format!("Index '{}' already exists on '{}'", index_name, collection),
                    });
                }
                indexes.insert(key, fields.clone());
                Ok(())
            }
            MigrationOperation::DropIndex { collection, name } => {
                let mut indexes = self.indexes.write().unwrap();
                let key = format!("{}.{}", collection, name);
                if !indexes.contains_key(&key) {
                    return Err(MigrationError::ExecutionFailed {
                        version: 0,
                        operation: "drop_index".to_string(),
                        reason: format!("Index '{}' does not exist on '{}'", name, collection),
                    });
                }
                indexes.remove(&key);
                Ok(())
            }
            MigrationOperation::AddField {
                collection,
                field,
                field_type: _,
                required: _,
                default: _,
            } => {
                // In-memory: just validate collection exists
                let collections = self.collections.read().unwrap();
                if !collections.contains(collection) {
                    return Err(MigrationError::ExecutionFailed {
                        version: 0,
                        operation: "add_field".to_string(),
                        reason: format!("Collection '{}' does not exist", collection),
                    });
                }
                // Real implementation would modify schema
                Ok(())
            }
            MigrationOperation::RemoveField { collection, field } => {
                let collections = self.collections.read().unwrap();
                if !collections.contains(collection) {
                    return Err(MigrationError::ExecutionFailed {
                        version: 0,
                        operation: "remove_field".to_string(),
                        reason: format!("Collection '{}' does not exist", collection),
                    });
                }
                Ok(())
            }
            MigrationOperation::RenameField {
                collection,
                from,
                to,
            } => {
                let collections = self.collections.read().unwrap();
                if !collections.contains(collection) {
                    return Err(MigrationError::ExecutionFailed {
                        version: 0,
                        operation: "rename_field".to_string(),
                        reason: format!("Collection '{}' does not exist", collection),
                    });
                }
                Ok(())
            }
            MigrationOperation::RenameCollection { from, to } => {
                let mut collections = self.collections.write().unwrap();
                if !collections.contains(from) {
                    return Err(MigrationError::ExecutionFailed {
                        version: 0,
                        operation: "rename_collection".to_string(),
                        reason: format!("Collection '{}' does not exist", from),
                    });
                }
                if collections.contains(to) {
                    return Err(MigrationError::ExecutionFailed {
                        version: 0,
                        operation: "rename_collection".to_string(),
                        reason: format!("Collection '{}' already exists", to),
                    });
                }
                collections.remove(from);
                collections.insert(to.clone());
                Ok(())
            }
            MigrationOperation::Raw { operation: _ } => {
                // Raw operations are pass-through
                // Real implementation would execute the raw operation
                Ok(())
            }
        }
    }

    fn collection_exists(&self, name: &str) -> MigrationResult<bool> {
        let collections = self.collections.read().unwrap();
        Ok(collections.contains(name))
    }

    fn index_exists(&self, collection: &str, name: &str) -> MigrationResult<bool> {
        let indexes = self.indexes.read().unwrap();
        let key = format!("{}.{}", collection, name);
        Ok(indexes.contains_key(&key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_collection() {
        let executor = InMemoryExecutor::new();
        let op = MigrationOperation::CreateCollection {
            name: "users".to_string(),
            schema: serde_json::json!({}),
        };

        executor.execute(&op).unwrap();
        assert!(executor.collection_exists("users").unwrap());
    }

    #[test]
    fn test_create_collection_duplicate() {
        let executor = InMemoryExecutor::new();
        let op = MigrationOperation::CreateCollection {
            name: "users".to_string(),
            schema: serde_json::json!({}),
        };

        executor.execute(&op).unwrap();
        let result = executor.execute(&op);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_drop_collection() {
        let executor = InMemoryExecutor::new();

        // First create
        executor
            .execute(&MigrationOperation::CreateCollection {
                name: "users".to_string(),
                schema: serde_json::json!({}),
            })
            .unwrap();

        // Then drop
        executor
            .execute(&MigrationOperation::DropCollection {
                name: "users".to_string(),
            })
            .unwrap();

        assert!(!executor.collection_exists("users").unwrap());
    }

    #[test]
    fn test_create_index() {
        let executor = InMemoryExecutor::new();

        // Create collection first
        executor
            .execute(&MigrationOperation::CreateCollection {
                name: "users".to_string(),
                schema: serde_json::json!({}),
            })
            .unwrap();

        // Create index
        executor
            .execute(&MigrationOperation::CreateIndex {
                collection: "users".to_string(),
                fields: vec!["email".to_string()],
                unique: true,
                name: Some("idx_email".to_string()),
            })
            .unwrap();

        assert!(executor.index_exists("users", "idx_email").unwrap());
    }
}
