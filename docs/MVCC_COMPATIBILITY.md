# MVCC_COMPATIBILITY.md

## AeroDB — MVCC Compatibility & System Integration

### Status

* This document is **authoritative**
* It defines how MVCC coexists with existing Phase-1 subsystems
* Phase-1 behavior is preserved exactly
* No implementation details appear here

---

## 1. Compatibility Principles

MVCC must integrate with AeroDB such that:

* All Phase-1 subsystems continue to behave **as specified**
* MVCC adds semantics without altering existing guarantees
* Any incompatibility is treated as a **design error**

MVCC adapts to AeroDB — not the other way around.

---

## 2. Query Engine Compatibility

### 2.1 Parser & Planner

* Query parsing is unchanged
* Query planning remains deterministic and bounded
* MVCC does not introduce new query syntax at this stage

Visibility is applied **after planning**, not during parsing.

---

### 2.2 Executor Semantics

* The executor evaluates queries against:

  * A fixed read view
  * MVCC visibility rules
* Executor behavior must be:

  * Deterministic
  * Independent of execution order

MVCC influences *what* is seen, never *how* queries are executed.

---

## 3. Index Compatibility

### 3.1 Conceptual Index Semantics

Indexes under MVCC are:

* Derived structures
* Visibility-aware
* Subordinate to version visibility rules

Indexes must never surface versions invisible to the active read view.

---

### 3.2 Rebuild-on-Start Compatibility

Phase-1 index behavior remains unchanged:

* Indexes may be rebuilt on startup
* Rebuilds must respect MVCC visibility
* WAL replay reconstructs version chains first

Index correctness follows MVCC correctness.

---

## 4. Schema System Compatibility

### 4.1 Schema Validation

* Schema validation applies per version
* Each version must:

  * Conform to the active schema at commit time
* Historical versions are validated under their commit schema

Schema correctness is immutable once committed.

---

### 4.2 Schema Evolution

* Schema versioning does not alter MVCC semantics
* Visibility rules are independent of schema changes
* Schema mismatches are explicit errors

MVCC does not “reinterpret” old data under new schemas.

---

## 5. Backup & Restore Compatibility

### 5.1 Backup Semantics

Backups must include:

* All versions visible at backup boundary
* MVCC metadata
* Snapshot commit boundary

Backups remain deterministic archives.

---

### 5.2 Restore Semantics

Restore must:

* Reconstruct full MVCC state
* Preserve commit identities
* Produce identical visibility outcomes

Restore never rewrites history.

---

## 6. Observability Compatibility

* Observability systems:

  * Report MVCC state
  * Never influence behavior
* MVCC events must be observable:

  * Version creation
  * Commit
  * GC

Observability remains side-effect free.

---

## 7. Configuration Compatibility

* MVCC behavior is:

  * Explicitly configured
  * Fully spec-governed
* No hidden defaults
* No adaptive behavior

If MVCC is enabled, its rules apply uniformly.

---

## 8. Phase-1 Behavioral Equivalence

When MVCC is enabled but unused:

* Single-version behavior is preserved
* Queries behave exactly as Phase-1
* Performance aside, results are identical

Phase-1 semantics are a strict subset of MVCC semantics.

---

## 9. Explicitly Forbidden Compatibility Breaks

MVCC must **never**:

* Change query results under identical inputs
* Alter WAL durability semantics
* Introduce silent schema coercion
* Affect snapshot or restore guarantees
* Bypass existing validation logic

Any such change is a regression.

---

## 10. Compatibility Summary

MVCC integrates with AeroDB by:

* Preserving all Phase-1 guarantees
* Respecting existing subsystem contracts
* Applying visibility rules consistently
* Avoiding any reinterpretation of history

Compatibility is correctness.
