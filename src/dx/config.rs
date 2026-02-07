//! DX Configuration
//!
//! Per DX_INVARIANTS.md §P4-16 (Complete Removability):
//! - Phase 4 MUST be fully disableable at compile time or startup
//! - Disabling MUST require no migration, data changes, or behavior changes
//!
//! Per DX_OBSERVABILITY_API.md §3.1:
//! - Transport: HTTP (local only)
//! - Localhost binding enforced

/// Configuration for Phase 4 Developer Experience features.
///
/// Read-only, Phase 4, no semantic authority.
#[derive(Debug, Clone)]
pub struct DxConfig {
    /// Whether Phase 4 observability is enabled.
    pub enabled: bool,
    /// Port for the observability API server.
    pub port: u16,
    /// Bind address (localhost only per spec).
    pub bind_address: String,
}

impl Default for DxConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default per spec
            port: 9191,
            bind_address: "127.0.0.1".to_string(), // Local only
        }
    }
}

impl DxConfig {
    /// Create config with observability enabled.
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Self::default()
        }
    }

    /// Create config with observability disabled.
    pub fn disabled() -> Self {
        Self::default()
    }

    /// Check if DX is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get the full bind address with port.
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.bind_address, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default_disabled() {
        let config = DxConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.port, 9191);
        assert_eq!(config.bind_address, "127.0.0.1");
    }

    #[test]
    fn test_config_enabled() {
        let config = DxConfig::enabled();
        assert!(config.is_enabled());
    }

    #[test]
    fn test_bind_addr() {
        let config = DxConfig::default();
        assert_eq!(config.bind_addr(), "127.0.0.1:9191");
    }

    #[test]
    fn test_localhost_only() {
        // Per DX_OBSERVABILITY_API.md §3.1: HTTP (local only)
        let config = DxConfig::default();
        assert!(config.bind_address.starts_with("127."));
    }
}
