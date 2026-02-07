# PHASE 3 — PERFORMANCE (CORRECTNESS-PRESERVING OPTIMIZATIONS)

## Status

- Phase: **3**
- State: **Design-first, proof-gated**
- Prerequisites:
  - Phase 1: Core Storage & Correctness — **Frozen**
  - Phase 2A: MVCC — **Frozen**
  - Phase 2B: Replication Semantics — **Frozen**

This document is **authoritative** for Phase 3 intent and scope.

---

## 1. Purpose of Phase 3

Phase 3 exists to **improve performance without altering semantics**.

This phase introduces **mechanical optimizations only**, under strict rules:
- No behavior changes
- No semantic relaxation
- No heuristic shortcuts
- No timing assumptions

Every optimization must be:
1. Explicitly specified
2. Correctness-proven
3. Opt-in (compile-time or config-gated)
4. Independently testable
5. Reversible without data impact

If an optimization cannot be proven correct, it is **forbidden**.

---

## 2. Definition of “Correctness-Preserving Performance”

An optimization is correctness-preserving if and only if:

- All externally observable behavior is **bit-for-bit equivalent**
- All failure modes remain **detectable and explicit**
- All crash-recovery outcomes remain **deterministic**
- All MVCC visibility rules remain **unchanged**
- All replication invariants remain **unchanged**
- All acknowledged writes maintain **identical durability guarantees**

Performance improvements must not introduce:
- New states
- New timing dependencies
- New partial-commit behavior
- New implicit ordering rules

---

## 3. Absolute Non-Goals

Phase 3 explicitly does **NOT** include:

- New features
- New APIs
- New consistency levels
- New isolation semantics
- New replication modes
- New storage formats
- New heuristics
- Adaptive or ML-based behavior
- Background threads that influence correctness

If a change would be marketed as a “feature”, it does not belong in Phase 3.

---

## 4. Frozen Foundations (Non-Negotiable)

Phase 3 MUST treat the following as immutable law:

### 4.1 Phase 1 Guarantees
- No acknowledged write is ever lost
- Corruption is detected, never repaired silently
- Recovery is deterministic
- Queries are bounded and deterministic
- Snapshots are read-only and manifest-driven
- Checkpoints bound WAL growth
- Observability never affects behavior

### 4.2 MVCC Guarantees
- CommitId authority is WAL-governed
- Snapshot isolation semantics are fixed
- Version chains are immutable
- Visibility rules are deterministic
- Garbage collection is WAL-governed
- Crash behavior is exhaustively defined

### 4.3 Replication Guarantees
- Single-writer invariant
- CommitId authority exists only on Primary
- WAL prefix rule is inviolable
- Replica state is always derivable
- No silent divergence
- Deterministic restart semantics

No optimization may reinterpret, soften, or bypass these guarantees.

---

## 5. Allowed Optimization Classes

Only the following classes of optimizations are allowed in Phase 3:

### 5.1 Mechanical Reordering (Proven Equivalent)
Examples:
- Grouping independent fsync calls
- Combining buffer flushes

Rules:
- Logical order must remain identical
- Failure boundaries must be preserved
- Acknowledgment semantics must be unchanged

---

### 5.2 Redundant Work Elimination
Examples:
- Avoiding duplicate checksum computation
- Memoizing schema validation results

Rules:
- Inputs must be provably identical
- Cached results must be immutable
- Cache invalidation must be explicit and total

---

### 5.3 Read-Only Fast Paths
Examples:
- Zero-copy reads
- Snapshot-local caching

Rules:
- No writes
- No visibility shortcuts
- MVCC rules must be explicitly enforced

---

### 5.4 Deterministic Batching
Examples:
- WAL record batching
- Commit group formation

Rules:
- No reordering across CommitId boundaries
- No partial acknowledgment
- Crash behavior must match unbatched execution

---

### 5.5 Memory Layout Optimization
Examples:
- Cache-line alignment
- Structure packing

Rules:
- No semantic coupling to layout
- No reliance on undefined behavior
- No platform-specific correctness assumptions

---

## 6. Forbidden Optimization Patterns

The following are **explicitly forbidden**:

- Lazy durability
- Async acknowledgment
- Background retries
- Time-based flushing
- Adaptive thresholds
- Best-effort behavior
- Silent fallback paths
- “Usually safe” logic
- Hardware-specific correctness dependencies

If an optimization requires a disclaimer, it is not allowed.

---

## 7. Proof Requirements

Every Phase 3 optimization document MUST include:

1. **Baseline Semantics Section**
   - What the system does today
   - Why it is correct

2. **Optimization Description**
   - Exact mechanical change
   - No implementation shortcuts

3. **Invariant Preservation Proof**
   - Durability
   - Determinism
   - MVCC visibility
   - Replication safety

4. **Failure Matrix**
   - Power loss
   - Process crash
   - Partial I/O
   - Disk error
   - Replica disconnect (if applicable)

5. **Equivalence Argument**
   - Why optimized execution is observationally identical

6. **Rollback Plan**
   - How optimization can be disabled
   - Proof that disabling does not affect stored data

No code may be written without an accepted proof.

---

## 8. Observability Rules in Phase 3

Performance instrumentation:
- May measure
- May record
- May expose metrics

It may **never**:
- Influence control flow
- Influence scheduling
- Influence batching decisions
- Influence retry behavior

Observability remains strictly passive.

---

## 9. Testing Requirements

Each optimization must introduce:

- Deterministic unit tests
- Crash-recovery equivalence tests
- Phase 1 regression tests
- MVCC regression tests
- Replication regression tests (if applicable)

All existing tests MUST pass unmodified.

If a test must change, the optimization is invalid.

---

## 10. Phase 3 Exit Criteria

Phase 3 is complete when:

- All selected optimizations are proven and implemented
- No correctness regressions exist
- No semantics are altered
- All optimizations are optional
- Documentation is complete and authoritative

Performance gains are secondary to proof quality.

---

## 11. Guiding Principle

> AeroDB would rather be **slow and correct** than **fast and ambiguous**.

Phase 3 exists to make AeroDB faster **only where correctness is untouched**.

If there is doubt, we do not optimize.

---

END OF DOCUMENT
