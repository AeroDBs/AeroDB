# PHASE 3 — PROOF RULES

## Status

- Phase: **3**
- Authority: **Normative**
- Applies to: **All Phase 3 optimization specifications and implementations**
- Dependency:
  - PERF_VISION.md
  - PERF_INVARIANTS.md

No Phase 3 code may be written, merged, or enabled without satisfying the proof rules in this document.

---

## 1. Purpose

Phase 3 introduces performance optimizations without changing semantics.

Because semantics are frozen, **proof precedes code**.

This document defines:
- What constitutes a valid proof
- What must be proven
- How equivalence is established
- When proof is considered insufficient
- When an optimization must be rejected

Proofs are not informal reasoning; they are **structured, explicit arguments** tied to invariants.

---

## 2. Definition of a “Proof” in AeroDB

A Phase 3 proof is a **structured, checkable argument** that demonstrates:

> Optimized execution is observationally equivalent to baseline execution under all allowed failures.

A proof MUST:
- Be written in the optimization’s markdown specification
- Reference invariants explicitly
- Enumerate failure cases
- Define equivalence criteria
- Be independent of implementation details where possible

If behavior cannot be proven equivalent, the optimization is invalid.

---

## 3. Proof Scope

Every proof MUST cover all of the following domains:

1. Durability
2. Determinism
3. MVCC Visibility
4. Replication Safety (if applicable)
5. Failure & Recovery
6. Disablement & Rollback

Omission of any domain constitutes proof failure.

---

## 4. Mandatory Proof Structure

Every Phase 3 optimization document MUST contain the following sections,
in the order listed.

---

### 4.1 Baseline Semantics

This section MUST describe:

- Current (pre-optimization) behavior
- Precise ordering of operations
- Existing durability boundaries
- WAL emission behavior
- MVCC visibility checkpoints

Rules:
- No hand-waving
- No “as implemented” shortcuts
- If baseline behavior is unclear, STOP and specify it first

---

### 4.2 Optimization Description

This section MUST describe:

- Exact mechanical change
- What is removed, reordered, or combined
- What remains unchanged
- What new structure (if any) is introduced

Rules:
- No pseudocode shortcuts
- No implicit behavior
- No future work placeholders

If the optimization cannot be described mechanically, it is invalid.

---

### 4.3 Invariant Impact Matrix

This section MUST list **every invariant** from `PERF_INVARIANTS.md`
and classify it as:

- Preserved without interaction
- Preserved with proof (reference section)
- Not applicable (with justification)

Rules:
- Each invariant MUST be mentioned explicitly
- Silence implies violation

---

### 4.4 Equivalence Argument

This section MUST prove that optimized execution is equivalent to baseline execution.

Equivalence MUST be argued in terms of:
- Observable outputs
- Persisted state
- Recovery results

At minimum, the argument MUST address:

- WAL contents equivalence
- Commit ordering equivalence
- Visibility equivalence
- Error surface equivalence

Rules:
- Performance differences are irrelevant
- Internal timing is irrelevant
- Only observable behavior matters

---

### 4.5 Failure Matrix

This section MUST enumerate behavior under failures at every boundary.

Minimum required failures:
- Process crash
- Power loss
- Partial WAL write
- Partial fsync
- Disk error
- Replica disconnect (if applicable)

For each failure:
- Describe baseline outcome
- Describe optimized outcome
- Prove equivalence

Rules:
- “Same as baseline” is not sufficient without explanation
- Failure ordering must be explicit

---

### 4.6 Recovery Proof

This section MUST prove:

- WAL replay remains deterministic
- Replay is idempotent
- No optimization-specific side effects occur during replay

Rules:
- Replay code MUST NOT branch on optimization presence
- If replay behavior differs, optimization is invalid

---

### 4.7 Disablement & Rollback Proof

This section MUST prove:

- Optimization can be disabled safely
- Data written with optimization enabled is readable without it
- No persistent ghost state exists

Rules:
- Disablement MUST NOT require data migration
- WAL format compatibility is mandatory

---

### 4.8 Residual Risk Analysis

This section MUST list:
- Known non-risks (explicitly ruled out)
- Any remaining assumptions (must be structural, not heuristic)

Rules:
- “Low probability” is not an argument
- Hardware reliability assumptions must match Phase 1

---

## 5. Proof Style Rules

Proofs MUST be:

- Explicit
- Deterministic
- Mechanically checkable
- Free of probabilistic language

The following phrases are forbidden in proofs:
- “Usually”
- “In practice”
- “Should be safe”
- “Expected to”
- “Assumed”
- “Likely”

If a proof relies on timing, load, or fairness, it is invalid.

---

## 6. What Is NOT a Proof

The following do NOT constitute proof:

- Benchmarks
- Microbenchmarks
- Performance graphs
- Empirical testing alone
- Fuzzing without equivalence arguments
- “Works on restart” claims

Testing supports proofs; it does not replace them.

---

## 7. Proof Review Rules

Before an optimization may be accepted:

- Proof must be reviewed independently
- All invariants must be checked off
- Failure matrix must be complete
- Equivalence argument must be explicit

If reviewers disagree:
- The optimization is rejected
- Ambiguity favors correctness

---

## 8. Proof vs Implementation Ordering

The required order is:

1. Baseline specification
2. Optimization specification
3. Proof completion
4. Proof review acceptance
5. Implementation
6. Regression testing

Deviation from this order is forbidden.

---

## 9. Handling Insufficient Proof

If proof is insufficient:

- The optimization is not “parked”
- The optimization is not “experimental”
- The optimization is rejected

Phase 3 does not allow speculative optimizations.

---

## 10. Final Rule

> In AeroDB, performance is optional.  
> Correctness is mandatory.  
> Proof is the price of optimization.

If proof cannot be completed, optimization does not proceed.

---

END OF DOCUMENT
