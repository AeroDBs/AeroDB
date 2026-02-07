# MVCC_WAL_INTERACTION.md

## AeroDB — MVCC and WAL Interaction

### Status

* This document is **authoritative**
* It defines how MVCC state is made durable and recoverable
* WAL is the sole source of truth
* No implementation-level encoding is specified

---

## 1. Role of the WAL in MVCC

In AeroDB, the Write-Ahead Log (WAL) is:

* The **only authority** on committed state
* The source of deterministic ordering
* The mechanism by which MVCC survives crashes

All MVCC-relevant state **must** be represented in the WAL.

---

## 2. Commit Identity Assignment

### 2.1 Deterministic Assignment

* Commit identities are assigned **exactly once**
* Assignment occurs as part of commit
* The ordering of commit identities is:

  * Total
  * Strict
  * Replayable

No commit identity exists outside the WAL.

---

### 2.2 Monotonicity

* Commit identities are strictly increasing
* Gaps are allowed only if explicitly represented
* Ordering must be reconstructible solely from WAL order

Commit identity order is **derived**, not inferred.

---

## 3. WAL Records and MVCC State

### 3.1 Required MVCC WAL Semantics

The WAL must record, in an unambiguous sequence:

1. Intent to commit a write set
2. The versions created by that write set
3. The commit identity assignment

Recovery must be able to reconstruct:

* Exact version chains
* Exact commit ordering
* Visibility boundaries

---

### 3.2 Atomic Visibility Guarantee

* Visibility is tied to commit identity durability
* A version becomes visible **only after** its commit identity is durable
* WAL fsync is the visibility barrier

If a commit identity is not durable, the commit does not exist.

---

## 4. Crash Scenarios

### 4.1 Crash Before Commit Record

If a crash occurs:

* Before commit identity is persisted:

  * No versions are visible
  * No partial MVCC state survives
* Recovery discards any incomplete version data

---

### 4.2 Crash After Commit Record

If a crash occurs:

* After commit identity persistence:

  * All versions associated with that commit are visible
  * Recovery must reconstruct full version chains

There is no intermediate state.

---

## 5. Interaction with Checkpointing

* Checkpoints represent a stable MVCC cut
* All versions visible at checkpoint time must be included
* Commit identities beyond the checkpoint boundary are excluded

Checkpointing does not alter MVCC semantics.

---

## 6. Interaction with Snapshots

* Snapshots capture:

  * Version data
  * MVCC metadata
  * Commit identity boundary
* Snapshots must be sufficient to:

  * Serve read views
  * Resume WAL replay

Snapshots are self-contained MVCC states.

---

## 7. WAL Replay Rules

During recovery:

1. WAL is replayed in strict order
2. Commit identities are re-established deterministically
3. Version chains are reconstructed
4. Visibility rules are reapplied

Recovery does not:

* Reassign commit identities
* Infer missing data
* Skip MVCC records

---

## 8. Garbage Collection Interaction

* GC decisions must be WAL-represented
* Version removal must be replayable
* GC must never:

  * Remove versions needed for recovery
  * Alter commit identity ordering

GC is subordinate to WAL correctness.

---

## 9. Explicitly Forbidden WAL Behaviors

The WAL must **never**:

* Contain ambiguous commit boundaries
* Implicitly encode MVCC state
* Allow out-of-order visibility
* Allow visibility without durability

If MVCC state is not in the WAL, it does not exist.

---

## 10. Summary

MVCC relies on WAL to provide:

* Deterministic commit ordering
* Crash-safe visibility
* Recoverable version history

The WAL remains the single source of truth.

