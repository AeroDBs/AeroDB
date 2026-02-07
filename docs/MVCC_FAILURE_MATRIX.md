# MVCC_FAILURE_MATRIX.md

## AeroDB — MVCC Failure & Crash Matrix

### Status

* This document is **authoritative**
* It enumerates all meaningful MVCC failure points
* Every outcome is deterministic and invariant-preserving
* No implementation details appear here

---

## 1. Purpose of the Failure Matrix

This document defines:

* Where crashes may occur
* What state may exist at each crash point
* What recovery **must** produce

There are **no undefined states**.
Every failure maps to a valid, explainable recovery outcome.

---

## 2. Failure Classification

All MVCC-related failures fall into one of four categories:

1. **Before commit intent**
2. **After commit intent, before commit identity**
3. **After commit identity, before full persistence**
4. **After full persistence**

Each category has strict recovery semantics.

---

## 3. Write Lifecycle Failure Points

### 3.1 Failure Before Commit Intent

**Crash Point**

* Before any commit intent is recorded in WAL

**State at Crash**

* No MVCC metadata exists
* No versions are authoritative

**Recovery Outcome**

* Write does not exist
* No versions are visible
* No cleanup required

✅ Invariants preserved.

---

### 3.2 Failure After Commit Intent, Before Commit Identity

**Crash Point**

* Commit intent recorded
* No commit identity recorded

**State at Crash**

* Potential version data may exist
* No visibility authority exists

**Recovery Outcome**

* Write is discarded
* Any partial version data is ignored
* No versions become visible

Commit intent without identity has **no effect**.

---

### 3.3 Failure After Commit Identity, Before Version Durability

**Crash Point**

* Commit identity recorded
* One or more versions not fully durable

**State at Crash**

* Commit identity exists
* Version data may be incomplete

**Recovery Outcome**

* Recovery must detect inconsistency
* Recovery must **fail explicitly**
* No partial visibility is allowed

Silent repair is forbidden.

---

### 3.4 Failure After Full Commit Persistence

**Crash Point**

* Commit identity recorded
* All versions fully durable

**State at Crash**

* Complete MVCC state exists

**Recovery Outcome**

* All versions are visible
* Version chains are intact
* Visibility rules apply normally

This is the success case.

---

## 4. Read View Failure Scenarios

### 4.1 Crash During Read View Creation

**Crash Point**

* While establishing read view

**Recovery Outcome**

* Read operation is aborted
* No persistent state is affected

Reads do not mutate MVCC state.

---

### 4.2 Crash During Read Execution

**Crash Point**

* While reading visible versions

**Recovery Outcome**

* Read is aborted
* No visibility state is altered
* MVCC state remains unchanged

Reads are restart-safe.

---

## 5. Snapshot-Related Failures

### 5.1 Crash Before Snapshot Boundary Selection

**Recovery Outcome**

* Snapshot does not exist
* WAL remains authoritative

---

### 5.2 Crash After Snapshot Boundary, Before Snapshot Completion

**Recovery Outcome**

* Snapshot is discarded
* WAL replay resumes normally

Partial snapshots are invalid.

---

### 5.3 Crash After Snapshot Completion

**Recovery Outcome**

* Snapshot is valid
* MVCC boundary is authoritative
* WAL replay resumes after snapshot boundary

---

## 6. Garbage Collection Failures

### 6.1 Crash Before GC Record

**Recovery Outcome**

* No versions are removed
* GC is effectively rolled back

---

### 6.2 Crash After GC Record

**Recovery Outcome**

* Version removal is replayed
* Versions remain collected
* No resurrection is allowed

---

## 7. WAL Truncation Failures

### 7.1 Crash Before Truncation

**Recovery Outcome**

* WAL remains intact
* Recovery proceeds normally

---

### 7.2 Crash After Truncation

**Recovery Outcome**

* Snapshot provides full MVCC state
* No visibility loss occurs

---

## 8. Forbidden Recovery Outcomes

Recovery must **never** produce:

* Partially visible commits
* Orphaned versions
* Missing commit identities
* Reordered commits
* Healed corruption

Any such state is a fatal error.

---

## 9. Determinism Guarantee

For any failure point:

* Recovery outcome is unique
* No ambiguity exists
* No heuristic decisions are made

Crash behavior is a function of WAL state alone.

---

## 10. Summary

MVCC failure handling in AeroDB ensures:

* No acknowledged commit is lost
* No uncommitted state becomes visible
* No crash introduces ambiguity
* All invariants are preserved
