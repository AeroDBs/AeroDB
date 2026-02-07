# REPLICA READ FAST PATH — PHASE 3

## Status

- Phase: **3**
- Authority: **Normative**
- Scope: **Replica-side read-only performance optimization**
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

This document specifies **Replica Read Fast Path** as a correctness-preserving,
read-only optimization.  
If any rule in this document cannot be proven, the optimization MUST NOT be implemented.

---

## 1. Purpose

Baseline replica behavior is conservative:

- Replica consumes WAL strictly as a prefix
- Replica serves reads only when MVCC-safe
- Replica performs the same read logic as primary

This guarantees correctness but may result in:
- Higher read latency
- Redundant MVCC checks
- Repeated visibility computation

**Replica Read Fast Path** improves performance by:
- Avoiding redundant work for already-proven-safe snapshots
- Reducing read-path overhead on replicas only

This optimization does **not**:
- Change replication semantics
- Change visibility rules
- Allow speculative or stale reads
- Introduce replica-only semantics

---

## 2. Baseline Reference (Normative)

Baseline replica read behavior is defined in:

- `PERFORMANCE_BASELINE.md` §9
- `CRITICAL_PATHS.md` §7
- Replication design documents (Phase 2B, frozen)

Baseline invariants:

- Replica WAL is a strict prefix of primary WAL
- Replica assigns no CommitIds
- Replica visibility is MVCC-governed
- Replica may serve reads only for MVCC-safe snapshots
- Replica state is always derivable from WAL + snapshots

These invariants MUST remain true.

---

## 3. Definition of Replica Read Fast Path

### 3.1 Conceptual Definition

Replica Read Fast Path allows:

> A replica to perform reads using **pre-validated snapshot visibility**
> when it is provably impossible for additional WAL to affect the result.

The optimization applies **only** when:
- Snapshot CommitId is ≤ replica’s durable WAL CommitId
- Replica is not lagging behind that snapshot
- No visibility ambiguity exists

---

### 3.2 Explicit Non-Definition (What This Is NOT)

Replica Read Fast Path does NOT allow:

- Reading ahead of WAL durability
- Reading unreplicated commits
- Divergent visibility rules
- Replica-local CommitId inference
- Stale reads masked as fresh
- Heuristic lag thresholds
- Time-based safety assumptions

If safety depends on “probably caught up”, the optimization is invalid.

---

## 4. Safety Preconditions

Replica Read Fast Path MAY be used only if **all** of the following hold:

1. Replica has durably applied WAL up to CommitId **R**
2. Requested snapshot CommitId **S ≤ R**
3. Snapshot **S** is immutable
4. No WAL gaps exist
5. Replica is not mid-recovery
6. Replica is not mid-snapshot-bootstrap

If any precondition fails, replica MUST fall back to baseline read behavior.

---

## 5. Mechanical Description

### 5.1 Baseline Replica Read Path

For each read:

1. Validate replica is MVCC-safe
2. Acquire snapshot
3. Traverse version chain
4. Apply visibility rules
5. Materialize document
6. Return result

---

### 5.2 Replica Read Fast Path

Under Replica Read Fast Path:

1. Validate safety preconditions
2. Reuse pre-validated snapshot visibility boundary
3. Traverse version chain
4. Materialize document
5. Return result

Differences:
- No re-validation of WAL prefix per read
- No re-computation of snapshot safety
- No speculative shortcuts

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
- REP-1 (Single Writer): **Preserved**
- REP-2 (WAL Prefix Rule): **Preserved**
- REP-3 (Replica Observational Equivalence): **Preserved**

### Failure & Recovery
- FR-1, FR-2, FR-3: **Preserved**

### Observability
- OBS-1, OBS-2: **Preserved**

### Disablement
- DIS-1, DIS-2, DIS-3: **Preserved**

---

## 7. Semantic Equivalence Argument

Replica Read Fast Path is semantically equivalent to baseline because:

- Reads are served only for snapshots already fully replicated
- No read observes data not present in replica WAL
- Version-chain traversal yields identical results
- Visibility rules are unchanged
- Primary and replica results match

The optimization removes redundant checks, not safety checks.

---

## 8. Failure Matrix

### 8.1 Replica Crash During Read

- Baseline: read aborted
- Optimized: read aborted

Equivalent.

---

### 8.2 Replica Crash After Safety Validation

- Validation state is in-memory
- State is lost on restart

Recovery:
- Safety is re-evaluated
- Baseline behavior resumes

Equivalent.

---

### 8.3 WAL Arrival During Read

- Baseline: WAL arrival handled normally
- Optimized: WAL arrival does not affect snapshot ≤ R

Equivalent.

---

### 8.4 Replica Lag Increase

- Safety precondition fails
- Fast path is disabled
- Baseline read path used

Equivalent.

---

## 9. Recovery Proof

- Replica recovery logic is unchanged
- WAL replay is unchanged
- Snapshot selection is unchanged
- No optimization state is persisted

Recovery behavior is identical.

---

## 10. Disablement & Rollback

### 10.1 Disablement Mechanism

Replica Read Fast Path MUST be disableable via:

- Compile-time flag **or**
- Startup configuration

Disablement restores:
- Baseline replica read behavior

---

### 10.2 Compatibility Proof

- WAL format unchanged
- Snapshot format unchanged
- Checkpoint format unchanged

Replica data is always readable.

---

### 10.3 No Ghost State

- No persistent replica-only metadata
- No WAL flags
- No snapshot annotations

All fast-path state is in-memory and discardable.

---

## 11. Observability

Permitted metrics (passive only):

- replica_read.fast_path_hits
- replica_read.fast_path_misses
- replica_read.safe_snapshot_commit_id
- replica_read.fallback_count

Metrics MUST NOT:
- Influence read selection
- Influence safety decisions
- Influence replication behavior

---

## 12. Testing Requirements

Replica Read Fast Path MUST introduce:

- Replica vs primary read equivalence tests
- Lag boundary tests
- Crash-during-read tests
- Disablement equivalence tests
- WAL gap tests
- Snapshot bootstrap interaction tests

All existing tests MUST pass unmodified.

---

## 13. Explicit Non-Goals

Replica Read Fast Path does NOT aim to:

- Make replicas eventually consistent
- Hide replication lag
- Provide weaker consistency reads
- Introduce read-your-own-writes on replicas

Replica correctness is not negotiable.

---

## 14. Final Rule

> A replica may be faster than the primary,  
> but it must never be **less correct**.

Replica Read Fast Path is valid only if it is impossible
to observe any divergence from baseline semantics.

---

END OF DOCUMENT
