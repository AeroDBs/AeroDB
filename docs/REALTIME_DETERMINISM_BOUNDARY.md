# Phase 10: Determinism Boundary

**Document Type:** Normative Specification  
**Phase:** 10 - Real-Time Subscriptions  
**Status:** Active

---

## The Boundary

AeroDB maintains strict determinism through Phase 7. Phase 10 introduces **explicit non-determinism** for real-time delivery.

```
┌──────────────────────────────────────────┐
│         DETERMINISTIC ZONE                │
│  WAL → Event Log → Sequence Numbers       │
│  Same input → Same output                 │
└──────────────────────────────────────────┘
                    ║
                    ║ ← DETERMINISM BOUNDARY
                    ║
┌──────────────────────────────────────────┐
│       NON-DETERMINISTIC ZONE              │
│  Dispatcher → WebSocket → Client          │
│  Best-effort, no ordering guarantees      │
└──────────────────────────────────────────┘
```

---

## What IS Deterministic

| Component | Guarantee |
|-----------|-----------|
| Event generation | Same WAL → Same events |
| Sequence numbers | Monotonically increasing |
| Event content | Immutable once created |
| Event ordering in log | Matches WAL order |

---

## What is NOT Deterministic

| Component | Why |
|-----------|-----|
| Delivery timing | Network latency varies |
| Client receive order | Multiple TCP connections |
| Reconnection behavior | Client-dependent |
| Presence state | Eventually consistent |

---

## Client Responsibilities

1. **Handle out-of-order events** using sequence numbers
2. **Implement idempotency** for event processing
3. **Support reconnection** with resume from last sequence
4. **Tolerate missed events** during disconnection

---

## Why This Design?

> "It is better to be **explicitly non-deterministic** than to pretend guarantees that don't exist."

Real-time delivery over networks cannot be deterministic. AeroDB acknowledges this honestly.
