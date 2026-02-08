//! Security Hardening Configuration
//!
//! Phase 8: Security Degradation Safety
//!
//! - Fail-closed enforcement: deny access on system errors
//! - Audit logging for security events

use serde::{Deserialize, Serialize};

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Whether to enable fail-closed mode
    ///
    /// If true, system errors (e.g., database down, policy error) will
    /// result in immediate access denial (503/500), preserving security
    /// over availability.
    ///
    /// Default: true
    pub fail_closed_mode: bool,

    /// Whether to log all auth failures
    pub audit_auth_failures: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            fail_closed_mode: true,
            audit_auth_failures: true,
        }
    }
}

/// Helper to check if we should fail closed
pub fn should_fail_closed(config: &SecurityConfig) -> bool {
    config.fail_closed_mode
}
