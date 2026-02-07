//! # REST API Errors
//!
//! Error types for the REST API module.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

use crate::auth::AuthError;

/// Result type for REST operations
pub type RestResult<T> = Result<T, RestError>;

/// REST API errors
#[derive(Debug, Clone, Error)]
pub enum RestError {
    // ==================
    // Client Errors (4xx)
    // ==================
    /// Invalid query parameter
    #[error("Invalid query parameter: {0}")]
    InvalidQueryParam(String),

    /// Invalid filter expression
    #[error("Invalid filter: {0}")]
    InvalidFilter(String),

    /// Missing required parameter
    #[error("Missing required parameter: {0}")]
    MissingParam(String),

    /// Invalid request body
    #[error("Invalid request body: {0}")]
    InvalidBody(String),

    /// Resource not found
    #[error("Resource not found")]
    NotFound,

    /// Collection not found
    #[error("Collection not found: {0}")]
    CollectionNotFound(String),

    /// Unbounded query not allowed
    #[error("Query must include limit (max: {0})")]
    UnboundedQuery(usize),

    /// Limit exceeds maximum
    #[error("Limit {0} exceeds maximum {1}")]
    LimitExceeded(usize, usize),

    // ==================
    // Auth Errors
    // ==================
    /// Authentication error
    #[error("{0}")]
    Auth(#[from] AuthError),

    // ==================
    // Server Errors (5xx)
    // ==================
    /// Internal error during query execution
    #[error("Internal error: {0}")]
    Internal(String),

    /// Schema loading error
    #[error("Schema error: {0}")]
    SchemaError(String),
}

impl RestError {
    /// Get HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            // 400 Bad Request
            RestError::InvalidQueryParam(_) => StatusCode::BAD_REQUEST,
            RestError::InvalidFilter(_) => StatusCode::BAD_REQUEST,
            RestError::MissingParam(_) => StatusCode::BAD_REQUEST,
            RestError::InvalidBody(_) => StatusCode::BAD_REQUEST,
            RestError::UnboundedQuery(_) => StatusCode::BAD_REQUEST,
            RestError::LimitExceeded(_, _) => StatusCode::BAD_REQUEST,

            // 401/403 from auth
            RestError::Auth(auth_err) => {
                StatusCode::from_u16(auth_err.status_code()).unwrap_or(StatusCode::UNAUTHORIZED)
            }

            // 404 Not Found
            RestError::NotFound => StatusCode::NOT_FOUND,
            RestError::CollectionNotFound(_) => StatusCode::NOT_FOUND,

            // 500 Internal Server Error
            RestError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RestError::SchemaError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// Error response body
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
}

impl From<RestError> for ErrorResponse {
    fn from(err: RestError) -> Self {
        Self {
            code: err.status_code().as_u16(),
            error: err.to_string(),
        }
    }
}

impl IntoResponse for RestError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = Json(ErrorResponse::from(self));
        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_codes() {
        assert_eq!(
            RestError::InvalidQueryParam("test".to_string()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(RestError::NotFound.status_code(), StatusCode::NOT_FOUND);
        assert_eq!(
            RestError::Internal("test".to_string()).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_auth_error_propagation() {
        let auth_err = AuthError::InvalidCredentials;
        let rest_err = RestError::from(auth_err);
        assert_eq!(rest_err.status_code(), StatusCode::UNAUTHORIZED);
    }
}
