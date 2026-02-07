# FAILOVER_READINESS.md — Failover & Promotion

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
  - FAILOVER_IMPLEMENTATION_ORDER.md
- Frozen Dependencies: **Phases 0–5**

---

## 1. Purpose

This document defines **when Phase 6 is considered complete, auditable, and ready
to be frozen**.

It also defines the **launch readiness bar** for AeroDB after Phase 6.

No implementation may be considered finished without satisfying this document.

---

## 2. Definition of “Ready”

Phase 6 is **ready** when:

- All specified behavior exists
- All forbidden behavior is absent
- All invariants are enforced
- All failures are deterministic
- All behavior is observable and explainable
- All frozen-phase guarantees remain intact

“Mostly correct” is insufficient.

---

## 3. Functional Readiness Criteria

Phase 6 MUST provide:

- Explicit promotion request capability
- Deterministic promotion validation
- Atomic authority transition
- Explicit promotion denial with explanation
- Crash-safe recovery with unambiguous authority
- No automatic or background failover

Every capability must map to:
- A spec section
- A code path
- One or more tests

---

## 4. Invariant Readiness Criteria

All Phase 6 invariants MUST be:

- Enforced in code
- Explicitly referenced in validation logic
- Covered by tests
- Observable on violation

Additionally:
- No Phase 0–5 invariant may be weakened
- No invariant may be conditionally enforced

If any invariant is unverifiable, Phase 6 is **not ready**.

---

## 5. Failure & Crash Readiness Criteria

Phase 6 MUST be correct under:

- Crash at every promotion boundary
- Crash during authority transition
- Crash immediately after promotion success
- Crash with stale or partial observability data

Required properties:
- No dual-primary state
- No loss of acknowledged writes
- No ambiguous authority after recovery

Crash matrices MUST be exhaustive and audited.

---

## 6. Determinism Readiness Criteria

Given identical:
- WAL state
- Replication state
- Promotion request parameters

The system MUST:
- Make the same decision
- Produce the same explanation
- Recover to the same authority state

Any nondeterminism is a blocking defect.

---

## 7. Observability & Explanation Readiness

Phase 6 MUST:

- Emit events for every promotion state transition
- Produce explanations for every allow/deny outcome
- Reference violated or satisfied invariants explicitly
- Preserve deterministic ordering of observability output

Observability gaps are acceptable; authority gaps are not.

---

## 8. Testing & Audit Readiness

Phase 6 is NOT ready unless:

- All Phase 6 tests pass
- All Phase 0–5 tests pass unchanged
- Crash tests cover all promotion boundaries
- Negative tests prove rejection paths
- Disablement behavior is verified

An audit checklist MUST be completed covering:
- Invariant mapping
- Failure coverage
- Recovery determinism
- Forbidden behavior absence

---

## 9. Performance Neutrality Requirement

Phase 6 MUST NOT:

- Introduce overhead on normal steady-state operation
- Add background threads or timers
- Affect write or read paths outside promotion attempts

Promotion is rare; steady state must remain unchanged.

---

## 10. Freeze Criteria

Phase 6 may be frozen only when:

- All readiness criteria are met
- Audit is complete
- No TODOs or speculative hooks remain
- All behavior is explainable to an operator

Once frozen:
- Phase 6 semantics are immutable
- No new behavior may be added
- Only correctness defects may be fixed

---

## 11. Launch Declaration

Upon freezing Phase 6:

> **AeroDB is launch-ready as a correctness-first, replicated database
> with explicit failover and promotion.**

Admin UI, operator tooling, and security enhancements
are explicitly deferred to later phases.

---

## 12. Final Rule

If Phase 6 is not provably correct under failure,
**AeroDB must refuse to promote rather than risk safety**.

Correctness remains the product.

---

END OF DOCUMENT
