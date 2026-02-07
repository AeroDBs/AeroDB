# MVCC_TESTING_STRATEGY.md

## AeroDB — MVCC Testing Strategy

### Status

* This document is **authoritative**
* It defines how MVCC correctness is proven
* All tests are deterministic and reproducible
* No performance benchmarks appear here

---

## 1. Testing Philosophy

MVCC testing in AeroDB exists to prove:

* Correctness under concurrency
* Determinism under all execution orders
* Crash safety at every boundary
* Invariant preservation across lifecycle events

Tests do **not** attempt to infer correctness from performance or timing.

---

## 2. Core Test Categories

### 2.1 Visibility Correctness Tests

These tests verify that visibility rules are applied exactly as specified.

**Required Coverage**

* Single-key reads across multiple versions
* Read views before, during, and after commits
* Tombstone visibility behavior
* Range queries with mixed version chains

**Assertions**

* Visible version matches `MVCC_VISIBILITY.md`
* No future commit leakage
* Stability across repeated reads

---

### 2.2 Snapshot Isolation Tests

These tests verify snapshot isolation semantics.

**Required Coverage**

* Concurrent reads and writes
* Reads spanning long write sequences
* Identical read views produce identical results
* Monotonic visibility across read views

**Assertions**

* No non-repeatable reads
* No phantom visibility changes
* Deterministic results regardless of scheduling

---

## 3. WAL & Recovery Tests

### 3.1 Commit Durability Tests

**Scenarios**

* Crash before commit intent
* Crash after commit intent
* Crash after commit identity
* Crash after full persistence

**Assertions**

* Visibility outcomes match `MVCC_FAILURE_MATRIX.md`
* No partial commits appear
* No silent repair occurs

---

### 3.2 WAL Replay Determinism Tests

**Scenarios**

* Replaying identical WAL multiple times
* Replay with interleaved MVCC and non-MVCC records

**Assertions**

* Identical MVCC state reconstructed
* Commit identity ordering preserved

---

## 4. Snapshot & Checkpoint Tests

### 4.1 Snapshot Integrity Tests

**Scenarios**

* Snapshot creation during concurrent writes
* Snapshot restore without WAL
* Snapshot restore with WAL continuation

**Assertions**

* Snapshot serves correct read views
* Snapshot boundaries are respected
* No missing or extra versions

---

### 4.2 Checkpoint Interaction Tests

**Scenarios**

* Checkpoint followed by crash
* Checkpoint followed by WAL truncation
* Checkpoint followed by restore

**Assertions**

* MVCC state remains correct
* No visibility loss
* WAL truncation is safe

---

## 5. Garbage Collection Tests

### 5.1 Eligibility Proof Tests

**Scenarios**

* Versions below visibility floor
* Versions above visibility floor
* Snapshot-retained versions

**Assertions**

* Only reclaimable versions are collected
* Live or obsolete versions are retained

---

### 5.2 GC Crash Tests

**Scenarios**

* Crash before GC record
* Crash after GC record

**Assertions**

* GC behavior is replay-safe
* No version resurrection
* No premature deletion

---

## 6. Concurrency Stress Tests (Deterministic)

Concurrency tests must:

* Use controlled scheduling
* Enumerate execution interleavings
* Avoid timing-based assertions

**Scenarios**

* Multiple readers + writers
* Long-lived readers with short writes
* Write-heavy sequences with GC enabled

---

## 7. Crash Injection Strategy

MVCC tests must integrate with the existing crash harness:

* Kill points at:

  * Commit intent
  * Commit identity
  * Version persistence
  * Snapshot boundaries
  * GC record points
* Deterministic recovery assertions

Crash testing is mandatory, not optional.

---

## 8. Negative Testing

Tests must explicitly verify failure cases:

* Corrupted MVCC metadata
* Missing WAL records
* Incomplete snapshots
* Invalid visibility boundaries

Expected outcome: **explicit failure**, never silent success.

---

## 9. Phase-1 Regression Guard

All Phase-1 tests must be re-run with MVCC enabled.

Assertions:

* Identical query results
* Identical durability guarantees
* No behavioral regressions

MVCC must be transparent to Phase-1 behavior.

---

## 10. Testing Summary

MVCC correctness is proven by:

* Deterministic unit tests
* Structured crash tests
* Explicit failure assertions
* Regression coverage

If a behavior is untestable, it is unspecifiable.
