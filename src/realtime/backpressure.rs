//! # Backpressure Configuration and Channel
//!
//! MANIFESTO ALIGNMENT: Configurable, deterministic backpressure for realtime events.
//! Per certification requirement: Configurable max_pending_messages and drop_policy.
//!
//! # Design Principles
//!
//! 1. **Configurable**: All backpressure parameters are explicit configuration
//! 2. **Deterministic**: Same buffer state + policy produces same drop decision
//! 3. **Observable**: All drops are logged with explicit counters
//! 4. **Non-blocking**: Dropping events never blocks the caller
//!
//! # Drop Policies
//!
//! - `OldestFirst`: When full, drop the oldest message in the buffer
//! - `NewestFirst`: When full, reject the new message being sent
//! - `Reject`: When full, reject the new message and return error to sender
//!
//! # What This Module Does NOT Do (MANIFESTO ALIGNMENT)
//!
//! - **No auto-scaling**: Buffer size is fixed at configuration
//! - **No retry logic**: Dropped messages are gone forever
//! - **No priority handling**: All messages are treated equally

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

/// Backpressure configuration
///
/// MANIFESTO ALIGNMENT: Configuration is explicit, no hidden defaults.
/// Per certification: Must be configurable via aerodb.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackpressureConfig {
    /// Maximum number of pending messages before backpressure kicks in
    ///
    /// MANIFESTO ALIGNMENT: Buffer size is explicit, not auto-scaled.
    #[serde(default = "default_max_pending_messages")]
    pub max_pending_messages: usize,

    /// Policy for handling messages when buffer is full
    ///
    /// MANIFESTO ALIGNMENT: Drop behavior is explicit and configurable.
    #[serde(default)]
    pub drop_policy: DropPolicy,
}

fn default_max_pending_messages() -> usize {
    1000
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            max_pending_messages: default_max_pending_messages(),
            drop_policy: DropPolicy::default(),
        }
    }
}

impl BackpressureConfig {
    /// Create configuration with custom max pending messages
    pub fn with_max_pending(max: usize) -> Self {
        Self {
            max_pending_messages: max,
            drop_policy: DropPolicy::default(),
        }
    }

    /// Create configuration with specific drop policy
    pub fn with_policy(policy: DropPolicy) -> Self {
        Self {
            max_pending_messages: default_max_pending_messages(),
            drop_policy: policy,
        }
    }
}

/// Drop policy for backpressure handling
///
/// MANIFESTO ALIGNMENT: Explicit, deterministic drop behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DropPolicy {
    /// When buffer is full, drop the oldest message
    ///
    /// New messages are always accepted, oldest are evicted.
    /// Use when recent data is more valuable than historical data.
    OldestFirst,

    /// When buffer is full, reject (drop) the new incoming message
    ///
    /// Existing messages are preserved, new ones are dropped silently.
    /// Use when preserving order from the beginning is important.
    NewestFirst,

    /// When buffer is full, reject the new message with an error
    ///
    /// Existing messages are preserved, sender gets explicit error.
    /// Use when the sender needs to know about backpressure.
    Reject,
}

impl Default for DropPolicy {
    fn default() -> Self {
        // MANIFESTO ALIGNMENT: Default to OldestFirst per certification guidance
        // This matches common realtime system behavior where recent data is preferred
        DropPolicy::OldestFirst
    }
}

/// Backpressure counters for observability
///
/// MANIFESTO ALIGNMENT: All backpressure events are observable.
#[derive(Debug, Default)]
pub struct BackpressureCounters {
    /// Number of messages successfully delivered
    pub delivered_count: AtomicU64,
    /// Number of messages dropped due to OldestFirst or NewestFirst policy
    pub dropped_count: AtomicU64,
    /// Number of messages rejected with Reject policy
    pub rejected_count: AtomicU64,
}

impl BackpressureCounters {
    /// Get snapshot of current counters
    pub fn snapshot(&self) -> BackpressureSnapshot {
        BackpressureSnapshot {
            delivered: self.delivered_count.load(Ordering::Relaxed),
            dropped: self.dropped_count.load(Ordering::Relaxed),
            rejected: self.rejected_count.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of backpressure counters at a point in time
#[derive(Debug, Clone, Copy, Default)]
pub struct BackpressureSnapshot {
    pub delivered: u64,
    pub dropped: u64,
    pub rejected: u64,
}

/// Error returned when message is rejected by Reject policy
#[derive(Debug, Clone)]
pub struct BackpressureRejected {
    pub message: String,
}

impl std::fmt::Display for BackpressureRejected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Backpressure: {}", self.message)
    }
}

impl std::error::Error for BackpressureRejected {}

/// Result type for backpressure operations
pub type BackpressureResult<T> = Result<T, BackpressureRejected>;

/// Action taken when sending a message
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendAction {
    /// Message was delivered successfully
    Delivered,
    /// A message was dropped due to backpressure
    Dropped,
}

/// Backpressure-aware bounded channel
///
/// MANIFESTO ALIGNMENT: Deterministic backpressure handling.
/// Per certification: Configurable max_pending_messages and drop_policy.
pub struct BackpressureChannel<T> {
    config: BackpressureConfig,
    buffer: Arc<RwLock<VecDeque<T>>>,
    counters: Arc<BackpressureCounters>,
}

impl<T> BackpressureChannel<T> {
    /// Create a new backpressure channel with the given configuration
    pub fn new(config: BackpressureConfig) -> Self {
        Self {
            buffer: Arc::new(RwLock::new(VecDeque::with_capacity(
                config.max_pending_messages,
            ))),
            config,
            counters: Arc::new(BackpressureCounters::default()),
        }
    }

    /// Get reference to the counters
    pub fn counters(&self) -> &BackpressureCounters {
        &self.counters
    }

    /// Get current buffer size
    pub fn len(&self) -> usize {
        self.buffer.read().map(|b| b.len()).unwrap_or(0)
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check if buffer is at capacity
    pub fn is_full(&self) -> bool {
        self.len() >= self.config.max_pending_messages
    }

    /// Send a message to the channel with backpressure handling
    ///
    /// MANIFESTO ALIGNMENT: Deterministic drop behavior based on policy.
    ///
    /// Returns:
    /// - `Ok(SendAction::Delivered)` if message was successfully queued
    /// - `Ok(SendAction::Dropped)` if a message was dropped (OldestFirst/NewestFirst)
    /// - `Err(BackpressureRejected)` if Reject policy returned error
    pub fn send(&self, message: T) -> BackpressureResult<SendAction> {
        let mut buffer = self
            .buffer
            .write()
            .map_err(|_| BackpressureRejected {
                message: "Lock poisoned".to_string(),
            })?;

        if buffer.len() >= self.config.max_pending_messages {
            match self.config.drop_policy {
                DropPolicy::OldestFirst => {
                    // Drop oldest, add new
                    buffer.pop_front();
                    self.counters.dropped_count.fetch_add(1, Ordering::Relaxed);
                    buffer.push_back(message);
                    self.counters
                        .delivered_count
                        .fetch_add(1, Ordering::Relaxed);

                    // Log drop event
                    self.log_drop_event("oldest_first");

                    Ok(SendAction::Dropped)
                }
                DropPolicy::NewestFirst => {
                    // Reject new message silently
                    self.counters.dropped_count.fetch_add(1, Ordering::Relaxed);

                    // Log drop event
                    self.log_drop_event("newest_first");

                    Ok(SendAction::Dropped)
                }
                DropPolicy::Reject => {
                    // Return error to sender
                    self.counters.rejected_count.fetch_add(1, Ordering::Relaxed);

                    // Log rejection event
                    self.log_reject_event();

                    Err(BackpressureRejected {
                        message: format!(
                            "Buffer full ({}/{}), message rejected",
                            buffer.len(),
                            self.config.max_pending_messages
                        ),
                    })
                }
            }
        } else {
            buffer.push_back(message);
            self.counters
                .delivered_count
                .fetch_add(1, Ordering::Relaxed);
            Ok(SendAction::Delivered)
        }
    }

    /// Receive a message from the channel
    ///
    /// Returns None if buffer is empty.
    pub fn recv(&self) -> Option<T> {
        self.buffer.write().ok()?.pop_front()
    }

    /// Log drop event
    fn log_drop_event(&self, policy: &str) {
        let snapshot = self.counters.snapshot();
        eprintln!(
            "{{\"level\":\"WARN\",\"event\":\"BACKPRESSURE_DROP\",\"policy\":\"{}\",\"dropped\":{},\"buffer_size\":{}}}",
            policy, snapshot.dropped, self.config.max_pending_messages
        );
    }

    /// Log reject event
    fn log_reject_event(&self) {
        let snapshot = self.counters.snapshot();
        eprintln!(
            "{{\"level\":\"WARN\",\"event\":\"BACKPRESSURE_REJECT\",\"rejected\":{},\"buffer_size\":{}}}",
            snapshot.rejected, self.config.max_pending_messages
        );
    }
}

impl<T: Clone> Clone for BackpressureChannel<T> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            buffer: Arc::clone(&self.buffer),
            counters: Arc::clone(&self.counters),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = BackpressureConfig::default();
        assert_eq!(config.max_pending_messages, 1000);
        assert_eq!(config.drop_policy, DropPolicy::OldestFirst);
    }

    #[test]
    fn test_config_with_max_pending() {
        let config = BackpressureConfig::with_max_pending(500);
        assert_eq!(config.max_pending_messages, 500);
    }

    #[test]
    fn test_config_with_policy() {
        let config = BackpressureConfig::with_policy(DropPolicy::Reject);
        assert_eq!(config.drop_policy, DropPolicy::Reject);
    }

    #[test]
    fn test_drop_policy_serde() {
        let policy = DropPolicy::OldestFirst;
        let json = serde_json::to_string(&policy).unwrap();
        assert_eq!(json, "\"oldest_first\"");

        let policy: DropPolicy = serde_json::from_str("\"newest_first\"").unwrap();
        assert_eq!(policy, DropPolicy::NewestFirst);

        let policy: DropPolicy = serde_json::from_str("\"reject\"").unwrap();
        assert_eq!(policy, DropPolicy::Reject);
    }

    #[test]
    fn test_channel_send_under_capacity() {
        let config = BackpressureConfig::with_max_pending(10);
        let channel: BackpressureChannel<i32> = BackpressureChannel::new(config);

        for i in 0..5 {
            let result = channel.send(i).unwrap();
            assert_eq!(result, SendAction::Delivered);
        }

        assert_eq!(channel.len(), 5);
        let snapshot = channel.counters().snapshot();
        assert_eq!(snapshot.delivered, 5);
        assert_eq!(snapshot.dropped, 0);
    }

    #[test]
    fn test_channel_oldest_first_policy() {
        let config = BackpressureConfig {
            max_pending_messages: 3,
            drop_policy: DropPolicy::OldestFirst,
        };
        let channel: BackpressureChannel<i32> = BackpressureChannel::new(config);

        // Fill the buffer
        channel.send(1).unwrap();
        channel.send(2).unwrap();
        channel.send(3).unwrap();

        // Send one more - should drop oldest (1)
        let result = channel.send(4).unwrap();
        assert_eq!(result, SendAction::Dropped);

        // Buffer should contain 2, 3, 4
        assert_eq!(channel.recv(), Some(2));
        assert_eq!(channel.recv(), Some(3));
        assert_eq!(channel.recv(), Some(4));

        let snapshot = channel.counters().snapshot();
        assert_eq!(snapshot.delivered, 4); // All 4 messages were delivered
        assert_eq!(snapshot.dropped, 1); // 1 was dropped
    }

    #[test]
    fn test_channel_newest_first_policy() {
        let config = BackpressureConfig {
            max_pending_messages: 3,
            drop_policy: DropPolicy::NewestFirst,
        };
        let channel: BackpressureChannel<i32> = BackpressureChannel::new(config);

        // Fill the buffer
        channel.send(1).unwrap();
        channel.send(2).unwrap();
        channel.send(3).unwrap();

        // Send one more - should drop new message (4)
        let result = channel.send(4).unwrap();
        assert_eq!(result, SendAction::Dropped);

        // Buffer should contain 1, 2, 3 (4 was dropped)
        assert_eq!(channel.recv(), Some(1));
        assert_eq!(channel.recv(), Some(2));
        assert_eq!(channel.recv(), Some(3));

        let snapshot = channel.counters().snapshot();
        assert_eq!(snapshot.delivered, 3); // Only 3 messages were kept
        assert_eq!(snapshot.dropped, 1); // 4 was dropped
    }

    #[test]
    fn test_channel_reject_policy() {
        let config = BackpressureConfig {
            max_pending_messages: 3,
            drop_policy: DropPolicy::Reject,
        };
        let channel: BackpressureChannel<i32> = BackpressureChannel::new(config);

        // Fill the buffer
        channel.send(1).unwrap();
        channel.send(2).unwrap();
        channel.send(3).unwrap();

        // Send one more - should return error
        let result = channel.send(4);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("rejected"));

        // Buffer should contain original 1, 2, 3
        let snapshot = channel.counters().snapshot();
        assert_eq!(snapshot.delivered, 3);
        assert_eq!(snapshot.rejected, 1);
    }

    #[test]
    fn test_channel_recv() {
        let config = BackpressureConfig::default();
        let channel: BackpressureChannel<i32> = BackpressureChannel::new(config);

        channel.send(1).unwrap();
        channel.send(2).unwrap();

        assert_eq!(channel.recv(), Some(1));
        assert_eq!(channel.recv(), Some(2));
        assert_eq!(channel.recv(), None);
    }

    #[test]
    fn test_channel_is_empty_and_full() {
        let config = BackpressureConfig::with_max_pending(2);
        let channel: BackpressureChannel<i32> = BackpressureChannel::new(config);

        assert!(channel.is_empty());
        assert!(!channel.is_full());

        channel.send(1).unwrap();
        assert!(!channel.is_empty());
        assert!(!channel.is_full());

        channel.send(2).unwrap();
        assert!(!channel.is_empty());
        assert!(channel.is_full());
    }

    #[test]
    fn test_counters_snapshot() {
        let counters = BackpressureCounters::default();
        counters.delivered_count.store(10, Ordering::Relaxed);
        counters.dropped_count.store(5, Ordering::Relaxed);
        counters.rejected_count.store(2, Ordering::Relaxed);

        let snapshot = counters.snapshot();
        assert_eq!(snapshot.delivered, 10);
        assert_eq!(snapshot.dropped, 5);
        assert_eq!(snapshot.rejected, 2);
    }
}
