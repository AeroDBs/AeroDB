//! HTTP Server Configuration
//!
//! Configuration for the HTTP server including host, port, and CORS settings.

use serde::{Deserialize, Serialize};

/// HTTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpServerConfig {
    /// Host to bind to (default: "0.0.0.0")
    #[serde(default = "default_host")]
    pub host: String,

    /// Port to bind to (default: 54321)
    #[serde(default = "default_port")]
    pub port: u16,

    /// CORS allowed origins (default: ["http://localhost:5173"])
    #[serde(default = "default_cors_origins")]
    pub cors_origins: Vec<String>,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    54321
}

fn default_cors_origins() -> Vec<String> {
    vec![
        "http://localhost:5173".to_string(), // Vite dev server
        "http://localhost:3000".to_string(), // Common dev port
        "http://127.0.0.1:5173".to_string(),
    ]
}

impl Default for HttpServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            cors_origins: default_cors_origins(),
        }
    }
}

impl HttpServerConfig {
    /// Create a new config with specified port
    pub fn with_port(port: u16) -> Self {
        Self {
            port,
            ..Default::default()
        }
    }

    /// Get the socket address string
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = HttpServerConfig::default();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 54321);
        assert!(!config.cors_origins.is_empty());
    }

    #[test]
    fn test_socket_addr() {
        let config = HttpServerConfig::with_port(8080);
        assert_eq!(config.socket_addr(), "0.0.0.0:8080");
    }
}
