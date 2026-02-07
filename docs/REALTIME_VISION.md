# Phase 10: Real-Time Vision

**Document Type:** Vision Statement  
**Phase:** 10 - Real-Time Subscriptions  
**Status:** Active

---

## Goal

Enable real-time event subscriptions over WebSocket, allowing clients to receive database changes as they happen, while preserving AeroDB's determinism guarantees where possible.

---

## Philosophy

### Determinism Boundary

AeroDB introduces a **clear determinism boundary** in Phase 10:

| Layer | Determinism | Guarantee |
|-------|-------------|-----------|
| WAL | Deterministic | Same writes → Same log |
| Event Log | Deterministic | Same WAL → Same events |
| Dispatcher | **Non-Deterministic** | Best-effort delivery |
| Client | Non-Deterministic | Must handle out-of-order |

> **Key Insight:** The Event Log is the last deterministic layer. Clients see a projection that may differ due to network conditions.

### Core Principles

1. **Event Log as Source of Truth** - Database events derived from WAL
2. **Explicit Non-Determinism** - Dispatcher makes no ordering guarantees
3. **RLS-Filtered Delivery** - Events filtered per-user before dispatch
4. **Fail-Open for Delivery** - Missed events don't crash the system
5. **Explicit Subscriptions** - No implicit broadcast

---

## Non-Goals

- Exactly-once delivery (at-least-once is acceptable)
- Global ordering of events across all clients
- Guaranteed delivery (clients must handle reconnection)
- Persistent event history (events are ephemeral)

---

## Success Criteria

1. Clients receive events within 100ms of commit (p95)
2. RLS filtering applied to all events
3. System handles 10k concurrent WebSocket connections
4. Clean reconnection with resume from sequence number
