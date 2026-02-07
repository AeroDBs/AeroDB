//! # Setup Guard Middleware
//!
//! MANIFESTO ALIGNMENT: Enforces explicit setup discipline.
//! Per Design Manifesto: "The platform cannot be used before configuration."
//!
//! # Purpose
//!
//! This middleware ensures that protected API routes cannot be accessed
//! until the first-run setup wizard is complete. This prevents:
//!
//! 1. Accidental use of unconfigured system
//! 2. Silent failures due to missing configuration
//! 3. Security vulnerabilities from default states
//!
//! # How It Works
//!
//! 1. Checks `SetupState::is_ready()` before allowing request
//! 2. Returns 503 Service Unavailable if setup not complete
//! 3. Only applies to protected routes (not /setup/* or /health)

use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use serde::Serialize;

use super::setup_routes::SetupState;

/// Response returned when setup is not complete
///
/// MANIFESTO ALIGNMENT: Explicit error, no silent degradation.
#[derive(Debug, Serialize)]
pub struct SetupRequiredResponse {
    pub error: &'static str,
    pub code: &'static str,
    pub message: &'static str,
    pub setup_url: &'static str,
}

impl SetupRequiredResponse {
    /// Create the standard "setup required" response
    pub fn new() -> Self {
        Self {
            error: "SETUP_REQUIRED",
            code: "AERO_SETUP_REQUIRED",
            message: "AeroDB setup is not complete. Complete the setup wizard before using the API.",
            setup_url: "/setup/status",
        }
    }
}

impl Default for SetupRequiredResponse {
    fn default() -> Self {
        Self::new()
    }
}

/// Setup guard middleware
///
/// MANIFESTO ALIGNMENT: This middleware enforces the manifesto requirement
/// that "The platform cannot be used before configuration."
///
/// # Behavior
///
/// - If setup is complete: Request proceeds normally
/// - If setup NOT complete: Returns 503 with explicit error
///
/// # Why 503 Service Unavailable?
///
/// Per HTTP semantics:
/// - 403 Forbidden = "You don't have permission" (wrong - it's not about permission)
/// - 401 Unauthorized = "You need to authenticate" (wrong - it's not about auth)
/// - 503 Service Unavailable = "Service not ready, try later" (correct - setup is pending)
///
/// MANIFESTO ALIGNMENT: The error code is explicit and actionable.
pub async fn setup_guard(
    State(setup_state): State<Arc<SetupState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<SetupRequiredResponse>)> {
    // MANIFESTO ALIGNMENT: Check setup status explicitly
    if !setup_state.is_ready() {
        // MANIFESTO ALIGNMENT: Fail explicitly with actionable error
        return Err((StatusCode::SERVICE_UNAVAILABLE, Json(SetupRequiredResponse::new())));
    }

    // Setup is complete, proceed with request
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_required_response() {
        let response = SetupRequiredResponse::new();
        assert_eq!(response.code, "AERO_SETUP_REQUIRED");
        assert!(response.message.contains("setup"));
        assert!(response.message.contains("wizard"));
    }

    #[test]
    fn test_setup_state_not_ready() {
        let state = SetupState::new();
        // Default state should NOT be ready
        // MANIFESTO ALIGNMENT: System starts in explicit "not configured" state
        assert!(!state.is_ready());
    }
}
