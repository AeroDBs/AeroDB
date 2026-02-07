//! Schema type definitions per SCHEMA.md
//!
//! Supported types (Phase 0):
//! - string: UTF-8 string
//! - int: 64-bit signed integer
//! - bool: Boolean
//! - float: 64-bit floating point
//! - object: Nested object with field schema
//! - array: Homogeneous array with element type

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported field types as defined in SCHEMA.md §136-153
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FieldType {
    /// UTF-8 string
    String,
    /// 64-bit signed integer
    Int,
    /// Boolean
    Bool,
    /// 64-bit floating point
    Float,
    /// Nested object with its own field schema
    Object {
        /// Nested field definitions
        fields: HashMap<String, FieldDef>,
    },
    /// Homogeneous array with single element type
    Array {
        /// Element type (boxed to allow recursive types)
        #[serde(rename = "element_type")]
        element_type: Box<FieldType>,
    },
}

impl FieldType {
    /// Returns the type name for error messages
    pub fn type_name(&self) -> &'static str {
        match self {
            FieldType::String => "string",
            FieldType::Int => "int",
            FieldType::Bool => "bool",
            FieldType::Float => "float",
            FieldType::Object { .. } => "object",
            FieldType::Array { .. } => "array",
        }
    }
}

/// Field definition as per SCHEMA.md §123-133
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDef {
    /// Field data type
    #[serde(flatten)]
    pub field_type: FieldType,
    /// Whether field must be present
    pub required: bool,
}

impl FieldDef {
    /// Create a required string field
    pub fn required_string() -> Self {
        Self {
            field_type: FieldType::String,
            required: true,
        }
    }

    /// Create an optional string field
    pub fn optional_string() -> Self {
        Self {
            field_type: FieldType::String,
            required: false,
        }
    }

    /// Create a required int field
    pub fn required_int() -> Self {
        Self {
            field_type: FieldType::Int,
            required: true,
        }
    }

    /// Create an optional int field
    pub fn optional_int() -> Self {
        Self {
            field_type: FieldType::Int,
            required: false,
        }
    }

    /// Create a required bool field
    pub fn required_bool() -> Self {
        Self {
            field_type: FieldType::Bool,
            required: true,
        }
    }

    /// Create a required float field
    pub fn required_float() -> Self {
        Self {
            field_type: FieldType::Float,
            required: true,
        }
    }

    /// Create a required object field
    pub fn required_object(fields: HashMap<String, FieldDef>) -> Self {
        Self {
            field_type: FieldType::Object { fields },
            required: true,
        }
    }

    /// Create an optional object field
    pub fn optional_object(fields: HashMap<String, FieldDef>) -> Self {
        Self {
            field_type: FieldType::Object { fields },
            required: false,
        }
    }

    /// Create a required array field
    pub fn required_array(element_type: FieldType) -> Self {
        Self {
            field_type: FieldType::Array {
                element_type: Box::new(element_type),
            },
            required: true,
        }
    }

    /// Create an optional array field
    pub fn optional_array(element_type: FieldType) -> Self {
        Self {
            field_type: FieldType::Array {
                element_type: Box::new(element_type),
            },
            required: false,
        }
    }
}

/// Complete schema definition as per SCHEMA.md §93-119
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Schema {
    /// Unique schema identifier
    pub schema_id: String,
    /// Schema version (monotonic or semantic)
    pub schema_version: String,
    /// Optional description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Field definitions
    pub fields: HashMap<String, FieldDef>,
}

impl Schema {
    /// Create a new schema
    pub fn new(
        schema_id: impl Into<String>,
        schema_version: impl Into<String>,
        fields: HashMap<String, FieldDef>,
    ) -> Self {
        Self {
            schema_id: schema_id.into(),
            schema_version: schema_version.into(),
            description: None,
            fields,
        }
    }

    /// Returns the unique key for this schema (id, version)
    pub fn key(&self) -> (&str, &str) {
        (&self.schema_id, &self.schema_version)
    }

    /// Validates the schema structure itself (not a document)
    pub fn validate_structure(&self) -> Result<(), String> {
        // Must have _id field per SCHEMA.md §156-168
        if !self.fields.contains_key("_id") {
            return Err("Schema must define an '_id' field".into());
        }

        // _id must be required
        if let Some(id_field) = self.fields.get("_id") {
            if !id_field.required {
                return Err("'_id' field must be required".into());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_schema() -> Schema {
        let mut fields = HashMap::new();
        fields.insert("_id".into(), FieldDef::required_string());
        fields.insert("name".into(), FieldDef::required_string());
        fields.insert("age".into(), FieldDef::optional_int());

        Schema::new("users", "v1", fields)
    }

    #[test]
    fn test_schema_structure_valid() {
        let schema = sample_schema();
        assert!(schema.validate_structure().is_ok());
    }

    #[test]
    fn test_schema_missing_id_field() {
        let mut fields = HashMap::new();
        fields.insert("name".into(), FieldDef::required_string());

        let schema = Schema::new("users", "v1", fields);
        assert!(schema.validate_structure().is_err());
    }

    #[test]
    fn test_schema_id_must_be_required() {
        let mut fields = HashMap::new();
        fields.insert("_id".into(), FieldDef::optional_string());

        let schema = Schema::new("users", "v1", fields);
        let result = schema.validate_structure();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("required"));
    }

    #[test]
    fn test_nested_object_type() {
        let mut address_fields = HashMap::new();
        address_fields.insert("city".into(), FieldDef::required_string());
        address_fields.insert("zip".into(), FieldDef::required_string());

        let mut fields = HashMap::new();
        fields.insert("_id".into(), FieldDef::required_string());
        fields.insert("address".into(), FieldDef::required_object(address_fields));

        let schema = Schema::new("users", "v1", fields);
        assert!(schema.validate_structure().is_ok());
    }

    #[test]
    fn test_array_type() {
        let mut fields = HashMap::new();
        fields.insert("_id".into(), FieldDef::required_string());
        fields.insert("tags".into(), FieldDef::required_array(FieldType::String));

        let schema = Schema::new("posts", "v1", fields);
        assert!(schema.validate_structure().is_ok());
    }

    #[test]
    fn test_field_type_names() {
        assert_eq!(FieldType::String.type_name(), "string");
        assert_eq!(FieldType::Int.type_name(), "int");
        assert_eq!(FieldType::Bool.type_name(), "bool");
        assert_eq!(FieldType::Float.type_name(), "float");
        assert_eq!(
            FieldType::Object {
                fields: HashMap::new()
            }
            .type_name(),
            "object"
        );
        assert_eq!(
            FieldType::Array {
                element_type: Box::new(FieldType::String)
            }
            .type_name(),
            "array"
        );
    }
}
