//! Setup Routes
//!
//! First-run setup wizard endpoints for AeroDB initialization.
//! These endpoints are only accessible before setup is complete.

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

// ==================
// Setup State
// ==================

/// Setup status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SetupStatus {
    /// No setup has been performed
    Uninitialized,
    /// Setup is in progress
    InProgress,
    /// Setup is complete, system is ready
    Ready,
}

impl Default for SetupStatus {
    fn default() -> Self {
        Self::Uninitialized
    }
}

/// Storage configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageConfig {
    pub data_dir: String,
    pub wal_dir: String,
    pub snapshot_dir: String,
}

/// Auth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub jwt_expiry_hours: u32,
    pub refresh_expiry_days: u32,
    pub password_min_length: u8,
    pub require_uppercase: bool,
    pub require_number: bool,
    pub require_special: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_expiry_hours: 24,
            refresh_expiry_days: 7,
            password_min_length: 8,
            require_uppercase: true,
            require_number: true,
            require_special: false,
        }
    }
}

/// Admin user configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AdminConfig {
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created: bool,
}

/// Complete setup state
#[derive(Debug, Default)]
pub struct SetupState {
    status: RwLock<SetupStatus>,
    storage: RwLock<Option<StorageConfig>>,
    auth: RwLock<Option<AuthConfig>>,
    admin: RwLock<Option<AdminConfig>>,
}

impl SetupState {
    pub fn new() -> Self {
        // In a real implementation, this would load from a persistent marker file
        Self::default()
    }

    /// Check if setup is complete
    pub fn is_ready(&self) -> bool {
        *self.status.read().unwrap() == SetupStatus::Ready
    }

    /// Get current status
    pub fn get_status(&self) -> SetupStatus {
        *self.status.read().unwrap()
    }

    /// Mark setup as in progress
    pub fn set_in_progress(&self) {
        let mut status = self.status.write().unwrap();
        if *status == SetupStatus::Uninitialized {
            *status = SetupStatus::InProgress;
        }
    }

    /// Mark setup as complete
    pub fn set_ready(&self) {
        *self.status.write().unwrap() = SetupStatus::Ready;
    }

    /// Store storage config
    pub fn set_storage(&self, config: StorageConfig) {
        *self.storage.write().unwrap() = Some(config);
        self.set_in_progress();
    }

    /// Store auth config
    pub fn set_auth(&self, config: AuthConfig) {
        *self.auth.write().unwrap() = Some(config);
    }

    /// Store admin config
    pub fn set_admin(&self, config: AdminConfig) {
        *self.admin.write().unwrap() = Some(config);
    }

    /// Get storage config
    pub fn get_storage(&self) -> Option<StorageConfig> {
        self.storage.read().unwrap().clone()
    }

    /// Get auth config
    pub fn get_auth(&self) -> Option<AuthConfig> {
        self.auth.read().unwrap().clone()
    }

    /// Get admin config
    pub fn get_admin(&self) -> Option<AdminConfig> {
        self.admin.read().unwrap().clone()
    }

    /// Check if all steps are complete
    pub fn all_steps_complete(&self) -> bool {
        self.storage.read().unwrap().is_some()
            && self.auth.read().unwrap().is_some()
            && self
                .admin
                .read()
                .unwrap()
                .as_ref()
                .map(|a| a.created)
                .unwrap_or(false)
    }
}

// ==================
// Request/Response Types
// ==================

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub status: SetupStatus,
    pub storage_configured: bool,
    pub auth_configured: bool,
    pub admin_created: bool,
}

#[derive(Debug, Deserialize)]
pub struct StorageRequest {
    pub data_dir: String,
    pub wal_dir: String,
    pub snapshot_dir: String,
}

#[derive(Debug, Serialize)]
pub struct StorageResponse {
    pub success: bool,
    pub data_dir: String,
    pub wal_dir: String,
    pub snapshot_dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_dir_exists: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wal_dir_exists: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_dir_exists: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AuthRequest {
    pub jwt_expiry_hours: u32,
    pub refresh_expiry_days: u32,
    #[serde(default = "default_password_min_length")]
    pub password_min_length: u8,
    #[serde(default)]
    pub require_uppercase: bool,
    #[serde(default)]
    pub require_number: bool,
    #[serde(default)]
    pub require_special: bool,
}

fn default_password_min_length() -> u8 {
    8
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub success: bool,
    pub jwt_expiry_hours: u32,
    pub refresh_expiry_days: u32,
}

#[derive(Debug, Deserialize)]
pub struct AdminRequest {
    pub email: String,
    pub password: String,
    pub confirm_password: String,
}

#[derive(Debug, Serialize)]
pub struct AdminResponse {
    pub success: bool,
    pub email: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct CompleteResponse {
    pub success: bool,
    pub status: SetupStatus,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
}

// ==================
// Route Handlers
// ==================

/// GET /setup/status - Check current setup status
async fn get_status(State(state): State<Arc<SetupState>>) -> Json<StatusResponse> {
    Json(StatusResponse {
        status: state.get_status(),
        storage_configured: state.get_storage().is_some(),
        auth_configured: state.get_auth().is_some(),
        admin_created: state.get_admin().map(|a| a.created).unwrap_or(false),
    })
}

/// POST /setup/storage - Configure storage directories
async fn configure_storage(
    State(state): State<Arc<SetupState>>,
    Json(request): Json<StorageRequest>,
) -> Result<Json<StorageResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Check if already complete
    if state.is_ready() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Setup already complete".to_string(),
                code: 403,
            }),
        ));
    }

    // Validate directories
    let data_path = PathBuf::from(&request.data_dir);
    let wal_path = PathBuf::from(&request.wal_dir);
    let snapshot_path = PathBuf::from(&request.snapshot_dir);

    // Create directories if they don't exist
    for (path, name) in [
        (&data_path, "data"),
        (&wal_path, "wal"),
        (&snapshot_path, "snapshot"),
    ] {
        if !path.exists() {
            std::fs::create_dir_all(path).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: format!("Failed to create {} directory: {}", name, e),
                        code: 400,
                    }),
                )
            })?;
        }
    }

    // Store configuration
    let config = StorageConfig {
        data_dir: request.data_dir.clone(),
        wal_dir: request.wal_dir.clone(),
        snapshot_dir: request.snapshot_dir.clone(),
    };
    state.set_storage(config);

    Ok(Json(StorageResponse {
        success: true,
        data_dir: request.data_dir,
        wal_dir: request.wal_dir,
        snapshot_dir: request.snapshot_dir,
        data_dir_exists: Some(data_path.exists()),
        wal_dir_exists: Some(wal_path.exists()),
        snapshot_dir_exists: Some(snapshot_path.exists()),
    }))
}

/// POST /setup/auth - Configure authentication settings
async fn configure_auth(
    State(state): State<Arc<SetupState>>,
    Json(request): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Check if already complete
    if state.is_ready() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Setup already complete".to_string(),
                code: 403,
            }),
        ));
    }

    // Validate
    if request.jwt_expiry_hours < 1 || request.jwt_expiry_hours > 168 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "JWT expiry must be between 1 and 168 hours".to_string(),
                code: 400,
            }),
        ));
    }

    if request.refresh_expiry_days < 1 || request.refresh_expiry_days > 30 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Refresh token expiry must be between 1 and 30 days".to_string(),
                code: 400,
            }),
        ));
    }

    // Store configuration
    let config = AuthConfig {
        jwt_expiry_hours: request.jwt_expiry_hours,
        refresh_expiry_days: request.refresh_expiry_days,
        password_min_length: request.password_min_length,
        require_uppercase: request.require_uppercase,
        require_number: request.require_number,
        require_special: request.require_special,
    };
    state.set_auth(config);

    Ok(Json(AuthResponse {
        success: true,
        jwt_expiry_hours: request.jwt_expiry_hours,
        refresh_expiry_days: request.refresh_expiry_days,
    }))
}

/// POST /setup/admin - Create the first admin user
async fn create_admin(
    State(state): State<Arc<SetupState>>,
    Json(request): Json<AdminRequest>,
) -> Result<Json<AdminResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Check if already complete
    if state.is_ready() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Setup already complete".to_string(),
                code: 403,
            }),
        ));
    }

    // Check if admin already created
    if state.get_admin().map(|a| a.created).unwrap_or(false) {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "Admin user already created".to_string(),
                code: 409,
            }),
        ));
    }

    // Validate email
    if !request.email.contains('@') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid email address".to_string(),
                code: 400,
            }),
        ));
    }

    // Validate password
    if request.password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Password must be at least 8 characters".to_string(),
                code: 400,
            }),
        ));
    }

    // Check password confirmation
    if request.password != request.confirm_password {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Passwords do not match".to_string(),
                code: 400,
            }),
        ));
    }

    // In a real implementation, this would:
    // 1. Hash the password with argon2
    // 2. Store in the users table with admin role
    // For now, store placeholder
    let config = AdminConfig {
        email: request.email.clone(),
        password_hash: format!("hashed_{}", request.password.len()), // Placeholder
        created: true,
    };
    state.set_admin(config);

    Ok(Json(AdminResponse {
        success: true,
        email: request.email,
        message: "Admin user created successfully".to_string(),
    }))
}

/// POST /setup/complete - Finalize setup and lock configuration
async fn complete_setup(
    State(state): State<Arc<SetupState>>,
) -> Result<Json<CompleteResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Check if already complete
    if state.is_ready() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Setup already complete".to_string(),
                code: 403,
            }),
        ));
    }

    // Verify all steps are done
    if !state.all_steps_complete() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "All setup steps must be completed first".to_string(),
                code: 400,
            }),
        ));
    }

    // In a real implementation, this would:
    // 1. Write a setup marker file
    // 2. Initialize the database with the config
    // 3. Start background services
    state.set_ready();

    Ok(Json(CompleteResponse {
        success: true,
        status: SetupStatus::Ready,
        message: "Setup complete! AeroDB is ready to use.".to_string(),
    }))
}

// ==================
// Router
// ==================

/// Create setup routes
pub fn setup_routes(state: Arc<SetupState>) -> Router {
    Router::new()
        .route("/status", get(get_status))
        .route("/storage", post(configure_storage))
        .route("/auth", post(configure_auth))
        .route("/admin", post(create_admin))
        .route("/complete", post(complete_setup))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_state_default() {
        let state = SetupState::new();
        assert_eq!(state.get_status(), SetupStatus::Uninitialized);
        assert!(!state.is_ready());
    }

    #[test]
    fn test_setup_flow() {
        let state = SetupState::new();

        // Configure storage
        state.set_storage(StorageConfig {
            data_dir: "/tmp/data".to_string(),
            wal_dir: "/tmp/wal".to_string(),
            snapshot_dir: "/tmp/snapshot".to_string(),
        });
        assert_eq!(state.get_status(), SetupStatus::InProgress);

        // Configure auth
        state.set_auth(AuthConfig::default());

        // Create admin
        state.set_admin(AdminConfig {
            email: "admin@example.com".to_string(),
            password_hash: "hashed".to_string(),
            created: true,
        });

        // All steps should be complete
        assert!(state.all_steps_complete());

        // Finalize
        state.set_ready();
        assert!(state.is_ready());
        assert_eq!(state.get_status(), SetupStatus::Ready);
    }
}
