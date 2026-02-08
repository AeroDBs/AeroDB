//! # Slow Query Tracking
//!
//! MANIFESTO ALIGNMENT: Threshold-based slow query detection for proactive performance management.
//! Per certification requirement: Must provide slow query tracking with configurable thresholds.
//!
//! # Design Principles
//!
//! 1. **Threshold-based**: Detection is based on explicit, configured thresholds
//! 2. **Non-blocking**: Slow query handling never blocks operation execution
//! 3. **Fire-and-forget**: Webhook failures are logged but do not affect execution
//! 4. **Deterministic**: Same duration + threshold produces same slow query decision
//! 5. **Observable**: All slow queries are explicitly logged
//!
//! # What This Module Does NOT Do (MANIFESTO ALIGNMENT)
//!
//! - **No automatic retries**: Webhook failures are logged, not retried
//! - **No sampling**: If enabled, all slow queries are tracked
//! - **No background threads**: Webhook calls are synchronous but timeout-bounded
//! - **No hidden aggregation**: Raw slow query events only

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Slow query configuration
///
/// MANIFESTO ALIGNMENT: Configuration is explicit, no hidden defaults.
/// Per certification: Must be configurable via aerodb.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowQueryConfig {
    /// Whether slow query tracking is enabled
    ///
    /// MANIFESTO ALIGNMENT: Tracking is opt-in, explicit.
    #[serde(default)]
    pub enabled: bool,

    /// Slow query threshold in milliseconds
    ///
    /// MANIFESTO ALIGNMENT: Threshold is configured, not guessed.
    /// Queries exceeding this duration are considered slow.
    #[serde(default = "default_threshold_ms")]
    pub threshold_ms: u64,

    /// Whether to emit structured log for slow queries
    ///
    /// MANIFESTO ALIGNMENT: Logging is explicit and configurable.
    #[serde(default = "default_emit_log")]
    pub emit_log: bool,

    /// Optional webhook URL for slow query alerts
    ///
    /// MANIFESTO ALIGNMENT: Alerting is explicit, opt-in.
    /// If provided, a POST request is sent with slow query details.
    #[serde(default)]
    pub webhook_url: Option<String>,

    /// Webhook timeout in milliseconds
    ///
    /// MANIFESTO ALIGNMENT: Timeout is explicit, bounded.
    /// Prevents webhook calls from blocking operations indefinitely.
    #[serde(default = "default_webhook_timeout_ms")]
    pub webhook_timeout_ms: u64,
}

fn default_threshold_ms() -> u64 {
    100 // 100ms default threshold
}

fn default_emit_log() -> bool {
    true // Log slow queries by default
}

fn default_webhook_timeout_ms() -> u64 {
    5000 // 5 second timeout
}

impl Default for SlowQueryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_ms: default_threshold_ms(),
            emit_log: default_emit_log(),
            webhook_url: None,
            webhook_timeout_ms: default_webhook_timeout_ms(),
        }
    }
}

impl SlowQueryConfig {
    /// Create a disabled configuration
    pub fn disabled() -> Self {
        Self::default()
    }

    /// Create an enabled configuration with default threshold
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Self::default()
        }
    }

    /// Create an enabled configuration with custom threshold
    pub fn with_threshold_ms(threshold_ms: u64) -> Self {
        Self {
            enabled: true,
            threshold_ms,
            ..Self::default()
        }
    }
}

/// Slow query event details
///
/// MANIFESTO ALIGNMENT: All fields are explicit and observable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowQueryEvent {
    /// Unique operation ID
    pub operation_id: uuid::Uuid,
    /// Collection name (if applicable)
    pub collection: Option<String>,
    /// Operation type (e.g., "find", "insert")
    pub operation_type: String,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Configured threshold in milliseconds
    pub threshold_ms: u64,
    /// User ID (if authenticated)
    pub user_id: Option<uuid::Uuid>,
    /// Index used (if any)
    pub index_used: Option<String>,
    /// Documents scanned (if applicable)
    pub documents_scanned: Option<usize>,
    /// Timestamp in ISO8601 format
    pub timestamp: String,
}

/// Slow query tracker
///
/// MANIFESTO ALIGNMENT: Deterministic slow query detection.
/// Per certification: Must track queries exceeding configured threshold.
pub struct SlowQueryTracker {
    config: SlowQueryConfig,
}

impl SlowQueryTracker {
    /// Create a new slow query tracker with the given configuration
    pub fn new(config: SlowQueryConfig) -> Self {
        Self { config }
    }

    /// Create a disabled tracker
    pub fn disabled() -> Self {
        Self {
            config: SlowQueryConfig::disabled(),
        }
    }

    /// Check if slow query tracking is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the configured threshold in milliseconds
    pub fn threshold_ms(&self) -> u64 {
        self.config.threshold_ms
    }

    /// Check if a duration exceeds the slow query threshold
    ///
    /// MANIFESTO ALIGNMENT: Deterministic comparison.
    pub fn is_slow(&self, duration_ms: u64) -> bool {
        self.config.enabled && duration_ms > self.config.threshold_ms
    }

    /// Track a slow query event
    ///
    /// MANIFESTO ALIGNMENT: Non-blocking slow query handling.
    /// - If emit_log is true, logs the slow query
    /// - If webhook_url is configured, sends a POST request
    /// - Webhook failures are logged but never crash the database
    pub fn track(&self, event: SlowQueryEvent) {
        if !self.config.enabled {
            return;
        }

        // Emit structured log if configured
        if self.config.emit_log {
            self.emit_log(&event);
        }

        // Send webhook if configured (fire-and-forget)
        if let Some(ref url) = self.config.webhook_url {
            self.send_webhook(url, &event);
        }
    }

    /// Emit structured log for slow query
    ///
    /// MANIFESTO ALIGNMENT: JSON structured logging.
    fn emit_log(&self, event: &SlowQueryEvent) {
        // Using eprintln for structured logging to stderr
        // In production, this would integrate with the Logger subsystem
        if let Ok(json) = serde_json::to_string(event) {
            eprintln!(
                "{{\"level\":\"WARN\",\"event\":\"SLOW_QUERY\",\"details\":{}}}",
                json
            );
        }
    }

    /// Send webhook notification for slow query
    ///
    /// MANIFESTO ALIGNMENT: Fire-and-forget, timeout-bounded.
    /// Failures are logged but never crash the database.
    fn send_webhook(&self, url: &str, event: &SlowQueryEvent) {
        // NOTE: This is a synchronous, blocking call with timeout.
        // In production, consider using a bounded async queue.
        // Per manifesto: We do NOT retry, we do NOT buffer.

        let timeout = Duration::from_millis(self.config.webhook_timeout_ms);

        // Attempt to send webhook - failure is non-fatal
        match self.try_send_webhook(url, event, timeout) {
            Ok(()) => {
                // Webhook sent successfully - no action needed
            }
            Err(e) => {
                // Log failure but do not crash
                eprintln!(
                    "{{\"level\":\"ERROR\",\"event\":\"SLOW_QUERY_WEBHOOK_FAILED\",\"url\":\"{}\",\"error\":\"{}\"}}",
                    url, e
                );
            }
        }
    }

    /// Try to send webhook with timeout
    ///
    /// Returns Ok(()) on success, Err(message) on failure.
    fn try_send_webhook(
        &self,
        url: &str,
        event: &SlowQueryEvent,
        timeout: Duration,
    ) -> Result<(), String> {
        // For production, this would use reqwest or ureq with timeout
        // For now, we implement a minimal HTTP POST using std::net

        use std::io::{Read, Write};
        use std::net::TcpStream;

        // Parse URL to extract host and path
        let url_without_protocol = url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))
            .ok_or_else(|| "Invalid URL protocol".to_string())?;

        let (host_port, path) = url_without_protocol
            .split_once('/')
            .map(|(h, p)| (h, format!("/{}", p)))
            .unwrap_or((url_without_protocol, "/".to_string()));

        let body = serde_json::to_string(event).map_err(|e| e.to_string())?;

        let request = format!(
            "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            path, host_port, body.len(), body
        );

        // Connect with timeout
        let mut stream = TcpStream::connect_timeout(
            &host_port
                .parse()
                .map_err(|e: std::net::AddrParseError| e.to_string())?,
            timeout,
        )
        .map_err(|e| e.to_string())?;

        stream
            .set_write_timeout(Some(timeout))
            .map_err(|e| e.to_string())?;
        stream
            .set_read_timeout(Some(timeout))
            .map_err(|e| e.to_string())?;

        stream
            .write_all(request.as_bytes())
            .map_err(|e| e.to_string())?;

        // Read response (we only care about success/failure)
        let mut response = [0u8; 128];
        let n = stream.read(&mut response).map_err(|e| e.to_string())?;

        // Check for 2xx status
        let response_str = std::str::from_utf8(&response[..n]).map_err(|e| e.to_string())?;
        if response_str.contains("200")
            || response_str.contains("201")
            || response_str.contains("202")
            || response_str.contains("204")
        {
            Ok(())
        } else {
            Err(format!("Webhook returned non-2xx response: {}", response_str))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_event(duration_ms: u64) -> SlowQueryEvent {
        SlowQueryEvent {
            operation_id: Uuid::new_v4(),
            collection: Some("test_collection".to_string()),
            operation_type: "find".to_string(),
            duration_ms,
            threshold_ms: 100,
            user_id: Some(Uuid::new_v4()),
            index_used: Some("pk_id".to_string()),
            documents_scanned: Some(1000),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    #[test]
    fn test_config_defaults() {
        let config = SlowQueryConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.threshold_ms, 100);
        assert!(config.emit_log);
        assert!(config.webhook_url.is_none());
        assert_eq!(config.webhook_timeout_ms, 5000);
    }

    #[test]
    fn test_config_enabled() {
        let config = SlowQueryConfig::enabled();
        assert!(config.enabled);
    }

    #[test]
    fn test_config_with_threshold() {
        let config = SlowQueryConfig::with_threshold_ms(500);
        assert!(config.enabled);
        assert_eq!(config.threshold_ms, 500);
    }

    #[test]
    fn test_tracker_disabled() {
        let tracker = SlowQueryTracker::disabled();
        assert!(!tracker.is_enabled());
        assert!(!tracker.is_slow(1000)); // Even 1000ms is not slow when disabled
    }

    #[test]
    fn test_tracker_enabled() {
        let tracker = SlowQueryTracker::new(SlowQueryConfig::enabled());
        assert!(tracker.is_enabled());
    }

    #[test]
    fn test_is_slow_below_threshold() {
        let tracker = SlowQueryTracker::new(SlowQueryConfig::with_threshold_ms(100));
        assert!(!tracker.is_slow(50));
        assert!(!tracker.is_slow(100)); // Equal to threshold is NOT slow
    }

    #[test]
    fn test_is_slow_above_threshold() {
        let tracker = SlowQueryTracker::new(SlowQueryConfig::with_threshold_ms(100));
        assert!(tracker.is_slow(101));
        assert!(tracker.is_slow(1000));
    }

    #[test]
    fn test_tracker_tracks_without_panic() {
        // Verify tracking doesn't panic even with no webhook configured
        let tracker = SlowQueryTracker::new(SlowQueryConfig::enabled());
        let event = create_test_event(200);
        tracker.track(event); // Should not panic
    }

    #[test]
    fn test_slow_query_event_serialization() {
        let event = create_test_event(150);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"duration_ms\":150"));
        assert!(json.contains("\"operation_type\":\"find\""));
    }

    #[test]
    fn test_webhook_failure_does_not_crash() {
        // CERTIFICATION REQUIREMENT: Webhook failure must not crash database
        let config = SlowQueryConfig {
            enabled: true,
            threshold_ms: 100,
            emit_log: false,
            webhook_url: Some("http://invalid-host-that-does-not-exist:9999/webhook".to_string()),
            webhook_timeout_ms: 100, // Very short timeout
        };
        let tracker = SlowQueryTracker::new(config);
        let event = create_test_event(200);

        // This should NOT panic even though webhook will fail
        tracker.track(event);
    }
}
