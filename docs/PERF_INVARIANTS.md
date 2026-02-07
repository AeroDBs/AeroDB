# PHASE 3 — INVARIANTS

## Status

- Phase: **3**
- Scope: **Global, cross-cutting**
- Authority: **Normative**
- Applies to: **All Phase 3 optimizations without exception**

This document defines the **non-negotiable invariants** that Phase 3 optimizations MUST preserve.
Violation of any invariant invalidates an optimization regardless of performance benefit.

---

## 1. Invariant Taxonomy

Phase 3 invariants are grouped by domain:

1. Durability Invariants
2. Determinism Invariants
3. Visibility & MVCC Invariants
4. Replication Invariants
5. Failure & Recovery Invariants
6. Observability Invariants
7. Disablement & Rollback Invariants

All invariants are **absolute**, not best-effort.

---

## 2. Durability Invariants

### D-1: Acknowledged Write Durability

Once a write is acknowledged to the caller:

- Its WAL record MUST be:
  - Fully written
  - Checksummed
  - fsync-completed
- The write MUST survive:
  - Process crash
  - Power loss
  - Kernel panic

**Phase 3 Rule:**  
No optimization may delay, weaken, batch, or amortize durability beyond what Phase 1 guarantees.

---

### D-2: Atomic Commit Boundary Preservation

- Each commit has a precise durability boundary
- Partial commits are forbidden
- Commit boundaries MUST NOT be inferred from timing or batching

**Phase 3 Rule:**  
If commits are grouped, the group MUST be equivalent to sequential commits with identical durability semantics.

---

### D-3: No Silent Durability Downgrade

- Durability behavior MUST NOT vary based on:
  - Load
  - Time
  - Configuration heuristics
  - Hardware characteristics

**Phase 3 Rule:**  
Durability is a constant, not a variable.

---

## 3. Determinism Invariants

### DET-1: Crash Determinism

Given identical inputs and crash points:

- Recovery MUST produce identical state
- WAL replay order MUST be identical
- MVCC visibility MUST be identical

**Phase 3 Rule:**  
Optimizations MUST NOT introduce scheduling, timing, or concurrency-dependent outcomes.

---

### DET-2: Replay Equivalence

Optimized execution MUST be replay-equivalent to baseline execution.

- WAL contents MUST be semantically identical
- Commit ordering MUST be preserved
- Replay logic MUST remain unchanged

**Phase 3 Rule:**  
If replay behavior differs, the optimization is invalid.

---

### DET-3: Bounded Execution

- All operations remain bounded
- No unbounded queues
- No unbounded memory growth

**Phase 3 Rule:**  
Performance optimizations may reduce bounds, never remove them.

---

## 4. Visibility & MVCC Invariants

### MVCC-1: Snapshot Isolation Preservation

- Snapshot visibility rules are immutable
- No read may observe:
  - Uncommitted data
  - Future commits
  - Cross-snapshot leakage

**Phase 3 Rule:**  
Read optimizations MUST explicitly re-check MVCC visibility.

---

### MVCC-2: CommitId Authority

- CommitIds originate only from WAL-governed commits
- No speculative or provisional CommitIds

**Phase 3 Rule:**  
Optimizations MUST NOT invent or infer CommitIds.

---

### MVCC-3: Version Chain Integrity

- Version chains are immutable
- Order is strictly CommitId-ordered

**Phase 3 Rule:**  
Optimizations MUST NOT shortcut version traversal without proof of equivalence.

---

## 5. Replication Invariants

### REP-1: Single-Writer Invariant

- Only Primary assigns CommitIds
- Replicas consume WAL strictly as a prefix

**Phase 3 Rule:**  
No optimization may alter CommitId assignment timing or authority.

---

### REP-2: WAL Prefix Rule

- Replica WAL is always a prefix of Primary WAL
- No gaps
- No reordering

**Phase 3 Rule:**  
Batching or grouping MUST preserve prefix structure exactly.

---

### REP-3: Replica Observational Equivalence

- Replica-visible state MUST be derivable from WAL + snapshots
- No hidden or transient state

**Phase 3 Rule:**  
Optimizations MUST NOT create replica-only or primary-only semantic paths.

---

## 6. Failure & Recovery Invariants

### FR-1: Failure Detectability

- Corruption MUST be detected
- No silent repair
- No best-effort recovery

**Phase 3 Rule:**  
Optimizations MUST preserve all existing checksums and validation points.

---

### FR-2: Failure Locality

- Failures affect only the operation in progress
- No cascading semantic damage

**Phase 3 Rule:**  
Optimizations MUST NOT widen failure blast radius.

---

### FR-3: Recovery Idempotence

- WAL replay is idempotent
- Replaying the same WAL twice yields identical state

**Phase 3 Rule:**  
Optimizations MUST NOT introduce side effects during replay.

---

## 7. Observability Invariants

### OBS-1: Observability Passivity

- Metrics and logs MUST NOT influence behavior
- No metric-driven decisions

**Phase 3 Rule:**  
Instrumentation is observational only.

---

### OBS-2: Deterministic Emission

- Metrics emission order is deterministic
- Logs do not affect control flow

**Phase 3 Rule:**  
Instrumentation must be ignorable without semantic change.

---

## 8. Disablement & Rollback Invariants

### DIS-1: Safe Disablement

- Any optimization can be disabled:
  - At compile time, or
  - At startup
- Disabling MUST NOT require data migration

---

### DIS-2: Data Compatibility

- Data written with optimization enabled MUST be readable with it disabled
- WAL and snapshot formats MUST remain compatible

---

### DIS-3: No Ghost State

- Optimizations MUST NOT introduce:
  - Hidden persistent state
  - Implicit metadata
  - Undocumented invariants

---

## 9. Invariant Enforcement

Each Phase 3 optimization MUST:

- Explicitly list which invariants it touches
- Prove preservation of each
- Include failure matrices per invariant
- Reference this document section-by-section

Failure to reference an invariant implies failure to preserve it.

---

## 10. Invariant Hierarchy

If invariants conflict, precedence is:

1. Durability
2. Determinism
3. MVCC Visibility
4. Replication
5. Failure Detection
6. Observability
7. Performance

Performance always loses.

---

## 11. Final Rule

> An optimization that cannot prove it preserves **every invariant** is not an optimization — it is a bug.

---

END OF DOCUMENT
