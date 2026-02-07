# MVCC.md

## AeroDB — MVCC Conceptual Model

### Status

* This document defines the **conceptual MVCC model**
* It specifies **what exists**, not how it is implemented
* No algorithms, data structures, or optimizations appear here
* All Phase-1 and Phase-2 invariants apply

---

## 1. Purpose of MVCC in AeroDB

MVCC exists to allow **concurrent, correct access** to data while preserving:

* Deterministic behavior
* Crash safety
* Historical correctness
* Read stability

MVCC is the mechanism by which AeroDB reasons about **time**, **visibility**, and **concurrency** without introducing ambiguity or heuristics.

---

## 2. Fundamental MVCC Concepts

### 2.1 Version

A **version** is a logically immutable representation of a document at a specific point in the database’s history.

A version has:

* A complete document payload **or** an explicit tombstone
* A commit identity (defined later)
* Deterministic visibility semantics

Once created, a version **never changes**.

---

### 2.2 Version Chain

For any logical document key:

* Versions form a **total order**
* Each version supersedes exactly one prior version (if any)
* There are no forks or branches

The version chain represents the full history of a document.

---

### 2.3 Commit Identity

Every committed version is associated with a **commit identity** that:

* Totally orders all commits
* Is deterministic across crashes and recovery
* Is independent of wall-clock time

The commit identity is the **only authority** for ordering versions.

---

## 3. Read Views

### 3.1 Definition

A **read view** is a stable description of which committed versions are visible to a read operation.

A read view:

* Is established at read start
* Never changes during the read
* Is defined purely in terms of commit identities

---

### 3.2 Read View Properties

A read view guarantees:

* No partial writes are visible
* No future commits are visible
* Visibility decisions are deterministic

Two reads with the same read view **must** observe identical results.

---

## 4. Writes and Version Creation

### 4.1 Write Semantics

A write operation:

* Does not modify existing versions
* Produces one or more new versions
* Becomes visible only after commit

Writes are **atomic** with respect to visibility.

---

### 4.2 Deletes

Deletes are represented as:

* Explicit tombstone versions
* Fully ordered in the version chain
* Subject to the same visibility rules as updates

There is no implicit deletion.

---

## 5. Transactions (Conceptual)

MVCC introduces the concept of a **transaction** purely as a logical grouping:

* A transaction may produce multiple versions
* Visibility of all versions is atomic at commit
* Transactions do not imply locking semantics in this document

Transaction behavior is defined by visibility rules, not execution strategy.

---

## 6. Visibility Rule (Conceptual)

For a given read view and a version chain:

* The visible version is the **latest committed version**
* Whose commit identity is **≤ the read view’s upper bound**
* And is not superseded by another visible version

This rule is absolute and admits no exceptions.

---

## 7. Interaction with Phase-1 Model

Without MVCC:

* Each document had exactly one visible version

With MVCC:

* Phase-1 behavior is equivalent to:

  * A single-version chain
  * A read view that always sees the latest commit

Thus, MVCC strictly generalizes Phase-1 semantics without altering them.

---

## 8. Crash and Recovery Perspective

From a crash-recovery standpoint:

* A version either exists fully or not at all
* Commit identity assignment is WAL-governed
* Recovery reconstructs version chains deterministically

There is no concept of “in-doubt” visibility after recovery.

---

## 9. Explicit Non-Goals

This document does **not** define:

* Isolation levels
* Locking or conflict detection
* Storage layout
* Garbage collection
* Performance characteristics

Those are handled in subsequent documents.

---

## 10. Conceptual Summary

MVCC in AeroDB introduces:

* Immutable version history
* Deterministic commit ordering
* Stable read views
* Explicit visibility rules

All without compromising:

* Durability
* Determinism
* Recoverability
* Phase-1 guarantees