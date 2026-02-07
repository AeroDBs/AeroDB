# FAILURE MODEL — PHASE 3

## Status

- Phase: **3**
- Authority: **Normative**
- Scope: **Failure assumptions and equivalence requirements**
- Dependencies:
  - PERF_VISION.md
  - PERF_INVARIANTS.md
  - PERF_PROOF_RULES.md
  - PERFORMANCE_BASELINE.md
  - SEMANTIC_EQUIVALENCE.md

This document defines the **authoritative failure model** for Phase 3.
It is inherited from Phase 1 and MUST NOT be weakened, narrowed, or reinterpreted.

All Phase 3 optimizations MUST re-prove correctness under this model.

---

## 1. Purpose

Phase 3 allows performance optimizations.
Failures are where optimizations usually break correctness.

This document defines:
- What failures are assumed possible
- When failures may occur
- What outcomes are required
- What optimizations must explicitly re-prove

If an optimization relies on a failure *not* occurring, it is invalid.

---

## 2. Failure Model Continuity

The Phase 3 failure model is **identical to Phase 1**.

No additional assumptions are allowed.
No failures may be excluded.
No probabilities may be assigned.

Phase 3 does NOT introduce:
- “Fast path” failure exclusions
- Reduced failure surfaces
- Hardware trust assumptions

---

## 3. Global Failure Assumptions

The system MUST be correct under the following assumptions:

- Process may terminate at any instruction
- Power may be lost at any time
- Kernel may crash or panic
- Disk writes may be partially completed
- fsync may return only when durability is achieved
- fsync may be delayed arbitrarily
- Reads may return corrupted data (detected by checksum)
- Memory is volatile and lost on crash
- Time does not advance monotonically across crashes

No assumption of:
- Graceful shutdown
- Ordered teardown
- Fair scheduling
- Bounded I/O latency

---

## 4. Process-Level Failures

### 4.1 Sudden Process Termination

The process may terminate:

- Between any two instructions
- During any system call
- While holding locks
- While mutating in-memory state

**Required Outcome:**
- On restart, recovery MUST succeed
- Persistent state MUST be consistent
- No acknowledged write may be lost

Optimizations MUST NOT:
- Depend on cleanup handlers
- Depend on RAII for correctness
- Leave persistent state in ambiguous form

---

### 4.2 Restart Semantics

After restart:

- Recovery begins from last durable checkpoint
- WAL is replayed deterministically
- In-memory state is rebuilt

Optimizations MUST NOT:
- Require optimization-specific replay logic
- Branch replay behavior on configuration

---

## 5. Power Loss Failures

### 5.1 Power Loss Timing

Power loss may occur:

- Before WAL write
- During WAL write
- After WAL write but before fsync
- During fsync
- After fsync but before acknowledgment
- During snapshot creation
- During checkpoint
- During WAL truncation

All boundaries are failure boundaries.

---

### 5.2 Required Properties

- Only fsync defines durability
- Data written before fsync MAY be lost
- Data written after fsync MUST persist

Optimizations MUST:
- Preserve fsync as the durability boundary
- Never infer durability earlier

---

## 6. Disk I/O Failures

### 6.1 Partial Writes

Disk writes may be:

- Partially written
- Torn across sectors
- Reordered internally (unless fsync completed)

**Required Outcome:**
- Partial writes MUST be detected via checksum
- Corruption MUST NOT be repaired silently

Optimizations MUST NOT:
- Remove checksums
- Coalesce writes without preserving detectability

---

### 6.2 Disk Errors

Disk operations may fail with errors:

- ENOSPC
- EIO
- Permission errors
- Transient failures

**Required Outcome:**
- Errors must be surfaced explicitly
- No silent fallback behavior

---

## 7. fsync Semantics

### 7.1 fsync Guarantees

fsync guarantees:
- All previous writes are durable upon successful return

fsync does NOT guarantee:
- Bounded latency
- Immediate hardware flush
- Ordering between different files unless specified

Optimizations MUST:
- Treat fsync return as the only durability signal
- Avoid assumptions about fsync speed or batching

---

### 7.2 fsync Failure

fsync may:
- Fail with error
- Succeed after long delay

**Required Outcome:**
- On fsync failure, commit MUST fail
- No acknowledgment may occur

---

## 8. Memory Failures

### 8.1 Volatile Memory Loss

On crash:
- All in-memory state is lost
- No memory content is preserved

Optimizations MUST NOT:
- Depend on memory persistence
- Depend on restart-resident caches

---

### 8.2 Memory Corruption

Memory corruption is NOT assumed correctable.

Phase 3 does not introduce:
- ECC assumptions
- Retry-on-corruption logic

---

## 9. Checkpoint & Snapshot Failures

### 9.1 Snapshot Creation Failures

Failures may occur:
- During snapshot enumeration
- During snapshot file write
- During snapshot fsync
- During manifest write

**Required Outcome:**
- Incomplete snapshots MUST be detected
- They MUST NOT be used for recovery

---

### 9.2 Checkpoint Failures

Failures may occur:
- After snapshot but before checkpoint marker
- After marker but before WAL truncation

**Required Outcome:**
- WAL truncation MUST NOT occur unless snapshot is durable
- Recovery MUST choose a valid checkpoint

Optimizations MUST preserve this ordering exactly.

---

## 10. WAL Truncation Failures

Failures may occur:
- During truncation
- After truncation but before fsync

**Required Outcome:**
- Truncation MUST be atomic or detectable
- WAL prefix must remain valid

Optimizations MUST NOT:
- Introduce speculative truncation
- Rely on background cleanup

---

## 11. Replication-Relevant Failures

Even if replication is not yet enabled, optimizations MUST respect:

### 11.1 Replica Disconnect

Replica may:
- Disconnect at any time
- Reconnect later
- Restart independently

**Required Outcome:**
- WAL prefix rule is preserved
- Replica state is derivable

---

### 11.2 Partial WAL Shipping

Replica may receive:
- Partial WAL segments
- Interrupted transfers

**Required Outcome:**
- Partial data is detected
- Replay waits for completion

Optimizations MUST NOT:
- Assume continuous streaming
- Assume synchronous replication

---

## 12. Failure Equivalence Requirement

For every optimization:

- Each failure point in baseline MUST be enumerated
- Optimized behavior MUST be described
- Resulting state MUST be equivalent

Equivalence must hold for:
- Success paths
- Failure paths
- Recovery outcomes

---

## 13. Forbidden Failure Assumptions

Optimizations MUST NOT assume:

- “Crashes are rare”
- “fsync is fast”
- “Writes are atomic”
- “Storage is reliable”
- “Shutdown is graceful”
- “Power loss won’t happen here”

Any such assumption invalidates the optimization.

---

## 14. Failure Proof Obligations

Each optimization specification MUST include:

- A failure timeline
- A before/after state comparison
- Recovery convergence proof
- Explicit reference to this document

If a failure case is omitted, proof is incomplete.

---

## 15. Final Rule

> AeroDB correctness is defined by its behavior  
> when everything goes wrong.

An optimization that only works when things go right
does not belong in Phase 3.

---

END OF DOCUMENT
