# CHECKPOINT PIPELINING — PHASE 3

## Status

- Phase: **3**
- Authority: **Normative**
- Scope: **Checkpoint execution performance optimization**
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

This document specifies **Checkpoint Pipelining** as a correctness-preserving
optimization. If any rule herein cannot be proven, this optimization MUST NOT
be implemented.

---

## 1. Purpose

Baseline AeroDB checkpointing is strictly sequential:

1. Snapshot creation
2. Snapshot persistence
3. Snapshot fsync
4. Checkpoint marker write
5. WAL truncation

This is maximally clear but can cause:
- Long pause times
- Poor write throughput during checkpoints

**Checkpoint Pipelining** improves performance by:
- Overlapping *preparatory* work with normal operation
- While preserving **all durability, ordering, and recovery semantics**

Checkpoint Pipelining does **not**:
- Change snapshot semantics
- Change checkpoint semantics
- Change WAL truncation rules
- Introduce speculative persistence

---

## 2. Baseline Reference (Normative)

Baseline checkpoint behavior is defined in:

- `PERFORMANCE_BASELINE.md` §7
- `CRITICAL_PATHS.md` §5

Baseline invariants:

- Snapshot represents a precise MVCC cut
- Snapshot must be fully durable before checkpoint marker
- WAL truncation occurs only after checkpoint durability
- Recovery selects last valid checkpoint deterministically

These invariants MUST remain true.

---

## 3. Definition of Checkpoint Pipelining

### 3.1 Conceptual Definition

Checkpoint Pipelining allows:

> Overlapping *non-authoritative* checkpoint preparation work  
> with normal database operation, while deferring **all authoritative
> durability decisions** to the baseline ordering.

Only work that has **no correctness authority** may be pipelined.

---

### 3.2 Explicit Non-Definition (What This Is NOT)

Checkpoint Pipelining does NOT allow:

- Writing a checkpoint marker early
- Truncating WAL early
- Using incomplete snapshots
- Making snapshots visible before durability
- Time-based checkpoint triggers
- Background speculative cleanup

If any step alters checkpoint authority, the optimization is invalid.

---

## 4. Mechanical Description

### 4.1 Baseline Checkpoint Path (Simplified)

1. Select checkpoint CommitId
2. Freeze snapshot visibility
3. Enumerate persistent state
4. Write snapshot files
5. fsync snapshot
6. Write checkpoint marker
7. fsync marker
8. Truncate WAL

---

### 4.2 Pipelined Checkpoint Path

Checkpoint Pipelining permits the following **restructuring**:

#### Phase A — Preparation (Pipeline-Eligible)

- Snapshot CommitId selection
- Snapshot visibility freeze
- Snapshot enumeration
- Snapshot file writes (not yet authoritative)

These steps:
- Produce *tentative* snapshot artifacts
- Have **no recovery authority**
- May overlap with normal reads and writes

#### Phase B — Authority (Non-Pipelined)

- Snapshot fsync
- Checkpoint marker write
- Checkpoint marker fsync
- WAL truncation

These steps:
- Remain strictly ordered
- Are identical to baseline semantics
- Define durability and recovery authority

---

### 4.3 Pipeline Rules

- Phase A work MUST be restart-discardable
- Phase B work MUST preserve baseline ordering exactly
- No read or write may observe Phase-A artifacts as authoritative
- No recovery logic may consult Phase-A artifacts

---

## 5. Invariant Preservation Matrix

(Referenced from `PERF_INVARIANTS.md`)

### Durability
- D-1 (Acknowledged Write Durability): **Preserved**
- D-2 (Atomic Commit Boundary): **Preserved**
- D-3 (No Silent Downgrade): **Preserved**

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

No invariant is weakened.

---

## 6. Semantic Equivalence Argument

Checkpoint Pipelining is semantically equivalent to baseline because:

- The checkpoint CommitId is identical
- The snapshot contents are identical
- The durability boundary is identical
- The checkpoint marker is written at the same semantic point
- WAL truncation rules are unchanged

Pipelined work only prepares data earlier; it does not change **when**
that data becomes authoritative.

---

## 7. Failure Matrix

### 7.1 Crash During Phase A (Preparation)

- Baseline: no checkpoint
- Pipelined: no authoritative checkpoint

Recovery:
- Discards tentative snapshot artifacts
- Uses previous checkpoint

Equivalent.

---

### 7.2 Crash Between Phase A and Phase B

- No snapshot fsync
- No checkpoint marker

Recovery:
- Tentative snapshot ignored
- WAL replay continues from last checkpoint

Equivalent.

---

### 7.3 Crash During Snapshot fsync (Phase B)

- fsync incomplete → snapshot not durable

Recovery:
- Snapshot rejected
- WAL replay used

Equivalent.

---

### 7.4 Crash After Snapshot fsync, Before Marker fsync

- Snapshot durable
- Marker not durable

Recovery:
- Snapshot not selected
- WAL replay used

Equivalent.

---

### 7.5 Crash After Marker fsync, Before WAL Truncation

- Checkpoint valid
- WAL not yet truncated

Recovery:
- Checkpoint selected
- WAL replay resumes correctly

Equivalent.

---

## 8. Recovery Proof

- Recovery logic remains unchanged
- Recovery selects checkpoints based only on durable markers
- Tentative artifacts are ignored
- Replay behavior is deterministic

No optimization-specific replay logic exists.

---

## 9. Disablement & Rollback

### 9.1 Disablement Mechanism

Checkpoint Pipelining MUST be disableable via:

- Compile-time flag **or**
- Startup configuration

Disablement restores:
- Fully sequential checkpoint behavior

---

### 9.2 Compatibility Proof

- Snapshot format unchanged
- Checkpoint marker format unchanged
- WAL format unchanged

Pipelined artifacts are compatible or discardable.

---

### 9.3 No Ghost State

- Tentative snapshot artifacts are clearly marked or isolated
- No persistent flags indicate “in-progress checkpoint”
- No metadata leaks into recovery logic

---

## 10. Observability

Permitted metrics (passive only):

- checkpoint.pipeline.prepare_duration
- checkpoint.pipeline.authority_duration
- checkpoint.pipeline.aborted_count

Metrics MUST NOT:
- Influence scheduling
- Influence pipeline depth
- Influence checkpoint triggering

---

## 11. Testing Requirements

Checkpoint Pipelining MUST introduce:

- Crash tests at every pipeline boundary
- Recovery equivalence tests
- Disablement equivalence tests
- WAL truncation correctness tests
- Replication compatibility tests

All existing tests MUST pass unmodified.

---

## 12. Explicit Non-Goals

Checkpoint Pipelining does NOT aim to:

- Change checkpoint frequency
- Reduce checkpoint durability guarantees
- Make checkpoints incremental
- Introduce background checkpoints

It improves overlap only.

---

## 13. Final Rule

> A checkpoint is defined by **when it becomes durable**,  
> not by when work starts.

Checkpoint Pipelining is valid only if recovery, replicas,
and clients cannot observe any difference.

---

END OF DOCUMENT
