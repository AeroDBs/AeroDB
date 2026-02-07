# REPLICATION_COMPATIBILITY.md

## AeroDB — Replication Compatibility & Invariant Preservation

### Status

* This document is **authoritative**
* It proves replication does **not** alter existing semantics
* All compatibility guarantees are **non-negotiable**
* No implementation details appear here
* Phase-1 and MVCC semantics are **frozen and preserved**

---

## 1. Purpose of Compatibility Guarantees

Replication must integrate with AeroDB such that:

* All existing guarantees remain true
* No subsystem requires reinterpretation
* No “replication-only” semantics exist

If replication changes meaning, it is incorrect.

---

## 2. Phase-1 Compatibility (Unchanged)

Replication must preserve **all Phase-1 guarantees exactly**.

### 2.1 WAL Semantics

* WAL remains the sole durability authority
* fsync semantics are unchanged
* WAL replay rules are identical
* Replication does not introduce WAL shortcuts

A non-replicated node and a Primary must behave identically.

---

### 2.2 Storage Semantics

* Append-only storage invariants remain intact
* Checksums remain mandatory
* Tombstone semantics are unchanged
* Corruption is detected and halts execution

Replication must not introduce silent repair paths.

---

### 2.3 Query Engine Semantics

* Parsing, planning, and execution are unchanged
* Determinism and boundedness are preserved
* Query results are identical for identical read views

Replication influences **where** reads occur, not **what** they return.

---

## 3. MVCC Compatibility (Unchanged)

Replication must preserve **all MVCC invariants**.

### 3.1 CommitId Semantics

* CommitIds are globally ordered
* CommitIds are immutable
* CommitIds originate only from the Primary
* Replicas must not infer or modify CommitIds

CommitId meaning is identical on all nodes.

---

### 3.2 Visibility Semantics

* Snapshot isolation rules are unchanged
* Read view construction rules are unchanged
* Visibility resolution is identical on Primary and Replica

Replication must not weaken isolation.

---

### 3.3 Garbage Collection Semantics

* GC eligibility rules remain unchanged
* Snapshots impose identical retention constraints
* WAL-recorded GC events are replayed identically

Replication must not introduce “local-only GC”.

---

## 4. Snapshot Compatibility

Replication must preserve snapshot semantics:

* Snapshots represent valid MVCC cuts
* Snapshot commit boundaries are authoritative
* Snapshot restore behavior is identical
* Snapshot format and validation rules are unchanged

Snapshot transfer does not reinterpret snapshots.

---

## 5. Backup & Restore Compatibility

Replication must not alter:

* Backup determinism
* Restore atomicity
* Restore validation rules

Backups taken on a Primary or Replica must be:

* Semantically equivalent
* Restore-identical

Replication does not create a new backup class.

---

## 6. Observability Compatibility

* Logging remains side-effect free
* Metrics remain observational only
* Replication state may be observed
* Replication state must not influence behavior

Observability must not affect correctness.

---

## 7. Configuration Compatibility

* Replication configuration must be explicit
* No implicit defaults
* No adaptive role changes

Misconfiguration must result in refusal, not guesswork.

---

## 8. Crash Testing Compatibility

Replication must integrate with:

* Existing crash harness
* Deterministic kill points
* Phase-1 and MVCC crash matrices

Replication crashes must produce outcomes consistent with:

* Phase-1 crash semantics
* MVCC crash semantics
* Replication failure matrix

---

## 9. Explicitly Forbidden Compatibility Breaks

Replication must **never**:

* Change Phase-1 observable behavior
* Change MVCC semantics
* Introduce new durability meanings
* Introduce timing-based behavior
* Introduce best-effort fallbacks

Any such change is a correctness violation.

---

## 10. Compatibility Summary

Replication compatibility guarantees:

* Phase-1 remains authoritative
* MVCC remains frozen
* All semantics are preserved
* Replication is a transparent extension

> **Replication adds nodes, not meanings.**
