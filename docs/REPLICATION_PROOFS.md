# REPLICATION_PROOFS.md

## AeroDB Phase 2 — MVCC Correctness Proofs

### Status

* This document is **authoritative**
* It provides argument-based proofs of MVCC correctness
* Proofs are invariant-driven, not implementation-dependent
* No algorithms or data structures appear here

---

## 1. Proof Methodology

All proofs in this document are based on:

* Explicit invariants (Phase-1 + Phase-2)
* Deterministic WAL semantics
* Exhaustive failure enumeration
* Absence of heuristics or timing assumptions

Each proof answers one question:

> “Why is this behavior *always* correct, regardless of execution order or crash timing?”

---

## 2. Proof of Visibility Correctness

### Claim

For any read operation, the set of visible versions is correct and deterministic.

### Argument

1. Visibility is defined solely by:

   * Version commit identity
   * Read view upper bound
2. Commit identities are:

   * Totally ordered
   * WAL-governed
   * Deterministically replayed
3. A read view is immutable once established
4. Visibility rule selects:

   * The maximum commit identity ≤ read upper bound

Therefore:

* Visibility does not depend on timing
* Visibility does not depend on concurrency
* Visibility does not depend on execution order

**Conclusion:**
Visibility is deterministic and correct by construction.

---

## 3. Proof of Snapshot Isolation

### Claim

AeroDB MVCC provides deterministic snapshot isolation.

### Argument

1. A read view fixes a commit identity boundary
2. No versions beyond this boundary are visible
3. All reads within an operation use the same boundary
4. Writes only become visible after commit identity durability

Therefore:

* No dirty reads are possible
* No non-repeatable reads are possible
* No phantom visibility changes are possible

**Conclusion:**
Snapshot isolation is guaranteed without locks or heuristics.

---

## 4. Proof of Write Atomicity

### Claim

Writes are atomically visible or invisible, never partial.

### Argument

1. Visibility is tied to commit identity durability
2. Commit identity is WAL-persisted as a single authority
3. Versions without a durable commit identity are invisible
4. Versions with a durable commit identity must be complete or recovery fails

Therefore:

* Partial visibility cannot occur
* Atomicity is enforced at the visibility boundary

**Conclusion:**
Write atomicity is guaranteed across crashes.

---

## 5. Proof of Recovery Determinism

### Claim

Recovery reconstructs the exact MVCC state deterministically.

### Argument

1. WAL is replayed in strict order
2. Commit identities are derived from WAL order
3. Version creation is replayed deterministically
4. GC and snapshot events are WAL-represented

Therefore:

* Recovery outcome is uniquely determined by WAL
* No ambiguity exists after replay

**Conclusion:**
Recovery is deterministic and reproducible.

---

## 6. Proof of Crash Safety

### Claim

All crash points resolve to valid MVCC states.

### Argument

1. All failure points are enumerated in `MVCC_FAILURE_MATRIX.md`
2. Each crash point maps to:

   * Commit exists fully, or
   * Commit does not exist at all
3. No intermediate visibility state is allowed

Therefore:

* Crashes cannot introduce undefined states
* Corruption is detected, not repaired

**Conclusion:**
Crash safety is complete and explicit.

---

## 7. Proof of Snapshot Correctness

### Claim

Snapshots represent valid, self-contained MVCC states.

### Argument

1. Snapshots capture:

   * All versions ≤ snapshot boundary
   * All MVCC metadata
2. WAL replay resumes strictly after snapshot boundary
3. Snapshots impose retention constraints on GC

Therefore:

* Snapshots can serve reads independently
* Snapshots preserve visibility semantics

**Conclusion:**
Snapshots are correct MVCC cuts.

---

## 8. Proof of Garbage Collection Safety

### Claim

GC never removes a version that could be visible.

### Argument

1. GC eligibility requires:

   * Version commit identity < visibility lower bound
   * No snapshot retention
   * Recovery safety
2. Visibility lower bound is explicit and deterministic
3. GC actions are WAL-recorded

Therefore:

* No visible or potentially visible version is reclaimed
* GC is replay-safe

**Conclusion:**
GC is safe by proof, not probability.

---

## 9. Proof of Phase-1 Compatibility

### Claim

MVCC does not alter Phase-1 behavior.

### Argument

1. Phase-1 behavior is equivalent to:

   * Single-version chains
   * Read views always at latest commit
2. MVCC generalizes, never replaces, Phase-1 semantics
3. All Phase-1 invariants remain unchanged

Therefore:

* Identical inputs produce identical outputs
* Phase-1 tests remain valid

**Conclusion:**
MVCC is a strict superset of Phase-1 behavior.

---

## 10. Proof of Replication Readiness

### Claim

MVCC semantics are replication-safe.

### Argument

1. All MVCC state is:

   * WAL-represented
   * Deterministic
2. Commit identities define a total order
3. Visibility rules are stateless beyond WAL

Therefore:

* MVCC state can be reconstructed on another node
* No local-only behavior exists

**Conclusion:**
Replication can consume MVCC without redefining it.

---

## 11. Global Correctness Theorem

### Statement

> Given the Phase-1 invariants, the Phase-2 MVCC invariants, and the WAL as the sole authority, AeroDB MVCC guarantees:
>
> * Deterministic visibility
> * Atomic writes
> * Snapshot isolation
> * Crash-safe recovery
> * Safe garbage collection
> * Phase-1 behavioral equivalence

Under all execution orders and crash scenarios.

---

## 12. End of MVCC Design Phase

At this point:

* MVCC design is **complete**
* All invariants are specified
* All failure modes are defined
* No implementation assumptions remain

