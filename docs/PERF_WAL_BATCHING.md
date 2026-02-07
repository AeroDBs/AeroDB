# WAL BATCHING — PHASE 3

## Status

- Phase: **3**
- Authority: **Normative**
- Scope: **Write-ahead log emission optimization**
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

This document specifies **WAL Batching** as a correctness-preserving optimization.
If any requirement in this document cannot be proven, WAL Batching MUST NOT be implemented.

---

## 1. Purpose

Baseline AeroDB performs:

- One logical WAL record write per commit
- One physical write per record (even if contiguous)

This results in:
- Excessive syscall overhead
- Redundant write amplification

**WAL Batching** reduces overhead by:
- Writing multiple WAL records in a single physical write
- Without changing WAL semantics, ordering, or durability

WAL Batching affects **how WAL is written**, not **what WAL means**.

---

## 2. Baseline Reference (Normative)

Baseline WAL behavior is defined in:

- `PERFORMANCE_BASELINE.md` §3.2
- `CRITICAL_PATHS.md` §2.2

Baseline invariants:

- WAL is append-only
- Logical record boundaries are explicit
- Each record is checksummed
- Ordering is strict
- fsync defines durability

These invariants MUST remain true.

---

## 3. Definition of WAL Batching

### 3.1 Conceptual Definition

WAL Batching allows:

> Multiple logically independent WAL records  
> to be written to disk using a **single physical write operation**,  
> while preserving logical record boundaries and ordering.

Each WAL record remains:
- Individually checksummed
- Individually parseable
- Individually replayable

Only the *physical I/O shape* changes.

---

### 3.2 Non-Definition (What WAL Batching Is NOT)

WAL Batching does NOT mean:

- Combining records into a new logical format
- Removing per-record checksums
- Changing replay semantics
- Reordering records
- Delaying fsync
- Time-based buffering
- Adaptive buffering based on load

If batching changes record semantics, it is invalid.

---

## 4. Mechanical Description

### 4.1 Baseline WAL Write Path

For each commit:

1. Serialize WAL record
2. Compute checksum
3. Write record to WAL file
4. (Later) fsync WAL

Each record write is independent.

---

### 4.2 WAL Batching Write Path

Under WAL Batching, the following mechanical change is permitted:

1. Serialize WAL record A
2. Serialize WAL record B
3. Serialize WAL record C
4. Concatenate serialized records into a contiguous buffer
5. Perform **one write()** for the buffer
6. (Later) fsync WAL

Rules:
- Serialization order MUST match commit order
- No record may be split across buffers
- Record boundaries MUST be preserved in the byte stream

---

### 4.3 Batch Formation Rules

Batches MAY be formed only by:

- Sequential availability of serialized records
- Explicit bounded buffering

Batches MUST NOT be formed by:
- Timers
- Load heuristics
- Background aggregation
- Dynamic resizing based on latency

Batch size MUST be:
- Explicitly bounded
- Deterministic
- Configuration-defined or compile-time defined

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
- MVCC-1, MVCC-2, MVCC-3: **Preserved**

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

WAL Batching is semantically equivalent to baseline because:

- The WAL byte stream is a strict concatenation of baseline records
- Record boundaries are unchanged
- Record ordering is unchanged
- Checksums validate each record independently
- fsync semantics are unchanged
- Replay reads records identically

From the perspective of:
- Clients
- Recovery
- Replicas

The WAL is indistinguishable from baseline emission.

---

## 7. Failure Matrix

### 7.1 Crash Before Write

- Baseline: no WAL data written
- Batched: no WAL data written

Equivalent.

---

### 7.2 Crash During Batched Write

Possible outcomes:
- No bytes written
- Partial buffer written

In all cases:
- Partial records are detected by checksum
- Replay behavior matches baseline partial-write behavior

Equivalent.

---

### 7.3 Crash After Batched Write, Before fsync

- Baseline: data may be lost
- Batched: data may be lost

Equivalent.

---

### 7.4 Crash After fsync

- Baseline: records durable
- Batched: records durable

Equivalent.

---

### 7.5 Disk Error During Write

- Error surfaced immediately
- Commit fails
- No acknowledgment

Equivalent.

---

## 8. Recovery Proof

- WAL replay logic is unchanged
- Replay reads sequential records
- Checksum validation is per-record
- No batching metadata exists

Replay determinism is preserved.

---

## 9. Disablement & Rollback

### 9.1 Disablement Mechanism

WAL Batching MUST be disableable via:

- Compile-time flag **or**
- Startup configuration

Disablement restores:
- One write per WAL record

---

### 9.2 Compatibility Proof

- WAL format unchanged
- Snapshot format unchanged
- Checkpoint format unchanged

Batched WAL is readable without batching enabled.

---

### 9.3 No Ghost State

- No persistent batching metadata
- No WAL flags
- No snapshot annotations

All batching state is in-memory and discardable.

---

## 10. Observability

Permitted metrics (passive only):

- wal_batch.records_per_write
- wal_batch.bytes_per_write
- wal_batch.write_syscalls

Metrics MUST NOT:
- Influence batch size
- Influence flushing
- Influence grouping

---

## 11. Testing Requirements

WAL Batching MUST introduce:

- WAL equivalence tests
- Partial-write fault injection tests
- Crash-recovery tests
- Enable → write → crash → disable → recover tests
- Replication prefix validation tests

All existing tests MUST pass unmodified.

---

## 12. Explicit Non-Goals

WAL Batching does NOT aim to:

- Change fsync frequency (see Group Commit)
- Reduce durability guarantees
- Introduce adaptive buffering
- Change WAL semantics

It optimizes syscall efficiency only.

---

## 13. Final Rule

> WAL Batching is valid only if the WAL  
> could have been written record-by-record  
> and no observer could ever tell.

If replay, replication, or recovery can distinguish it,
WAL Batching is invalid.

---

END OF DOCUMENT
