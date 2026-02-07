# FAILOVER_FAILURE_MODEL.md — Failover & Promotion

## Status
- Phase: **6**
- Authority: **Normative**
- Depends on:
  - FAILOVER_VISION.md
  - FAILOVER_SCOPE.md
  - FAILOVER_INVARIANTS.md
  - FAILOVER_ARCHITECTURE.md
- Frozen Dependencies: **Phases 0–5**

---

## 1. Purpose

This document defines the **failure model for Phase 6**.

It specifies:
- What failures are assumed possible
- Where failures may occur
- What outcomes are required
- How promotion must behave under failure

Phase 6 **inherits all failure assumptions** from Phases 0–5.
No assumptions are weakened, removed, or narrowed.

---

## 2. Failure Model Continuity

Phase 6 assumes **exactly the same failure surface** as earlier phases:

- Process may crash at any instruction
- Power loss may occur at any time
- Disk writes may be partial or reordered
- fsync is the only durability boundary
- Memory state is volatile
- Time is not monotonic across crashes
- No graceful shutdown is assumed

Phase 6 introduces **no new reliability assumptions**.

---

## 3. Promotion-Specific Failure Points

Promotion introduces new *logical* failure boundaries.

These boundaries must be explicitly handled.

---

### 3.1 Failure Before Promotion Validation

**Examples**
- Crash before validation begins
- Validation request rejected immediately
- Operator aborts request

**Required Outcome**
- No authority change
- System remains in pre-promotion state
- No recovery ambiguity

---

### 3.2 Failure During Promotion Validation

**Examples**
- Crash while evaluating WAL position
- Crash while checking replica safety
- Partial validation execution

**Required Outcome**
- Promotion is considered **not attempted**
- No authority transition occurs
- On recovery, system behaves as if promotion never started

Validation has **no durable effect**.

---

### 3.3 Failure After Validation, Before Authority Transition

**Examples**
- Crash after validation succeeds
- Crash before authority transition is applied

**Required Outcome**
- Promotion is **not applied**
- Authority remains unchanged
- Validation must be re-run on retry

There is no “validated but pending” state after recovery.

---

### 3.4 Failure During Authority Transition

**Examples**
- Crash while updating replication role
- Crash while rebinding authority
- Crash during atomic transition

**Required Outcome**
- Authority transition MUST be atomic
- Recovery MUST observe:
  - Either the old authority state
  - Or the new authority state
- Never a mixed or ambiguous state

Partial authority transitions are forbidden.

---

### 3.5 Failure After Authority Transition Completes

**Examples**
- Crash immediately after promotion completes
- Crash before observability events are flushed

**Required Outcome**
- New authority state is authoritative
- Recovery MUST reestablish the promoted primary
- Observability gaps are acceptable; authority gaps are not

---

## 4. Primary Failure Scenarios

### 4.1 Primary Crash Before Promotion

**Scenario**
- Primary crashes
- No promotion attempted yet

**Required Outcome**
- System has no writable primary
- Replicas remain replicas
- Reads obey replica read rules
- Writes are rejected

Availability loss is acceptable; unsafe promotion is not.

---

### 4.2 Primary Crash During Promotion Attempt

**Scenario**
- Promotion is attempted
- Primary crashes mid-process

**Required Outcome**
- Promotion outcome depends solely on authority transition completion
- No reliance on primary liveness
- No automatic retry

Promotion safety must be provable without the primary.

---

## 5. Replica Failure Scenarios

### 5.1 Replica Crash Before Promotion

**Required Outcome**
- Promotion request fails explicitly
- No authority change
- No recovery ambiguity

---

### 5.2 Replica Crash During Promotion

**Required Outcome**
- Promotion fails or is rolled back
- Authority remains unchanged
- Recovery is deterministic

---

## 6. Split-Brain Risk Handling

Phase 6 explicitly **does not tolerate split-brain**.

If:
- Authority safety cannot be proven
- Primary liveness is ambiguous
- Replica divergence is suspected

**Required Outcome**
- Promotion MUST be rejected
- System MUST fail closed

No best-effort behavior is allowed.

---

## 7. Network Failure Considerations

Phase 6 assumes:
- Network partitions are possible
- Messages may be delayed or lost
- No reliable membership service exists

Phase 6 MUST NOT:
- Infer authority from network reachability
- Infer primary death from timeouts
- Use network health as a promotion signal

---

## 8. Deterministic Failure Outcomes

For any failure scenario:
- Outcome MUST be deterministic
- Outcome MUST be explainable
- Outcome MUST map to Phase 6 invariants

Failure handling MUST NOT depend on:
- Timing
- Retry count
- Environmental factors

---

## 9. Forbidden Failure Handling

Phase 6 MUST NOT:
- Retry promotion automatically
- Mask failures
- Escalate privileges implicitly
- Enter degraded authority modes
- “Guess” the safest outcome

Explicit failure is always preferred.

---

## 10. Testing Obligations

Phase 6 failure handling MUST be validated by:

- Crash tests at every promotion boundary
- Recovery tests for each failure scenario
- Deterministic replay verification
- Invariant enforcement tests

All Phase 0–5 tests MUST pass unmodified.

---

## 11. Failure Model Completeness Rule

The failure model is complete when:

- Every promotion boundary is a failure boundary
- Every failure produces a single valid outcome
- No recovery ambiguity exists
- No silent behavior exists

---

END OF DOCUMENT
