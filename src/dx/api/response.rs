//! API Response Envelope
//!
//! Per DX_OBSERVABILITY_API.md §4 (Global Response Envelope):
//! All responses MUST follow this structure:
//! - api_version: "v1"
//! - observed_at: { snapshot: "explicit|live", commit_id: number }
//! - data: { ... }
//! - notes: [ "optional explanatory strings" ]
//!
//! Read-only, Phase 4, no semantic authority.

use serde::{Deserialize, Serialize};

/// API version string.
pub const API_VERSION: &str = "v1";

/// Snapshot type indicator.
///
/// Per DX_OBSERVABILITY_API.md §4:
/// - "explicit" for user-specified snapshot
/// - "live" for current state observation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SnapshotType {
    /// Observation from an explicit snapshot.
    Explicit,
    /// Observation from current live state.
    Live,
}

/// Observation point metadata.
///
/// Per DX_OBSERVABILITY_API.md §4:
/// - observed_at is mandatory
/// - All numeric identifiers are raw, not prettified
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedAt {
    /// Type of snapshot observed.
    pub snapshot: SnapshotType,
    /// CommitId at observation point.
    pub commit_id: u64,
}

impl ObservedAt {
    /// Create observation metadata for live state.
    pub fn live(commit_id: u64) -> Self {
        Self {
            snapshot: SnapshotType::Live,
            commit_id,
        }
    }

    /// Create observation metadata for explicit snapshot.
    pub fn explicit(commit_id: u64) -> Self {
        Self {
            snapshot: SnapshotType::Explicit,
            commit_id,
        }
    }
}

/// Standard API response envelope.
///
/// Per DX_OBSERVABILITY_API.md §4:
/// - api_version is always "v1"
/// - observed_at is mandatory
/// - notes MUST NOT include heuristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T: Serialize> {
    /// API version.
    pub api_version: String,
    /// Observation point metadata.
    pub observed_at: ObservedAt,
    /// Response data.
    pub data: T,
    /// Optional explanatory notes (no heuristics allowed).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl<T: Serialize> ApiResponse<T> {
    /// Create a new API response.
    pub fn new(observed_at: ObservedAt, data: T) -> Self {
        Self {
            api_version: API_VERSION.to_string(),
            observed_at,
            data,
            notes: Vec::new(),
        }
    }

    /// Create response with notes.
    pub fn with_notes(observed_at: ObservedAt, data: T, notes: Vec<String>) -> Self {
        Self {
            api_version: API_VERSION.to_string(),
            observed_at,
            data,
            notes,
        }
    }
}

/// Error response per DX_OBSERVABILITY_API.md §8.
///
/// Errors MUST:
/// - Be explicit
/// - Reference internal error codes
/// - Include invariant identifiers where applicable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    /// Error code (e.g., "MVCC_VISIBILITY_VIOLATION").
    pub code: String,
    /// Related invariant (e.g., "MVCC-1").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invariant: Option<String>,
    /// Error message.
    pub message: String,
}

impl ApiError {
    /// Create a new API error.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            invariant: None,
            message: message.into(),
        }
    }

    /// Create error with invariant reference.
    pub fn with_invariant(
        code: impl Into<String>,
        invariant: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            invariant: Some(invariant.into()),
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observed_at_live() {
        let obs = ObservedAt::live(12345);
        assert_eq!(obs.snapshot, SnapshotType::Live);
        assert_eq!(obs.commit_id, 12345);
    }

    #[test]
    fn test_observed_at_explicit() {
        let obs = ObservedAt::explicit(99);
        assert_eq!(obs.snapshot, SnapshotType::Explicit);
        assert_eq!(obs.commit_id, 99);
    }

    #[test]
    fn test_api_response_version() {
        let resp = ApiResponse::new(ObservedAt::live(1), "test");
        assert_eq!(resp.api_version, "v1");
    }

    #[test]
    fn test_api_error_with_invariant() {
        let err = ApiError::with_invariant("TEST_ERROR", "P4-1", "Test message");
        assert_eq!(err.code, "TEST_ERROR");
        assert_eq!(err.invariant, Some("P4-1".to_string()));
    }
}
