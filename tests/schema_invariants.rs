//! Schema Invariant Tests
//!
//! Tests for schema validation invariants per SCHEMA.md:
//! - Validation occurs before WAL append (S2)
//! - Validation is deterministic
//! - All required fields must be present
//! - No undeclared fields allowed
//! - Type matching is exact

use aerodb::schema::{FieldDef, Schema, SchemaLoader, SchemaValidator};
use serde_json::json;
use std::collections::HashMap;
use tempfile::TempDir;

// =============================================================================
// Helper Functions
// =============================================================================

fn setup_test_loader() -> (TempDir, SchemaLoader) {
    let tmp = TempDir::new().unwrap();
    let mut loader = SchemaLoader::new(tmp.path());

    let mut fields = HashMap::new();
    fields.insert("_id".to_string(), FieldDef::required_string());
    fields.insert("name".to_string(), FieldDef::required_string());
    fields.insert("age".to_string(), FieldDef::optional_int());

    let schema = Schema::new("users", "1.0", fields);
    loader.register(schema).unwrap();

    (tmp, loader)
}

// =============================================================================
// Validation Determinism Tests
// =============================================================================

/// Same document validates the same way every time.
#[test]
fn test_validation_is_deterministic() {
    let (_tmp, loader) = setup_test_loader();
    let validator = SchemaValidator::new(&loader);

    let doc = json!({
        "_id": "user1",
        "name": "Alice"
    });

    // Validate 100 times, all should pass
    for _ in 0..100 {
        let result = validator.validate_document("users", "1.0", &doc);
        assert!(result.is_ok());
    }
}

/// Invalid document fails consistently.
#[test]
fn test_invalid_document_fails_consistently() {
    let (_tmp, loader) = setup_test_loader();
    let validator = SchemaValidator::new(&loader);

    let doc = json!({
        "_id": "user1"
        // Missing required "name" field
    });

    for _ in 0..100 {
        let result = validator.validate_document("users", "1.0", &doc);
        assert!(result.is_err());
    }
}

// =============================================================================
// Required Field Tests
// =============================================================================

/// Missing required field fails validation.
#[test]
fn test_missing_required_field() {
    let (_tmp, loader) = setup_test_loader();
    let validator = SchemaValidator::new(&loader);

    let doc = json!({
        "_id": "user1"
        // Missing "name"
    });

    let result = validator.validate_document("users", "1.0", &doc);
    assert!(result.is_err());
}

/// Present required field passes validation.
#[test]
fn test_present_required_field() {
    let (_tmp, loader) = setup_test_loader();
    let validator = SchemaValidator::new(&loader);

    let doc = json!({
        "_id": "user1",
        "name": "Bob"
    });

    let result = validator.validate_document("users", "1.0", &doc);
    assert!(result.is_ok());
}

/// Missing _id field fails validation.
#[test]
fn test_missing_id_fails() {
    let (_tmp, loader) = setup_test_loader();
    let validator = SchemaValidator::new(&loader);

    let doc = json!({
        "name": "NoId"
    });

    let result = validator.validate_document("users", "1.0", &doc);
    assert!(result.is_err());
}

// =============================================================================
// Optional Field Tests
// =============================================================================

/// Optional field can be omitted.
#[test]
fn test_optional_field_omitted() {
    let (_tmp, loader) = setup_test_loader();
    let validator = SchemaValidator::new(&loader);

    let doc = json!({
        "_id": "user1",
        "name": "Alice"
        // "age" is optional, omitted
    });

    let result = validator.validate_document("users", "1.0", &doc);
    assert!(result.is_ok());
}

/// Optional field can be present.
#[test]
fn test_optional_field_present() {
    let (_tmp, loader) = setup_test_loader();
    let validator = SchemaValidator::new(&loader);

    let doc = json!({
        "_id": "user1",
        "name": "Alice",
        "age": 30
    });

    let result = validator.validate_document("users", "1.0", &doc);
    assert!(result.is_ok());
}

// =============================================================================
// Type Matching Tests
// =============================================================================

/// Type mismatch fails validation.
#[test]
fn test_type_mismatch_fails() {
    let (_tmp, loader) = setup_test_loader();
    let validator = SchemaValidator::new(&loader);

    let doc = json!({
        "_id": "user1",
        "name": 12345  // String expected, got number
    });

    let result = validator.validate_document("users", "1.0", &doc);
    assert!(result.is_err());
}

/// Correct type passes validation.
#[test]
fn test_correct_type_passes() {
    let (_tmp, loader) = setup_test_loader();
    let validator = SchemaValidator::new(&loader);

    let doc = json!({
        "_id": "user1",
        "name": "StringValue"
    });

    let result = validator.validate_document("users", "1.0", &doc);
    assert!(result.is_ok());
}

// =============================================================================
// Undeclared Field Tests
// =============================================================================

/// Extra undeclared field fails validation.
#[test]
fn test_extra_field_fails() {
    let (_tmp, loader) = setup_test_loader();
    let validator = SchemaValidator::new(&loader);

    let doc = json!({
        "_id": "user1",
        "name": "Alice",
        "undeclared": "field"  // Not in schema
    });

    let result = validator.validate_document("users", "1.0", &doc);
    assert!(result.is_err());
}

// =============================================================================
// Schema Not Found Tests
// =============================================================================

/// Unknown schema fails validation.
#[test]
fn test_unknown_schema_fails() {
    let (_tmp, loader) = setup_test_loader();
    let validator = SchemaValidator::new(&loader);

    let doc = json!({
        "_id": "doc1"
    });

    let result = validator.validate_document("nonexistent", "1.0", &doc);
    assert!(result.is_err());
}

/// Unknown schema version fails validation.
#[test]
fn test_unknown_version_fails() {
    let (_tmp, loader) = setup_test_loader();
    let validator = SchemaValidator::new(&loader);

    let doc = json!({
        "_id": "user1",
        "name": "Test"
    });

    let result = validator.validate_document("users", "999.0", &doc);
    assert!(result.is_err());
}

// =============================================================================
// Update Immutability Tests
// =============================================================================

/// _id cannot change on update.
#[test]
fn test_id_immutable_on_update() {
    let (_tmp, loader) = setup_test_loader();
    let validator = SchemaValidator::new(&loader);

    let updated_doc = json!({
        "_id": "different_id",  // Changed!
        "name": "Updated"
    });

    let result = validator.validate_update("users", "1.0", "original_id", &updated_doc);
    assert!(result.is_err());
}

/// Same _id on update is valid.
#[test]
fn test_same_id_on_update_valid() {
    let (_tmp, loader) = setup_test_loader();
    let validator = SchemaValidator::new(&loader);

    let updated_doc = json!({
        "_id": "user1",
        "name": "Updated"
    });

    let result = validator.validate_update("users", "1.0", "user1", &updated_doc);
    assert!(result.is_ok());
}
