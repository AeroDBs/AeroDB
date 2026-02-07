//! # Real-Time Events
//!
//! Event types for database changes and broadcasts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Type of database event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum EventType {
    /// New record inserted
    Insert,
    /// Existing record updated
    Update,
    /// Record deleted
    Delete,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventType::Insert => write!(f, "INSERT"),
            EventType::Update => write!(f, "UPDATE"),
            EventType::Delete => write!(f, "DELETE"),
        }
    }
}

/// Database event generated from WAL entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseEvent {
    /// Monotonically increasing sequence number (RT-E2)
    pub sequence: u64,

    /// Event type
    pub event_type: EventType,

    /// Collection name
    pub collection: String,

    /// Schema name (default: "public")
    #[serde(default = "default_schema")]
    pub schema: String,

    /// Record ID
    pub record_id: String,

    /// New record data (for INSERT/UPDATE)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_data: Option<Value>,

    /// Old record data (for UPDATE/DELETE)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_data: Option<Value>,

    /// Timestamp of the event
    pub timestamp: DateTime<Utc>,

    /// User who made the change (if authenticated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Uuid>,
}

fn default_schema() -> String {
    "public".to_string()
}

impl DatabaseEvent {
    /// Create an INSERT event
    pub fn insert(
        sequence: u64,
        collection: String,
        record_id: String,
        data: Value,
        user_id: Option<Uuid>,
    ) -> Self {
        Self {
            sequence,
            event_type: EventType::Insert,
            collection,
            schema: default_schema(),
            record_id,
            new_data: Some(data),
            old_data: None,
            timestamp: Utc::now(),
            user_id,
        }
    }

    /// Create an UPDATE event
    pub fn update(
        sequence: u64,
        collection: String,
        record_id: String,
        old_data: Value,
        new_data: Value,
        user_id: Option<Uuid>,
    ) -> Self {
        Self {
            sequence,
            event_type: EventType::Update,
            collection,
            schema: default_schema(),
            record_id,
            new_data: Some(new_data),
            old_data: Some(old_data),
            timestamp: Utc::now(),
            user_id,
        }
    }

    /// Create a DELETE event
    pub fn delete(
        sequence: u64,
        collection: String,
        record_id: String,
        data: Value,
        user_id: Option<Uuid>,
    ) -> Self {
        Self {
            sequence,
            event_type: EventType::Delete,
            collection,
            schema: default_schema(),
            record_id,
            new_data: None,
            old_data: Some(data),
            timestamp: Utc::now(),
            user_id,
        }
    }

    /// Get the topic string for this event
    pub fn topic(&self) -> String {
        format!("realtime:{}:{}", self.schema, self.collection)
    }

    /// Serialize to Supabase-compatible format
    pub fn to_wire_format(&self) -> Value {
        serde_json::json!({
            "type": "postgres_changes",
            "payload": {
                "event": self.event_type.to_string(),
                "schema": self.schema,
                "table": self.collection,
                "new": self.new_data,
                "old": self.old_data,
                "commit_timestamp": self.timestamp.to_rfc3339(),
            }
        })
    }
}

/// Broadcast event for user-generated messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastEvent {
    /// Channel name
    pub channel: String,

    /// Event name (user-defined)
    pub event: String,

    /// Payload
    pub payload: Value,

    /// Sender user ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_id: Option<Uuid>,

    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl BroadcastEvent {
    /// Create a new broadcast event
    pub fn new(channel: String, event: String, payload: Value, sender_id: Option<Uuid>) -> Self {
        Self {
            channel,
            event,
            payload,
            sender_id,
            timestamp: Utc::now(),
        }
    }

    /// Get the topic string for this event
    pub fn topic(&self) -> String {
        format!("realtime:broadcast:{}", self.channel)
    }

    /// Serialize to wire format
    pub fn to_wire_format(&self) -> Value {
        serde_json::json!({
            "type": "broadcast",
            "topic": self.topic(),
            "event": self.event,
            "payload": self.payload,
        })
    }
}

/// Presence event for user presence updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceEvent {
    /// Channel name
    pub channel: String,

    /// Event type (join, leave, sync)
    pub event: PresenceEventType,

    /// User state(s)
    pub state: Value,

    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Type of presence event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PresenceEventType {
    Join,
    Leave,
    Sync,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_display() {
        assert_eq!(EventType::Insert.to_string(), "INSERT");
        assert_eq!(EventType::Update.to_string(), "UPDATE");
        assert_eq!(EventType::Delete.to_string(), "DELETE");
    }

    #[test]
    fn test_insert_event() {
        let event = DatabaseEvent::insert(
            1,
            "posts".to_string(),
            "abc123".to_string(),
            serde_json::json!({"title": "Hello"}),
            None,
        );

        assert_eq!(event.sequence, 1);
        assert_eq!(event.event_type, EventType::Insert);
        assert_eq!(event.collection, "posts");
        assert!(event.new_data.is_some());
        assert!(event.old_data.is_none());
    }

    #[test]
    fn test_update_event() {
        let event = DatabaseEvent::update(
            2,
            "posts".to_string(),
            "abc123".to_string(),
            serde_json::json!({"title": "Hello"}),
            serde_json::json!({"title": "Updated"}),
            None,
        );

        assert_eq!(event.event_type, EventType::Update);
        assert!(event.new_data.is_some());
        assert!(event.old_data.is_some());
    }

    #[test]
    fn test_delete_event() {
        let event = DatabaseEvent::delete(
            3,
            "posts".to_string(),
            "abc123".to_string(),
            serde_json::json!({"title": "Goodbye"}),
            None,
        );

        assert_eq!(event.event_type, EventType::Delete);
        assert!(event.new_data.is_none());
        assert!(event.old_data.is_some());
    }

    #[test]
    fn test_event_topic() {
        let event = DatabaseEvent::insert(
            1,
            "posts".to_string(),
            "abc123".to_string(),
            serde_json::json!({}),
            None,
        );

        assert_eq!(event.topic(), "realtime:public:posts");
    }

    #[test]
    fn test_wire_format() {
        let event = DatabaseEvent::insert(
            1,
            "posts".to_string(),
            "abc123".to_string(),
            serde_json::json!({"title": "Test"}),
            None,
        );

        let wire = event.to_wire_format();
        assert_eq!(wire["type"], "postgres_changes");
        assert_eq!(wire["payload"]["event"], "INSERT");
        assert_eq!(wire["payload"]["table"], "posts");
    }

    #[test]
    fn test_broadcast_event() {
        let event = BroadcastEvent::new(
            "chat-room".to_string(),
            "message".to_string(),
            serde_json::json!({"text": "Hello!"}),
            None,
        );

        assert_eq!(event.topic(), "realtime:broadcast:chat-room");

        let wire = event.to_wire_format();
        assert_eq!(wire["type"], "broadcast");
        assert_eq!(wire["event"], "message");
    }
}
