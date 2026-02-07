# PHASE 5 — READINESS & FREEZE

## Status

- Phase: **5**
- Authority: **Normative (Closure Document)**
- Scope: **Replication implementation readiness and freeze**
- Depends on:
  - `OBSERVABILITY_VISION.md`
  - `OBSERVABILITY_INVARIANTS.md`
  - `OBSERVABILITY_IMPLEMENTATION_ORDER.md`
  - `REPLICATION_RUNTIME_ARCHITECTURE.md`
  - `OBSERVABILITY_FAILURE_MATRIX.md`
  - `OBSERVABILITY_TESTING_STRATEGY.md`
  - `OBSERVABILITY_OBSERVABILITY_MAPPING.md`
  - All `REPL_*`, `MVCC_*`, and `CORE_*` documents

This document formally declares whether **Phase 5 is complete, correct, and frozen**.

---

## 1. Purpose

Phase 5 is the point where AeroDB becomes **actually replicated**, not just designed to be.

This document exists to:

- Prevent “almost done” replication
- Prevent silent semantic drift after implementation
- Provide a hard correctness boundary
- Declare replication semantics *implemented and frozen*

No new replication behavior may be introduced after this document is accepted.

---

## 2. Readiness Definition

Phase 5 is considered **READY** only if:

- All replication semantics from Phase 2B are implemented
- No Phase 2B or Phase 3 invariants are weakened
- Phase 4 observability fully covers replication
- All failure modes are tested and explained
- Replication behavior is deterministic and auditable

Readiness is binary.
There is no “mostly ready”.

---

## 3. Mandatory Completion Checklist

All items below MUST be satisfied.

---

### 3.1 Implementation Completeness

- [ ] Primary / Replica role enforcement implemented
- [ ] WAL shipping implemented (prefix rule enforced)
- [ ] WAL validation implemented (checksum + continuity)
- [ ] WAL application implemented (crash-safe)
- [ ] Snapshot bootstrap implemented
- [ ] Replica recovery implemented
- [ ] Replica read-safety gate implemented
- [ ] Replication disablement path verified

---

### 3.2 Invariant Compliance

- [ ] All `OBSERVABILITY_INVARIANTS.md` invariants enforced
- [ ] All `REPL_INVARIANTS.md` invariants enforced
- [ ] No temporary invariant violations exist
- [ ] No heuristic or timing-based logic introduced

---

### 3.3 Failure Coverage

- [ ] All scenarios in `OBSERVABILITY_FAILURE_MATRIX.md` tested
- [ ] Crash injection verified for:
  - WAL receive
  - WAL validation
  - WAL apply
  - Snapshot bootstrap
  - Replica recovery
- [ ] No silent failure paths exist

---

### 3.4 Testing Coverage

- [ ] All tests in `OBSERVABILITY_TESTING_STRATEGY.md` implemented
- [ ] Crash tests deterministic
- [ ] No skipped or ignored tests
- [ ] Phase 0–4 tests unchanged and still passing

---

### 3.5 Observability & Explainability

- [ ] `/v1/replication` fully implemented
- [ ] Replication states observable
- [ ] Blocking reasons explicit
- [ ] `/v1/explain/replication` correct and evidence-based
- [ ] Replica recovery explainable
- [ ] Read-safety decisions explainable

---

### 3.6 Determinism & Replay

- [ ] Replica restart produces identical state
- [ ] WAL replay deterministic
- [ ] Snapshot + WAL continuity verified
- [ ] Observability output deterministic

---

## 4. Explicit Non-Readiness Conditions

Phase 5 MUST be considered **NOT READY** if any of the following are true:

- Replica serves reads without proof
- WAL prefix rule is violated even transiently
- Validation and application overlap incorrectly
- Failure states auto-clear without explanation
- Any replication behavior depends on timing
- Any invariant is “usually” but not always true

If any apply, Phase 5 cannot be frozen.

---

## 5. Freeze Declaration

Once all checklist items are satisfied, the following show be declared:

> **Phase 5 Replication Implementation is COMPLETE and FROZEN.**

After freeze:

- Replication semantics MUST NOT change
- New replication features require Phase 6
- Any bug fix must preserve semantics
- Any deviation requires a new audit

---

## 6. Post-Freeze Guarantees

After Phase 5 freeze:

- AeroDB supports correct, deterministic replication
- Replication behavior is observable and explainable
- Replica reads are safe or explicitly refused
- Failures are loud and diagnosable

Availability is still secondary.
Correctness is absolute.

---

## 7. Forward Boundary

After Phase 5, the next allowed phases are:

- **Phase 6: Failover & Promotion**
- **Phase 6: Security & Hardening**
- **Phase 6: Operational Tooling**

None of these may begin until Phase 5 is frozen.

---

## 8. Final Rule

> Replication that cannot be frozen  
> is replication that cannot be trusted.

This document exists to draw the line.

---

END OF DOCUMENT
