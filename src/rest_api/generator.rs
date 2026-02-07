//! # Schema Introspection and Endpoint Generator
//!
//! Auto-generates REST API endpoints from database schemas.

use std::collections::HashMap;
use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::auth::rls::RlsPolicy;

/// Field definition in a schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    /// Field name
    pub name: String,

    /// Field type (uuid, string, number, boolean, datetime, json)
    #[serde(rename = "type")]
    pub field_type: FieldType,

    /// Whether this field is required on insert
    #[serde(default)]
    pub required: bool,

    /// Whether this is the primary key
    #[serde(default)]
    pub primary: bool,

    /// Default value (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
}

/// Field types supported by the schema
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Uuid,
    String,
    Number,
    Boolean,
    Datetime,
    Json,
}

impl FieldType {
    /// Validate a JSON value against this field type
    pub fn validate(&self, value: &Value) -> bool {
        match self {
            FieldType::Uuid => value
                .as_str()
                .map(|s| uuid::Uuid::parse_str(s).is_ok())
                .unwrap_or(false),
            FieldType::String => value.is_string(),
            FieldType::Number => value.is_number(),
            FieldType::Boolean => value.is_boolean(),
            FieldType::Datetime => value
                .as_str()
                .map(|s| chrono::DateTime::parse_from_rfc3339(s).is_ok())
                .unwrap_or(false),
            FieldType::Json => value.is_object() || value.is_array(),
        }
    }
}

/// Schema definition for a collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDef {
    /// Collection name
    pub name: String,

    /// Field definitions
    pub fields: Vec<FieldDef>,

    /// RLS policy for this collection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rls_policy: Option<RlsPolicyDef>,
}

impl SchemaDef {
    /// Get the primary key field
    pub fn primary_key(&self) -> Option<&FieldDef> {
        self.fields.iter().find(|f| f.primary)
    }

    /// Get required fields
    pub fn required_fields(&self) -> Vec<&FieldDef> {
        self.fields.iter().filter(|f| f.required).collect()
    }

    /// Validate data against this schema
    pub fn validate(&self, data: &Value) -> Result<(), String> {
        let obj = data
            .as_object()
            .ok_or_else(|| "Data must be an object".to_string())?;

        // Check required fields
        for field in self.required_fields() {
            if !obj.contains_key(&field.name) {
                return Err(format!("Missing required field: {}", field.name));
            }
        }

        // Validate field types
        for (key, value) in obj {
            if let Some(field) = self.fields.iter().find(|f| f.name == *key) {
                if !value.is_null() && !field.field_type.validate(value) {
                    return Err(format!(
                        "Field '{}' has invalid type, expected {}",
                        key,
                        format!("{:?}", field.field_type).to_lowercase()
                    ));
                }
            }
        }

        Ok(())
    }
}

/// RLS policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlsPolicyDef {
    /// Policy type
    #[serde(rename = "type")]
    pub policy_type: RlsPolicyType,

    /// Owner field (for ownership policies)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_field: Option<String>,
}

impl RlsPolicyDef {
    /// Convert to RlsPolicy
    pub fn to_rls_policy(&self) -> Option<RlsPolicy> {
        match self.policy_type {
            RlsPolicyType::Ownership => self.owner_field.as_ref().map(|f| RlsPolicy::Ownership {
                owner_field: f.clone(),
            }),
            RlsPolicyType::Public => Some(RlsPolicy::PublicRead {
                owner_field: self
                    .owner_field
                    .clone()
                    .unwrap_or_else(|| "owner_id".to_string()),
            }),
            RlsPolicyType::None => None,
        }
    }
}

/// Type of RLS policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RlsPolicyType {
    Ownership,
    Public,
    None,
}

/// Endpoint definition generated from schema
#[derive(Debug, Clone)]
pub struct SchemaEndpoint {
    /// Collection name
    pub collection: String,

    /// Schema definition
    pub schema: SchemaDef,

    /// RLS policy
    pub rls_policy: Option<RlsPolicy>,
}

impl SchemaEndpoint {
    /// Create from schema definition
    pub fn from_schema(schema: SchemaDef) -> Self {
        let rls_policy = schema.rls_policy.as_ref().and_then(|p| p.to_rls_policy());

        Self {
            collection: schema.name.clone(),
            schema,
            rls_policy,
        }
    }
}

/// Endpoint registry for all generated endpoints
#[derive(Debug, Default)]
pub struct EndpointRegistry {
    endpoints: RwLock<HashMap<String, SchemaEndpoint>>,
}

impl EndpointRegistry {
    /// Create new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an endpoint
    pub fn register(&self, endpoint: SchemaEndpoint) -> Result<(), String> {
        let mut endpoints = self
            .endpoints
            .write()
            .map_err(|_| "Lock poisoned".to_string())?;
        endpoints.insert(endpoint.collection.clone(), endpoint);
        Ok(())
    }

    /// Get an endpoint by collection name
    pub fn get(&self, collection: &str) -> Option<SchemaEndpoint> {
        self.endpoints.read().ok()?.get(collection).cloned()
    }

    /// List all registered collections
    pub fn collections(&self) -> Vec<String> {
        self.endpoints
            .read()
            .map(|e| e.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Reload endpoints from schema definitions
    pub fn reload(&self, schemas: Vec<SchemaDef>) -> Result<usize, String> {
        let mut endpoints = self
            .endpoints
            .write()
            .map_err(|_| "Lock poisoned".to_string())?;

        endpoints.clear();

        for schema in schemas {
            let endpoint = SchemaEndpoint::from_schema(schema);
            endpoints.insert(endpoint.collection.clone(), endpoint);
        }

        Ok(endpoints.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_posts_schema() -> SchemaDef {
        SchemaDef {
            name: "posts".to_string(),
            fields: vec![
                FieldDef {
                    name: "id".to_string(),
                    field_type: FieldType::Uuid,
                    required: true,
                    primary: true,
                    default: None,
                },
                FieldDef {
                    name: "title".to_string(),
                    field_type: FieldType::String,
                    required: true,
                    primary: false,
                    default: None,
                },
                FieldDef {
                    name: "author_id".to_string(),
                    field_type: FieldType::Uuid,
                    required: true,
                    primary: false,
                    default: None,
                },
            ],
            rls_policy: Some(RlsPolicyDef {
                policy_type: RlsPolicyType::Ownership,
                owner_field: Some("author_id".to_string()),
            }),
        }
    }

    #[test]
    fn test_field_type_validation() {
        assert!(FieldType::String.validate(&serde_json::json!("hello")));
        assert!(!FieldType::String.validate(&serde_json::json!(123)));

        assert!(FieldType::Number.validate(&serde_json::json!(42)));
        assert!(FieldType::Number.validate(&serde_json::json!(3.14)));
        assert!(!FieldType::Number.validate(&serde_json::json!("42")));

        assert!(FieldType::Boolean.validate(&serde_json::json!(true)));
        assert!(!FieldType::Boolean.validate(&serde_json::json!("true")));

        assert!(
            FieldType::Uuid.validate(&serde_json::json!("550e8400-e29b-41d4-a716-446655440000"))
        );
        assert!(!FieldType::Uuid.validate(&serde_json::json!("not-a-uuid")));
    }

    #[test]
    fn test_schema_validation() {
        let schema = create_posts_schema();

        // Valid data
        let valid = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "title": "Hello World",
            "author_id": "660f9500-f30c-52e5-b827-557766551111"
        });
        assert!(schema.validate(&valid).is_ok());

        // Missing required field
        let missing = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "title": "Hello World"
        });
        assert!(schema.validate(&missing).is_err());

        // Invalid field type
        let invalid = serde_json::json!({
            "id": "not-a-uuid",
            "title": "Hello World",
            "author_id": "660f9500-f30c-52e5-b827-557766551111"
        });
        assert!(schema.validate(&invalid).is_err());
    }

    #[test]
    fn test_endpoint_registry() {
        let registry = EndpointRegistry::new();
        let schema = create_posts_schema();

        let endpoint = SchemaEndpoint::from_schema(schema);
        registry.register(endpoint).unwrap();

        assert!(registry.get("posts").is_some());
        assert!(registry.get("nonexistent").is_none());
        assert_eq!(registry.collections(), vec!["posts"]);
    }

    #[test]
    fn test_rls_policy_conversion() {
        let ownership = RlsPolicyDef {
            policy_type: RlsPolicyType::Ownership,
            owner_field: Some("owner_id".to_string()),
        };
        assert!(ownership.to_rls_policy().is_some());

        let public = RlsPolicyDef {
            policy_type: RlsPolicyType::Public,
            owner_field: None,
        };
        assert!(public.to_rls_policy().is_some());

        let none = RlsPolicyDef {
            policy_type: RlsPolicyType::None,
            owner_field: None,
        };
        assert!(none.to_rls_policy().is_none());
    }
}
