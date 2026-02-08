//! Settings HTTP Routes
//!
//! Runtime configuration management for AeroDB.
//! Allows admin console to view and update settings without restart.

use std::sync::{Arc, RwLock};

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, patch},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::observability::slow_query::SlowQueryConfig;
use crate::realtime::backpressure::BackpressureConfig;

// ==================
// Settings Types
// ==================

/// Observability settings that can be configured at runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilitySettings {
    /// Slow query tracking configuration
    pub slow_query: SlowQueryConfig,
    /// Whether operation log is enabled
    pub operation_log_enabled: bool,
}

impl Default for ObservabilitySettings {
    fn default() -> Self {
        Self {
            slow_query: SlowQueryConfig::default(),
            operation_log_enabled: false,
        }
    }
}

/// Realtime settings that can be configured at runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeSettings {
    /// Backpressure configuration
    pub backpressure: BackpressureConfig,
}

impl Default for RealtimeSettings {
    fn default() -> Self {
        Self {
            backpressure: BackpressureConfig::default(),
        }
    }
}

/// All runtime settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllSettings {
    pub observability: ObservabilitySettings,
    pub realtime: RealtimeSettings,
}

// ==================
// Shared State
// ==================

/// Settings state shared across handlers
///
/// MANIFESTO ALIGNMENT: Thread-safe, explicit state management.
pub struct SettingsState {
    observability: Arc<RwLock<ObservabilitySettings>>,
    realtime: Arc<RwLock<RealtimeSettings>>,
}

impl SettingsState {
    /// Create new settings state with defaults
    pub fn new() -> Self {
        Self {
            observability: Arc::new(RwLock::new(ObservabilitySettings::default())),
            realtime: Arc::new(RwLock::new(RealtimeSettings::default())),
        }
    }

    /// Get current observability settings
    pub fn get_observability(&self) -> ObservabilitySettings {
        self.observability.read().unwrap().clone()
    }

    /// Update observability settings
    pub fn update_observability(&self, settings: ObservabilitySettings) {
        *self.observability.write().unwrap() = settings;
    }

    /// Get current realtime settings
    pub fn get_realtime(&self) -> RealtimeSettings {
        self.realtime.read().unwrap().clone()
    }

    /// Update realtime settings
    pub fn update_realtime(&self, settings: RealtimeSettings) {
        *self.realtime.write().unwrap() = settings;
    }

    /// Get all settings
    pub fn get_all(&self) -> AllSettings {
        AllSettings {
            observability: self.get_observability(),
            realtime: self.get_realtime(),
        }
    }
}

impl Default for SettingsState {
    fn default() -> Self {
        Self::new()
    }
}

// ==================
// Request/Response Types
// ==================

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
}

#[derive(Debug, Deserialize)]
pub struct UpdateObservabilityRequest {
    #[serde(default)]
    pub slow_query: Option<SlowQueryConfig>,
    #[serde(default)]
    pub operation_log_enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRealtimeRequest {
    #[serde(default)]
    pub backpressure: Option<BackpressureConfig>,
}

// ==================
// Route Handlers
// ==================

/// GET /settings - Get all current settings
async fn get_all_settings(
    State(state): State<Arc<SettingsState>>,
) -> Json<AllSettings> {
    Json(state.get_all())
}

/// GET /settings/observability - Get observability settings
async fn get_observability_settings(
    State(state): State<Arc<SettingsState>>,
) -> Json<ObservabilitySettings> {
    Json(state.get_observability())
}

/// PATCH /settings/observability - Update observability settings
async fn update_observability_settings(
    State(state): State<Arc<SettingsState>>,
    Json(request): Json<UpdateObservabilityRequest>,
) -> Result<Json<ObservabilitySettings>, (StatusCode, Json<ErrorResponse>)> {
    let mut settings = state.get_observability();

    // Update fields if provided
    if let Some(slow_query) = request.slow_query {
        // Validate slow query config
        if slow_query.threshold_ms == 0 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Slow query threshold must be greater than 0".to_string(),
                    code: 400,
                }),
            ));
        }

        if slow_query.threshold_ms > 60000 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Slow query threshold must be less than 60000ms".to_string(),
                    code: 400,
                }),
            ));
        }

        // Validate webhook URL if provided
        if let Some(ref url) = slow_query.webhook_url {
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "Webhook URL must start with http:// or https://".to_string(),
                        code: 400,
                    }),
                ));
            }
        }

        settings.slow_query = slow_query;
    }

    if let Some(enabled) = request.operation_log_enabled {
        settings.operation_log_enabled = enabled;
    }

    // Update state
    state.update_observability(settings.clone());

    Ok(Json(settings))
}

/// GET /settings/realtime - Get realtime settings
async fn get_realtime_settings(
    State(state): State<Arc<SettingsState>>,
) -> Json<RealtimeSettings> {
    Json(state.get_realtime())
}

/// PATCH /settings/realtime - Update realtime settings
async fn update_realtime_settings(
    State(state): State<Arc<SettingsState>>,
    Json(request): Json<UpdateRealtimeRequest>,
) -> Result<Json<RealtimeSettings>, (StatusCode, Json<ErrorResponse>)> {
    let mut settings = state.get_realtime();

    // Update fields if provided
    if let Some(backpressure) = request.backpressure {
        // Validate backpressure config
        if backpressure.max_pending_messages == 0 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Max pending messages must be greater than 0".to_string(),
                    code: 400,
                }),
            ));
        }

        if backpressure.max_pending_messages > 1_000_000 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Max pending messages must be less than 1,000,000".to_string(),
                    code: 400,
                }),
            ));
        }

        settings.backpressure = backpressure;
    }

    // Update state
    state.update_realtime(settings.clone());

    Ok(Json(settings))
}

// ==================
// Router
// ==================

/// Create settings routes
pub fn settings_routes(state: Arc<SettingsState>) -> Router {
    Router::new()
        // Get all settings
        .route("/", get(get_all_settings))
        // Observability settings
        .route("/observability", get(get_observability_settings))
        .route("/observability", patch(update_observability_settings))
        // Realtime settings
        .route("/realtime", get(get_realtime_settings))
        .route("/realtime", patch(update_realtime_settings))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_state_creation() {
        let state = SettingsState::new();
        let obs = state.get_observability();
        assert!(!obs.slow_query.enabled);
    }

    #[test]
    fn test_update_observability() {
        let state = SettingsState::new();
        
        let mut settings = state.get_observability();
        settings.slow_query.enabled = true;
        settings.slow_query.threshold_ms = 200;
        
        state.update_observability(settings.clone());
        
        let updated = state.get_observability();
        assert!(updated.slow_query.enabled);
        assert_eq!(updated.slow_query.threshold_ms, 200);
    }

    #[test]
    fn test_update_realtime() {
        let state = SettingsState::new();
        
        let mut settings = state.get_realtime();
        settings.backpressure.max_pending_messages = 500;
        
        state.update_realtime(settings.clone());
        
        let updated = state.get_realtime();
        assert_eq!(updated.backpressure.max_pending_messages, 500);
    }

    #[test]
    fn test_get_all_settings() {
        let state = SettingsState::new();
        let all = state.get_all();
        
        assert!(!all.observability.slow_query.enabled);
        assert_eq!(all.realtime.backpressure.max_pending_messages, 1000);
    }
}
