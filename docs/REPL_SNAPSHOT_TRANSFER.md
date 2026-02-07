# REPLICATION_SNAPSHOT_TRANSFER.md

## AeroDB — Replication Snapshot Transfer Semantics

### Status

* This document is **authoritative**
* It defines **snapshot-based replication bootstrap**
* It preserves **MVCC snapshot semantics exactly**
* No transport, protocol, or implementation details appear here
* Phase-1 and MVCC invariants are **fully assumed and preserved**

---

## 1. Purpose of Snapshot Transfer in Replication

Snapshot transfer exists to solve **one problem only**:

> Efficiently initializing or re-synchronizing a Replica **without replaying the entire WAL history**,
> **without altering correctness semantics**.

Snapshot transfer is **not** an optimization of semantics.
It is a **logically equivalent alternative** to WAL-only bootstrap.

---

## 2. Snapshot Authority in Replication

A snapshot used for replication:

* Is produced **only by the Primary**
* Represents a **valid MVCC cut**
* Is identical in meaning to a local snapshot
* Is authoritative up to its commit boundary

Replica must treat the snapshot as **ground truth** up to that boundary.

---

## 3. Snapshot Eligibility for Transfer

A snapshot is eligible for replication transfer **if and only if**:

1. It was created by the Primary
2. It has a valid `snapshot_commit_boundary`
3. It is complete and self-contained
4. It passed all snapshot integrity checks
5. It has not been superseded or invalidated

Any uncertainty → snapshot must be rejected.

---

## 4. Snapshot Transfer Semantics (Conceptual)

Snapshot transfer is defined abstractly as:

1. Primary selects a snapshot with boundary `C_snap`
2. Snapshot data is transferred to the Replica
3. Replica validates snapshot integrity
4. Replica installs snapshot atomically
5. Replica resumes WAL replay at:

   ```
   CommitId > C_snap
   ```

No step may be skipped or reordered.

---

## 5. Snapshot Installation Rules

### 5.1 Atomicity

Snapshot installation on a Replica must be:

* Atomic
* All-or-nothing
* Crash-safe

If a crash occurs:

* Before completion → snapshot is discarded
* After completion → snapshot is fully authoritative

Partial snapshot state is forbidden.

---

### 5.2 Snapshot Validation on Replica

Before accepting a snapshot, the Replica must validate:

* Snapshot checksum
* Manifest integrity
* Commit boundary consistency
* MVCC metadata completeness
* Absence of implicit WAL dependency

Failure → snapshot rejected explicitly.

---

## 6. WAL Resume Semantics After Snapshot

After snapshot installation:

* Replica must discard:

  * Any WAL state ≤ `C_snap`
* Replica must accept:

  * WAL records with `CommitId > C_snap`
* WAL records must be:

  * Contiguous
  * Ordered
  * Gap-free

Any violation → replication halts.

---

## 7. Snapshot vs Local State Interaction

### 7.1 Empty Replica

If Replica has no prior state:

* Snapshot becomes the initial state
* WAL replay begins strictly after boundary

---

### 7.2 Previously Initialized Replica

If Replica already has state:

* Snapshot may be applied **only if**:

  * Existing state is discarded explicitly
  * No mixed-state reuse occurs

State reuse is forbidden unless explicitly validated (future phase).

---

## 8. Interaction with MVCC & GC

Snapshot transfer must preserve:

* All version chains ≤ boundary
* All tombstones required for visibility
* All MVCC metadata
* GC eligibility constraints

Replica must not:

* Recompute GC state
* Drop versions eagerly
* Apply local GC rules beyond snapshot semantics

GC continues normally after replication resumes.

---

## 9. Crash Scenarios

### 9.1 Crash During Snapshot Transfer

* Snapshot is incomplete
* Replica must discard it
* Replica remains uninitialized or in prior state

---

### 9.2 Crash After Snapshot Install, Before WAL Resume

* Snapshot is authoritative
* WAL replay resumes after restart

No ambiguity is allowed.

---

## 10. Explicitly Forbidden Behaviors

Snapshot-based replication must **never**:

* Merge snapshot with partial WAL state
* Infer missing WAL entries
* Skip validation for speed
* Apply snapshot incrementally
* Modify snapshot contents
* Allow Replica to create snapshots

Correctness > convenience.

---

## 11. Equivalence to WAL-Only Bootstrap

Formally:

> A Replica initialized via snapshot + WAL replay
> must reach **exactly the same state**
> as a Replica initialized via full WAL replay.

If this equivalence cannot be proven → snapshot transfer is invalid.

---

## 12. Snapshot Transfer Summary

Snapshot transfer provides:

* Efficient replication bootstrap
* Deterministic MVCC state
* WAL-compatible continuation
* Crash-safe installation

> **Snapshots shorten time — they never shorten history.**
