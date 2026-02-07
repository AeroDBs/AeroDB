//! # Broadcast Channels
//!
//! Pub/sub channels for user-generated messages.

use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use super::errors::{RealtimeError, RealtimeResult};
use super::event::BroadcastEvent;

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum messages per second
    pub max_per_second: usize,
    /// Maximum message size in bytes
    pub max_message_size: usize,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_per_second: 10,
            max_message_size: 65536, // 64KB
        }
    }
}

/// A broadcast channel
#[derive(Debug)]
pub struct BroadcastChannel {
    /// Channel name
    pub name: String,

    /// Whether the channel is public
    pub is_public: bool,

    /// Subscribers (connection IDs)
    subscribers: RwLock<HashSet<String>>,

    /// Rate limit tracking (connection_id -> (count, window_start))
    rate_limits: RwLock<HashMap<String, (usize, DateTime<Utc>)>>,

    /// Rate limit config
    rate_config: RateLimitConfig,
}

impl BroadcastChannel {
    /// Create a new channel
    pub fn new(name: String, is_public: bool) -> Self {
        Self {
            name,
            is_public,
            subscribers: RwLock::new(HashSet::new()),
            rate_limits: RwLock::new(HashMap::new()),
            rate_config: RateLimitConfig::default(),
        }
    }

    /// Subscribe to the channel
    pub fn subscribe(&self, connection_id: &str) -> RealtimeResult<()> {
        if let Ok(mut subs) = self.subscribers.write() {
            subs.insert(connection_id.to_string());
            Ok(())
        } else {
            Err(RealtimeError::Internal("Lock poisoned".into()))
        }
    }

    /// Unsubscribe from the channel
    pub fn unsubscribe(&self, connection_id: &str) {
        if let Ok(mut subs) = self.subscribers.write() {
            subs.remove(connection_id);
        }
    }

    /// Check rate limit for a connection
    fn check_rate_limit(&self, connection_id: &str) -> RealtimeResult<()> {
        let now = Utc::now();

        let mut limits = self
            .rate_limits
            .write()
            .map_err(|_| RealtimeError::Internal("Lock poisoned".into()))?;

        let entry = limits.entry(connection_id.to_string()).or_insert((0, now));

        // Reset if in new window
        if (now - entry.1).num_seconds() >= 1 {
            entry.0 = 0;
            entry.1 = now;
        }

        // Check limit
        if entry.0 >= self.rate_config.max_per_second {
            return Err(RealtimeError::RateLimitExceeded);
        }

        entry.0 += 1;
        Ok(())
    }

    /// Broadcast a message to all subscribers
    pub fn broadcast(
        &self,
        event: String,
        payload: Value,
        sender_id: Option<Uuid>,
        sender_connection: &str,
    ) -> RealtimeResult<BroadcastEvent> {
        // Check rate limit
        self.check_rate_limit(sender_connection)?;

        // Check message size
        let payload_size = serde_json::to_string(&payload)
            .map(|s| s.len())
            .unwrap_or(0);

        if payload_size > self.rate_config.max_message_size {
            return Err(RealtimeError::MessageTooLarge(
                self.rate_config.max_message_size,
            ));
        }

        Ok(BroadcastEvent::new(
            self.name.clone(),
            event,
            payload,
            sender_id,
        ))
    }

    /// Get all subscribers
    pub fn subscribers(&self) -> Vec<String> {
        self.subscribers
            .read()
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get subscriber count
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.read().map(|s| s.len()).unwrap_or(0)
    }
}

/// Registry of broadcast channels
#[derive(Debug, Default)]
pub struct BroadcastRegistry {
    /// Channels by name
    channels: RwLock<HashMap<String, BroadcastChannel>>,
}

impl BroadcastRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create a channel
    pub fn get_or_create(&self, name: &str, is_public: bool) -> RealtimeResult<()> {
        let mut channels = self
            .channels
            .write()
            .map_err(|_| RealtimeError::Internal("Lock poisoned".into()))?;

        if !channels.contains_key(name) {
            channels.insert(
                name.to_string(),
                BroadcastChannel::new(name.to_string(), is_public),
            );
        }

        Ok(())
    }

    /// Subscribe to a channel
    pub fn subscribe(&self, channel_name: &str, connection_id: &str) -> RealtimeResult<()> {
        let channels = self
            .channels
            .read()
            .map_err(|_| RealtimeError::Internal("Lock poisoned".into()))?;

        if let Some(channel) = channels.get(channel_name) {
            channel.subscribe(connection_id)
        } else {
            Err(RealtimeError::ChannelNotFound(channel_name.to_string()))
        }
    }

    /// Unsubscribe from a channel
    pub fn unsubscribe(&self, channel_name: &str, connection_id: &str) {
        if let Ok(channels) = self.channels.read() {
            if let Some(channel) = channels.get(channel_name) {
                channel.unsubscribe(connection_id);
            }
        }
    }

    /// Broadcast to a channel
    pub fn broadcast(
        &self,
        channel_name: &str,
        event: String,
        payload: Value,
        sender_id: Option<Uuid>,
        sender_connection: &str,
    ) -> RealtimeResult<(BroadcastEvent, Vec<String>)> {
        let channels = self
            .channels
            .read()
            .map_err(|_| RealtimeError::Internal("Lock poisoned".into()))?;

        if let Some(channel) = channels.get(channel_name) {
            let event = channel.broadcast(event, payload, sender_id, sender_connection)?;
            let subscribers = channel.subscribers();
            Ok((event, subscribers))
        } else {
            Err(RealtimeError::ChannelNotFound(channel_name.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_channel_subscribe() {
        let channel = BroadcastChannel::new("test".to_string(), true);

        channel.subscribe("conn-1").unwrap();
        channel.subscribe("conn-2").unwrap();

        assert_eq!(channel.subscriber_count(), 2);

        channel.unsubscribe("conn-1");
        assert_eq!(channel.subscriber_count(), 1);
    }

    #[test]
    fn test_broadcast() {
        let channel = BroadcastChannel::new("test".to_string(), true);
        channel.subscribe("conn-1").unwrap();

        let event = channel
            .broadcast(
                "message".to_string(),
                json!({"text": "Hello"}),
                None,
                "conn-1",
            )
            .unwrap();

        assert_eq!(event.channel, "test");
        assert_eq!(event.event, "message");
    }

    #[test]
    fn test_rate_limit() {
        let channel = BroadcastChannel::new("test".to_string(), true);

        // Should hit rate limit after 10 messages
        for i in 0..10 {
            let result = channel.broadcast("msg".to_string(), json!({"i": i}), None, "conn-1");
            assert!(result.is_ok());
        }

        // 11th should fail
        let result = channel.broadcast("msg".to_string(), json!({}), None, "conn-1");
        assert!(matches!(result, Err(RealtimeError::RateLimitExceeded)));
    }

    #[test]
    fn test_message_size_limit() {
        let channel = BroadcastChannel::new("test".to_string(), true);

        // Create a large payload
        let large_payload = json!({
            "data": "x".repeat(100_000)
        });

        let result = channel.broadcast("msg".to_string(), large_payload, None, "conn-1");

        assert!(matches!(result, Err(RealtimeError::MessageTooLarge(_))));
    }

    #[test]
    fn test_registry() {
        let registry = BroadcastRegistry::new();

        registry.get_or_create("chat-room", true).unwrap();
        registry.subscribe("chat-room", "conn-1").unwrap();

        let (event, subscribers) = registry
            .broadcast(
                "chat-room",
                "message".to_string(),
                json!({"text": "Hi"}),
                None,
                "conn-1",
            )
            .unwrap();

        assert_eq!(event.channel, "chat-room");
        assert_eq!(subscribers, vec!["conn-1"]);
    }
}
