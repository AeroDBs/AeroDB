# Phase 10: Broadcast Model

**Document Type:** Technical Specification  
**Phase:** 10 - Real-Time Subscriptions  
**Status:** Active

---

## Overview

Broadcast channels allow clients to send messages to each other without persisting to the database.

---

## Channel Types

### Public Channels

Anyone can join and broadcast:

```
realtime:broadcast:public-announcements
```

### Private Channels

Require membership (checked via RLS):

```
realtime:broadcast:private-team-123
```

---

## Message Format

### Send

```json
{
  "type": "broadcast",
  "topic": "realtime:broadcast:chat-room-1",
  "event": "message",
  "payload": {
    "text": "Hello, world!"
  }
}
```

### Receive

```json
{
  "type": "broadcast",
  "topic": "realtime:broadcast:chat-room-1",
  "event": "message",
  "payload": {
    "text": "Hello, world!",
    "sender_id": "user-uuid"
  }
}
```

---

## Channel Isolation

- Channels are isolated by name
- No cross-channel message leakage
- RLS can restrict channel access

---

## Rate Limiting

| Limit | Value |
|-------|-------|
| Messages per second | 10 |
| Message size | 64KB |
| Channels per connection | 100 |
