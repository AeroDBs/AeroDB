# GROUP COMMIT — PHASE 3

## Status

- Phase: **3**
- Authority: **Normative**
- Scope: **Write-path performance optimization**
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

This document specifies **Group Commit** as a correctness-preserving optimization.
If any requirement in this document cannot be satisfied, Group Commit MUST NOT be implemented.

---

## 1. Purpose

Baseline AeroDB performs:

- One fsync per acknowledged commit

This yields maximal durability clarity but poor throughput under concurrent writes.

**Group Commit** reduces fsync frequency by:
- Allowing multiple commits to share a single fsync
- While preserving *exact* durability, ordering, and visibility semantics

Group Commit does **not**:
- Change acknowledgment semantics
- Change CommitId semantics
- Change WAL meaning
- Introduce async durability

---

## 2. Baseline Reference (Normative)

This optimization is defined relative to:

- Section 3 of `PERFORMANCE_BASELINE.md`
- Section 2.1 and 2.2 of `CRITICAL_PATHS.md`

Baseline properties that MUST remain true:

- A commit is acknowledged only after fsync
- CommitIds are assigned after fsync
- WAL append order == commit order
- Each commit is independently durable

---

## 3. Definition of Group Commit

### 3.1 Conceptual Definition

Group Commit allows:

> Multiple logically independent commits to wait on the **same fsync call**,  
> provided that each commit’s WAL record is fully written *before* that fsync.

Each commit remains:
- Logically independent
- Separately represented in WAL
- Separately ordered
- Separately acknowledged

The fsync is **shared**, not amortized semantically.

---

### 3.2 Non-Definition (What Group Commit Is NOT)

Group Commit does NOT mean:

- Delaying acknowledgment beyond durability
- Acknowledging commits before fsync
- Combining WAL records into a single logical commit
- Assigning CommitIds early
- Time-based flushing
- Adaptive grouping

If grouping depends on timing heuristics, it is invalid.

---

## 4. Mechanical Description

### 4.1 Baseline Write Path (Simplified)

For each commit:

1. Append WAL record
2. fsync WAL
3. Assign CommitId
4. Acknowledge commit

---

### 4.2 Group Commit Write Path

Under Group Commit, the following mechanical change is permitted:

1. Append WAL record for commit A
2. Append WAL record for commit B
3. Append WAL record for commit C
4. fsync WAL **once**
5. Assign CommitIds to A, B, C (in append order)
6. Acknowledge A, B, C (in order)

Key rule:
- No commit is acknowledged before fsync returns

---

### 4.3 Group Formation Rules

Group Commit groups are formed ONLY by:

- Concurrent arrival of commits
- Explicit queueing before fsync

Groups MUST NOT be formed by:
- Timers
- Delays
- Load thresholds
- Background batching

If only one commit is present, behavior is identical to baseline.

---

## 5. Invariant Preservation Matrix

This section references `PERF_INVARIANTS.md`.

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
- REP-1 (Single Writer): **Preserved**
- REP-2 (WAL Prefix Rule): **Preserved**
- REP-3 (Replica Equivalence): **Preserved**

### Failure & Recovery
- FR-1, FR-2, FR-3: **Preserved**

### Observability
- OBS-1, OBS-2: **Preserved**

### Disablement
- DIS-1, DIS-2, DIS-3: **Preserved**

No invariant is weakened or reinterpreted.

---

## 6. Semantic Equivalence Argument

Group Commit is semantically equivalent to baseline execution because:

- WAL contains the same sequence of logical commit records
- Commit ordering is identical
- CommitIds are assigned in the same order
- Acknowledgment occurs after fsync in all cases
- Crash recovery replays the same WAL

The only difference is **which commits wait on which fsync**, which is not observable.

---

## 7. Failure Matrix

### 7.1 Crash Before WAL Append

- Baseline: commit lost
- Group Commit: commit lost

Equivalent.

---

### 7.2 Crash After WAL Append, Before fsync

- Baseline: commit not durable, replay drops it
- Group Commit: commit not durable, replay drops it

Equivalent.

---

### 7.3 Crash During fsync

- Baseline: durability depends on fsync completion
- Group Commit: durability depends on fsync completion

Equivalent.

---

### 7.4 Crash After fsync, Before Acknowledgment

- Baseline: commit durable, replay restores it
- Group Commit: commit durable, replay restores it

Equivalent.

---

### 7.5 Partial WAL Write

- Detected by checksum
- Commit rejected or replay-failed identically

Equivalent.

---

## 8. Recovery Proof

- WAL replay logic is unchanged
- WAL contents are unchanged
- Replay order is unchanged
- No grouping state is persisted

Therefore:
- Replay is deterministic
- Replay is idempotent
- Replay is optimization-agnostic

---

## 9. Disablement & Rollback

### 9.1 Disablement Mechanism

Group Commit MUST be disableable via:

- Compile-time flag **or**
- Startup configuration

Disablement means:
- Each commit performs its own fsync
- No shared fsync paths exist

---

### 9.2 Compatibility Proof

- WAL format is unchanged
- Snapshot format is unchanged
- Checkpoint format is unchanged

Data written with Group Commit enabled is readable with it disabled.

---

### 9.3 No Ghost State

- No persistent grouping metadata
- No WAL flags
- No snapshot annotations

All grouping state is in-memory and discardable.

---

## 10. Observability

Permitted metrics (passive only):

- group_commit.size
- group_commit.fsync_count
- group_commit.waiters

Metrics MUST NOT:
- Influence grouping
- Influence commit ordering
- Influence scheduling

---

## 11. Testing Requirements

Group Commit MUST introduce:

- Equivalence tests vs baseline
- Crash tests at all boundaries
- Enable → write → crash → disable → recover tests
- Replication-prefix validation tests

All Phase 1 and Phase 2 tests MUST pass unmodified.

---

## 12. Explicit Non-Goals

Group Commit does NOT aim to:

- Reduce latency of individual commits
- Change commit acknowledgment timing
- Introduce adaptive batching
- Reduce fsync durability guarantees

It improves throughput only.

---

## 13. Final Rule

> Group Commit is acceptable only if it is  
> indistinguishable from doing every fsync separately.

If a client, replica, or recovery process can tell the difference,
Group Commit is invalid.

---

END OF DOCUMENT
