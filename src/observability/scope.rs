//! ObservationScope for automatic start/complete logging
//!
//! Per OBSERVABILITY.md:
//! - Logs START event on creation
//! - Logs COMPLETE event on drop
//! - Logs ERROR/FATAL on early exit

use std::cell::Cell;

use super::logger::{Logger, Severity};

/// A scope that automatically logs start and complete events
///
/// # Usage
///
/// ```ignore
/// let scope = ObservationScope::new("SNAPSHOT");
/// // ... do work ...
/// scope.complete(); // logs SNAPSHOT_COMPLETE
/// // if not completed, logs SNAPSHOT_ERROR on drop
/// ```
///
/// # Behavior
///
/// - Logs `{name}_START` on creation (at INFO level)
/// - Logs `{name}_COMPLETE` when `complete()` is called (at INFO level)
/// - Logs `{name}_ERROR` on drop if not completed (at ERROR level)
pub struct ObservationScope<'a> {
    name: &'a str,
    completed: Cell<bool>,
    fields: Vec<(&'a str, String)>,
}

impl<'a> ObservationScope<'a> {
    /// Create a new observation scope
    ///
    /// Logs `{name}_BEGIN` immediately.
    pub fn new(name: &'a str) -> Self {
        let event = format!("{}_BEGIN", name);
        Logger::info(&event, &[]);

        Self {
            name,
            completed: Cell::new(false),
            fields: Vec::new(),
        }
    }

    /// Create a new observation scope with additional fields
    pub fn with_fields(name: &'a str, fields: &[(&'a str, &str)]) -> Self {
        let event = format!("{}_BEGIN", name);
        let field_refs: Vec<(&str, &str)> = fields.iter().map(|(k, v)| (*k, *v)).collect();
        Logger::info(&event, &field_refs);

        Self {
            name,
            completed: Cell::new(false),
            fields: fields.iter().map(|(k, v)| (*k, v.to_string())).collect(),
        }
    }

    /// Mark the scope as successfully completed
    ///
    /// Logs `{name}_COMPLETE` at INFO level.
    pub fn complete(self) {
        self.completed.set(true);
        let event = format!("{}_COMPLETE", self.name);
        let field_refs: Vec<(&str, &str)> =
            self.fields.iter().map(|(k, v)| (*k, v.as_str())).collect();
        Logger::info(&event, &field_refs);
    }

    /// Mark the scope as successfully completed with additional fields
    pub fn complete_with_fields(self, extra_fields: &[(&str, &str)]) {
        self.completed.set(true);
        let event = format!("{}_COMPLETE", self.name);

        let mut all_fields: Vec<(&str, &str)> =
            self.fields.iter().map(|(k, v)| (*k, v.as_str())).collect();
        all_fields.extend(extra_fields.iter().copied());

        Logger::info(&event, &all_fields);
    }

    /// Mark the scope as failed with a reason
    ///
    /// Logs `{name}_FAILED` at ERROR level.
    pub fn fail(self, reason: &str) {
        self.completed.set(true);
        let event = format!("{}_FAILED", self.name);
        Logger::error(&event, &[("reason", reason)]);
    }

    /// Mark the scope as failed with FATAL severity
    ///
    /// Logs `{name}_FAILED` at FATAL level.
    pub fn fail_fatal(self, reason: &str) {
        self.completed.set(true);
        let event = format!("{}_FAILED", self.name);
        Logger::fatal(&event, &[("reason", reason)]);
    }

    /// Check if the scope has been completed
    pub fn is_completed(&self) -> bool {
        self.completed.get()
    }
}

impl Drop for ObservationScope<'_> {
    fn drop(&mut self) {
        // Only log error if not already completed
        if !self.completed.get() {
            let event = format!("{}_INCOMPLETE", self.name);
            Logger::warn(&event, &[("reason", "scope dropped without completion")]);
        }
    }
}

/// A simple duration timer for logging elapsed time
pub struct Timer {
    start: std::time::Instant,
}

impl Timer {
    /// Create a new timer
    pub fn new() -> Self {
        Self {
            start: std::time::Instant::now(),
        }
    }

    /// Get elapsed milliseconds as a string
    pub fn elapsed_ms(&self) -> String {
        self.start.elapsed().as_millis().to_string()
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_creation_logs_begin() {
        // This test verifies the scope can be created
        // (actual log output goes to stdout in tests)
        let scope = ObservationScope::new("TEST");
        assert!(!scope.is_completed());
    }

    #[test]
    fn test_scope_complete() {
        let scope = ObservationScope::new("TEST");
        scope.complete();
        // No assertion needed - just verify it doesn't panic
    }

    #[test]
    fn test_scope_with_fields() {
        let scope = ObservationScope::with_fields("TEST", &[("key", "value")]);
        scope.complete();
    }

    #[test]
    fn test_scope_complete_with_extra_fields() {
        let scope = ObservationScope::new("TEST");
        scope.complete_with_fields(&[("result", "success")]);
    }

    #[test]
    fn test_scope_fail() {
        let scope = ObservationScope::new("TEST");
        scope.fail("something went wrong");
    }

    #[test]
    fn test_scope_fail_fatal() {
        let scope = ObservationScope::new("TEST");
        scope.fail_fatal("unrecoverable error");
    }

    #[test]
    fn test_scope_drop_without_complete() {
        // This should log a warning but not panic
        let scope = ObservationScope::new("TEST");
        drop(scope);
    }

    #[test]
    fn test_timer() {
        let timer = Timer::new();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = timer.elapsed_ms();
        let ms: u64 = elapsed.parse().unwrap();
        assert!(ms >= 10);
    }
}
