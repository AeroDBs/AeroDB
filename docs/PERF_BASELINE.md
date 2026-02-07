# PERFORMANCE BASELINE — PHASE 3

## Status

- Phase: **3**
- Authority: **Normative**
- Scope: **Baseline-only (no optimization)**
- Dependency:
  - PERF_VISION.md
  - PERF_INVARIANTS.md
  - PERF_PROOF_RULES.md

This document defines the **reference execution behavior** against which all Phase 3 optimizations are proven equivalent.

If baseline behavior is not explicitly described here, it MUST NOT be optimized.

---

## 1. Purpose

The purpose of this document is to:

- Freeze the **current execution model**
- Identify **cost centers without changing behavior**
- Provide a **canonical reference** for equivalence proofs
- Prevent accidental semantic drift during optimization

This document contains **no optimizations**, only descriptions.

---

## 2. Baseline Execution Model Overview

AeroDB baseline behavior is defined by:

- Single-node execution (Phase 1)
- MVCC-enabled visibility (Phase 2A)
- Replication-compatible semantics (Phase 2B)
- Deterministic WAL-governed state transitions

Execution is **synchronous, explicit, and ordered**.

---

## 3. Baseline Write Path

### 3.1 Client Write Request Lifecycle

For a single write request:

1. Client submits document write
2. Schema validation is performed (strict, versioned)
3. MVCC version object is created (uncommitted)
4. WAL record is constructed
5. WAL record is written to disk
6. WAL record is fsync’ed
7. CommitId is assigned
8. Write is acknowledged to client
9. In-memory indexes are updated
10. Version becomes visible to eligible snapshots

**Critical Rule:**  
Acknowledgment occurs **only after fsync completion**.

---

### 3.2 WAL Characteristics (Baseline)

- WAL records are:
  - Appended sequentially
  - Checksummed individually
  - Written one logical commit at a time
- fsync is performed per acknowledged commit
- No grouping, batching, or coalescing occurs

---

### 3.3 Write Cost Centers

Primary contributors:

- fsync latency
- WAL serialization
- Checksum computation
- Schema validation
- In-memory index updates

No cost is amortized across requests.

---

## 4. Baseline Read Path

### 4.1 Snapshot Creation

- Snapshot is created with a fixed `visible_commit_id`
- Snapshot does not change over time
- Snapshot is immutable

---

### 4.2 Document Read Execution

For each read:

1. Snapshot visibility rules are applied
2. Version chain is traversed
3. First visible version is selected
4. Document is materialized
5. Result is returned

No caching beyond existing structures is assumed.

---

### 4.3 Read Cost Centers

Primary contributors:

- Version chain traversal
- Snapshot visibility checks
- Document materialization
- Index lookups (if applicable)

---

## 5. Baseline Query Execution

### 5.1 Query Planning

- Query is parsed deterministically
- Planner produces a bounded plan
- No adaptive planning
- No runtime plan modification

---

### 5.2 Query Execution

- Execution follows plan strictly
- No speculative execution
- No parallelism unless explicitly specified by Phase 1
- All bounds are enforced

---

## 6. Baseline MVCC Behavior

### 6.1 CommitId Assignment

- CommitId is assigned **only after WAL fsync**
- CommitIds are strictly increasing
- No speculative or provisional IDs

---

### 6.2 Visibility Rules

- Snapshot sees all commits ≤ visible_commit_id
- No snapshot sees partial commits
- Visibility is deterministic and reproducible

---

### 6.3 Garbage Collection (Baseline)

- GC is WAL-governed
- GC does not run opportunistically
- GC decisions are deterministic

---

## 7. Baseline Snapshot & Checkpoint Behavior

### 7.1 Snapshot Creation

- Snapshot captures a full MVCC-consistent view
- Snapshot is read-only
- Snapshot is manifest-driven

---

### 7.2 Checkpoint Execution

1. Snapshot is created
2. Snapshot is persisted
3. WAL truncation occurs only after snapshot durability
4. No concurrent checkpoint pipelining

---

### 7.3 Cost Centers

- Snapshot I/O
- Manifest generation
- WAL truncation coordination

---

## 8. Baseline Recovery Path

### 8.1 Startup Recovery

1. Detect last checkpoint
2. Load snapshot
3. Replay WAL from checkpoint forward
4. Validate checksums
5. Rebuild in-memory indexes

Recovery is:

- Deterministic
- Idempotent
- Single-threaded unless specified elsewhere

---

### 8.2 Recovery Cost Centers

- WAL replay time
- Index rebuild time
- Checksum validation

---

## 9. Baseline Replication Interaction

Even in single-node Phase 1:

- WAL format is replication-ready
- Commit ordering is authoritative
- No replication-specific shortcuts exist

Baseline behavior MUST remain compatible with Phase 2B replication semantics.

---

## 10. Baseline Observability

- Logs are emitted synchronously
- Metrics are collected deterministically
- Observability does not influence control flow

Instrumentation cost exists but is not optimized.

---

## 11. Baseline Resource Usage Characteristics

### 11.1 CPU

- Serialization
- Validation
- Checksums
- Query execution

---

### 11.2 I/O

- WAL append
- fsync per commit
- Snapshot writes
- Checkpoint I/O

---

### 11.3 Memory

- In-memory indexes
- MVCC version chains
- Snapshot metadata

Memory usage is bounded and explicit.

---

## 12. Explicit Non-Assumptions

Baseline behavior does NOT assume:

- SSD-specific guarantees
- Write-back caching
- Power-loss protection
- Kernel-level reordering safety
- Fair scheduling
- Low latency fsync

All correctness holds under worst-case assumptions.

---

## 13. Baseline as Proof Anchor

All Phase 3 optimizations MUST:

- Reference this document explicitly
- State which sections are affected
- Prove equivalence against this behavior

If behavior is not described here, it MUST NOT be optimized.

---

## 14. Final Rule

> Phase 3 does not optimize AeroDB.  
> It optimizes **this document’s behavior**, without changing it.

---

END OF DOCUMENT
