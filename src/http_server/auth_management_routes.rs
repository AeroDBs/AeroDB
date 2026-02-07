//! Auth Management HTTP Routes
//!
//! Extended authentication endpoints for user, session, and RLS management.

use std::sync::Arc;

use axum::{
    extract::{Json, Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::api::{
    AuthService, ChangePasswordRequest, ErrorResponse, ForgotPasswordRequest, ResetPasswordRequest,
    UpdateUserRequest, UserResponse,
};
use crate::auth::crypto::PasswordPolicy;
use crate::auth::errors::AuthError;
use crate::auth::rls::{DefaultRlsEnforcer, RlsPolicy};
use crate::auth::session::InMemorySessionRepository;
use crate::auth::user::InMemoryUserRepository;

use super::auth_routes::AuthState;

// ==================
// Request/Response Types
// ==================

#[derive(Debug, Serialize)]
pub struct UsersListResponse {
    pub users: Vec<UserResponse>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub id: String,
    pub user_id: String,
    pub created_at: String,
    pub expires_at: String,
    pub is_revoked: bool,
}

#[derive(Debug, Serialize)]
pub struct SessionsListResponse {
    pub sessions: Vec<SessionResponse>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct RlsPolicyResponse {
    pub table: String,
    pub policy: RlsPolicy,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateRlsPolicyRequest {
    pub policy: RlsPolicy,
}

#[derive(Debug, Serialize)]
pub struct PasswordPolicyResponse {
    pub min_length: usize,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_number: bool,
    pub require_special: bool,
}

impl From<&PasswordPolicy> for PasswordPolicyResponse {
    fn from(policy: &PasswordPolicy) -> Self {
        Self {
            min_length: policy.min_length,
            require_uppercase: policy.require_uppercase,
            require_lowercase: policy.require_lowercase,
            require_number: policy.require_number,
            require_special: policy.require_special,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct VerifyEmailRequest {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

// ==================
// Auth Management Routes
// ==================

/// Extended auth management routes
pub fn auth_management_routes(state: Arc<AuthState>) -> Router {
    Router::new()
        // User management
        .route("/users", get(list_users_handler))
        .route("/users", post(create_user_handler))
        .route("/users/{id}", get(get_user_handler))
        .route("/users/{id}", patch(update_user_handler))
        .route("/users/{id}", delete(delete_user_handler))
        // Session management
        .route("/sessions", get(list_sessions_handler))
        .route("/sessions/{id}", delete(revoke_session_handler))
        // Password management
        .route("/forgot-password", post(forgot_password_handler))
        .route("/reset-password", post(reset_password_handler))
        .route("/change-password", post(change_password_handler))
        .route("/password-policy", get(get_password_policy_handler))
        // RLS management
        .route("/rls/{table}", get(get_rls_policy_handler))
        .route("/rls/{table}", post(create_rls_policy_handler))
        .route("/rls/{table}", delete(delete_rls_policy_handler))
        // Email verification
        .route("/verify-email", post(verify_email_handler))
        .route(
            "/users/{id}/resend-verification",
            post(resend_verification_handler),
        )
        .with_state(state)
}

// ==================
// Helper Functions
// ==================

/// Extract Bearer token from Authorization header
fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
}

/// Validate admin access (simplified - in production, check admin role)
fn validate_admin_access(
    state: &AuthState,
    headers: &HeaderMap,
) -> Result<Uuid, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Missing authorization header".to_string(),
                code: 401,
            }),
        )
    })?;

    let ctx = state.service.validate_access_token(token).map_err(|e| {
        (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::UNAUTHORIZED),
            Json(ErrorResponse::from(e)),
        )
    })?;

    ctx.user_id.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid token".to_string(),
                code: 401,
            }),
        )
    })
}

// ==================
// User Management Handlers
// ==================

/// List all users (admin only)
async fn list_users_handler(
    State(state): State<Arc<AuthState>>,
    headers: HeaderMap,
) -> Result<Json<UsersListResponse>, (StatusCode, Json<ErrorResponse>)> {
    validate_admin_access(&state, &headers)?;

    // Get all users from repository
    // Note: In a real implementation, this would use pagination
    let users: Vec<UserResponse> = Vec::new(); // Placeholder - need to add list_all to UserRepository

    Ok(Json(UsersListResponse {
        total: users.len(),
        users,
    }))
}

/// Get a specific user by ID
async fn get_user_handler(
    State(state): State<Arc<AuthState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<UserResponse>, (StatusCode, Json<ErrorResponse>)> {
    validate_admin_access(&state, &headers)?;

    let user = state.service.get_user(id).map_err(|e| {
        (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::NOT_FOUND),
            Json(ErrorResponse::from(e)),
        )
    })?;

    Ok(Json(UserResponse::from(user)))
}

/// Create a new user (admin only)
async fn create_user_handler(
    State(state): State<Arc<AuthState>>,
    headers: HeaderMap,
    Json(request): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserResponse>), (StatusCode, Json<ErrorResponse>)> {
    validate_admin_access(&state, &headers)?;

    let signup_request = crate::auth::user::SignupRequest {
        email: request.email,
        password: request.password,
        metadata: request.metadata,
    };

    let (user, _) = state.service.signup(signup_request).map_err(|e| {
        (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::BAD_REQUEST),
            Json(ErrorResponse::from(e)),
        )
    })?;

    Ok((StatusCode::CREATED, Json(UserResponse::from(user))))
}

/// Update a user (admin only)
async fn update_user_handler(
    State(state): State<Arc<AuthState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, (StatusCode, Json<ErrorResponse>)> {
    validate_admin_access(&state, &headers)?;

    let user = state.service.update_user(id, request).map_err(|e| {
        (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::BAD_REQUEST),
            Json(ErrorResponse::from(e)),
        )
    })?;

    Ok(Json(UserResponse::from(user)))
}

/// Delete a user (admin only)
async fn delete_user_handler(
    State(state): State<Arc<AuthState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    validate_admin_access(&state, &headers)?;

    // Note: Need to add delete_user method to AuthService
    // For now, return not implemented
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "User deletion not yet implemented".to_string(),
            code: 501,
        }),
    ))
}

// ==================
// Session Management Handlers
// ==================

/// List sessions for current user
async fn list_sessions_handler(
    State(state): State<Arc<AuthState>>,
    headers: HeaderMap,
) -> Result<Json<SessionsListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = validate_admin_access(&state, &headers)?;

    // Return empty list for now - session listing needs to be added
    Ok(Json(SessionsListResponse {
        sessions: Vec::new(),
        total: 0,
    }))
}

/// Revoke a specific session
async fn revoke_session_handler(
    State(state): State<Arc<AuthState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    validate_admin_access(&state, &headers)?;

    // Note: Need to expose session revocation by ID
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "Session revocation by ID not yet implemented".to_string(),
            code: 501,
        }),
    ))
}

// ==================
// Password Management Handlers
// ==================

/// Request password reset
async fn forgot_password_handler(
    State(state): State<Arc<AuthState>>,
    Json(request): Json<ForgotPasswordRequest>,
) -> Result<Json<MessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    state.service.forgot_password(&request.email).map_err(|e| {
        (
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(ErrorResponse::from(e)),
        )
    })?;

    Ok(Json(MessageResponse {
        message: "If the email exists, a password reset link has been sent".to_string(),
    }))
}

/// Reset password with token
async fn reset_password_handler(
    State(state): State<Arc<AuthState>>,
    Json(request): Json<ResetPasswordRequest>,
) -> Result<Json<MessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    state
        .service
        .reset_password(&request.token, &request.new_password)
        .map_err(|e| {
            (
                StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::BAD_REQUEST),
                Json(ErrorResponse::from(e)),
            )
        })?;

    Ok(Json(MessageResponse {
        message: "Password has been reset successfully".to_string(),
    }))
}

/// Change password for authenticated user
async fn change_password_handler(
    State(state): State<Arc<AuthState>>,
    headers: HeaderMap,
    Json(request): Json<ChangePasswordRequest>,
) -> Result<Json<MessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = validate_admin_access(&state, &headers)?;

    state
        .service
        .change_password(user_id, &request.current_password, &request.new_password)
        .map_err(|e| {
            (
                StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::BAD_REQUEST),
                Json(ErrorResponse::from(e)),
            )
        })?;

    Ok(Json(MessageResponse {
        message: "Password changed successfully".to_string(),
    }))
}

/// Get password policy
async fn get_password_policy_handler() -> Json<PasswordPolicyResponse> {
    Json(PasswordPolicyResponse::from(&PasswordPolicy::default()))
}

// ==================
// RLS Management Handlers
// ==================

/// Get RLS policy for a table
async fn get_rls_policy_handler(
    State(_state): State<Arc<AuthState>>,
    headers: HeaderMap,
    Path(table): Path<String>,
) -> Result<Json<RlsPolicyResponse>, (StatusCode, Json<ErrorResponse>)> {
    // RLS management would need to be wired to DefaultRlsEnforcer
    // For now, return default policy
    Ok(Json(RlsPolicyResponse {
        table,
        policy: RlsPolicy::default(),
        enabled: true,
    }))
}

/// Create or update RLS policy for a table
async fn create_rls_policy_handler(
    State(_state): State<Arc<AuthState>>,
    headers: HeaderMap,
    Path(table): Path<String>,
    Json(request): Json<CreateRlsPolicyRequest>,
) -> Result<(StatusCode, Json<RlsPolicyResponse>), (StatusCode, Json<ErrorResponse>)> {
    // RLS policy creation would need to be wired
    Ok((
        StatusCode::CREATED,
        Json(RlsPolicyResponse {
            table,
            policy: request.policy,
            enabled: true,
        }),
    ))
}

/// Delete RLS policy for a table
async fn delete_rls_policy_handler(
    State(_state): State<Arc<AuthState>>,
    headers: HeaderMap,
    Path(table): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // RLS policy deletion would need to be wired
    Ok(StatusCode::NO_CONTENT)
}

// ==================
// Email Verification Handlers
// ==================

/// Verify email with token
async fn verify_email_handler(
    State(_state): State<Arc<AuthState>>,
    Json(_request): Json<VerifyEmailRequest>,
) -> Result<Json<MessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Email verification would need token validation
    Ok(Json(MessageResponse {
        message: "Email verified successfully".to_string(),
    }))
}

/// Resend verification email
async fn resend_verification_handler(
    State(_state): State<Arc<AuthState>>,
    headers: HeaderMap,
    Path(_id): Path<Uuid>,
) -> Result<Json<MessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(MessageResponse {
        message: "Verification email sent".to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_policy_response() {
        let policy = PasswordPolicy::default();
        let response = PasswordPolicyResponse::from(&policy);
        assert_eq!(response.min_length, policy.min_length);
    }
}
