# PHASE 5 — TESTING STRATEGY (REPLICATION IMPLEMENTATION)

## Status

- Phase: **5**
- Authority: **Normative**
- Scope: **All replication-related tests**
- Depends on:
  - `OBSERVABILITY_VISION.md`
  - `OBSERVABILITY_INVARIANTS.md`
  - `OBSERVABILITY_IMPLEMENTATION_ORDER.md`
  - `REPLICATION_RUNTIME_ARCHITECTURE.md`
  - `OBSERVABILITY_FAILURE_MATRIX.md`
  - `REPL_*` specifications
  - `MVCC_*` specifications
  - `CORE_INVARIANTS.md`
  - `CRASH_TESTING.md`

This document defines **how replication correctness is proven**.

If a behavior is not tested according to this strategy,
it is not considered correct.

---

## 1. Purpose

Replication bugs are rarely logical.
They are almost always:
- Ordering bugs
- Crash-window bugs
- Partial-state bugs
- Recovery bugs

This testing strategy exists to ensure that:
- Every replication invariant is enforced
- Every failure mode is exercised
- Every outcome is deterministic and explainable

Tests are **first-class correctness artifacts**.

---

## 2. Testing Philosophy

### T-1: Tests Are Proof, Not Examples

Replication tests MUST:
- Prove invariants
- Prove crash safety
- Prove determinism

“Happy path only” tests are insufficient.

---

### T-2: Crash Testing Is Mandatory

Any replication logic that:
- Writes state
- Applies WAL
- Advances CommitId

MUST be tested under crash injection.

---

### T-3: Determinism Is Enforced

Given identical inputs and crash points:
- Final state MUST be identical
- Explanations MUST be identical

Non-deterministic outcomes are failures.

---

## 3. Test Layers (Mandatory)

Replication tests are organized into **five layers**.
All layers MUST exist.

---

## 4. Layer 1 — Unit Tests (Pure Logic)

### Scope
- State machines
- Read-safety predicates
- WAL ordering logic
- Validation rules

### Requirements
- No IO
- No threads
- No timing assumptions

### Examples
- WAL prefix validation
- CommitId monotonicity checks
- Read-safety predicate evaluation

---

## 5. Layer 2 — Component Tests (Isolated IO)

### Scope
- WAL Receiver
- WAL Validator
- WAL Applier
- Replica State Store

### Requirements
- Single component under test
- Explicit inputs
- Explicit outputs

### Mandatory Scenarios
- Invalid WAL rejection
- Partial WAL persistence
- Crash before/after fsync
- Idempotent WAL replay

---

## 6. Layer 3 — Integration Tests (Primary ↔ Replica)

### Scope
- End-to-end replication flow
- WAL shipping
- Snapshot bootstrap
- Replica recovery

### Mandatory Scenarios
- Fresh replica bootstrap
- Replica catch-up from WAL
- Primary crash + replica recovery
- Replica crash mid-apply

### Requirements
- Deterministic setup
- Explicit sequencing
- No background retries

---

## 7. Layer 4 — Crash Matrix Tests (Critical)

### Scope
- All crash points defined in `OBSERVABILITY_FAILURE_MATRIX.md`

### Mandatory Crash Points
- Before WAL persist
- After WAL persist
- During WAL validation
- During WAL application
- During snapshot transfer
- During replica recovery

### Requirements
For each crash point:
1. Inject crash
2. Restart system
3. Verify invariants
4. Verify explanation output

Skipping a crash point is forbidden.

---

## 8. Layer 5 — Explanation Verification Tests

### Scope
- `/v1/explain/replication`
- `/v1/explain/recovery`
- Read-safety explanations

### Requirements
- Explanations must:
  - Reference real WAL offsets
  - Reference real CommitIds
  - Reference invariant IDs
- No heuristic language
- No missing evidence

Tests MUST fail if explanations are incomplete or misleading.

---

## 9. Required Test Matrix (Authoritative)

Every test MUST map to at least one invariant.

| Invariant | Test Required |
|---------|---------------|
| P5-I4 (WAL prefix) | ✔ |
| P5-I5 (Validate before apply) | ✔ |
| P5-I6 (CommitId authority) | ✔ |
| P5-I8 (Snapshot cut) | ✔ |
| P5-I9 (Snapshot/WAL continuity) | ✔ |
| P5-I11 (Crash safety) | ✔ |
| P5-I12 (Read safety) | ✔ |
| P5-I14 (Observability) | ✔ |

No invariant may remain untested.

---

## 10. Forbidden Testing Patterns

Explicitly forbidden:

- Sleeping to “wait” for replication
- Time-based assertions
- Randomized fuzz without determinism
- Ignoring failed tests “temporarily”
- Disabling crash tests for speed

If a test is flaky, the implementation is wrong.

---

## 11. Test Execution Rules

- All replication tests MUST run in CI
- Crash tests MUST run deterministically
- Tests MUST pass with replication disabled
- Phase 0–4 tests MUST remain unchanged

Replication MUST NOT weaken existing guarantees.

---

## 12. Completion Criteria (Testing)

Phase 5 testing is complete only when:

- All test layers exist
- All failure scenarios are covered
- All explanations are validated
- Test suite is deterministic
- No tests are ignored or skipped

Only then may replication be considered implemented.

---

## 13. Final Rule

> If replication correctness is not testable,
> it is not correct.

Tests are not optional.
They are the proof.

---

END OF DOCUMENT
