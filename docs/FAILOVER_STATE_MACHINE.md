# FAILOVER_STATE_MACHINE.md — Failover & Promotion

## Status
- Phase: **6**
- Authority: **Normative**
- Depends on:
  - FAILOVER_VISION.md
  - FAILOVER_SCOPE.md
  - FAILOVER_INVARIANTS.md
  - FAILOVER_ARCHITECTURE.md
  - FAILOVER_FAILURE_MODEL.md
- Frozen Dependencies: **Phases 0–5**

---

## 1. Purpose

This document defines the **explicit state machine** governing failover and
promotion in Phase 6.

It specifies:
- States
- Transitions
- Entry and exit conditions
- Forbidden transitions

This state machine is **authoritative** for Phase 6 behavior.

---

## 2. Design Rules

The Phase 6 state machine obeys the following rules:

1. States are **explicit and enumerable**
2. Transitions are **event-driven**, never inferred
3. All transitions are **deterministic**
4. No background or time-based transitions exist
5. All authority changes are **atomic**
6. All failures are **explicit**

If a transition is not listed here, it is **forbidden**.

---

## 3. Relationship to Phase 5 State Machine

Phase 6 does **not replace** the Phase 5 replication state machine.

Instead:
- Phase 5 defines **replication role**
- Phase 6 defines **promotion lifecycle**

Phase 6 states **observe and constrain** Phase 5 transitions;
they do not add hidden paths.

---

## 4. Phase 6 States

### 4.1 `Steady`

**Meaning**
- System is operating normally
- No promotion attempt in progress
- Replication roles are stable

**Entry Conditions**
- System startup after successful recovery
- Completion of a promotion attempt (success or failure)

**Exit Conditions**
- Explicit promotion request received

---

### 4.2 `PromotionRequested`

**Meaning**
- An explicit promotion request has been issued
- No validation has begun yet

**Entry Conditions**
- Operator or control-plane requests promotion

**Exit Conditions**
- Transition to `PromotionValidating`
- Transition to `Steady` (request rejected immediately)

---

### 4.3 `PromotionValidating`

**Meaning**
- System is validating whether promotion is allowed
- No authority change has occurred

**Actions**
- Validate WAL safety
- Validate replication invariants
- Validate single-writer guarantees
- Validate crash safety

**Exit Conditions**
- Transition to `PromotionApproved`
- Transition to `PromotionDenied`

---

### 4.4 `PromotionApproved`

**Meaning**
- Promotion has been fully validated
- Authority transition is permitted but not yet applied

**Properties**
- Approval has **no durable effect**
- Approval may be invalidated by crash

**Exit Conditions**
- Transition to `AuthorityTransitioning`

---

### 4.5 `AuthorityTransitioning`

**Meaning**
- Atomic authority transfer is in progress

**Actions**
- Apply authority rebinding
- Update replication role explicitly
- Ensure atomicity

**Exit Conditions**
- Transition to `PromotionSucceeded`
- System crash (handled by recovery rules)

---

### 4.6 `PromotionSucceeded`

**Meaning**
- Promotion completed successfully
- New primary is authoritative

**Entry Conditions**
- Authority transition completed atomically

**Exit Conditions**
- Transition to `Steady`

---

### 4.7 `PromotionDenied`

**Meaning**
- Promotion validation failed

**Properties**
- Failure reasons are explicit
- No authority change occurred

**Exit Conditions**
- Transition to `Steady`

---

## 5. Terminal and Recovery Behavior

There are **no terminal states** in Phase 6.

On crash and recovery:
- System MUST re-enter `Steady`
- Authority state MUST be reconstructed deterministically
- No partial promotion state may persist

---

## 6. Allowed Transitions (Complete List)

```

Steady
→ PromotionRequested

PromotionRequested
→ PromotionValidating
→ Steady

PromotionValidating
→ PromotionApproved
→ PromotionDenied

PromotionApproved
→ AuthorityTransitioning

AuthorityTransitioning
→ PromotionSucceeded

PromotionSucceeded
→ Steady

PromotionDenied
→ Steady

```

No other transitions are permitted.

---

## 7. Forbidden Transitions (Explicit)

The following transitions are **forbidden**:

- `Steady → AuthorityTransitioning`
- `PromotionRequested → PromotionApproved`
- `PromotionValidating → AuthorityTransitioning`
- `PromotionDenied → AuthorityTransitioning`
- Any transition driven by timeouts or retries
- Any implicit re-entry into `PromotionApproved` after crash

---

## 8. Crash Semantics per State

| State | Crash Outcome |
|-----|--------------|
| Steady | No effect |
| PromotionRequested | Promotion forgotten |
| PromotionValidating | Promotion forgotten |
| PromotionApproved | Promotion forgotten |
| AuthorityTransitioning | Atomic outcome enforced |
| PromotionSucceeded | Authority preserved |
| PromotionDenied | Promotion forgotten |

Crash behavior is deterministic and invariant-preserving.

---

## 9. Observability Requirements

Each state transition MUST emit:
- State entry event
- Transition reason
- Relevant invariant references

Silent transitions are forbidden.

---

## 10. State Machine Completeness Rule

This state machine is complete when:
- Every promotion attempt follows exactly one path
- No ambiguity exists after crash
- All invariant violations result in `PromotionDenied`
- All success paths converge to `Steady`

---

END OF DOCUMENT
