# CRITICAL PATHS — PHASE 3

## Status

- Phase: **3**
- Authority: **Normative**
- Scope: **Identification-only**
- Dependencies:
  - PERF_VISION.md
  - PERF_INVARIANTS.md
  - PERF_PROOF_RULES.md
  - PERFORMANCE_BASELINE.md

This document defines the **canonical critical execution paths** in AeroDB.
It does not introduce optimizations. It only marks *where* time is spent
and *which boundaries are semantically sensitive*.

If a path is not listed here, it MUST NOT be optimized in Phase 3.

---

## 1. Definition of “Critical Path”

A critical path is an execution sequence where:

- End-to-end latency is user-visible, or
- Throughput is bounded by synchronous steps, or
- Durability / visibility boundaries exist

Critical paths are **semantic fault lines**.
Optimizing them requires proof that boundaries remain intact.

---

## 2. Write-Side Critical Paths

### 2.1 Primary Write Commit Path (Single Document)

This is the **most sensitive path in AeroDB**.

#### Sequence (Canonical)

1. Client request accepted
2. Schema validation
3. MVCC uncommitted version creation
4. WAL record construction
5. WAL append
6. WAL checksum write
7. fsync completion
8. CommitId assignment
9. Acknowledgment to client
10. In-memory index update
11. Version visibility transition

#### Critical Boundaries

- **Durability boundary**: between steps 7 and 8
- **Visibility boundary**: after step 11
- **Acknowledgment boundary**: after step 8

Any optimization touching this path MUST preserve:
- Exact acknowledgment point
- Exact durability semantics
- CommitId monotonicity

---

### 2.2 WAL Append + fsync Path

This is the **primary latency bottleneck**.

#### Characteristics

- Sequential file append
- Per-record checksum
- Synchronous fsync
- No batching in baseline

#### Critical Properties

- Append order == commit order
- fsync defines durability
- Partial writes are detectable

This path is eligible for *mechanical equivalence optimizations only*.

---

## 3. Read-Side Critical Paths

### 3.1 Snapshot-Based Point Read

#### Sequence

1. Snapshot acquired
2. Index lookup (if applicable)
3. Version chain traversal
4. Visibility check per version
5. Document materialization
6. Result return

#### Critical Boundaries

- Visibility decision (step 4)
- Version chain ordering

Any optimization MUST:
- Re-evaluate visibility explicitly
- Preserve traversal semantics

---

### 3.2 Range / Query Read Execution

#### Sequence

1. Query parse
2. Plan selection
3. Iterator creation
4. Repeated snapshot-visible reads
5. Result accumulation

#### Critical Properties

- Plan is deterministic
- Execution is bounded
- No adaptive behavior

Optimizations may not:
- Skip visibility checks
- Short-circuit evaluation without proof

---

## 4. MVCC-Specific Critical Paths

### 4.1 CommitId Assignment

#### Properties

- Occurs after fsync
- WAL-governed
- Single authority

This path MUST remain:
- Synchronous
- Linear
- Non-speculative

---

### 4.2 Version Chain Traversal

#### Properties

- Ordered by CommitId
- Immutable once committed
- Deterministic traversal order

Optimizations may reduce traversal cost,
but MUST NOT alter traversal logic or visibility semantics.

---

## 5. Snapshot & Checkpoint Critical Paths

### 5.1 Snapshot Creation Path

#### Sequence

1. Determine consistent CommitId
2. Freeze snapshot visibility
3. Enumerate state
4. Persist snapshot files
5. Write snapshot manifest

#### Critical Boundaries

- Snapshot CommitId selection
- Manifest durability

---

### 5.2 Checkpoint Path

#### Sequence

1. Snapshot completion
2. Snapshot fsync
3. Checkpoint marker written
4. WAL truncation

#### Critical Properties

- WAL truncation only after snapshot durability
- No pipelining in baseline

Any optimization MUST preserve this strict ordering.

---

## 6. Recovery Critical Paths

### 6.1 Startup Recovery

#### Sequence

1. Locate last checkpoint
2. Load snapshot
3. Replay WAL forward
4. Validate checksums
5. Rebuild in-memory state

#### Critical Properties

- Deterministic replay order
- Idempotent operations
- No optimization-dependent branches

Optimizations MUST NOT:
- Add replay-time conditionals
- Depend on runtime configuration during replay

---

## 7. Replication-Sensitive Paths

Even before implementation, the following paths are replication-critical:

### 7.1 WAL Emission Order

- WAL is the replication stream
- Order defines replica state

Any optimization touching WAL emission MUST preserve prefix ordering exactly.

---

### 7.2 Commit Visibility vs WAL Shipping

- Commit becomes visible only after WAL durability
- Replica reads depend on MVCC safety

No optimization may:
- Make data visible before it is replicable
- Introduce transient primary-only visibility

---

## 8. Observability Interaction Points

### 8.1 Logging

- Occurs synchronously on critical paths
- Must not affect control flow

---

### 8.2 Metrics

- Emitted at defined boundaries
- Deterministic order

Metrics MUST NOT be used to gate or adjust behavior.

---

## 9. Explicit Non-Critical Paths

The following are **not** critical paths in Phase 3:

- Background maintenance (non-GC)
- Diagnostics
- Offline tooling
- Administrative queries

These paths MUST NOT influence critical-path optimization decisions.

---

## 10. Optimization Eligibility Matrix

| Path | Eligible for Phase 3 Optimization |
|-----|----------------------------------|
| Write commit | Yes (strict proof required) |
| WAL fsync | Yes (mechanical equivalence only) |
| Read traversal | Yes (MVCC-safe only) |
| Snapshot creation | Yes (ordering preserved) |
| Checkpoint | Yes (no semantic reordering) |
| Recovery | No (must remain baseline-equivalent) |
| Observability | No (passive only) |

---

## 11. Role of This Document

This document:

- Identifies *where* optimization may occur
- Marks *which boundaries are sacred*
- Serves as a proof reference

It does NOT:
- Propose optimizations
- Allow shortcuts
- Relax invariants

---

## 12. Final Rule

> If an optimization changes the shape of a critical path,  
> it must prove the path is **observationally identical**.

Otherwise, the optimization is invalid.

---

END OF DOCUMENT
