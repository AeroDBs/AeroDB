//! Schema validator for document validation per SCHEMA.md
//!
//! Validation semantics (§180-190):
//! - All required fields are present
//! - No undeclared fields exist
//! - Field types exactly match schema types
//! - _id is present and valid
//! - Schema version exists and is known
//!
//! Forbidden behaviors (§194-205):
//! - Missing required fields
//! - Extra undeclared fields
//! - Implicit type coercion
//! - Default values
//! - Null values
//! - Partial validation

use serde_json::Value;
use std::collections::HashMap;

use super::errors::{SchemaError, SchemaResult, ValidationDetails};
use super::loader::SchemaLoader;
use super::types::{FieldDef, FieldType};

/// Schema validator that enforces schema rules on documents.
///
/// Validation occurs BEFORE WAL append (invariant S2).
/// Validator does not mutate documents.
/// Validation is deterministic.
pub struct SchemaValidator<'a> {
    loader: &'a SchemaLoader,
}

impl<'a> SchemaValidator<'a> {
    /// Creates a new validator backed by the given schema loader.
    pub fn new(loader: &'a SchemaLoader) -> Self {
        Self { loader }
    }

    /// Validates a document against a schema.
    ///
    /// # Arguments
    ///
    /// * `schema_id` - The schema identifier
    /// * `schema_version` - The schema version
    /// * `document` - The document to validate
    ///
    /// # Errors
    ///
    /// Returns `SchemaError` if:
    /// - Schema ID not found (AERO_UNKNOWN_SCHEMA)
    /// - Schema version not found (AERO_UNKNOWN_SCHEMA_VERSION)
    /// - Document validation fails (AERO_SCHEMA_VALIDATION_FAILED)
    pub fn validate_document(
        &self,
        schema_id: &str,
        schema_version: &str,
        document: &Value,
    ) -> SchemaResult<()> {
        // Check schema exists
        if !self.loader.schema_id_exists(schema_id) {
            return Err(SchemaError::unknown_schema(schema_id));
        }

        let schema = self
            .loader
            .get(schema_id, schema_version)
            .ok_or_else(|| SchemaError::unknown_version(schema_id, schema_version))?;

        // Document must be an object
        let doc_obj = document.as_object().ok_or_else(|| {
            SchemaError::validation_failed(
                schema_id,
                schema_version,
                ValidationDetails::type_mismatch("$root", "object", json_type_name(document)),
            )
        })?;

        // Validate _id is present (required by SCHEMA.md §156-168)
        if !doc_obj.contains_key("_id") {
            return Err(SchemaError::validation_failed(
                schema_id,
                schema_version,
                ValidationDetails::missing_field("_id"),
            ));
        }

        // Validate all fields
        self.validate_object(schema_id, schema_version, doc_obj, &schema.fields, "")?;

        Ok(())
    }

    /// Validates a document for update, checking _id immutability.
    ///
    /// # Arguments
    ///
    /// * `schema_id` - The schema identifier
    /// * `schema_version` - The schema version
    /// * `existing_id` - The _id of the existing document
    /// * `document` - The new document to validate
    ///
    /// # Errors
    ///
    /// Returns error if _id in document differs from existing_id.
    pub fn validate_update(
        &self,
        schema_id: &str,
        schema_version: &str,
        existing_id: &str,
        document: &Value,
    ) -> SchemaResult<()> {
        // First validate the document normally
        self.validate_document(schema_id, schema_version, document)?;

        // Check _id immutability
        let doc_obj = document.as_object().unwrap(); // Already validated above
        let new_id = doc_obj.get("_id").and_then(|v| v.as_str());

        if let Some(new_id_str) = new_id {
            if new_id_str != existing_id {
                return Err(SchemaError::validation_failed(
                    schema_id,
                    schema_version,
                    ValidationDetails::new(
                        "_id",
                        format!("immutable value '{}'", existing_id),
                        format!("attempted change to '{}'", new_id_str),
                    ),
                ));
            }
        }

        Ok(())
    }

    /// Validates an object against field definitions.
    fn validate_object(
        &self,
        schema_id: &str,
        schema_version: &str,
        obj: &serde_json::Map<String, Value>,
        fields: &HashMap<String, FieldDef>,
        path_prefix: &str,
    ) -> SchemaResult<()> {
        // Check for extra fields (no undeclared fields allowed)
        for key in obj.keys() {
            if !fields.contains_key(key) {
                let field_path = make_path(path_prefix, key);
                return Err(SchemaError::validation_failed(
                    schema_id,
                    schema_version,
                    ValidationDetails::extra_field(field_path),
                ));
            }
        }

        // Validate each declared field
        for (field_name, field_def) in fields {
            let field_path = make_path(path_prefix, field_name);

            match obj.get(field_name) {
                Some(value) => {
                    // Check for null (forbidden in Phase 0)
                    if value.is_null() {
                        return Err(SchemaError::validation_failed(
                            schema_id,
                            schema_version,
                            ValidationDetails::null_value(&field_path),
                        ));
                    }

                    // Validate type
                    self.validate_value(
                        schema_id,
                        schema_version,
                        value,
                        &field_def.field_type,
                        &field_path,
                    )?;
                }
                None => {
                    // Missing field - check if required
                    if field_def.required {
                        return Err(SchemaError::validation_failed(
                            schema_id,
                            schema_version,
                            ValidationDetails::missing_field(field_path),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Validates a value against a field type.
    fn validate_value(
        &self,
        schema_id: &str,
        schema_version: &str,
        value: &Value,
        expected_type: &FieldType,
        field_path: &str,
    ) -> SchemaResult<()> {
        match expected_type {
            FieldType::String => {
                if !value.is_string() {
                    return Err(type_error(
                        schema_id,
                        schema_version,
                        field_path,
                        "string",
                        value,
                    ));
                }
            }
            FieldType::Int => {
                // Must be an integer (not a float)
                if !value.is_i64() && !value.is_u64() {
                    // Check if it's a float that could be confused for int
                    if value.is_f64() {
                        return Err(type_error(
                            schema_id,
                            schema_version,
                            field_path,
                            "int",
                            value,
                        ));
                    }
                    return Err(type_error(
                        schema_id,
                        schema_version,
                        field_path,
                        "int",
                        value,
                    ));
                }
            }
            FieldType::Bool => {
                if !value.is_boolean() {
                    return Err(type_error(
                        schema_id,
                        schema_version,
                        field_path,
                        "bool",
                        value,
                    ));
                }
            }
            FieldType::Float => {
                // Accept both integers and floats as float
                if !value.is_number() {
                    return Err(type_error(
                        schema_id,
                        schema_version,
                        field_path,
                        "float",
                        value,
                    ));
                }
            }
            FieldType::Object { fields } => {
                let obj = value.as_object().ok_or_else(|| {
                    type_error(schema_id, schema_version, field_path, "object", value)
                })?;
                self.validate_object(schema_id, schema_version, obj, fields, field_path)?;
            }
            FieldType::Array { element_type } => {
                let arr = value.as_array().ok_or_else(|| {
                    type_error(schema_id, schema_version, field_path, "array", value)
                })?;

                // Validate each element
                for (i, elem) in arr.iter().enumerate() {
                    let elem_path = format!("{}[{}]", field_path, i);

                    // Check for null elements
                    if elem.is_null() {
                        return Err(SchemaError::validation_failed(
                            schema_id,
                            schema_version,
                            ValidationDetails::null_value(&elem_path),
                        ));
                    }

                    self.validate_value(schema_id, schema_version, elem, element_type, &elem_path)?;
                }
            }
        }

        Ok(())
    }
}

/// Returns the JSON type name for error messages.
fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "int"
            } else {
                "float"
            }
        }
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Creates a field path from prefix and field name.
fn make_path(prefix: &str, field: &str) -> String {
    if prefix.is_empty() {
        field.to_string()
    } else {
        format!("{}.{}", prefix, field)
    }
}

/// Creates a type mismatch error.
fn type_error(
    schema_id: &str,
    schema_version: &str,
    field_path: &str,
    expected: &str,
    actual: &Value,
) -> SchemaError {
    SchemaError::validation_failed(
        schema_id,
        schema_version,
        ValidationDetails::type_mismatch(field_path, expected, json_type_name(actual)),
    )
}

#[cfg(test)]
mod tests {
    use super::super::types::Schema;
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn setup_loader() -> (TempDir, SchemaLoader) {
        let temp_dir = TempDir::new().unwrap();
        let mut loader = SchemaLoader::new(temp_dir.path());

        // Create a sample schema
        let mut fields = HashMap::new();
        fields.insert("_id".into(), FieldDef::required_string());
        fields.insert("name".into(), FieldDef::required_string());
        fields.insert("age".into(), FieldDef::optional_int());
        fields.insert("active".into(), FieldDef::required_bool());

        loader.register(Schema::new("users", "v1", fields)).unwrap();

        (temp_dir, loader)
    }

    #[test]
    fn test_valid_document_passes() {
        let (_temp_dir, loader) = setup_loader();
        let validator = SchemaValidator::new(&loader);

        let doc = json!({
            "_id": "user_123",
            "name": "Alice",
            "active": true
        });

        let result = validator.validate_document("users", "v1", &doc);
        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_document_with_optional_field() {
        let (_temp_dir, loader) = setup_loader();
        let validator = SchemaValidator::new(&loader);

        let doc = json!({
            "_id": "user_123",
            "name": "Alice",
            "age": 30,
            "active": true
        });

        let result = validator.validate_document("users", "v1", &doc);
        assert!(result.is_ok());
    }

    #[test]
    fn test_missing_required_field_fails() {
        let (_temp_dir, loader) = setup_loader();
        let validator = SchemaValidator::new(&loader);

        let doc = json!({
            "_id": "user_123",
            "active": true
            // missing "name"
        });

        let result = validator.validate_document("users", "v1", &doc);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code().code(), "AERO_SCHEMA_VALIDATION_FAILED");
        assert!(err.message().contains("name"));
    }

    #[test]
    fn test_extra_field_fails() {
        let (_temp_dir, loader) = setup_loader();
        let validator = SchemaValidator::new(&loader);

        let doc = json!({
            "_id": "user_123",
            "name": "Alice",
            "active": true,
            "unknown_field": "value"
        });

        let result = validator.validate_document("users", "v1", &doc);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.details().unwrap().field.contains("unknown_field"));
    }

    #[test]
    fn test_type_mismatch_fails() {
        let (_temp_dir, loader) = setup_loader();
        let validator = SchemaValidator::new(&loader);

        let doc = json!({
            "_id": "user_123",
            "name": 123,  // should be string
            "active": true
        });

        let result = validator.validate_document("users", "v1", &doc);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let details = err.details().unwrap();
        assert_eq!(details.field, "name");
        assert_eq!(details.expected, "string");
    }

    #[test]
    fn test_null_rejected() {
        let (_temp_dir, loader) = setup_loader();
        let validator = SchemaValidator::new(&loader);

        let doc = json!({
            "_id": "user_123",
            "name": null,
            "active": true
        });

        let result = validator.validate_document("users", "v1", &doc);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let details = err.details().unwrap();
        assert!(details.actual.contains("null"));
    }

    #[test]
    fn test_unknown_schema_rejected() {
        let (_temp_dir, loader) = setup_loader();
        let validator = SchemaValidator::new(&loader);

        let doc = json!({ "_id": "x" });
        let result = validator.validate_document("nonexistent", "v1", &doc);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code().code(), "AERO_UNKNOWN_SCHEMA");
    }

    #[test]
    fn test_unknown_version_rejected() {
        let (_temp_dir, loader) = setup_loader();
        let validator = SchemaValidator::new(&loader);

        let doc = json!({ "_id": "x" });
        let result = validator.validate_document("users", "v999", &doc);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().code().code(),
            "AERO_UNKNOWN_SCHEMA_VERSION"
        );
    }

    #[test]
    fn test_id_required() {
        let (_temp_dir, loader) = setup_loader();
        let validator = SchemaValidator::new(&loader);

        let doc = json!({
            "name": "Alice",
            "active": true
        });

        let result = validator.validate_document("users", "v1", &doc);
        assert!(result.is_err());
        assert!(result.unwrap_err().details().unwrap().field.contains("_id"));
    }

    #[test]
    fn test_id_immutable_on_update() {
        let (_temp_dir, loader) = setup_loader();
        let validator = SchemaValidator::new(&loader);

        let doc = json!({
            "_id": "new_id",  // Should be "old_id"
            "name": "Alice",
            "active": true
        });

        let result = validator.validate_update("users", "v1", "old_id", &doc);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message().contains("immutable"));
    }

    #[test]
    fn test_nested_object_validation() {
        let temp_dir = TempDir::new().unwrap();
        let mut loader = SchemaLoader::new(temp_dir.path());

        // Schema with nested object
        let mut address_fields = HashMap::new();
        address_fields.insert("city".into(), FieldDef::required_string());
        address_fields.insert("zip".into(), FieldDef::required_string());

        let mut fields = HashMap::new();
        fields.insert("_id".into(), FieldDef::required_string());
        fields.insert("address".into(), FieldDef::required_object(address_fields));

        loader.register(Schema::new("users", "v1", fields)).unwrap();
        let validator = SchemaValidator::new(&loader);

        // Valid nested object
        let doc = json!({
            "_id": "u1",
            "address": {
                "city": "NYC",
                "zip": "10001"
            }
        });
        assert!(validator.validate_document("users", "v1", &doc).is_ok());

        // Missing nested field
        let doc = json!({
            "_id": "u1",
            "address": {
                "city": "NYC"
                // missing zip
            }
        });
        let result = validator.validate_document("users", "v1", &doc);
        assert!(result.is_err());
        assert!(result.unwrap_err().details().unwrap().field.contains("zip"));
    }

    #[test]
    fn test_array_element_validation() {
        let temp_dir = TempDir::new().unwrap();
        let mut loader = SchemaLoader::new(temp_dir.path());

        let mut fields = HashMap::new();
        fields.insert("_id".into(), FieldDef::required_string());
        fields.insert("tags".into(), FieldDef::required_array(FieldType::String));

        loader.register(Schema::new("posts", "v1", fields)).unwrap();
        let validator = SchemaValidator::new(&loader);

        // Valid array
        let doc = json!({
            "_id": "post1",
            "tags": ["rust", "database"]
        });
        assert!(validator.validate_document("posts", "v1", &doc).is_ok());

        // Array with wrong element type
        let doc = json!({
            "_id": "post1",
            "tags": ["rust", 123, "db"]
        });
        let result = validator.validate_document("posts", "v1", &doc);
        assert!(result.is_err());
        assert!(result.unwrap_err().details().unwrap().field.contains("[1]"));
    }

    #[test]
    fn test_array_with_null_element() {
        let temp_dir = TempDir::new().unwrap();
        let mut loader = SchemaLoader::new(temp_dir.path());

        let mut fields = HashMap::new();
        fields.insert("_id".into(), FieldDef::required_string());
        fields.insert("values".into(), FieldDef::required_array(FieldType::Int));

        loader.register(Schema::new("data", "v1", fields)).unwrap();
        let validator = SchemaValidator::new(&loader);

        let doc = json!({
            "_id": "d1",
            "values": [1, null, 3]
        });

        let result = validator.validate_document("data", "v1", &doc);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .details()
            .unwrap()
            .actual
            .contains("null"));
    }

    #[test]
    fn test_float_accepts_integers() {
        let temp_dir = TempDir::new().unwrap();
        let mut loader = SchemaLoader::new(temp_dir.path());

        let mut fields = HashMap::new();
        fields.insert("_id".into(), FieldDef::required_string());
        fields.insert("score".into(), FieldDef::required_float());

        loader
            .register(Schema::new("scores", "v1", fields))
            .unwrap();
        let validator = SchemaValidator::new(&loader);

        // Integer value for float field is acceptable
        let doc = json!({
            "_id": "s1",
            "score": 100
        });
        assert!(validator.validate_document("scores", "v1", &doc).is_ok());

        // Float value
        let doc = json!({
            "_id": "s1",
            "score": 99.5
        });
        assert!(validator.validate_document("scores", "v1", &doc).is_ok());
    }
}
