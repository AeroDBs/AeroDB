# REPLICATION_LOG_FLOW.md

## AeroDB — Replication Log Flow & State Propagation

### Status

* This document is **authoritative**
* It defines **how history moves from Primary to Replica**
* WAL is the **only replicated unit of truth**
* No implementation details or optimizations appear here
* Phase-1 and MVCC semantics are **frozen and assumed correct**

---

## 1. Purpose of Log Flow Design

Replication correctness in AeroDB depends on one fact:

> **All authoritative history is encoded in the WAL.**

This document defines:

* What is replicated
* In what order
* With what completeness guarantees
* How gaps and inconsistencies are detected
* When replication must stop

Replication is **history replication**, not state reconstruction.

---

## 2. Replication Units

### 2.1 WAL as the Replication Unit

The WAL is:

* Totally ordered
* Checksummed
* CommitId-governed
* Deterministic under replay

Replication **must replicate WAL records verbatim**.

Replication must **not**:

* Re-encode WAL entries
* Reorder WAL entries
* Merge or split WAL entries
* Infer missing WAL entries

---

### 2.2 Replicated Record Scope

All WAL records are replicated, including but not limited to:

* Commit identity records
* Version persistence records
* Snapshot markers
* GC records
* Checkpoint markers

If a WAL record affects recovery or visibility, it **must** be replicated.

---

## 3. Ordering Guarantees

### 3.1 Strict Order Preservation

Replication must preserve:

* Byte-level WAL order
* CommitId ordering
* Logical causality

For any two WAL records `A` and `B`:

```
If A precedes B on Primary WAL
→ A must precede B on Replica WAL
```

No exceptions.

---

### 3.2 Prefix Application Rule

At any time, a Replica’s applied WAL must satisfy:

```
Replica_WAL == Prefix(Primary_WAL)
```

Replicas may lag, but must never:

* Skip records
* Reorder records
* Invent records

---

## 4. WAL Transfer Semantics

### 4.1 Transfer Model (Abstract)

Replication is defined abstractly as:

* Primary emits WAL records
* Replica receives WAL records
* Replica appends them to its WAL
* Replica replays them deterministically

The transport mechanism is **out of scope**.

---

### 4.2 Durability Boundary

A WAL record is considered **replicated** only when:

* It is durably appended to the Replica’s WAL
* Its checksum is verified
* Its ordering is validated

Partial receipt does not count.

---

## 5. Gap Detection

### 5.1 Explicit Gap Definition

A WAL gap exists if:

* Replica receives record `N+1`
* Record `N` is missing or corrupted

Gaps are detected by:

* WAL sequence metadata
* Checksums
* Explicit ordering markers

---

### 5.2 Gap Handling Rule

If a gap is detected:

* Replica **must stop applying WAL**
* Replica enters `ReplicationHalted`
* No reads or writes are allowed

Gaps are **fatal until resolved explicitly**.

---

## 6. Snapshot vs WAL Bootstrap

Replication supports **two bootstrap paths**, both correctness-equivalent.

---

### 6.1 WAL-Only Bootstrap

Allowed only if:

* Replica has an empty state
* Full WAL history is available
* WAL replay cost is acceptable

Replica behavior:

* Receive WAL from genesis
* Replay deterministically

---

### 6.2 Snapshot-Based Bootstrap

Allowed when:

* WAL history is too large
* Replica is too far behind
* Operator initiates snapshot transfer

Snapshot must:

* Represent a valid MVCC cut
* Include commit boundary
* Be self-contained

After snapshot restore:

* Replica resumes WAL replay **strictly after snapshot boundary**

---

## 7. Snapshot + WAL Consistency Rule

If snapshot boundary commit is `C_snap`:

* WAL replay must begin at:

  ```
  CommitId > C_snap
  ```
* WAL entries ≤ `C_snap` must **not** be replayed
* WAL entries > `C_snap` must be contiguous

Violation → fatal error.

---

## 8. Replica WAL Validation

Before applying replicated WAL, Replica must verify:

* WAL record integrity
* CommitId monotonicity
* No divergence from local history
* No conflicting records

If validation fails → replication halts.

---

## 9. Interaction with MVCC

Replication must preserve:

* CommitId assignment order
* Version creation order
* Snapshot boundaries
* GC eligibility semantics

Replication must **not**:

* Recompute MVCC state
* Skip MVCC metadata
* Compact history implicitly

Replica MVCC state must match Primary for the same WAL prefix.

---

## 10. Crash Semantics During Replication

### 10.1 Crash During WAL Transfer

If Replica crashes:

* Before WAL append → record is lost, retransmission required
* After WAL append → record is durable

Recovery replays WAL deterministically.

---

### 10.2 Crash During Snapshot Transfer

If crash occurs:

* Snapshot is either:

  * Fully applied, or
  * Fully discarded

No partial snapshot state is valid.

---

## 11. Explicitly Forbidden Behaviors

Replication must **never**:

* Apply WAL out of order
* Skip WAL records
* “Fill gaps” heuristically
* Reconstruct WAL from state
* Apply WAL without checksum verification
* Continue replication after divergence

Correctness > liveness.

---

## 12. Log Flow Summary

Replication log flow guarantees:

* Single authoritative history
* Deterministic state propagation
* Explicit detection of inconsistency
* Safe lag, never silent divergence

> **If history cannot be proven identical, replication must stop.**
