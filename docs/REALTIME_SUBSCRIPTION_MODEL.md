# Phase 10: Subscription Model

**Document Type:** Technical Specification  
**Phase:** 10 - Real-Time Subscriptions  
**Status:** Active

---

## Subscription Types

### Database Changes

Subscribe to INSERT/UPDATE/DELETE on collections:

```json
{
  "type": "subscribe",
  "topic": "realtime:public:posts",
  "event": "*",
  "filter": "author_id=eq.123"
}
```

### Broadcast

Subscribe to user-defined channels:

```json
{
  "type": "subscribe",
  "topic": "realtime:broadcast:chat-room-1"
}
```

### Presence

Subscribe to user presence:

```json
{
  "type": "subscribe",
  "topic": "realtime:presence:lobby"
}
```

---

## Subscription Lifecycle

```
┌─────────┐    subscribe    ┌────────────┐
│  NONE   │ ───────────────► │  PENDING   │
└─────────┘                  └────────────┘
                                   │
                                   │ ack
                                   ▼
                             ┌────────────┐
                             │   ACTIVE   │
                             └────────────┘
                                   │
                                   │ unsubscribe / disconnect
                                   ▼
                             ┌────────────┐
                             │   REMOVED  │
                             └────────────┘
```

---

## Subscription Filtering

### Collection Filters

PostgREST-style syntax:

- `author_id=eq.123` - Equals
- `status=in.(draft,published)` - In list
- `created_at=gt.2026-01-01` - Greater than

### RLS Filtering

Before delivery, each event is checked against RLS:

1. Extract user_id from subscription context
2. Check if user can access the record
3. Only deliver if RLS passes

---

## Subscription Registry

```rust
pub struct SubscriptionRegistry {
    /// Subscriptions by topic
    by_topic: HashMap<String, Vec<Subscription>>,
    
    /// Subscriptions by connection ID
    by_connection: HashMap<String, Vec<String>>,
}
```
