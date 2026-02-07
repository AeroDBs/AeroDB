# Phase 10: Real-Time Architecture

**Document Type:** Technical Architecture  
**Phase:** 10 - Real-Time Subscriptions  
**Status:** Active

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                     Client Applications                          │
│              WebSocket connections to /realtime                  │
└─────────────────────────────────────────────────────────────────┘
                              ▲
                              │ WebSocket (wss://)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     WebSocket Server                             │
│  - Connection management                                         │
│  - Authentication (JWT from ?token= or header)                   │
│  - Subscription routing                                          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Subscription Manager                         │
│  - Track client subscriptions                                    │
│  - Filter events by subscription predicates                      │
│  - Apply RLS before delivery                                     │
└─────────────────────────────────────────────────────────────────┘
                              ▲
                              │
┌─────────────────────────────────────────────────────────────────┐
│                     Event Dispatcher                             │
│  - Receive events from Event Log                                 │
│  - Fan-out to subscribed clients                                 │
│  - Non-deterministic (best-effort)                               │
└─────────────────────────────────────────────────────────────────┘
                              ▲
                              │
┌─────────────────────────────────────────────────────────────────┐
│                     Event Log (Deterministic)                    │
│  - WAL → Event transformation                                    │
│  - Sequence numbers for ordering                                 │
│  - In-memory ring buffer (configurable size)                     │
└─────────────────────────────────────────────────────────────────┘
                              ▲
                              │
┌─────────────────────────────────────────────────────────────────┐
│                     WAL (Write-Ahead Log)                        │
│  - Source of truth for all mutations                             │
│  - Deterministic and durable                                     │
└─────────────────────────────────────────────────────────────────┘
```

---

## Module Structure

```
src/realtime/
├── mod.rs           # Module entry, exports
├── errors.rs        # Real-time error types
├── event.rs         # Event types (DatabaseEvent, BroadcastEvent)
├── event_log.rs     # WAL → Event transformation
├── subscription.rs  # Subscription management
├── dispatcher.rs    # Event fan-out
├── broadcast.rs     # Pub/sub channels
├── presence.rs      # User presence tracking
└── server.rs        # WebSocket server (tokio-tungstenite)
```

---

## Data Flow

1. **Write committed to WAL**
2. **Event Log observes WAL** → Creates `DatabaseEvent`
3. **Dispatcher receives event** → Looks up subscriptions
4. **Per-subscription:** Apply RLS filter → Deliver if allowed
5. **Client receives event** over WebSocket

---

## Integration Points

| Component | Integration |
|-----------|-------------|
| WAL (Phase 1) | Event Log reads committed entries |
| Auth (Phase 8) | JWT validation, RlsContext |
| REST API (Phase 9) | Shared RLS enforcement |
