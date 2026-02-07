//! Schema error types following ERRORS.md specification
//!
//! Error codes:
//! - AERO_SCHEMA_REQUIRED (REJECT)
//! - AERO_UNKNOWN_SCHEMA (REJECT)
//! - AERO_UNKNOWN_SCHEMA_VERSION (REJECT)
//! - AERO_SCHEMA_VALIDATION_FAILED (REJECT)
//! - AERO_SCHEMA_IMMUTABLE (REJECT)

use std::fmt;

/// Severity levels for schema errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Client request rejected
    Reject,
    /// aerodb must terminate (for loader errors during recovery)
    Fatal,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Reject => write!(f, "REJECT"),
            Severity::Fatal => write!(f, "FATAL"),
        }
    }
}

/// Schema-specific error codes as defined in ERRORS.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaErrorCode {
    /// No schema specified
    AeroSchemaRequired,
    /// Schema ID not found
    AeroUnknownSchema,
    /// Schema version not found
    AeroUnknownSchemaVersion,
    /// Document violates schema
    AeroSchemaValidationFailed,
    /// Attempt to modify existing schema
    AeroSchemaImmutable,
    /// Schema missing during recovery (FATAL)
    AeroRecoverySchemaMissing,
}

impl SchemaErrorCode {
    /// Returns the string code as defined in ERRORS.md
    pub fn code(&self) -> &'static str {
        match self {
            SchemaErrorCode::AeroSchemaRequired => "AERO_SCHEMA_REQUIRED",
            SchemaErrorCode::AeroUnknownSchema => "AERO_UNKNOWN_SCHEMA",
            SchemaErrorCode::AeroUnknownSchemaVersion => "AERO_UNKNOWN_SCHEMA_VERSION",
            SchemaErrorCode::AeroSchemaValidationFailed => "AERO_SCHEMA_VALIDATION_FAILED",
            SchemaErrorCode::AeroSchemaImmutable => "AERO_SCHEMA_IMMUTABLE",
            SchemaErrorCode::AeroRecoverySchemaMissing => "AERO_RECOVERY_SCHEMA_MISSING",
        }
    }

    /// Returns the severity level for this error
    pub fn severity(&self) -> Severity {
        match self {
            SchemaErrorCode::AeroRecoverySchemaMissing => Severity::Fatal,
            _ => Severity::Reject,
        }
    }

    /// Returns the invariant violated by this error
    pub fn invariant(&self) -> &'static str {
        match self {
            SchemaErrorCode::AeroSchemaRequired => "S1",
            SchemaErrorCode::AeroUnknownSchema => "S3",
            SchemaErrorCode::AeroUnknownSchemaVersion => "S3",
            SchemaErrorCode::AeroSchemaValidationFailed => "S2",
            SchemaErrorCode::AeroSchemaImmutable => "S4",
            SchemaErrorCode::AeroRecoverySchemaMissing => "S3",
        }
    }
}

impl fmt::Display for SchemaErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// Validation failure details
#[derive(Debug, Clone)]
pub struct ValidationDetails {
    /// Field path (e.g., "user.address.city")
    pub field: String,
    /// Expected type or condition
    pub expected: String,
    /// Actual value or type found
    pub actual: String,
}

impl ValidationDetails {
    pub fn new(
        field: impl Into<String>,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    pub fn missing_field(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            expected: "field to be present".into(),
            actual: "missing".into(),
        }
    }

    pub fn extra_field(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            expected: "no undeclared fields".into(),
            actual: "extra field present".into(),
        }
    }

    pub fn type_mismatch(
        field: impl Into<String>,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    pub fn null_value(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            expected: "non-null value".into(),
            actual: "null".into(),
        }
    }
}

impl fmt::Display for ValidationDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "field '{}': expected {}, got {}",
            self.field, self.expected, self.actual
        )
    }
}

/// Schema error type with full context
#[derive(Debug)]
pub struct SchemaError {
    /// Error code
    code: SchemaErrorCode,
    /// Human-readable message
    message: String,
    /// Schema ID if applicable
    schema_id: Option<String>,
    /// Schema version if applicable
    schema_version: Option<String>,
    /// Validation details if applicable
    details: Option<ValidationDetails>,
}

impl SchemaError {
    /// Create a schema required error
    pub fn schema_required() -> Self {
        Self {
            code: SchemaErrorCode::AeroSchemaRequired,
            message: "Schema ID and version are required for all writes".into(),
            schema_id: None,
            schema_version: None,
            details: None,
        }
    }

    /// Create an unknown schema error
    pub fn unknown_schema(schema_id: impl Into<String>) -> Self {
        let id = schema_id.into();
        Self {
            code: SchemaErrorCode::AeroUnknownSchema,
            message: format!("Schema '{}' not found", id),
            schema_id: Some(id),
            schema_version: None,
            details: None,
        }
    }

    /// Create an unknown schema version error
    pub fn unknown_version(schema_id: impl Into<String>, version: impl Into<String>) -> Self {
        let id = schema_id.into();
        let ver = version.into();
        Self {
            code: SchemaErrorCode::AeroUnknownSchemaVersion,
            message: format!("Schema '{}' version '{}' not found", id, ver),
            schema_id: Some(id.clone()),
            schema_version: Some(ver),
            details: None,
        }
    }

    /// Create a validation failed error
    pub fn validation_failed(
        schema_id: impl Into<String>,
        schema_version: impl Into<String>,
        details: ValidationDetails,
    ) -> Self {
        let id = schema_id.into();
        let ver = schema_version.into();
        Self {
            code: SchemaErrorCode::AeroSchemaValidationFailed,
            message: format!("Document validation failed: {}", details),
            schema_id: Some(id),
            schema_version: Some(ver),
            details: Some(details),
        }
    }

    /// Create a schema immutable error
    pub fn schema_immutable(schema_id: impl Into<String>, version: impl Into<String>) -> Self {
        let id = schema_id.into();
        let ver = version.into();
        Self {
            code: SchemaErrorCode::AeroSchemaImmutable,
            message: format!("Schema '{}' version '{}' is immutable", id, ver),
            schema_id: Some(id),
            schema_version: Some(ver),
            details: None,
        }
    }

    /// Create a recovery schema missing error (FATAL)
    pub fn recovery_schema_missing(
        schema_id: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        let id = schema_id.into();
        let ver = version.into();
        Self {
            code: SchemaErrorCode::AeroRecoverySchemaMissing,
            message: format!("Schema '{}' version '{}' missing during recovery", id, ver),
            schema_id: Some(id),
            schema_version: Some(ver),
            details: None,
        }
    }

    /// Create an error for malformed schema file
    pub fn malformed_schema(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            code: SchemaErrorCode::AeroRecoverySchemaMissing,
            message: format!("Malformed schema file '{}': {}", path.into(), reason.into()),
            schema_id: None,
            schema_version: None,
            details: None,
        }
    }

    /// Returns the error code
    pub fn code(&self) -> SchemaErrorCode {
        self.code
    }

    /// Returns the severity level
    pub fn severity(&self) -> Severity {
        self.code.severity()
    }

    /// Returns the invariant violated
    pub fn invariant(&self) -> &'static str {
        self.code.invariant()
    }

    /// Returns the error message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the schema ID if applicable
    pub fn schema_id(&self) -> Option<&str> {
        self.schema_id.as_deref()
    }

    /// Returns the schema version if applicable
    pub fn schema_version(&self) -> Option<&str> {
        self.schema_version.as_deref()
    }

    /// Returns validation details if applicable
    pub fn details(&self) -> Option<&ValidationDetails> {
        self.details.as_ref()
    }

    /// Returns whether this is a fatal error
    pub fn is_fatal(&self) -> bool {
        self.severity() == Severity::Fatal
    }
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {}: {}",
            self.code.severity(),
            self.code.code(),
            self.message
        )?;
        write!(f, " [violates {}]", self.code.invariant())?;
        Ok(())
    }
}

impl std::error::Error for SchemaError {}

/// Result type for schema operations
pub type SchemaResult<T> = Result<T, SchemaError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes_match_spec() {
        assert_eq!(
            SchemaErrorCode::AeroSchemaRequired.code(),
            "AERO_SCHEMA_REQUIRED"
        );
        assert_eq!(
            SchemaErrorCode::AeroUnknownSchema.code(),
            "AERO_UNKNOWN_SCHEMA"
        );
        assert_eq!(
            SchemaErrorCode::AeroUnknownSchemaVersion.code(),
            "AERO_UNKNOWN_SCHEMA_VERSION"
        );
        assert_eq!(
            SchemaErrorCode::AeroSchemaValidationFailed.code(),
            "AERO_SCHEMA_VALIDATION_FAILED"
        );
        assert_eq!(
            SchemaErrorCode::AeroSchemaImmutable.code(),
            "AERO_SCHEMA_IMMUTABLE"
        );
    }

    #[test]
    fn test_severity_levels() {
        assert_eq!(
            SchemaErrorCode::AeroSchemaRequired.severity(),
            Severity::Reject
        );
        assert_eq!(
            SchemaErrorCode::AeroSchemaValidationFailed.severity(),
            Severity::Reject
        );
        assert_eq!(
            SchemaErrorCode::AeroRecoverySchemaMissing.severity(),
            Severity::Fatal
        );
    }

    #[test]
    fn test_validation_details_display() {
        let details = ValidationDetails::type_mismatch("age", "int", "string");
        let display = format!("{}", details);
        assert!(display.contains("age"));
        assert!(display.contains("int"));
        assert!(display.contains("string"));
    }

    #[test]
    fn test_error_includes_invariant() {
        let err = SchemaError::validation_failed(
            "users",
            "v1",
            ValidationDetails::missing_field("email"),
        );
        let display = format!("{}", err);
        assert!(display.contains("S2"));
    }
}
