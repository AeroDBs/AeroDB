# Phase 10: Event Model

**Document Type:** Technical Specification  
**Phase:** 10 - Real-Time Subscriptions  
**Status:** Active

---

## Event Types

### Database Events

Generated from WAL entries:

```rust
pub enum EventType {
    INSERT,
    UPDATE,
    DELETE,
}

pub struct DatabaseEvent {
    /// Monotonically increasing sequence number
    pub sequence: u64,
    
    /// Event type
    pub event_type: EventType,
    
    /// Collection name
    pub collection: String,
    
    /// Record ID
    pub record_id: String,
    
    /// New record data (for INSERT/UPDATE)
    pub new_data: Option<Value>,
    
    /// Old record data (for UPDATE/DELETE)
    pub old_data: Option<Value>,
    
    /// Timestamp of the event
    pub timestamp: DateTime<Utc>,
    
    /// User who made the change (if authenticated)
    pub user_id: Option<Uuid>,
}
```

### Broadcast Events

User-generated messages to channels:

```rust
pub struct BroadcastEvent {
    /// Channel name
    pub channel: String,
    
    /// Event name (user-defined)
    pub event: String,
    
    /// Payload
    pub payload: Value,
    
    /// Sender user ID
    pub sender_id: Option<Uuid>,
    
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}
```

---

## WAL → Event Transformation

### Deterministic Mapping

| WAL Record Type | Event Type | Behavior |
|-----------------|------------|----------|
| Insert | INSERT | new_data = record |
| Update | UPDATE | old_data = before, new_data = after |
| Delete | DELETE | old_data = record |

### Invariant: RT-E1

> Same WAL sequence → Same event sequence

The Event Log transformation is deterministic and reproducible.

---

## Event Serialization

Events sent over WebSocket as JSON:

```json
{
  "type": "postgres_changes",
  "payload": {
    "event": "INSERT",
    "schema": "public",
    "table": "posts",
    "new": { "id": "...", "title": "..." },
    "old": null,
    "commit_timestamp": "2026-02-06T00:00:00Z"
  }
}
```

Compatible with Supabase Realtime protocol for client SDK compatibility.
