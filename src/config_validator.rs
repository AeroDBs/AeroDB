//! Configuration Validation
//!
//! HARDENING: Validates all configuration at startup.
//! Rejects invalid values with explicit error messages.

use std::path::Path;

/// Configuration validation errors
#[derive(Debug)]
pub struct ConfigValidationError {
    pub field: String,
    pub value: String,
    pub message: String,
}

impl std::fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid configuration for '{}': {} (value: {})",
            self.field, self.message, self.value
        )
    }
}

impl std::error::Error for ConfigValidationError {}

/// Result of config validation
pub type ConfigResult<T> = Result<T, Vec<ConfigValidationError>>;

/// Configuration validator
pub struct ConfigValidator {
    errors: Vec<ConfigValidationError>,
}

impl ConfigValidator {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Add an error
    fn error(&mut self, field: &str, value: impl std::fmt::Display, message: &str) {
        self.errors.push(ConfigValidationError {
            field: field.to_string(),
            value: value.to_string(),
            message: message.to_string(),
        });
    }

    /// Validate port number (1-65535)
    pub fn validate_port(&mut self, field: &str, port: u16) -> &mut Self {
        if port == 0 {
            self.error(field, port, "Port must be between 1 and 65535");
        }
        self
    }

    /// Validate positive integer
    pub fn validate_positive(&mut self, field: &str, value: i64) -> &mut Self {
        if value <= 0 {
            self.error(field, value, "Value must be positive");
        }
        self
    }

    /// Validate non-negative integer
    pub fn validate_non_negative(&mut self, field: &str, value: i64) -> &mut Self {
        if value < 0 {
            self.error(field, value, "Value must be non-negative");
        }
        self
    }

    /// Validate range (inclusive)
    pub fn validate_range(&mut self, field: &str, value: i64, min: i64, max: i64) -> &mut Self {
        if value < min || value > max {
            self.error(field, value, &format!("Value must be between {} and {}", min, max));
        }
        self
    }

    /// Validate path exists
    pub fn validate_path_exists(&mut self, field: &str, path: &Path) -> &mut Self {
        if !path.exists() {
            self.error(field, path.display(), "Path does not exist");
        }
        self
    }

    /// Validate path is directory
    pub fn validate_is_directory(&mut self, field: &str, path: &Path) -> &mut Self {
        if path.exists() && !path.is_dir() {
            self.error(field, path.display(), "Path is not a directory");
        }
        self
    }

    /// Validate path is writable
    pub fn validate_writable(&mut self, field: &str, path: &Path) -> &mut Self {
        if path.exists() {
            // Try to check write permission
            let test_file = path.join(".aerodb_write_test");
            match std::fs::write(&test_file, "test") {
                Ok(_) => {
                    let _ = std::fs::remove_file(&test_file);
                }
                Err(_) => {
                    self.error(field, path.display(), "Path is not writable");
                }
            }
        }
        self
    }

    /// Validate non-empty string
    pub fn validate_non_empty(&mut self, field: &str, value: &str) -> &mut Self {
        if value.trim().is_empty() {
            self.error(field, value, "Value cannot be empty");
        }
        self
    }

    /// Validate URL format
    pub fn validate_url(&mut self, field: &str, value: &str) -> &mut Self {
        if !value.is_empty() && !value.starts_with("http://") && !value.starts_with("https://") {
            self.error(field, value, "URL must start with http:// or https://");
        }
        self
    }

    /// Validate duration in milliseconds
    pub fn validate_duration_ms(&mut self, field: &str, ms: u64, min_ms: u64, max_ms: u64) -> &mut Self {
        if ms < min_ms || ms > max_ms {
            self.error(
                field,
                format!("{}ms", ms),
                &format!("Duration must be between {}ms and {}ms", min_ms, max_ms),
            );
        }
        self
    }

    /// Validate bytes with human-readable limits
    pub fn validate_bytes(&mut self, field: &str, bytes: u64, min_bytes: u64, max_bytes: u64) -> &mut Self {
        if bytes < min_bytes || bytes > max_bytes {
            self.error(
                field,
                format_bytes(bytes),
                &format!(
                    "Size must be between {} and {}",
                    format_bytes(min_bytes),
                    format_bytes(max_bytes)
                ),
            );
        }
        self
    }

    /// Finish validation and return result
    pub fn finish(self) -> ConfigResult<()> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors)
        }
    }

    /// Check if any errors occurred
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get current errors
    pub fn errors(&self) -> &[ConfigValidationError] {
        &self.errors
    }
}

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Format bytes as human-readable string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}

/// Format validation errors for display
pub fn format_validation_errors(errors: &[ConfigValidationError]) -> String {
    errors
        .iter()
        .map(|e| format!("  - {}", e))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_validation() {
        let mut v = ConfigValidator::new();
        v.validate_port("port", 8080);
        assert!(v.finish().is_ok());

        let mut v = ConfigValidator::new();
        v.validate_port("port", 0);
        assert!(v.finish().is_err());
    }

    #[test]
    fn test_range_validation() {
        let mut v = ConfigValidator::new();
        v.validate_range("timeout", 100, 1, 1000);
        assert!(v.finish().is_ok());

        let mut v = ConfigValidator::new();
        v.validate_range("timeout", 2000, 1, 1000);
        let err = v.finish().unwrap_err();
        assert_eq!(err.len(), 1);
        assert!(err[0].message.contains("between"));
    }

    #[test]
    fn test_multiple_errors() {
        let mut v = ConfigValidator::new();
        v.validate_port("port", 0)
            .validate_positive("timeout", -5)
            .validate_non_empty("name", "");
        
        let errors = v.finish().unwrap_err();
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500B");
        assert_eq!(format_bytes(1024), "1.0KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0GB");
    }
}
