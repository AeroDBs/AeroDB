# FAILOVER_TESTING_STRATEGY.md — Failover & Promotion

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
- Frozen Dependencies: **Phases 0–5 Test Suites**

---

## 1. Purpose

This document defines the **mandatory testing requirements** for Phase 6.

Its goals are to:
- Prove correctness of failover & promotion
- Enforce Phase 6 invariants
- Preserve all frozen-phase guarantees
- Eliminate ambiguity under failure and crash

If a behavior is not tested, it is not considered correct.

---

## 2. Testing Principles (Non-Negotiable)

Phase 6 testing MUST obey:

1. **Invariant-first testing**
2. **Crash-before-optimization**
3. **No test weakening**
4. **Deterministic reproduction**
5. **Explicit failure validation**

All Phase 0–5 tests MUST pass **unchanged**.

---

## 3. Test Categories

Phase 6 introduces tests in the following categories.

---

### 3.1 Unit Tests — Promotion Logic

**Scope**
- Promotion Controller
- Promotion Validator
- Authority Transition Manager (logic-level)

**Required Coverage**
- Single-writer enforcement
- WAL prefix validation
- MVCC visibility preservation
- Deterministic decision-making
- Explicit rejection paths

**Examples**
- Promotion denied when replica WAL lags
- Promotion denied when authority ambiguity exists
- Promotion allowed only when all invariants are satisfied

---

### 3.2 State Machine Tests

**Scope**
- Phase 6 state transitions

**Required Coverage**
- All allowed transitions
- All forbidden transitions
- No implicit or skipped states
- Correct reset to `Steady` after completion

**Examples**
- `PromotionRequested → PromotionValidating`
- Rejection paths return to `Steady`
- Crash causes re-entry into `Steady`

---

### 3.3 Integration Tests — Replication Interaction

**Scope**
- Phase 6 + Phase 5 integration

**Required Coverage**
- Promotion of a fully synced replica
- Promotion denial when replication invariants fail
- No Phase 5 state machine corruption

**Constraints**
- Replication behavior MUST remain unchanged
- Promotion logic MUST only read or explicitly transition roles

---

### 3.4 Crash Tests — Promotion Boundaries

**Scope**
- All Phase 6 failure boundaries

**Required Crash Points**
- Before validation
- During validation
- After validation, before authority transition
- During authority transition
- Immediately after authority transition

**Required Outcomes**
- No dual-primary state
- No lost acknowledged writes
- Deterministic recovery
- Explicit abort or completion

Crash tests are **mandatory**, not optional.

---

### 3.5 Recovery Determinism Tests

**Scope**
- Restart behavior after crashes during promotion

**Required Coverage**
- Recovery from every Phase 6 state
- Authority state unambiguous after restart
- Promotion outcome deterministic

Recovery MUST never infer intent.

---

### 3.6 Observability & Explanation Tests

**Scope**
- Event emission
- Metrics stability
- Explanation artifacts

**Required Coverage**
- All promotion attempts emit events
- Failure explanations map to invariants
- Explanation output deterministic

Observability MUST NOT affect behavior.

---

## 4. Negative Testing (Required)

Phase 6 MUST include tests that verify **rejection**:

- Promotion with stale replica
- Promotion under ambiguous authority
- Promotion with missing replication metadata
- Promotion under simulated split-brain conditions

Failure must be explicit and explainable.

---

## 5. Regression Protection

Phase 6 test suite MUST assert:

- No new warnings in Phase 0–5 tests
- No behavior drift in replication
- No WAL format changes
- No MVCC behavior changes

Any regression is a **blocking defect**.

---

## 6. Disablement & Isolation Tests

If Phase 6 logic is disabled or bypassed:
- System MUST behave exactly like Phase 5
- No promotion paths exist
- No hidden state introduced

Disablement behavior MUST be tested.

---

## 7. Determinism Enforcement

All tests MUST:
- Be reproducible
- Avoid timing dependencies
- Avoid randomized ordering
- Avoid flaky assertions

Non-deterministic tests are invalid.

---

## 8. Test Completion Criteria

Phase 6 testing is complete when:

- All new tests pass
- All existing tests pass unchanged
- All invariants are covered by at least one test
- Crash matrices are exhaustive
- No untested failure paths exist

---

## 9. Audit Requirement

Before Phase 6 can be frozen:

- Test coverage must be reviewed
- Crash coverage must be audited
- Invariant mapping must be verified

No audit → no freeze.

---

END OF DOCUMENT
