//! # Auth Errors
//!
//! Error types for the authentication module.

use thiserror::Error;

/// Result type for auth operations
pub type AuthResult<T> = Result<T, AuthError>;

/// Authentication and authorization errors
#[derive(Debug, Clone, Error)]
pub enum AuthError {
    // ==================
    // Authentication Errors
    // ==================
    /// User not found (generic - don't leak whether email exists)
    #[error("Invalid credentials")]
    InvalidCredentials,

    /// Email already registered
    #[error("Email already registered")]
    EmailAlreadyExists,

    /// Email not verified
    #[error("Email not verified")]
    EmailNotVerified,

    /// Password does not meet requirements
    #[error("Password does not meet requirements: {0}")]
    WeakPassword(String),

    // ==================
    // Session Errors
    // ==================
    /// Session not found or expired
    #[error("Session expired or invalid")]
    SessionInvalid,

    /// Refresh token is invalid or already used
    #[error("Invalid refresh token")]
    InvalidRefreshToken,

    /// Session has been revoked
    #[error("Session has been revoked")]
    SessionRevoked,

    // ==================
    // JWT Errors
    // ==================
    /// JWT token is malformed
    #[error("Malformed token")]
    MalformedToken,

    /// JWT token has expired
    #[error("Token expired")]
    TokenExpired,

    /// JWT signature is invalid
    #[error("Invalid token signature")]
    InvalidSignature,

    // ==================
    // RLS Errors
    // ==================
    /// User must be authenticated
    #[error("Authentication required")]
    AuthenticationRequired,

    /// User not authorized for this resource
    #[error("Not authorized to access this resource")]
    Unauthorized,

    /// Missing owner field in document
    #[error("Document missing owner field: {0}")]
    MissingOwnerField(String),

    /// Invalid RLS policy configuration
    #[error("Invalid RLS policy: {0}")]
    InvalidPolicy(String),

    // ==================
    // Internal Errors
    // ==================
    /// Password hashing failed
    #[error("Internal error: password hashing failed")]
    HashingFailed,

    /// Token generation failed
    #[error("Internal error: token generation failed")]
    TokenGenerationFailed,

    /// Storage operation failed
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Invalid or expired token (for password reset, email verification)
    #[error("Invalid or expired token")]
    InvalidToken,

    /// Email sending failed
    #[error("Email error: {0}")]
    EmailError(String),
}

impl AuthError {
    /// Returns the HTTP status code for this error
    pub fn status_code(&self) -> u16 {
        match self {
            // 400 Bad Request
            AuthError::WeakPassword(_) => 400,
            AuthError::MalformedToken => 400,
            AuthError::InvalidPolicy(_) => 400,

            // 401 Unauthorized
            AuthError::InvalidCredentials => 401,
            AuthError::SessionInvalid => 401,
            AuthError::InvalidRefreshToken => 401,
            AuthError::SessionRevoked => 401,
            AuthError::TokenExpired => 401,
            AuthError::InvalidSignature => 401,
            AuthError::AuthenticationRequired => 401,
            AuthError::InvalidToken => 401,

            // 403 Forbidden
            AuthError::EmailNotVerified => 403,
            AuthError::Unauthorized => 403,
            AuthError::MissingOwnerField(_) => 403,

            // 409 Conflict
            AuthError::EmailAlreadyExists => 409,

            // 500 Internal Server Error
            AuthError::HashingFailed => 500,
            AuthError::TokenGenerationFailed => 500,
            AuthError::StorageError(_) => 500,
            AuthError::EmailError(_) => 500,
        }
    }

    /// Returns whether this error should be logged at warn level
    pub fn is_client_error(&self) -> bool {
        self.status_code() < 500
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(AuthError::InvalidCredentials.status_code(), 401);
        assert_eq!(AuthError::Unauthorized.status_code(), 403);
        assert_eq!(AuthError::EmailAlreadyExists.status_code(), 409);
        assert_eq!(AuthError::HashingFailed.status_code(), 500);
    }

    #[test]
    fn test_error_messages_do_not_leak_info() {
        // InvalidCredentials should be generic
        let err = AuthError::InvalidCredentials;
        assert!(!err.to_string().contains("password"));
        assert!(!err.to_string().contains("email"));
    }
}
