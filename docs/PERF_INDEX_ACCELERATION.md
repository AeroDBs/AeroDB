# INDEX ACCELERATION — PHASE 3

## Status

- Phase: **3**
- Authority: **Normative**
- Scope: **Read-path and query-execution acceleration via derived indexes**
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

This document specifies **Index Acceleration** as a Phase-3 optimization.
Indexes remain **derived, non-authoritative, rebuildable**, and correctness-neutral.

If any index influences correctness or visibility, it is invalid.

---

## 1. Purpose

Baseline AeroDB indexes are:

- In-memory
- Derived from persistent state
- Rebuilt on startup
- Advisory to query execution

They are correct but not aggressively optimized.

**Index Acceleration** improves performance by:
- Reducing scan width
- Reducing per-read work
- Improving query selectivity

While strictly preserving:
- MVCC semantics
- Query determinism
- Recovery behavior
- Replication compatibility

---

## 2. Baseline Reference (Normative)

Baseline index behavior is defined in:

- `PERFORMANCE_BASELINE.md` §4, §5
- `CRITICAL_PATHS.md` §3

Baseline properties:

- Indexes are never authoritative
- Indexes may be stale only transiently during rebuild
- Query correctness never depends on index completeness
- Visibility checks are always enforced

These properties MUST remain true.

---

## 3. Definition of Index Acceleration

### 3.1 Conceptual Definition

Index Acceleration is any optimization that:

> Improves the performance of locating **candidate records**  
> while preserving the requirement that **final visibility and correctness
> are determined independently of the index**.

Indexes may only:
- Narrow search space
- Suggest candidates

Indexes may never:
- Decide visibility
- Decide existence
- Decide commit ordering

---

### 3.2 Explicit Non-Definition (What This Is NOT)

Index Acceleration does NOT allow:

- Persistent authoritative indexes
- Index-driven visibility
- Index-only reads
- Skipping MVCC checks
- Skipping document validation
- Query result correctness dependent on index health

If an index can cause a wrong answer, it is invalid.

---

## 4. Allowed Index Acceleration Techniques

Only the following classes are permitted.

---

### 4.1 Improved In-Memory Data Structures

#### Description

Replace baseline index structures with more efficient but equivalent ones:

- Better hash distributions
- Balanced trees
- Cache-friendly layouts

#### Rules

- Key semantics unchanged
- Value semantics unchanged
- Ordering semantics unchanged (if applicable)

#### Proof Obligation

- Index lookup returns a superset of valid candidates
- No valid candidate is omitted

---

### 4.2 Multi-Attribute Derived Indexes

#### Description

Construct composite in-memory indexes derived from:

- Existing document fields
- Existing schema definitions

#### Rules

- Indexes MUST be derivable entirely from stored documents
- Index build order MUST be deterministic
- No additional persistent metadata allowed

#### Proof Obligation

- Query filtering still validates predicates explicitly
- Missing index entries do not affect correctness

---

### 4.3 Predicate Pre-Filtering (Advisory Only)

#### Description

Use indexes to pre-filter candidates before:

- Version-chain traversal
- MVCC visibility checks

#### Rules

- Pre-filtering MUST be conservative
- False positives allowed
- False negatives forbidden

#### Proof Obligation

- All correct results remain reachable
- Index absence yields baseline behavior

---

### 4.4 Read-Only Index Hints (Planner-Visible, Not Binding)

#### Description

Allow query planner to prefer certain indexes when multiple options exist.

#### Rules

- Planner remains deterministic
- Plan selection remains bounded
- Hints do not override correctness checks

#### Proof Obligation

- Different plans produce identical result sets
- Plan choice does not affect semantics

---

## 5. Forbidden Index Optimizations

The following are **explicitly forbidden**:

- Persistent on-disk indexes
- Incremental index persistence
- Index-based visibility shortcuts
- Index-only execution paths
- Lazy index maintenance affecting correctness
- Background index mutation affecting reads
- Heuristic index selection

If an index becomes required for correctness, it violates Phase 3.

---

## 6. Invariant Preservation Matrix

(Referenced from `PERF_INVARIANTS.md`)

### Durability
- D-1, D-2, D-3: **Not Applicable (Derived Only)**

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

Index Acceleration is semantically equivalent to baseline because:

- Indexes only restrict candidate enumeration
- All candidates are validated independently
- MVCC visibility is enforced post-selection
- Missing or incorrect index entries only reduce performance, not correctness

Query results remain identical.

---

## 8. Failure Matrix

### 8.1 Crash During Index Build

- Index state is lost
- Index is rebuilt on restart

Equivalent.

---

### 8.2 Crash During Query Execution

- Baseline: query aborted
- Optimized: query aborted

Equivalent.

---

### 8.3 Index Corruption (Memory)

- Index discarded
- Query falls back to baseline behavior

Equivalent.

---

## 9. Recovery Proof

- Indexes are not persisted
- Recovery ignores index state
- Index rebuild is deterministic

Recovery behavior is unchanged.

---

## 10. Disablement & Rollback

### 10.1 Disablement Mechanism

Index Acceleration MUST be disableable via:

- Compile-time flag **or**
- Startup configuration

Disablement restores:
- Baseline index structures
- Baseline query execution paths

---

### 10.2 Compatibility Proof

- WAL unaffected
- Snapshots unaffected
- Checkpoints unaffected

Index-accelerated data is always readable.

---

### 10.3 No Ghost State

- No persistent index files
- No WAL index metadata
- No snapshot annotations

All index state is transient.

---

## 11. Observability

Permitted metrics (passive only):

- index.lookup.count
- index.candidate.count
- index.filter.false_positives
- index.rebuild.duration

Metrics MUST NOT:
- Influence planner decisions
- Influence index selection
- Influence execution flow

---

## 12. Testing Requirements

Index Acceleration MUST introduce:

- Query equivalence tests
- Index absence tests
- Corrupted index tests
- Disablement equivalence tests
- Crash during index rebuild tests

All existing tests MUST pass unmodified.

---

## 13. Explicit Non-Goals

Index Acceleration does NOT aim to:

- Replace scans with indexes
- Change query semantics
- Persist index state
- Introduce secondary storage structures

Indexes are helpers, not authorities.

---

## 14. Final Rule

> If removing an index changes a query result,
> the index is incorrect.

Index Acceleration exists to make queries faster,
never different.

---

END OF DOCUMENT
