# READ PATH OPTIMIZATION — PHASE 3

## Status

- Phase: **3**
- Authority: **Normative**
- Scope: **Read-only performance optimizations**
- Dependencies:
  - PERF_VISION.md
  - PERF_INVARIANTS.md
  - PERF_PROOF_RULES.md
  - PERFORMANCE_BASELINE.md
  - CRITICAL_PATHS.md
  - SEMANTIC_EQUIVALENCE.md
  - FAILURE_MODEL_PHASE3.md
  - ROLLBACK_AND_DISABLEMENT.md
  - PERFORMANCE_OBSERVABILITY.md

This document specifies **read-path optimizations** that preserve
snapshot isolation, determinism, and replication safety.

Any optimization described here MUST be provably read-only and
MUST NOT alter MVCC visibility, ordering, or failure behavior.

---

## 1. Purpose

Baseline AeroDB read behavior prioritizes correctness:

- Explicit MVCC visibility checks
- Deterministic version-chain traversal
- No speculative caching
- No read shortcuts

This yields correct but potentially expensive reads.

**Read Path Optimization** improves performance by:
- Eliminating redundant work
- Reducing repeated traversal
- Avoiding unnecessary allocations

While strictly preserving:
- Snapshot isolation
- Visibility semantics
- Deterministic behavior

---

## 2. Baseline Reference (Normative)

Baseline read behavior is defined in:

- `PERFORMANCE_BASELINE.md` §4
- `CRITICAL_PATHS.md` §3

Baseline invariants:

- Every read evaluates MVCC visibility explicitly
- Version chains are traversed in CommitId order
- No read observes uncommitted or future data
- Reads do not mutate persistent state

These properties MUST remain true.

---

## 3. Definition of Read Path Optimization

### 3.1 Conceptual Definition

A read-path optimization is any change that:

> Reduces the cost of determining **which committed version is visible**
> to a snapshot, **without changing the result**.

All optimizations are:
- Read-only
- Snapshot-scoped
- Deterministic
- Discardable

---

### 3.2 Explicit Non-Definition (What This Is NOT)

Read Path Optimization does NOT allow:

- Skipping MVCC visibility checks
- Pre-committing visibility
- Read-your-own-writes shortcuts
- Cross-snapshot caching
- Adaptive or speculative behavior
- Time-based cache expiry

If visibility is inferred instead of proven, the optimization is invalid.

---

## 4. Allowed Optimization Classes

Only the following read optimizations are permitted.

---

### 4.1 Snapshot-Local Visibility Caching

#### Description

Within a **single snapshot**, cache the result of:

- Version-chain traversal
- Visibility decision

Keyed by:
- Document identifier
- Snapshot CommitId

#### Rules

- Cache is valid ONLY for one snapshot
- Cache MUST be discarded when snapshot ends
- Cache entries are immutable

#### Proof Obligation

- Cached result equals traversal result
- No snapshot mutation occurs
- No cross-snapshot leakage

---

### 4.2 Deterministic Short-Circuit Traversal

#### Description

If version chains are ordered by CommitId:

- Stop traversal once first visible version is found

(This matches baseline semantics but avoids unnecessary work.)

#### Rules

- Order MUST be explicit
- Visibility MUST be checked
- No speculative skipping

#### Proof Obligation

- Later versions cannot be visible
- Earlier versions are superseded

---

### 4.3 Zero-Copy Read Materialization

#### Description

Avoid copying document data when:

- Data is immutable
- Lifetime exceeds read duration
- Snapshot guarantees immutability

#### Rules

- No mutable aliasing
- No shared ownership beyond snapshot
- Memory safety must be explicit

#### Proof Obligation

- Data cannot change
- Snapshot lifetime bounds usage

---

### 4.4 Index-Assisted Visibility Filtering (Read-Only)

#### Description

Use existing in-memory indexes to:

- Narrow candidate version sets
- Without skipping visibility checks

#### Rules

- Indexes are advisory only
- Visibility check is mandatory
- Index absence must not affect correctness

---

## 5. Forbidden Read Optimizations

The following are **explicitly forbidden**:

- Skipping visibility checks
- Caching across snapshots
- Global read caches
- Heuristic pruning
- Time-based invalidation
- Read-write coupling
- Replica-only visibility shortcuts
- Opportunistic consistency

If correctness depends on cache freshness, the optimization is invalid.

---

## 6. Invariant Preservation Matrix

(Referenced from `PERF_INVARIANTS.md`)

### Durability
- D-1, D-2, D-3: **Not Applicable (Read-Only)**

### Determinism
- DET-1 (Crash Determinism): **Preserved**
- DET-2 (Replay Equivalence): **Preserved**
- DET-3 (Bounded Execution): **Preserved**

### MVCC
- MVCC-1 (Snapshot Isolation): **Preserved**
- MVCC-2 (CommitId Authority): **Preserved**
- MVCC-3 (Version Chain Integrity): **Preserved**

### Replication
- REP-1, REP-2, REP-3: **Preserved**

### Failure & Recovery
- FR-1, FR-2, FR-3: **Preserved**

### Observability
- OBS-1, OBS-2: **Preserved**

### Disablement
- DIS-1, DIS-2, DIS-3: **Preserved**

---

## 7. Semantic Equivalence Argument

Read Path Optimization is semantically equivalent to baseline because:

- For any snapshot and document:
  - The same version is selected
  - The same data is returned
- Visibility rules are evaluated identically
- No persistent state is modified
- No ordering is changed

Optimizations only reduce *how much work is done*, not *what is decided*.

---

## 8. Failure Matrix

### 8.1 Crash During Read

- Baseline: read aborted, no state change
- Optimized: read aborted, no state change

Equivalent.

---

### 8.2 Crash After Cache Population

- Cache is in-memory only
- Cache is discarded on restart

Equivalent.

---

### 8.3 Memory Allocation Failure

- Baseline: read fails
- Optimized: read fails

Equivalent.

---

## 9. Recovery Proof

- Read optimizations do not affect WAL
- No persistent read state exists
- Recovery logic is unchanged

Replay behavior is identical.

---

## 10. Disablement & Rollback

### 10.1 Disablement Mechanism

Read optimizations MUST be disableable via:

- Compile-time flag **or**
- Startup configuration

Disablement restores:
- Baseline traversal
- Baseline allocation behavior

---

### 10.2 Compatibility Proof

- No WAL changes
- No snapshot changes
- No checkpoint changes

Data is always readable.

---

### 10.3 No Ghost State

- No persistent caches
- No hidden metadata
- No optimization markers

All state is in-memory and snapshot-scoped.

---

## 11. Observability

Permitted metrics (passive only):

- read_path.version_chain_traversals
- read_path.cache_hits
- read_path.cache_misses
- read_path.materialization_copies

Metrics MUST NOT:
- Influence caching
- Influence traversal
- Influence allocation

---

## 12. Testing Requirements

Read Path Optimization MUST introduce:

- Snapshot equivalence tests
- Cache correctness tests
- Cross-snapshot isolation tests
- Crash-during-read tests
- Disablement equivalence tests

All existing tests MUST pass unmodified.

---

## 13. Explicit Non-Goals

Read Path Optimization does NOT aim to:

- Change isolation levels
- Improve write performance
- Introduce speculative reads
- Relax MVCC rules

It optimizes **execution cost only**.

---

## 14. Final Rule

> A faster read that returns the wrong version  
> is worse than a slow read.

Read-path optimizations are valid **only**
when they are invisible to all observers.

---

END OF DOCUMENT
