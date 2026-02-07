//! # Presence Tracking
//!
//! User presence for real-time channels.
//!
//! ## Invariant: RT-P1
//! Presence is eventually consistent, not immediately consistent.

use std::collections::HashMap;
use std::sync::RwLock;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::errors::{RealtimeError, RealtimeResult};
use super::event::{PresenceEvent, PresenceEventType};

/// Presence state for a user in a channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceState {
    /// User ID
    pub user_id: Uuid,

    /// Connection ID
    pub connection_id: String,

    /// User metadata (custom state)
    pub metadata: Value,

    /// When user joined
    pub joined_at: DateTime<Utc>,

    /// Last heartbeat
    pub last_seen: DateTime<Utc>,
}

impl PresenceState {
    /// Create a new presence state
    pub fn new(user_id: Uuid, connection_id: String, metadata: Value) -> Self {
        let now = Utc::now();
        Self {
            user_id,
            connection_id,
            metadata,
            joined_at: now,
            last_seen: now,
        }
    }

    /// Update the heartbeat
    pub fn heartbeat(&mut self) {
        self.last_seen = Utc::now();
    }

    /// Check if presence is stale (no heartbeat for timeout period)
    pub fn is_stale(&self, timeout: Duration) -> bool {
        Utc::now() - self.last_seen > timeout
    }
}

/// Configuration for presence tracking
#[derive(Debug, Clone)]
pub struct PresenceConfig {
    /// Heartbeat interval (expected from clients)
    pub heartbeat_interval: Duration,

    /// Timeout for considering a user offline
    pub timeout: Duration,
}

impl Default for PresenceConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::seconds(30),
            timeout: Duration::seconds(60),
        }
    }
}

/// Presence tracker for a channel
#[derive(Debug)]
pub struct PresenceTracker {
    /// Channel name
    pub channel: String,

    /// Configuration
    config: PresenceConfig,

    /// Active presence states by connection_id
    states: RwLock<HashMap<String, PresenceState>>,
}

impl PresenceTracker {
    /// Create a new presence tracker
    pub fn new(channel: String) -> Self {
        Self {
            channel,
            config: PresenceConfig::default(),
            states: RwLock::new(HashMap::new()),
        }
    }

    /// Create with custom config
    pub fn with_config(channel: String, config: PresenceConfig) -> Self {
        Self {
            channel,
            config,
            states: RwLock::new(HashMap::new()),
        }
    }

    /// Track a user (join)
    pub fn track(
        &self,
        user_id: Uuid,
        connection_id: String,
        metadata: Value,
    ) -> RealtimeResult<PresenceEvent> {
        let state = PresenceState::new(user_id, connection_id.clone(), metadata.clone());

        let mut states = self
            .states
            .write()
            .map_err(|_| RealtimeError::Internal("Lock poisoned".into()))?;

        states.insert(connection_id, state);

        Ok(PresenceEvent {
            channel: self.channel.clone(),
            event: PresenceEventType::Join,
            state: serde_json::json!({
                "user_id": user_id.to_string(),
                "metadata": metadata,
            }),
            timestamp: Utc::now(),
        })
    }

    /// Untrack a user (leave)
    pub fn untrack(&self, connection_id: &str) -> RealtimeResult<Option<PresenceEvent>> {
        let mut states = self
            .states
            .write()
            .map_err(|_| RealtimeError::Internal("Lock poisoned".into()))?;

        if let Some(state) = states.remove(connection_id) {
            Ok(Some(PresenceEvent {
                channel: self.channel.clone(),
                event: PresenceEventType::Leave,
                state: serde_json::json!({
                    "user_id": state.user_id.to_string(),
                }),
                timestamp: Utc::now(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Update heartbeat for a connection
    pub fn heartbeat(&self, connection_id: &str) -> RealtimeResult<()> {
        let mut states = self
            .states
            .write()
            .map_err(|_| RealtimeError::Internal("Lock poisoned".into()))?;

        if let Some(state) = states.get_mut(connection_id) {
            state.heartbeat();
            Ok(())
        } else {
            Err(RealtimeError::NotTracking)
        }
    }

    /// Get current presence state (sync)
    pub fn sync(&self) -> RealtimeResult<PresenceEvent> {
        let states = self
            .states
            .read()
            .map_err(|_| RealtimeError::Internal("Lock poisoned".into()))?;

        let state_map: HashMap<String, Value> = states
            .values()
            .map(|s| {
                (
                    s.user_id.to_string(),
                    serde_json::json!({
                        "metadata": s.metadata,
                        "joined_at": s.joined_at.to_rfc3339(),
                        "last_seen": s.last_seen.to_rfc3339(),
                    }),
                )
            })
            .collect();

        Ok(PresenceEvent {
            channel: self.channel.clone(),
            event: PresenceEventType::Sync,
            state: serde_json::to_value(state_map).unwrap_or(Value::Object(Default::default())),
            timestamp: Utc::now(),
        })
    }

    /// Clean up stale connections
    pub fn cleanup(&self) -> Vec<PresenceEvent> {
        let stale_ids: Vec<String> = {
            if let Ok(states) = self.states.read() {
                states
                    .iter()
                    .filter(|(_, s)| s.is_stale(self.config.timeout))
                    .map(|(id, _)| id.clone())
                    .collect()
            } else {
                return Vec::new();
            }
        };

        let mut events = Vec::new();
        for id in stale_ids {
            if let Ok(Some(event)) = self.untrack(&id) {
                events.push(event);
            }
        }

        events
    }

    /// Get count of tracked users
    pub fn count(&self) -> usize {
        self.states.read().map(|s| s.len()).unwrap_or(0)
    }

    /// Check if a connection is tracked
    pub fn is_tracked(&self, connection_id: &str) -> bool {
        self.states
            .read()
            .map(|s| s.contains_key(connection_id))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_track_untrack() {
        let tracker = PresenceTracker::new("lobby".to_string());
        let user_id = Uuid::new_v4();

        // Track
        let event = tracker
            .track(user_id, "conn-1".to_string(), json!({"status": "online"}))
            .unwrap();

        assert_eq!(event.event, PresenceEventType::Join);
        assert_eq!(tracker.count(), 1);

        // Untrack
        let event = tracker.untrack("conn-1").unwrap().unwrap();
        assert_eq!(event.event, PresenceEventType::Leave);
        assert_eq!(tracker.count(), 0);
    }

    #[test]
    fn test_heartbeat() {
        let tracker = PresenceTracker::new("lobby".to_string());
        let user_id = Uuid::new_v4();

        tracker
            .track(user_id, "conn-1".to_string(), json!({}))
            .unwrap();

        // Heartbeat should work
        assert!(tracker.heartbeat("conn-1").is_ok());

        // Heartbeat on unknown connection should fail
        assert!(matches!(
            tracker.heartbeat("conn-unknown"),
            Err(RealtimeError::NotTracking)
        ));
    }

    #[test]
    fn test_sync() {
        let tracker = PresenceTracker::new("lobby".to_string());

        tracker
            .track(
                Uuid::new_v4(),
                "conn-1".to_string(),
                json!({"status": "online"}),
            )
            .unwrap();
        tracker
            .track(
                Uuid::new_v4(),
                "conn-2".to_string(),
                json!({"status": "away"}),
            )
            .unwrap();

        let event = tracker.sync().unwrap();
        assert_eq!(event.event, PresenceEventType::Sync);

        let state_obj = event.state.as_object().unwrap();
        assert_eq!(state_obj.len(), 2);
    }

    #[test]
    fn test_stale_detection() {
        let state = PresenceState::new(Uuid::new_v4(), "conn-1".to_string(), json!({}));

        // Not stale initially
        assert!(!state.is_stale(Duration::seconds(60)));
    }

    #[test]
    fn test_is_tracked() {
        let tracker = PresenceTracker::new("lobby".to_string());

        assert!(!tracker.is_tracked("conn-1"));

        tracker
            .track(Uuid::new_v4(), "conn-1".to_string(), json!({}))
            .unwrap();

        assert!(tracker.is_tracked("conn-1"));
        assert!(!tracker.is_tracked("conn-2"));
    }
}
