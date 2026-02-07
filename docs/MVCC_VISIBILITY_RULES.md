# MVCC_VISIBILITY.md

## AeroDB — MVCC Visibility & Isolation Semantics

### Status

* This document is **authoritative**
* It formally defines **what a reader can observe**
* All rules are deterministic and invariant-driven
* No implementation details appear here

---

## 1. Scope of This Document

This document defines:

* Read visibility rules
* Read view structure
* Isolation semantics provided by MVCC
* What outcomes are **allowed** and **forbidden**

This document does **not** define:

* Locking strategies
* Conflict detection
* Transaction execution models
* Performance optimizations

---

## 2. Fundamental Definitions

### 2.1 Commit Identity

A **commit identity** is a totally ordered identifier assigned to a committed write set.

Properties:

* Strict total order
* Deterministic across crashes
* Monotonic with respect to WAL replay
* Independent of wall-clock time

No two commits share the same commit identity.

---

### 2.2 Read View

A **read view** is defined by a single scalar value:

* **`read_upper_bound`**

This value represents:

> “The maximum commit identity visible to this read.”

All versions with commit identities **greater than** this value are invisible.

---

## 3. Visibility Rule (Formal)

Given:

* A read view `R`
* A logical document key `K`
* A version chain `V₀ … Vₙ` ordered by commit identity (ascending)

The visible version `V*` is defined as:

1. Consider only versions where
   `V.commit_id ≤ R.read_upper_bound`
2. From those, select the version with the **largest commit_id**
3. If that version is a tombstone, `K` is invisible

This rule is **absolute** and admits no exceptions.

---

## 4. Isolation Level Provided

### 4.1 Guaranteed Isolation: **Snapshot Isolation (Deterministic)**

AeroDB MVCC provides **deterministic snapshot isolation** with the following guarantees:

* Readers observe a stable snapshot
* Reads never block writes
* Writes never block reads for correctness
* No dirty reads
* No non-repeatable reads
* No phantom visibility changes within a read

This isolation level is **explicit** and **fixed**.

---

### 4.2 Explicitly Not Provided

MVCC does **not** provide:

* Read Uncommitted
* Read Committed
* Serializable isolation (yet)
* Time-travel queries (unless explicitly added later)

If an isolation level is not specified, it does not exist.

---

## 5. Visibility Across Operations

### 5.1 Point Reads

* Visibility is determined exactly once
* Subsequent reads of the same key return the same version
* Repeated reads are stable

---

### 5.2 Range Queries

* Range queries use **one read view**
* All keys in the range are evaluated against the same visibility boundary
* No key may reflect a later commit than another

---

### 5.3 Index Interaction (Conceptual)

Indexes:

* Must respect read view visibility
* Must not surface versions invisible to the read view
* Are conceptually filtered by visibility

Index correctness is subordinate to visibility correctness.

---

## 6. Write Visibility

### 6.1 Uncommitted Writes

* Uncommitted versions are **never visible**
* Not to the writer
* Not to other readers
* Not across crashes

There is no concept of “read your own uncommitted write.”

---

### 6.2 Committed Writes

* All versions from a committed write set become visible **atomically**
* Visibility begins strictly after commit identity assignment
* Partial visibility is forbidden

---

## 7. Cross-Transaction Observations

Given two read operations `R1` and `R2`:

* If `R1.read_upper_bound == R2.read_upper_bound`
  → They must observe identical visibility
* If `R1.read_upper_bound < R2.read_upper_bound`
  → `R2` may see strictly more versions, never fewer

Visibility is monotonic across read views.

---

## 8. Crash Safety & Visibility

After a crash and recovery:

* All committed versions with commit identities ≤ replay boundary are visible
* No version may appear without its commit record
* Visibility rules are reapplied identically

Crashes do not introduce new visibility states.

---

## 9. Forbidden Visibility Behaviors

MVCC must **never** allow:

* Partial visibility of a transaction
* Time-based visibility
* Thread-dependent visibility
* Heuristic snapshot selection
* Visibility influenced by index state
* Visibility differences between equivalent queries

If it cannot be expressed as a visibility rule, it is invalid.

---

## 10. Visibility Summary

AeroDB MVCC visibility is:

* Deterministic
* Snapshot-based
* Commit-identity-driven
* Crash-safe
* Explicitly limited

There is exactly **one** read model.
