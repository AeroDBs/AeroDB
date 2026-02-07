# MVCC_SNAPSHOT_INTEGRATION.md

## AeroDB — MVCC and Snapshot Integration

### Status

* This document is **authoritative**
* It defines how MVCC interacts with snapshots and checkpoints
* Snapshot correctness is non-negotiable
* No implementation details appear here

---

## 1. Purpose of MVCC Snapshots

Snapshots in AeroDB serve two distinct roles:

1. **Read-only durability artifacts**
2. **Recovery acceleration mechanisms**

With MVCC, snapshots additionally define a **stable visibility boundary** for historical versions.

---

## 2. Snapshot as an MVCC Cut

### 2.1 Definition

An **MVCC snapshot** represents:

* A complete database state
* At a specific commit identity boundary
* With all visible versions included

Formally, a snapshot captures all versions with:

```
commit_id ≤ snapshot_commit_boundary
```

No version beyond this boundary exists in the snapshot.

---

### 2.2 Snapshot Completeness Invariant

A snapshot must include:

* All document versions visible at the boundary
* All required MVCC metadata to interpret visibility
* No dangling references to WAL-only state

A snapshot is invalid if any visible version is missing.

---

## 3. Snapshot Creation Rules

### 3.1 Atomicity

* Snapshot creation observes a **single commit boundary**
* That boundary is immutable for the snapshot
* Snapshot creation never observes partial commits

---

### 3.2 Concurrency

* Snapshot creation may run concurrently with writes
* Concurrent commits with commit identities:

  * ≤ boundary → included
  * > boundary → excluded

Visibility rules remain unchanged.

---

## 4. Snapshot Usage for Reads

### 4.1 Read-Only Operation

When serving reads from a snapshot:

* Read views are derived from the snapshot boundary
* No WAL data beyond the snapshot is consulted
* Visibility is fully determined by snapshot state

Snapshots are fully self-contained for reads.

---

### 4.2 Snapshot Isolation Preservation

Reads served from snapshots:

* Provide the same snapshot isolation semantics
* Are indistinguishable from live reads at that boundary
* Do not introduce special cases

---

## 5. Interaction with Checkpoints

### 5.1 Checkpoint Semantics

A checkpoint:

* Produces a snapshot
* Establishes a WAL truncation point
* Preserves MVCC correctness

Checkpointing does not alter MVCC rules.

---

### 5.2 WAL Truncation Safety

WAL entries may be truncated **only if**:

* Their effects are fully represented in the snapshot
* MVCC state remains reconstructible
* No version required for recovery is lost

---

## 6. Snapshot Restore and MVCC

On restore:

* Snapshot MVCC state is loaded first
* WAL replay resumes strictly after snapshot boundary
* Commit identities remain monotonic

Restore produces the same MVCC state as uninterrupted operation.

---

## 7. Garbage Collection Interaction

Snapshots impose **retention constraints**:

* Versions required by any snapshot boundary:

  * Must not be reclaimed
* GC must be snapshot-aware

Snapshots are authoritative visibility anchors.

---

## 8. Crash Safety During Snapshotting

If a crash occurs during snapshot creation:

* Snapshot is either:

  * Fully valid, or
  * Fully discarded
* No partially written snapshot is usable
* MVCC state remains intact via WAL

Snapshots never introduce ambiguity.

---

## 9. Explicitly Forbidden Snapshot Behaviors

Snapshots must **never**:

* Reference WAL state implicitly
* Omit MVCC metadata
* Contain partially visible commits
* Alter visibility semantics

If a snapshot cannot serve correct reads alone, it is invalid.

---

## 10. Summary

MVCC snapshots in AeroDB are:

* Deterministic MVCC cuts
* Self-contained and durable
* Safe for reads and recovery
* Fully consistent with Phase-1 snapshot guarantees

