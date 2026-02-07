# FAILOVER_INVARIANTS.md — Failover & Promotion

## Status
- Phase: **6**
- Authority: **Normative**
- Depends on:
  - FAILOVER_VISION.md
  - FAILOVER_SCOPE.md
- Frozen Dependencies: **Phases 0–5**

---

## 1. Purpose

This document defines the **new invariants introduced by Phase 6**.

These invariants:
- Are **additive only**
- Apply **only** to failover & promotion
- MUST NOT weaken, reinterpret, or bypass any invariant from Phases 0–5

If any Phase 6 behavior violates a Phase 0–5 invariant, the behavior is **forbidden**.

---

## 2. Invariant Hierarchy

Invariant authority order remains unchanged:

1. Phase 0–5 Invariants (Absolute, Frozen)
2. Phase 6 Invariants (Additive, Subordinate)

Phase 6 invariants exist to **constrain promotion**, not to enable availability.

---

## 3. Authority Invariants (Phase 6)

### P6-A1 — Single Write Authority

At any point in time:
- **At most one node** may hold write authority
- Write authority MUST be explicit and observable

No promotion may result in overlapping write authority.

---

### P6-A1a — Force Override Semantics

The `force` flag allows promotion when:
- Operator has external confirmation that primary is unavailable
- Normal detection mechanisms have failed

When `force = true`:
- P6-A1 (single-writer) is NOT bypassed
- Operator EXPLICITLY ASSERTS that primary is down
- System trusts operator assertion over detection failure
- This is NOT an invariant violation but an operator override

Misuse of `force` is an **operator error**, not a system defect.
Audit logs MUST record force flag usage.

---

### P6-A2 — Authority Transfer Is Atomic

Promotion MUST be:
- Atomic with respect to authority
- All-or-nothing

There is no intermediate state where:
- Two nodes are writable
- Or no node is authoritative *after* a successful promotion

---

### P6-A3 — No Implicit Authority Claim

A node MUST NOT:
- Assume primary authority automatically
- Infer authority from liveness
- Infer authority from timeouts
- Infer authority from replica position

Authority is granted only via explicit promotion.

---

## 4. Safety Invariants (Phase 6)

### P6-S1 — No Acknowledged Write Loss

Promotion MUST NOT be allowed if:
- Any acknowledged write could be lost
- Replica state is provably behind committed WAL

This invariant inherits and strengthens:
- Phase-1 durability guarantees
- Phase-5 replication guarantees

---

### P6-S2 — WAL Prefix Rule Preservation

A promoted replica MUST:
- Have a WAL state that is a **prefix-compatible** continuation
- Preserve commit ordering and identity

Promotion MUST NOT introduce:
- WAL divergence
- Reordered commits
- Gaps in committed history

---

### P6-S3 — MVCC Visibility Preservation

After promotion:
- All MVCC visibility rules MUST remain valid
- No read may observe:
  - Partially committed data
  - Reordered versions
  - Phantom commits

Promotion MUST NOT alter snapshot semantics.

---

## 5. Failure Invariants (Phase 6)

### P6-F1 — Fail Closed, Not Open

If promotion safety cannot be proven:
- Promotion MUST be rejected
- System MUST NOT guess
- System MUST NOT degrade into unsafe availability

Explicit failure is preferred over unsafe success.

---

### P6-F2 — Promotion Is Crash-Safe

If a crash occurs:
- Before promotion completes → no authority change
- After promotion completes → new authority is authoritative

There MUST be no ambiguous authority state after recovery.

---

### P6-F3 — No Automatic Retry

Promotion failures:
- MUST NOT be retried automatically
- MUST require explicit re-attempt

This prevents hidden behavior and repeated unsafe attempts.

---

## 6. Determinism Invariants (Phase 6)

### P6-D1 — Deterministic Promotion Outcome

Given identical:
- Replica state
- WAL state
- Promotion request

The promotion decision MUST be identical.

No randomness, timing, or environment influence is allowed.

---

### P6-D2 — Deterministic Recovery Outcome

After crash and recovery:
- Authority state MUST be unambiguous
- Promotion result MUST be identical to pre-crash intent

Recovery MUST NOT “decide” promotion outcomes.

---

## 7. Observability Invariants (Phase 6)

### P6-O1 — Promotion Is Observable

Every promotion attempt MUST emit:
- Start event
- Validation result
- Final decision (success or failure)

Silent promotion is forbidden.

---

### P6-O2 — Promotion Is Explainable

For every promotion decision, the system MUST be able to explain:
- Why promotion was allowed or denied
- Which invariant permitted or blocked it

Explanation is mandatory, not optional.

---

## 8. Forbidden Invariant Violations

Phase 6 MUST NOT:
- Trade safety for availability
- Introduce heuristics
- Introduce timing-based decisions
- Introduce background authority changes
- Override operator intent silently

Any such behavior is a correctness violation.

---

## 9. Invariant Enforcement Rule

All Phase 6 logic MUST:
- Explicitly reference applicable invariants
- Fail if invariants cannot be proven
- Be covered by tests validating invariant preservation

If an invariant cannot be tested, it is incomplete.

---

## 10. Exit Condition

Phase 6 invariants are considered complete when:
- Every promotion path maps to invariants
- Every failure path maps to invariants
- No ambiguity exists in authority transitions
- All Phase 0–5 invariants remain untouched

---

END OF DOCUMENT
