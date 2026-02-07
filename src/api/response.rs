//! API response types
//!
//! JSON response formatting for all operations.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::errors::ApiError;

/// Success response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResponse {
    pub status: String,
    pub data: Value,
}

impl SuccessResponse {
    /// Create a new success response
    pub fn new(data: Value) -> Self {
        Self {
            status: "ok".to_string(),
            data,
        }
    }

    /// Create an empty success response
    pub fn empty() -> Self {
        Self {
            status: "ok".to_string(),
            data: Value::Null,
        }
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("SuccessResponse serialization cannot fail")
    }
}

/// Error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub status: String,
    pub code: String,
    pub message: String,
}

impl ErrorResponse {
    /// Create from an API error
    pub fn from_error(err: &ApiError) -> Self {
        Self {
            status: "error".to_string(),
            code: err.code().to_string(),
            message: err.message().to_string(),
        }
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("ErrorResponse serialization cannot fail")
    }
}

/// Unified response type
#[derive(Debug, Clone)]
pub enum Response {
    Success(SuccessResponse),
    Error(ErrorResponse),
}

impl Response {
    /// Create a success response
    pub fn success(data: Value) -> Self {
        Response::Success(SuccessResponse::new(data))
    }

    /// Create an empty success response
    pub fn ok() -> Self {
        Response::Success(SuccessResponse::empty())
    }

    /// Create an error response
    pub fn error(err: &ApiError) -> Self {
        Response::Error(ErrorResponse::from_error(err))
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> String {
        match self {
            Response::Success(r) => r.to_json(),
            Response::Error(r) => r.to_json(),
        }
    }

    /// Check if this is a success response
    pub fn is_success(&self) -> bool {
        matches!(self, Response::Success(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_success_response() {
        let resp = SuccessResponse::new(json!([{"name": "Alice"}]));
        let json = resp.to_json();
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("Alice"));
    }

    #[test]
    fn test_error_response() {
        let err = ApiError::invalid_request("test error");
        let resp = ErrorResponse::from_error(&err);
        let json = resp.to_json();
        assert!(json.contains("\"status\":\"error\""));
        assert!(json.contains("AERO_INVALID_REQUEST"));
    }
}
