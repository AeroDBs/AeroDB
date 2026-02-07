//! Observability subsystem for AeroDB
//!
//! Per OBSERVABILITY.md, this module provides:
//! - Structured logging (JSON)
//! - Deterministic metrics
//! - Lifecycle event tracing
//!
//! # Principles
//!
//! 1. Observability is read-only
//! 2. No side effects on execution
//! 3. No async or background threads
//! 4. Deterministic output
//! 5. Zero allocations in hot paths (where possible)
//!
//! # Usage
//!
//! ```ignore
//! use aerodb::observability::{Logger, Event, MetricsRegistry, ObservationScope};
//!
//! // Log an event
//! Logger::info("QUERY_COMPLETE", &[("rows", "42")]);
//!
//! // Track metrics
//! let metrics = MetricsRegistry::new();
//! metrics.increment_queries_executed();
//!
//! // Scope-based logging
//! let scope = ObservationScope::new("CHECKPOINT");
//! // ... do work ...
//! scope.complete();
//! ```

pub mod audit;
mod events;
mod logger;
mod metrics;
mod scope;

pub use audit::{AuditAction, AuditLog, AuditOutcome, AuditRecord, FileAuditLog, MemoryAuditLog};
pub use events::Event;
pub use logger::{Logger, Severity};
pub use metrics::{MetricsRegistry, MetricsSnapshot};
pub use scope::{ObservationScope, Timer};

use std::fmt;
use std::io;

/// Observability error code
///
/// Per ERRORS.md format: AERO_CATEGORY_NAME
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservabilityErrorCode {
    /// Observability operation failed
    AeroObservabilityFailed,
}

impl ObservabilityErrorCode {
    /// Returns the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ObservabilityErrorCode::AeroObservabilityFailed => "AERO_OBSERVABILITY_FAILED",
        }
    }

    /// Returns the severity level (always ERROR for observability)
    pub fn severity(&self) -> Severity {
        Severity::Error
    }
}

impl fmt::Display for ObservabilityErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Observability error
///
/// Per ERRORS.md, observability errors are ERROR severity only.
/// Observability failure must never crash AeroDB.
#[derive(Debug)]
pub struct ObservabilityError {
    code: ObservabilityErrorCode,
    message: String,
    source: Option<io::Error>,
}

impl ObservabilityError {
    /// Create a new observability error
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            code: ObservabilityErrorCode::AeroObservabilityFailed,
            message: message.into(),
            source: None,
        }
    }

    /// Create with source error
    pub fn with_source(message: impl Into<String>, source: io::Error) -> Self {
        Self {
            code: ObservabilityErrorCode::AeroObservabilityFailed,
            message: message.into(),
            source: Some(source),
        }
    }

    /// Get the error code
    pub fn code(&self) -> ObservabilityErrorCode {
        self.code
    }

    /// Get the message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Observability errors are never fatal
    pub fn is_fatal(&self) -> bool {
        false
    }
}

impl fmt::Display for ObservabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[ERROR] {}: {}", self.code, self.message)?;
        if let Some(ref source) = self.source {
            write!(f, " (caused by: {})", source)?;
        }
        Ok(())
    }
}

impl std::error::Error for ObservabilityError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

/// Result type for observability operations
pub type ObservabilityResult<T> = Result<T, ObservabilityError>;

/// Log a lifecycle event
pub fn log_event(event: Event) {
    let severity = if event.is_fatal() {
        Severity::Fatal
    } else {
        Severity::Info
    };
    Logger::log(severity, event.as_str(), &[]);
}

/// Log a lifecycle event with fields
pub fn log_event_with_fields(event: Event, fields: &[(&str, &str)]) {
    let severity = if event.is_fatal() {
        Severity::Fatal
    } else {
        Severity::Info
    };
    Logger::log(severity, event.as_str(), fields);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observability_error_not_fatal() {
        let err = ObservabilityError::new("test error");
        assert!(!err.is_fatal());
    }

    #[test]
    fn test_observability_error_code() {
        let err = ObservabilityError::new("test error");
        assert_eq!(err.code(), ObservabilityErrorCode::AeroObservabilityFailed);
    }

    #[test]
    fn test_observability_error_display() {
        let err = ObservabilityError::new("test message");
        let display = format!("{}", err);
        assert!(display.contains("ERROR"));
        assert!(display.contains("AERO_OBSERVABILITY_FAILED"));
        assert!(display.contains("test message"));
    }

    #[test]
    fn test_log_event() {
        // This just verifies no panic
        log_event(Event::BootStart);
        log_event(Event::BootComplete);
    }

    #[test]
    fn test_log_event_with_fields() {
        log_event_with_fields(Event::ConfigLoaded, &[("data_dir", "/tmp/test")]);
    }
}
