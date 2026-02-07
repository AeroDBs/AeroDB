# FAILOVER_IMPLEMENTATION_ORDER.md — Failover & Promotion

## Status
- Phase: **6**
- Authority: **Normative**
- Depends on:
  - FAILOVER_VISION.md
  - FAILOVER_SCOPE.md
  - FAILOVER_INVARIANTS.md
  - FAILOVER_ARCHITECTURE.md
  - FAILOVER_FAILURE_MODEL.md
  - FAILOVER_STATE_MACHINE.md
  - FAILOVER_OBSERVABILITY_MAPPING.md
  - FAILOVER_TESTING_STRATEGY.md
- Frozen Dependencies: **Phases 0–5**

---

## 1. Purpose

This document defines the **strict, linear implementation order** for Phase 6.

The order exists to:
- Preserve correctness
- Prevent partial semantics
- Ensure testability at every step
- Avoid speculative or coupled implementation

Stages MUST be completed **in order**.
Skipping, reordering, or merging stages is forbidden.

---

## 2. Global Rules

Phase 6 implementation MUST obey:

1. One stage at a time
2. Each stage ends with tests
3. No stage may weaken a frozen phase
4. No later stage may compensate for an earlier shortcut
5. Failure to complete a stage blocks all subsequent work

---

## 3. Stage Breakdown (Authoritative)

### Stage 6.1 — Promotion State Definitions

**Goal**
- Introduce Phase 6 promotion states
- Integrate them as a distinct, orthogonal state machine

**Requirements**
- States exactly as defined in `FAILOVER_STATE_MACHINE.md`
- No implicit transitions
- No persistence of transient promotion states

**Exit Criteria**
- State machine compiles
- State transitions enumerated
- Unit tests validate allowed/forbidden transitions

---

### Stage 6.2 — Promotion Request Interface

**Goal**
- Define explicit entry point for promotion attempts

**Requirements**
- Promotion requests are explicit
- No background triggers
- No retries
- Clear rejection on invalid input

**Exit Criteria**
- Promotion request can be issued
- Invalid requests rejected deterministically
- Unit tests for request validation

---

### Stage 6.3 — Promotion Validation Logic

**Goal**
- Implement Promotion Validator

**Requirements**
- Validate all Phase 6 safety invariants
- Read-only interaction with replication, WAL, MVCC
- Deterministic decision-making

**Exit Criteria**
- Validator produces allow/deny decision
- Failure reasons map to invariants
- Unit tests cover all validation branches

---

### Stage 6.4 — Authority Transition Mechanism

**Goal**
- Implement atomic authority rebinding

**Requirements**
- Single-writer authority preserved
- No dual-primary state possible
- Crash-safe transition

**Exit Criteria**
- Authority transition is atomic
- Recovery yields unambiguous authority
- Crash tests pass for transition boundaries

---

### Stage 6.5 — Integration with Replication State (Phase 5)

**Goal**
- Wire promotion outcomes to replication roles

**Requirements**
- Use existing Phase 5 role transitions
- No new replication semantics
- Explicit role rebinding only

**Exit Criteria**
- Promoted replica becomes primary
- Old primary loses authority
- Phase 5 invariants preserved

---

### Stage 6.6 — Observability & Explanation Wiring

**Goal**
- Emit Phase 6 observability signals
- Generate explanation artifacts

**Requirements**
- Events emitted for all promotion states
- Explanations deterministic
- No observability-driven behavior

**Exit Criteria**
- Observability tests pass
- Explanation outputs validated

---

### Stage 6.7 — Crash & Recovery Validation

**Goal**
- Validate Phase 6 under crash conditions

**Requirements**
- Crash injection at all promotion boundaries
- Deterministic recovery
- No ambiguous authority state

**Exit Criteria**
- All crash tests pass
- Recovery behavior audited

---

### Stage 6.8 — End-to-End Promotion Tests

**Goal**
- Validate full promotion lifecycle

**Requirements**
- Successful promotion path
- Rejected promotion path
- Repeated promotion attempts
- Disablement behavior

**Exit Criteria**
- End-to-end tests pass
- No regression in Phases 0–5

---

## 4. Stage Completion Rule

A stage is complete only when:

- Code compiles
- Tests pass
- Invariants are enforced
- No TODOs remain
- Behavior is explainable

Incomplete stages MUST NOT be merged.

---

## 5. Implementation Freeze Rule

Once all stages are complete:

- Phase 6 enters audit
- No new behavior may be added
- Only defect fixes allowed

After audit:
- Phase 6 is frozen

---

## 6. Implementation Order Completeness

This document is complete when:
- Every Phase 6 capability maps to a stage
- Every stage has exit criteria
- No circular dependencies exist

---

END OF DOCUMENT
