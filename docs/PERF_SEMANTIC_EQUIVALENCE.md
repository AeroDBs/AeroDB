# SEMANTIC EQUIVALENCE — PHASE 3

## Status

- Phase: **3**
- Authority: **Normative**
- Scope: **Equivalence definition for all Phase 3 optimizations**
- Dependencies:
  - PERF_VISION.md
  - PERF_INVARIANTS.md
  - PERF_PROOF_RULES.md
  - PERFORMANCE_BASELINE.md
  - CRITICAL_PATHS.md

No optimization is valid unless it is proven semantically equivalent
under the definitions in this document.

---

## 1. Purpose

Phase 3 permits performance changes **only if behavior is unchanged**.

This document defines:
- What “behavior” means in AeroDB
- What is observable vs non-observable
- What must remain identical
- What differences are permitted
- How equivalence is evaluated under failures

Equivalence is **not similarity**.  
Equivalence is **indistinguishability** to all permitted observers.

---

## 2. Definition of Semantic Equivalence

Two executions are **semantically equivalent** if and only if:

> For all permitted observers, under all allowed failure scenarios,
> the observable outcomes are identical.

This must hold for:
- Successful execution
- Partial execution
- Crash and recovery
- Restart and replay
- Replication consumption

Performance differences are irrelevant.

---

## 3. Permitted Observers

Semantic equivalence is evaluated relative to the following observers:

### 3.1 Client Observer

A client may observe:
- Acknowledgment timing (relative to fsync only)
- Success or failure
- Returned query results
- Error codes

A client may NOT observe:
- Internal scheduling
- Intermediate states
- Background activity

---

### 3.2 Crash-Recovery Observer

The recovery process may observe:
- WAL contents
- Snapshot contents
- Checkpoints
- Checksums

Recovery MUST produce identical state.

---

### 3.3 Replica Observer

A replica may observe:
- WAL stream
- Snapshot bootstrap data
- Commit ordering
- Visibility boundaries

Replica-visible state MUST be derivable identically.

---

### 3.4 Diagnostic Observer

Logs and metrics may observe:
- Events
- Counters
- Timing

Diagnostics MUST NOT influence semantics and are not authoritative for equivalence.

---

## 4. Observable State Surface

The following are **observable and must be equivalent**:

### 4.1 Persistent State

- WAL records (logical content and ordering)
- Snapshot files
- Snapshot manifests
- Checkpoint markers

Binary equality is not required, but **logical equivalence is mandatory**.

---

### 4.2 Logical Database State

- Document contents
- MVCC version chains
- CommitId assignments
- Tombstones
- Index-visible state

After recovery, state MUST be identical.

---

### 4.3 Visibility Semantics

- Snapshot isolation behavior
- Version visibility
- Query results

A read must return the same result set.

---

### 4.4 Failure Outcomes

- Which operations succeed
- Which operations fail
- Which operations are retried
- Which operations are rolled back

Failures MUST be detected at the same semantic boundaries.

---

## 5. Non-Observable Differences (Permitted)

The following differences are allowed **only if they are not observable**:

- Internal buffering
- Memory layout
- CPU instruction count
- Syscall batching
- Temporary allocations
- Lock granularity (if determinism preserved)

If a difference becomes observable under any failure, it is forbidden.

---

## 6. Temporal Considerations

### 6.1 Time Is Not a Semantic Dimension

Equivalence MUST NOT depend on:
- Wall-clock time
- CPU speed
- I/O latency
- Scheduling fairness

An optimization that relies on “fast enough” behavior is invalid.

---

### 6.2 Ordering Is Semantic

Ordering of:
- WAL records
- CommitIds
- Visibility transitions

is **semantically observable** and MUST be preserved.

---

## 7. Write Path Equivalence

Two executions are equivalent on the write path if:

- Acknowledgment occurs after equivalent durability
- CommitIds are assigned in the same order
- WAL records represent the same logical commits
- Partial writes fail identically

Batching is allowed **only** if it produces the same logical sequence.

---

## 8. Read Path Equivalence

Two executions are equivalent on the read path if:

- The same snapshot sees the same versions
- Version chain traversal yields the same result
- Queries return identical result sets
- Errors occur at the same semantic points

Caching is allowed only if visibility rules are re-applied.

---

## 9. MVCC Equivalence

MVCC behavior is equivalent if:

- Snapshot isolation holds identically
- No snapshot observes new or missing versions
- CommitId visibility rules are unchanged
- Garbage collection effects are identical

Optimizations MUST NOT:
- Skip visibility checks
- Pre-compute visibility without proof

---

## 10. Replication Equivalence

Replication behavior is equivalent if:

- WAL stream is a strict prefix-equivalent sequence
- Snapshots represent the same MVCC cut
- Replicas reach the same logical state
- Replica restarts behave identically

Any optimization affecting WAL emission MUST prove prefix preservation.

---

## 11. Failure Equivalence

For each failure point:

- Baseline outcome MUST be described
- Optimized outcome MUST match
- Recovery MUST converge to identical state

Failure equivalence is evaluated at:
- Before fsync
- During WAL write
- After fsync but before acknowledgment
- During checkpoint
- During snapshot
- During replication transfer (if applicable)

---

## 12. Equivalence Under Disablement

An optimization is equivalent under disablement if:

- Data written with optimization enabled
  can be read with it disabled
- WAL replay behavior is identical
- No migration or cleanup is required

Disablement MUST NOT alter semantics.

---

## 13. Prohibited Equivalence Arguments

The following arguments are invalid:

- “Equivalent in practice”
- “Equivalent under normal load”
- “Equivalent assuming no crash”
- “Equivalent because tests pass”
- “Equivalent because performance improved”

Equivalence is structural, not empirical.

---

## 14. Equivalence Proof Obligations

Every optimization MUST:

- Reference this document explicitly
- State which equivalence clauses apply
- Prove equivalence clause-by-clause
- Include failure equivalence

Missing a clause is proof failure.

---

## 15. Final Rule

> In AeroDB, two executions are equivalent  
> if no observer can ever tell them apart.

If an observer can distinguish them,
the optimization is rejected.

---

END OF DOCUMENT
