//! # Event Log
//!
//! Deterministic transformation from WAL to events.
//!
//! ## Invariant: RT-E1
//! Same WAL → Same events. Event generation is reproducible.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

use serde_json::Value;
use uuid::Uuid;

use super::event::{DatabaseEvent, EventType};

/// Configuration for the event log
#[derive(Debug, Clone)]
pub struct EventLogConfig {
    /// Maximum number of events to keep in memory
    pub max_events: usize,
}

impl Default for EventLogConfig {
    fn default() -> Self {
        Self { max_events: 10_000 }
    }
}

/// Event log that transforms WAL entries into events
///
/// This is the deterministic layer - same WAL produces same events.
#[derive(Debug)]
pub struct EventLog {
    /// Configuration
    config: EventLogConfig,

    /// Next sequence number (monotonically increasing)
    next_sequence: AtomicU64,

    /// Ring buffer of recent events
    events: RwLock<VecDeque<DatabaseEvent>>,
}

impl Default for EventLog {
    fn default() -> Self {
        Self::new(EventLogConfig::default())
    }
}

impl EventLog {
    /// Create a new event log
    pub fn new(config: EventLogConfig) -> Self {
        let capacity = config.max_events;
        Self {
            config,
            next_sequence: AtomicU64::new(1),
            events: RwLock::new(VecDeque::with_capacity(capacity)),
        }
    }

    /// Get the next sequence number (for testing/inspection)
    pub fn next_sequence(&self) -> u64 {
        self.next_sequence.load(Ordering::Acquire)
    }

    /// Record an INSERT event from WAL
    pub fn record_insert(
        &self,
        collection: String,
        record_id: String,
        data: Value,
        user_id: Option<Uuid>,
    ) -> DatabaseEvent {
        let sequence = self.next_sequence.fetch_add(1, Ordering::SeqCst);
        let event = DatabaseEvent::insert(sequence, collection, record_id, data, user_id);
        self.append(event.clone());
        event
    }

    /// Record an UPDATE event from WAL
    pub fn record_update(
        &self,
        collection: String,
        record_id: String,
        old_data: Value,
        new_data: Value,
        user_id: Option<Uuid>,
    ) -> DatabaseEvent {
        let sequence = self.next_sequence.fetch_add(1, Ordering::SeqCst);
        let event =
            DatabaseEvent::update(sequence, collection, record_id, old_data, new_data, user_id);
        self.append(event.clone());
        event
    }

    /// Record a DELETE event from WAL
    pub fn record_delete(
        &self,
        collection: String,
        record_id: String,
        data: Value,
        user_id: Option<Uuid>,
    ) -> DatabaseEvent {
        let sequence = self.next_sequence.fetch_add(1, Ordering::SeqCst);
        let event = DatabaseEvent::delete(sequence, collection, record_id, data, user_id);
        self.append(event.clone());
        event
    }

    /// Append event to the ring buffer
    fn append(&self, event: DatabaseEvent) {
        if let Ok(mut events) = self.events.write() {
            events.push_back(event);

            // Trim if over capacity
            while events.len() > self.config.max_events {
                events.pop_front();
            }
        }
    }

    /// Get events since a given sequence number
    pub fn events_since(&self, since_sequence: u64) -> Vec<DatabaseEvent> {
        if let Ok(events) = self.events.read() {
            events
                .iter()
                .filter(|e| e.sequence > since_sequence)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get events for a specific collection since a given sequence
    pub fn events_for_collection(
        &self,
        collection: &str,
        since_sequence: u64,
    ) -> Vec<DatabaseEvent> {
        if let Ok(events) = self.events.read() {
            events
                .iter()
                .filter(|e| e.sequence > since_sequence && e.collection == collection)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get the most recent events (up to limit)
    pub fn recent_events(&self, limit: usize) -> Vec<DatabaseEvent> {
        if let Ok(events) = self.events.read() {
            events.iter().rev().take(limit).cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Get count of events in buffer
    pub fn len(&self) -> usize {
        self.events.read().map(|e| e.len()).unwrap_or(0)
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequence_numbers_increment() {
        let log = EventLog::default();

        let e1 = log.record_insert(
            "posts".to_string(),
            "1".to_string(),
            serde_json::json!({}),
            None,
        );
        let e2 = log.record_insert(
            "posts".to_string(),
            "2".to_string(),
            serde_json::json!({}),
            None,
        );
        let e3 = log.record_insert(
            "posts".to_string(),
            "3".to_string(),
            serde_json::json!({}),
            None,
        );

        assert_eq!(e1.sequence, 1);
        assert_eq!(e2.sequence, 2);
        assert_eq!(e3.sequence, 3);
        assert_eq!(log.next_sequence(), 4);
    }

    #[test]
    fn test_events_since() {
        let log = EventLog::default();

        log.record_insert(
            "posts".to_string(),
            "1".to_string(),
            serde_json::json!({}),
            None,
        );
        log.record_insert(
            "posts".to_string(),
            "2".to_string(),
            serde_json::json!({}),
            None,
        );
        log.record_insert(
            "posts".to_string(),
            "3".to_string(),
            serde_json::json!({}),
            None,
        );

        let events = log.events_since(1);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].sequence, 2);
        assert_eq!(events[1].sequence, 3);
    }

    #[test]
    fn test_events_for_collection() {
        let log = EventLog::default();

        log.record_insert(
            "posts".to_string(),
            "1".to_string(),
            serde_json::json!({}),
            None,
        );
        log.record_insert(
            "comments".to_string(),
            "1".to_string(),
            serde_json::json!({}),
            None,
        );
        log.record_insert(
            "posts".to_string(),
            "2".to_string(),
            serde_json::json!({}),
            None,
        );

        let events = log.events_for_collection("posts", 0);
        assert_eq!(events.len(), 2);
        assert!(events.iter().all(|e| e.collection == "posts"));
    }

    #[test]
    fn test_ring_buffer_capacity() {
        let log = EventLog::new(EventLogConfig { max_events: 5 });

        for i in 0..10 {
            log.record_insert(
                "posts".to_string(),
                i.to_string(),
                serde_json::json!({}),
                None,
            );
        }

        assert_eq!(log.len(), 5);

        // Should have events 6-10
        let events = log.events_since(0);
        assert_eq!(events[0].sequence, 6);
        assert_eq!(events[4].sequence, 10);
    }

    #[test]
    fn test_event_types() {
        let log = EventLog::default();

        let insert = log.record_insert(
            "posts".to_string(),
            "1".to_string(),
            serde_json::json!({"a": 1}),
            None,
        );
        let update = log.record_update(
            "posts".to_string(),
            "1".to_string(),
            serde_json::json!({"a": 1}),
            serde_json::json!({"a": 2}),
            None,
        );
        let delete = log.record_delete(
            "posts".to_string(),
            "1".to_string(),
            serde_json::json!({"a": 2}),
            None,
        );

        assert_eq!(insert.event_type, EventType::Insert);
        assert_eq!(update.event_type, EventType::Update);
        assert_eq!(delete.event_type, EventType::Delete);
    }

    #[test]
    fn test_deterministic_transformation() {
        // RT-E1: Same WAL → Same events
        // Simulate same sequence of operations
        let log1 = EventLog::default();
        let log2 = EventLog::default();

        // Same operations on both logs
        log1.record_insert(
            "posts".to_string(),
            "1".to_string(),
            serde_json::json!({"title": "Hello"}),
            None,
        );
        log1.record_update(
            "posts".to_string(),
            "1".to_string(),
            serde_json::json!({"title": "Hello"}),
            serde_json::json!({"title": "World"}),
            None,
        );

        log2.record_insert(
            "posts".to_string(),
            "1".to_string(),
            serde_json::json!({"title": "Hello"}),
            None,
        );
        log2.record_update(
            "posts".to_string(),
            "1".to_string(),
            serde_json::json!({"title": "Hello"}),
            serde_json::json!({"title": "World"}),
            None,
        );

        let events1 = log1.events_since(0);
        let events2 = log2.events_since(0);

        assert_eq!(events1.len(), events2.len());
        for (e1, e2) in events1.iter().zip(events2.iter()) {
            assert_eq!(e1.sequence, e2.sequence);
            assert_eq!(e1.event_type, e2.event_type);
            assert_eq!(e1.collection, e2.collection);
            assert_eq!(e1.record_id, e2.record_id);
            assert_eq!(e1.new_data, e2.new_data);
            assert_eq!(e1.old_data, e2.old_data);
        }
    }
}
