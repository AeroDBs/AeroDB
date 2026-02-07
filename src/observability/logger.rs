//! Structured JSON logger for AeroDB
//!
//! Per OBSERVABILITY.md:
//! - Structured logs (JSON)
//! - Deterministic key ordering
//! - Explicit severity levels
//! - One log line = one event
//! - Synchronous, no buffering

use std::fmt;
use std::io::{self, Write};

/// Log severity levels per OBSERVABILITY.md
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Debug-level detail
    Trace = 0,
    /// Normal operations
    Info = 1,
    /// Recoverable issues
    Warn = 2,
    /// Operation failures
    Error = 3,
    /// Unrecoverable, process exits
    Fatal = 4,
}

impl Severity {
    /// Returns the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Trace => "TRACE",
            Severity::Info => "INFO",
            Severity::Warn => "WARN",
            Severity::Error => "ERROR",
            Severity::Fatal => "FATAL",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A structured logger that outputs JSON logs
///
/// Per OBSERVABILITY.md:
/// - Logs are synchronous
/// - No buffering
/// - Deterministic key ordering
pub struct Logger;

impl Logger {
    /// Log an event with the given severity and fields
    ///
    /// Fields are output in deterministic order (alphabetical by key)
    pub fn log(severity: Severity, event: &str, fields: &[(&str, &str)]) {
        Self::log_to_writer(severity, event, fields, &mut io::stdout());
    }

    /// Log to stderr (for errors and fatal messages)
    pub fn log_stderr(severity: Severity, event: &str, fields: &[(&str, &str)]) {
        Self::log_to_writer(severity, event, fields, &mut io::stderr());
    }

    /// Internal log implementation that writes to a given writer
    fn log_to_writer<W: Write>(
        severity: Severity,
        event: &str,
        fields: &[(&str, &str)],
        writer: &mut W,
    ) {
        // Build JSON manually to avoid allocations and ensure deterministic ordering
        let mut output = String::with_capacity(256);

        output.push('{');

        // Always output event first
        output.push_str("\"event\":\"");
        Self::escape_json_string(&mut output, event);
        output.push('"');

        // Then severity
        output.push_str(",\"severity\":\"");
        output.push_str(severity.as_str());
        output.push('"');

        // Sort fields alphabetically for deterministic output
        let mut sorted_fields: Vec<_> = fields.iter().collect();
        sorted_fields.sort_by_key(|(k, _)| *k);

        for (key, value) in sorted_fields {
            output.push_str(",\"");
            Self::escape_json_string(&mut output, key);
            output.push_str("\":\"");
            Self::escape_json_string(&mut output, value);
            output.push('"');
        }

        output.push('}');
        output.push('\n');

        // Write atomically (one syscall)
        let _ = writer.write_all(output.as_bytes());
        let _ = writer.flush();
    }

    /// Escape special characters for JSON strings
    fn escape_json_string(output: &mut String, s: &str) {
        for c in s.chars() {
            match c {
                '"' => output.push_str("\\\""),
                '\\' => output.push_str("\\\\"),
                '\n' => output.push_str("\\n"),
                '\r' => output.push_str("\\r"),
                '\t' => output.push_str("\\t"),
                c if c.is_control() => {
                    output.push_str(&format!("\\u{:04x}", c as u32));
                }
                c => output.push(c),
            }
        }
    }

    /// Log at TRACE level
    pub fn trace(event: &str, fields: &[(&str, &str)]) {
        Self::log(Severity::Trace, event, fields);
    }

    /// Log at INFO level
    pub fn info(event: &str, fields: &[(&str, &str)]) {
        Self::log(Severity::Info, event, fields);
    }

    /// Log at WARN level
    pub fn warn(event: &str, fields: &[(&str, &str)]) {
        Self::log(Severity::Warn, event, fields);
    }

    /// Log at ERROR level
    pub fn error(event: &str, fields: &[(&str, &str)]) {
        Self::log_stderr(Severity::Error, event, fields);
    }

    /// Log at FATAL level
    pub fn fatal(event: &str, fields: &[(&str, &str)]) {
        Self::log_stderr(Severity::Fatal, event, fields);
    }
}

/// Capture logs to a buffer for testing
#[cfg(test)]
pub fn capture_log(severity: Severity, event: &str, fields: &[(&str, &str)]) -> String {
    let mut buffer = Vec::new();
    Logger::log_to_writer(severity, event, fields, &mut buffer);
    String::from_utf8(buffer).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Trace < Severity::Info);
        assert!(Severity::Info < Severity::Warn);
        assert!(Severity::Warn < Severity::Error);
        assert!(Severity::Error < Severity::Fatal);
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(Severity::Trace.as_str(), "TRACE");
        assert_eq!(Severity::Info.as_str(), "INFO");
        assert_eq!(Severity::Warn.as_str(), "WARN");
        assert_eq!(Severity::Error.as_str(), "ERROR");
        assert_eq!(Severity::Fatal.as_str(), "FATAL");
    }

    #[test]
    fn test_log_json_format() {
        let output = capture_log(Severity::Info, "TEST_EVENT", &[]);

        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["event"], "TEST_EVENT");
        assert_eq!(parsed["severity"], "INFO");
    }

    #[test]
    fn test_log_with_fields() {
        let output = capture_log(
            Severity::Info,
            "TEST_EVENT",
            &[("key1", "value1"), ("key2", "value2")],
        );

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["key1"], "value1");
        assert_eq!(parsed["key2"], "value2");
    }

    #[test]
    fn test_log_deterministic_ordering() {
        // Fields should be sorted alphabetically
        let output1 = capture_log(
            Severity::Info,
            "TEST",
            &[("zebra", "1"), ("apple", "2"), ("mango", "3")],
        );
        let output2 = capture_log(
            Severity::Info,
            "TEST",
            &[("apple", "2"), ("mango", "3"), ("zebra", "1")],
        );

        // Both should produce identical output
        assert_eq!(output1, output2);

        // Verify order in output
        let apple_pos = output1.find("apple").unwrap();
        let mango_pos = output1.find("mango").unwrap();
        let zebra_pos = output1.find("zebra").unwrap();

        assert!(apple_pos < mango_pos);
        assert!(mango_pos < zebra_pos);
    }

    #[test]
    fn test_log_escapes_special_chars() {
        let output = capture_log(
            Severity::Info,
            "TEST",
            &[("message", "hello \"world\"\nline2")],
        );

        // Should be valid JSON with escaped characters
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["message"], "hello \"world\"\nline2");
    }

    #[test]
    fn test_log_one_line() {
        let output = capture_log(
            Severity::Info,
            "TEST",
            &[("a", "1"), ("b", "2"), ("c", "3")],
        );

        // Should be exactly one line
        assert_eq!(output.chars().filter(|c| *c == '\n').count(), 1);
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn test_log_event_first() {
        let output = capture_log(Severity::Info, "MY_EVENT", &[]);

        // Event should come first in the JSON
        let event_pos = output.find("\"event\"").unwrap();
        let severity_pos = output.find("\"severity\"").unwrap();

        assert!(event_pos < severity_pos);
    }
}
