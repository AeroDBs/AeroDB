# Phase 10: Presence Model

**Document Type:** Technical Specification  
**Phase:** 10 - Real-Time Subscriptions  
**Status:** Active

---

## Overview

Presence tracks which users are currently connected to a channel.

---

## Presence State

```rust
pub struct PresenceState {
    /// User ID
    pub user_id: Uuid,
    
    /// User metadata (online status, custom data)
    pub metadata: Value,
    
    /// When user joined
    pub joined_at: DateTime<Utc>,
    
    /// Last heartbeat
    pub last_seen: DateTime<Utc>,
}
```

---

## Events

### Track (Join)

```json
{
  "type": "presence",
  "topic": "realtime:presence:lobby",
  "event": "track",
  "payload": {
    "status": "online",
    "typing": false
  }
}
```

### Untrack (Leave)

```json
{
  "type": "presence",
  "event": "untrack"
}
```

### Sync (Get All)

Server sends current state on join:

```json
{
  "type": "presence",
  "event": "sync",
  "payload": {
    "user-1": { "status": "online" },
    "user-2": { "status": "away" }
  }
}
```

---

## Liveness Detection

- **Heartbeat interval:** 30 seconds
- **Timeout:** 60 seconds (2 missed heartbeats)
- **Explicit leave** on disconnect

---

## Invariant: RT-P1

> Presence is eventually consistent, not immediately consistent.

Users may appear briefly after disconnect due to heartbeat window.
