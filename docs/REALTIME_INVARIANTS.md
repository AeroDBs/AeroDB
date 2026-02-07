# Phase 10: Real-Time Invariants

**Document Type:** Normative Specification  
**Phase:** 10 - Real-Time Subscriptions  
**Status:** Active

---

## Event Log Invariants

### RT-E1: Deterministic Transformation

> Same WAL → Same events. Event generation is reproducible.

### RT-E2: Monotonic Sequence

> Event sequence numbers are monotonically increasing, gapless.

### RT-E3: Immutable Events

> Once an event is created, it cannot be modified.

---

## Delivery Invariants

### RT-D1: Best-Effort Delivery

> No guarantee of delivery. Clients must handle missed events.

### RT-D2: No Ordering Guarantee

> Events may arrive out of order. Use sequence numbers for ordering.

### RT-D3: RLS Filtered

> Events are filtered by RLS before delivery. No unauthorized access.

---

## Subscription Invariants

### RT-S1: Explicit Subscribe

> No implicit subscriptions. Clients must explicitly subscribe.

### RT-S2: Scoped Delivery

> Events only delivered to subscribed clients.

### RT-S3: Clean Unsubscribe

> Unsubscribe immediately stops delivery (best-effort).

---

## Presence Invariants

### RT-P1: Eventually Consistent

> Presence is eventually consistent, not immediately consistent.

### RT-P2: Heartbeat Required

> No heartbeat for 60s = considered offline.

---

## Broadcast Invariants

### RT-B1: No Persistence

> Broadcast messages are not persisted.

### RT-B2: Channel Isolation

> Messages in channel A never leak to channel B.
