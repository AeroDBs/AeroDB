//! Auth HTTP Routes
//!
//! HTTP endpoints for authentication using the existing AuthService.

use std::sync::Arc;

use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};

use crate::auth::api::AuthService;
use crate::auth::crypto::PasswordPolicy;
use crate::auth::errors::AuthError;
use crate::auth::jwt::{JwtConfig, JwtManager, TokenResponse};
use crate::auth::session::{InMemorySessionRepository, SessionConfig};
use crate::auth::user::{InMemoryUserRepository, LoginRequest, SignupRequest, User};

/// Shared auth state
pub struct AuthState {
    pub service: AuthService<InMemoryUserRepository, InMemorySessionRepository>,
}

impl AuthState {
    /// Create new auth state with default config
    pub fn new() -> Self {
        Self {
            service: AuthService::new(
                InMemoryUserRepository::new(),
                InMemorySessionRepository::new(),
                JwtConfig::default(),
                SessionConfig::default(),
                PasswordPolicy::default(),
            ),
        }
    }
}

impl Default for AuthState {
    fn default() -> Self {
        Self::new()
    }
}

/// Auth routes with shared state
pub fn auth_routes(state: Arc<AuthState>) -> Router {
    Router::new()
        .route("/signup", post(signup_handler))
        .route("/login", post(login_handler))
        .route("/refresh", post(refresh_handler))
        .route("/logout", post(logout_handler))
        .route("/user", get(get_user_handler))
        .with_state(state)
}

// ==================
// Request/Response Types
// ==================

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserResponse,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub created_at: String,
}

impl From<&User> for UserResponse {
    fn from(user: &User) -> Self {
        Self {
            id: user.id.to_string(),
            email: user.email.clone(),
            created_at: user.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
}

impl From<AuthError> for ErrorResponse {
    fn from(err: AuthError) -> Self {
        Self {
            error: err.to_string(),
            code: err.status_code(),
        }
    }
}

// ==================
// Handlers
// ==================

/// Signup handler
async fn signup_handler(
    State(state): State<Arc<AuthState>>,
    Json(request): Json<SignupRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, Json<ErrorResponse>)> {
    match state.service.signup(request) {
        Ok((user, tokens)) => {
            let response = AuthResponse {
                user: UserResponse::from(&user),
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                expires_in: tokens.expires_in as u64,
            };
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(e) => {
            let status =
                StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            Err((status, Json(ErrorResponse::from(e))))
        }
    }
}

/// Login handler
async fn login_handler(
    State(state): State<Arc<AuthState>>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.service.login(request) {
        Ok((user, tokens)) => {
            let response = AuthResponse {
                user: UserResponse::from(&user),
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                expires_in: tokens.expires_in as u64,
            };
            Ok(Json(response))
        }
        Err(e) => {
            let status = StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::UNAUTHORIZED);
            Err((status, Json(ErrorResponse::from(e))))
        }
    }
}

/// Refresh token handler
async fn refresh_handler(
    State(state): State<Arc<AuthState>>,
    Json(request): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.service.refresh(&request.refresh_token) {
        Ok(tokens) => {
            let response = RefreshResponse {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                expires_in: tokens.expires_in as u64,
            };
            Ok(Json(response))
        }
        Err(e) => {
            let status = StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::UNAUTHORIZED);
            Err((status, Json(ErrorResponse::from(e))))
        }
    }
}

/// Logout handler
async fn logout_handler(
    State(state): State<Arc<AuthState>>,
    Json(request): Json<LogoutRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match state.service.logout(&request.refresh_token) {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => {
            let status = StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::BAD_REQUEST);
            Err((status, Json(ErrorResponse::from(e))))
        }
    }
}

/// Get current user handler (requires Authorization header)
async fn get_user_handler(
    State(state): State<Arc<AuthState>>,
    headers: HeaderMap,
) -> Result<Json<UserResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Extract bearer token
    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));

    let token = match auth_header {
        Some(t) => t,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Missing authorization header".to_string(),
                    code: 401,
                }),
            ))
        }
    };

    // Validate token and get user
    match state.service.validate_access_token(token) {
        Ok(ctx) => {
            if let Some(user_id) = ctx.user_id {
                match state.service.get_user(user_id) {
                    Ok(user) => Ok(Json(UserResponse::from(&user))),
                    Err(e) => {
                        let status =
                            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::NOT_FOUND);
                        Err((status, Json(ErrorResponse::from(e))))
                    }
                }
            } else {
                Err((
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse {
                        error: "Invalid token".to_string(),
                        code: 401,
                    }),
                ))
            }
        }
        Err(e) => {
            let status = StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::UNAUTHORIZED);
            Err((status, Json(ErrorResponse::from(e))))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_state_creation() {
        let state = AuthState::new();
        // Just verify it creates without panic
        assert!(true);
        let _ = state;
    }
}
