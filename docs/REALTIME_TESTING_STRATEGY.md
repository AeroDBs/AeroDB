# Phase 10: Testing Strategy

**Document Type:** Technical Specification  
**Phase:** 10 - Real-Time Subscriptions  
**Status:** Active

---

## Test Categories

### Unit Tests

| Component | Coverage Target |
|-----------|-----------------|
| event.rs | 100% |
| event_log.rs | 95% |
| subscription.rs | 90% |
| dispatcher.rs | 85% |
| broadcast.rs | 90% |
| presence.rs | 90% |

### Integration Tests

| Scenario | Dependencies |
|----------|--------------|
| End-to-end event flow | Event Log, Dispatcher |
| Subscription filtering | Subscription, RLS |
| Broadcast channels | Broadcast, WebSocket |
| Presence sync | Presence, WebSocket |

---

## Critical Test Scenarios

### Event Log

1. WAL entry produces correct event type
2. Sequence numbers are gapless
3. Same WAL replayed = same events

### Subscriptions

1. Subscribe receives confirmation
2. Matching events delivered
3. Non-matching events filtered
4. RLS prevents unauthorized access

### Dispatcher

1. Events fan-out to all subscribers
2. Unsubscribed clients don't receive
3. Disconnected clients cleaned up

### Broadcast

1. Messages delivered to channel
2. Non-subscribers don't receive
3. Rate limiting enforced

### Presence

1. Track adds user to presence
2. Untrack removes user
3. Timeout removes user after 60s

---

## Determinism Tests

### RT-E1 Test

```rust
#[test]
fn same_wal_same_events() {
    let wal = create_test_wal();
    let events1 = EventLog::replay(&wal);
    let events2 = EventLog::replay(&wal);
    assert_eq!(events1, events2);
}
```

---

## Coverage Requirements

| Component | Minimum |
|-----------|---------|
| Event Log | 95% |
| Subscription | 90% |
| Overall | 85% |
