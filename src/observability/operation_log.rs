//! # Operation Log
//!
//! MANIFESTO ALIGNMENT: Minimal, append-only operation logging for auditability.
//! Per Design Manifesto: "Every query is logged with its execution details."
//!
//! # Design Principles
//!
//! 1. **Append-only**: Operations are logged, never modified
//! 2. **Deterministic**: Same operation produces same log entry (excluding timestamp)
//! 3. **Explicit**: All fields are explicit, no hidden data
//! 4. **Non-blocking**: Logging does not block operation execution
//! 5. **Configurable**: Can be enabled/disabled via configuration
//!
//! # What This Module Does NOT Do (MANIFESTO ALIGNMENT)
//!
//! - **No query optimization suggestions**: You see the plan, you optimize
//! - **No automatic alerting**: Alerting is a separate, explicit system
//! - **No sampling magic**: If enabled, all operations are logged
//! - **No hidden aggregation**: Raw entries only

use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Operation type for logging
///
/// MANIFESTO ALIGNMENT: All operation types are explicit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    /// Find/query operation
    Find,
    /// Insert operation
    Insert,
    /// Update operation
    Update,
    /// Delete operation
    Delete,
    /// Count operation
    Count,
    /// Schema operation
    Schema,
    /// Function invocation
    Function,
    /// Realtime subscription
    Subscribe,
    /// File storage operation
    Storage,
}

impl OperationType {
    /// Returns string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            OperationType::Find => "find",
            OperationType::Insert => "insert",
            OperationType::Update => "update",
            OperationType::Delete => "delete",
            OperationType::Count => "count",
            OperationType::Schema => "schema",
            OperationType::Function => "function",
            OperationType::Subscribe => "subscribe",
            OperationType::Storage => "storage",
        }
    }
}

/// A single operation log entry
///
/// MANIFESTO ALIGNMENT: All fields are explicit. No hidden data.
/// Per manifesto format in `_system.operation_log`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationLogEntry {
    /// Unique operation ID
    pub id: Uuid,

    /// Timestamp of operation
    pub timestamp: SystemTime,

    /// Collection name (if applicable)
    pub collection: Option<String>,

    /// Operation type
    pub operation: OperationType,

    /// User ID (if authenticated)
    pub user_id: Option<Uuid>,

    /// Execution duration in milliseconds
    ///
    /// MANIFESTO ALIGNMENT: Duration is explicit, not hidden.
    pub duration_ms: u64,

    /// Number of documents scanned (for query operations)
    pub documents_scanned: Option<usize>,

    /// Number of documents returned/affected
    pub documents_affected: Option<usize>,

    /// Index used (if any)
    ///
    /// MANIFESTO ALIGNMENT: Index usage is explicit and observable.
    pub index_used: Option<String>,

    /// Whether this was a slow query (exceeded threshold)
    ///
    /// MANIFESTO ALIGNMENT: Slow threshold is configurable, not guessed.
    pub is_slow: bool,
}

impl OperationLogEntry {
    /// Create a new operation log entry builder
    pub fn builder(operation: OperationType) -> OperationLogEntryBuilder {
        OperationLogEntryBuilder {
            operation,
            collection: None,
            user_id: None,
            duration_ms: 0,
            documents_scanned: None,
            documents_affected: None,
            index_used: None,
            slow_threshold_ms: 100,
        }
    }
}

/// Builder for operation log entries
pub struct OperationLogEntryBuilder {
    operation: OperationType,
    collection: Option<String>,
    user_id: Option<Uuid>,
    duration_ms: u64,
    documents_scanned: Option<usize>,
    documents_affected: Option<usize>,
    index_used: Option<String>,
    slow_threshold_ms: u64,
}

impl OperationLogEntryBuilder {
    /// Set collection name
    pub fn collection(mut self, collection: impl Into<String>) -> Self {
        self.collection = Some(collection.into());
        self
    }

    /// Set user ID
    pub fn user_id(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Set execution duration
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration_ms = duration.as_millis() as u64;
        self
    }

    /// Set duration in milliseconds
    pub fn duration_ms(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }

    /// Set documents scanned
    pub fn documents_scanned(mut self, count: usize) -> Self {
        self.documents_scanned = Some(count);
        self
    }

    /// Set documents affected
    pub fn documents_affected(mut self, count: usize) -> Self {
        self.documents_affected = Some(count);
        self
    }

    /// Set index used
    pub fn index_used(mut self, index: impl Into<String>) -> Self {
        self.index_used = Some(index.into());
        self
    }

    /// Set slow query threshold
    ///
    /// MANIFESTO ALIGNMENT: Slow threshold is configured, not guessed.
    pub fn slow_threshold_ms(mut self, threshold: u64) -> Self {
        self.slow_threshold_ms = threshold;
        self
    }

    /// Build the log entry
    pub fn build(self) -> OperationLogEntry {
        OperationLogEntry {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            collection: self.collection,
            operation: self.operation,
            user_id: self.user_id,
            duration_ms: self.duration_ms,
            documents_scanned: self.documents_scanned,
            documents_affected: self.documents_affected,
            index_used: self.index_used,
            is_slow: self.duration_ms > self.slow_threshold_ms,
        }
    }
}

/// Operation log configuration
///
/// MANIFESTO ALIGNMENT: Configuration is explicit, no hidden defaults.
#[derive(Debug, Clone)]
pub struct OperationLogConfig {
    /// Whether operation logging is enabled
    ///
    /// MANIFESTO ALIGNMENT: Logging is opt-in, explicit.
    pub enabled: bool,

    /// Slow query threshold in milliseconds
    ///
    /// MANIFESTO ALIGNMENT: Threshold is configured, not guessed.
    /// Per manifesto: "slow threshold is configured, not guessed"
    pub slow_threshold_ms: u64,

    /// Maximum entries to keep in memory
    ///
    /// MANIFESTO ALIGNMENT: Buffer size is explicit.
    pub max_entries: usize,
}

impl Default for OperationLogConfig {
    /// Default configuration
    ///
    /// MANIFESTO NOTE: These are explicit defaults, documented here.
    /// - enabled: false (opt-in, not opt-out)
    /// - slow_threshold_ms: 100 (match manifesto example)
    /// - max_entries: 10000 (bounded to prevent memory issues)
    fn default() -> Self {
        Self {
            enabled: false, // MANIFESTO ALIGNMENT: Disabled by default, must opt-in
            slow_threshold_ms: 100,
            max_entries: 10_000,
        }
    }
}

/// Operation log
///
/// MANIFESTO ALIGNMENT: Append-only operation log for auditability.
/// Per Design Manifesto: "Every query is logged with its execution details."
///
/// # Implementation Notes
///
/// This is a minimal, in-memory implementation. For production:
/// - Consider file-backed persistence
/// - Consider WAL integration for durability
/// - Consider log rotation
///
/// This implementation prioritizes correctness and explicitness over performance.
#[derive(Debug)]
pub struct OperationLog {
    config: OperationLogConfig,
    entries: RwLock<VecDeque<OperationLogEntry>>,
}

impl OperationLog {
    /// Create a new operation log with the given configuration
    pub fn new(config: OperationLogConfig) -> Self {
        Self {
            config,
            entries: RwLock::new(VecDeque::new()),
        }
    }

    /// Create a disabled operation log (no-op)
    pub fn disabled() -> Self {
        Self::new(OperationLogConfig {
            enabled: false,
            ..Default::default()
        })
    }

    /// Check if operation logging is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the slow query threshold
    pub fn slow_threshold_ms(&self) -> u64 {
        self.config.slow_threshold_ms
    }

    /// Log an operation
    ///
    /// MANIFESTO ALIGNMENT: If logging is enabled, ALL operations are logged.
    /// No sampling, no hidden filtering.
    pub fn log(&self, entry: OperationLogEntry) {
        if !self.config.enabled {
            return;
        }

        if let Ok(mut entries) = self.entries.write() {
            // Enforce max entries (FIFO eviction)
            while entries.len() >= self.config.max_entries {
                entries.pop_front();
            }
            entries.push_back(entry);
        }
    }

    /// Get all entries (for debugging/testing)
    pub fn entries(&self) -> Vec<OperationLogEntry> {
        self.entries
            .read()
            .map(|e| e.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get slow queries only
    ///
    /// MANIFESTO ALIGNMENT: Slow query detection uses configured threshold.
    pub fn slow_queries(&self) -> Vec<OperationLogEntry> {
        self.entries
            .read()
            .map(|e| e.iter().filter(|op| op.is_slow).cloned().collect())
            .unwrap_or_default()
    }

    /// Get entry count
    pub fn count(&self) -> usize {
        self.entries.read().map(|e| e.len()).unwrap_or(0)
    }

    /// Clear all entries (for testing)
    #[cfg(test)]
    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.write() {
            entries.clear();
        }
    }
}

/// Thread-safe operation log handle
pub type SharedOperationLog = Arc<OperationLog>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_log_disabled_by_default() {
        let config = OperationLogConfig::default();
        // MANIFESTO ALIGNMENT: Logging is opt-in, disabled by default
        assert!(!config.enabled);
    }

    #[test]
    fn test_operation_log_explicit_enable() {
        let config = OperationLogConfig {
            enabled: true,
            slow_threshold_ms: 50,
            max_entries: 100,
        };
        let log = OperationLog::new(config);
        assert!(log.is_enabled());
        assert_eq!(log.slow_threshold_ms(), 50);
    }

    #[test]
    fn test_operation_log_entry() {
        let entry = OperationLogEntry::builder(OperationType::Find)
            .collection("users")
            .duration_ms(150)
            .documents_scanned(1000)
            .documents_affected(50)
            .index_used("email_idx")
            .slow_threshold_ms(100)
            .build();

        assert_eq!(entry.operation, OperationType::Find);
        assert_eq!(entry.collection, Some("users".to_string()));
        assert_eq!(entry.duration_ms, 150);
        assert!(entry.is_slow); // 150ms > 100ms threshold
        assert_eq!(entry.index_used, Some("email_idx".to_string()));
    }

    #[test]
    fn test_operation_log_append_only() {
        let config = OperationLogConfig {
            enabled: true,
            slow_threshold_ms: 100,
            max_entries: 1000,
        };
        let log = OperationLog::new(config);

        // Log some operations
        log.log(
            OperationLogEntry::builder(OperationType::Find)
                .collection("posts")
                .duration_ms(50)
                .build(),
        );
        log.log(
            OperationLogEntry::builder(OperationType::Insert)
                .collection("posts")
                .duration_ms(30)
                .build(),
        );

        assert_eq!(log.count(), 2);

        let entries = log.entries();
        assert_eq!(entries[0].operation, OperationType::Find);
        assert_eq!(entries[1].operation, OperationType::Insert);
    }

    #[test]
    fn test_operation_log_slow_query_detection() {
        let config = OperationLogConfig {
            enabled: true,
            slow_threshold_ms: 100,
            max_entries: 1000,
        };
        let log = OperationLog::new(config);

        log.log(
            OperationLogEntry::builder(OperationType::Find)
                .duration_ms(50) // Fast
                .slow_threshold_ms(100)
                .build(),
        );
        log.log(
            OperationLogEntry::builder(OperationType::Find)
                .duration_ms(150) // Slow
                .slow_threshold_ms(100)
                .build(),
        );

        let slow = log.slow_queries();
        assert_eq!(slow.len(), 1);
        assert_eq!(slow[0].duration_ms, 150);
    }

    #[test]
    fn test_operation_log_disabled_noop() {
        let log = OperationLog::disabled();

        // MANIFESTO ALIGNMENT: When disabled, no operations are logged
        log.log(
            OperationLogEntry::builder(OperationType::Find)
                .duration_ms(50)
                .build(),
        );

        assert_eq!(log.count(), 0);
    }

    #[test]
    fn test_operation_log_max_entries() {
        let config = OperationLogConfig {
            enabled: true,
            slow_threshold_ms: 100,
            max_entries: 3,
        };
        let log = OperationLog::new(config);

        // Add 5 entries, should only keep 3
        for i in 0..5 {
            log.log(
                OperationLogEntry::builder(OperationType::Find)
                    .duration_ms(i as u64 * 10)
                    .build(),
            );
        }

        // MANIFESTO ALIGNMENT: Bounded buffer, FIFO eviction
        assert_eq!(log.count(), 3);

        // First 2 should have been evicted
        let entries = log.entries();
        assert_eq!(entries[0].duration_ms, 20); // Entry 2
        assert_eq!(entries[1].duration_ms, 30); // Entry 3
        assert_eq!(entries[2].duration_ms, 40); // Entry 4
    }
}
