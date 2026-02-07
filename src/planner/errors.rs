//! Planner error types following ERRORS.md specification
//!
//! Error codes:
//! - AERO_QUERY_INVALID (REJECT)
//! - AERO_QUERY_UNBOUNDED (REJECT)
//! - AERO_QUERY_UNINDEXED_FIELD (REJECT)
//! - AERO_QUERY_LIMIT_REQUIRED (REJECT)
//! - AERO_QUERY_SORT_NOT_INDEXED (REJECT)
//! - AERO_QUERY_SCHEMA_MISMATCH (REJECT)
//! - AERO_SCHEMA_VERSION_REQUIRED (REJECT)

use std::fmt;

/// Severity levels for planner errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Client request rejected
    Reject,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Reject => write!(f, "REJECT"),
        }
    }
}

/// Planner-specific error codes as defined in ERRORS.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlannerErrorCode {
    /// Malformed query structure
    AeroQueryInvalid,
    /// Cannot prove bounded execution
    AeroQueryUnbounded,
    /// Filter or sort on non-indexed field
    AeroQueryUnindexedField,
    /// Missing or invalid limit
    AeroQueryLimitRequired,
    /// Sort field not indexed
    AeroQuerySortNotIndexed,
    /// Query schema version mismatch
    AeroQuerySchemaMismatch,
    /// Missing schema version in query
    AeroSchemaVersionRequired,
    /// Schema ID not found
    AeroUnknownSchema,
    /// Schema version not found
    AeroUnknownSchemaVersion,
}

impl PlannerErrorCode {
    /// Returns the string code as defined in ERRORS.md
    pub fn code(&self) -> &'static str {
        match self {
            PlannerErrorCode::AeroQueryInvalid => "AERO_QUERY_INVALID",
            PlannerErrorCode::AeroQueryUnbounded => "AERO_QUERY_UNBOUNDED",
            PlannerErrorCode::AeroQueryUnindexedField => "AERO_QUERY_UNINDEXED_FIELD",
            PlannerErrorCode::AeroQueryLimitRequired => "AERO_QUERY_LIMIT_REQUIRED",
            PlannerErrorCode::AeroQuerySortNotIndexed => "AERO_QUERY_SORT_NOT_INDEXED",
            PlannerErrorCode::AeroQuerySchemaMismatch => "AERO_QUERY_SCHEMA_MISMATCH",
            PlannerErrorCode::AeroSchemaVersionRequired => "AERO_SCHEMA_VERSION_REQUIRED",
            PlannerErrorCode::AeroUnknownSchema => "AERO_UNKNOWN_SCHEMA",
            PlannerErrorCode::AeroUnknownSchemaVersion => "AERO_UNKNOWN_SCHEMA_VERSION",
        }
    }

    /// Returns the severity level for this error
    pub fn severity(&self) -> Severity {
        Severity::Reject
    }

    /// Returns the invariant violated by this error
    pub fn invariant(&self) -> &'static str {
        match self {
            PlannerErrorCode::AeroQueryInvalid => "Q3",
            PlannerErrorCode::AeroQueryUnbounded => "Q1",
            PlannerErrorCode::AeroQueryUnindexedField => "Q2",
            PlannerErrorCode::AeroQueryLimitRequired => "Q1",
            PlannerErrorCode::AeroQuerySortNotIndexed => "Q2",
            PlannerErrorCode::AeroQuerySchemaMismatch => "S3",
            PlannerErrorCode::AeroSchemaVersionRequired => "S3",
            PlannerErrorCode::AeroUnknownSchema => "S3",
            PlannerErrorCode::AeroUnknownSchemaVersion => "S3",
        }
    }
}

impl fmt::Display for PlannerErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// Planner error type with full context
#[derive(Debug, Clone)]
pub struct PlannerError {
    /// Error code
    code: PlannerErrorCode,
    /// Human-readable message
    message: String,
    /// Field name if applicable
    field: Option<String>,
}

impl PlannerError {
    /// Create a query invalid error
    pub fn query_invalid(reason: impl Into<String>) -> Self {
        Self {
            code: PlannerErrorCode::AeroQueryInvalid,
            message: reason.into(),
            field: None,
        }
    }

    /// Create an unbounded query error
    pub fn unbounded(reason: impl Into<String>) -> Self {
        Self {
            code: PlannerErrorCode::AeroQueryUnbounded,
            message: reason.into(),
            field: None,
        }
    }

    /// Create an unindexed field error
    pub fn unindexed_field(field: impl Into<String>) -> Self {
        let f = field.into();
        Self {
            code: PlannerErrorCode::AeroQueryUnindexedField,
            message: format!("Field '{}' is not indexed", f),
            field: Some(f),
        }
    }

    /// Create a limit required error
    pub fn limit_required() -> Self {
        Self {
            code: PlannerErrorCode::AeroQueryLimitRequired,
            message: "Query must include a positive limit".into(),
            field: None,
        }
    }

    /// Create a sort not indexed error
    pub fn sort_not_indexed(field: impl Into<String>) -> Self {
        let f = field.into();
        Self {
            code: PlannerErrorCode::AeroQuerySortNotIndexed,
            message: format!("Sort field '{}' is not indexed", f),
            field: Some(f),
        }
    }

    /// Create a schema mismatch error
    pub fn schema_mismatch(reason: impl Into<String>) -> Self {
        Self {
            code: PlannerErrorCode::AeroQuerySchemaMismatch,
            message: reason.into(),
            field: None,
        }
    }

    /// Create a schema version required error
    pub fn schema_version_required() -> Self {
        Self {
            code: PlannerErrorCode::AeroSchemaVersionRequired,
            message: "Query must specify schema_version".into(),
            field: None,
        }
    }

    /// Create an unknown schema error
    pub fn unknown_schema(schema_id: impl Into<String>) -> Self {
        let id = schema_id.into();
        Self {
            code: PlannerErrorCode::AeroUnknownSchema,
            message: format!("Schema '{}' not found", id),
            field: None,
        }
    }

    /// Create an unknown schema version error
    pub fn unknown_schema_version(
        schema_id: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            code: PlannerErrorCode::AeroUnknownSchemaVersion,
            message: format!(
                "Schema '{}' version '{}' not found",
                schema_id.into(),
                version.into()
            ),
            field: None,
        }
    }

    /// Returns the error code
    pub fn code(&self) -> PlannerErrorCode {
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

    /// Returns the field name if applicable
    pub fn field(&self) -> Option<&str> {
        self.field.as_deref()
    }
}

impl fmt::Display for PlannerError {
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

impl std::error::Error for PlannerError {}

/// Result type for planner operations
pub type PlannerResult<T> = Result<T, PlannerError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes_match_spec() {
        assert_eq!(
            PlannerErrorCode::AeroQueryInvalid.code(),
            "AERO_QUERY_INVALID"
        );
        assert_eq!(
            PlannerErrorCode::AeroQueryUnbounded.code(),
            "AERO_QUERY_UNBOUNDED"
        );
        assert_eq!(
            PlannerErrorCode::AeroQueryUnindexedField.code(),
            "AERO_QUERY_UNINDEXED_FIELD"
        );
        assert_eq!(
            PlannerErrorCode::AeroQueryLimitRequired.code(),
            "AERO_QUERY_LIMIT_REQUIRED"
        );
        assert_eq!(
            PlannerErrorCode::AeroQuerySortNotIndexed.code(),
            "AERO_QUERY_SORT_NOT_INDEXED"
        );
    }

    #[test]
    fn test_invariant_mapping() {
        assert_eq!(PlannerErrorCode::AeroQueryUnbounded.invariant(), "Q1");
        assert_eq!(PlannerErrorCode::AeroQueryUnindexedField.invariant(), "Q2");
        assert_eq!(PlannerErrorCode::AeroQueryInvalid.invariant(), "Q3");
    }

    #[test]
    fn test_error_display() {
        let err = PlannerError::unindexed_field("age");
        let display = format!("{}", err);
        assert!(display.contains("AERO_QUERY_UNINDEXED_FIELD"));
        assert!(display.contains("age"));
        assert!(display.contains("Q2"));
    }
}
