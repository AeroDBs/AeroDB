# REPLICATION_VISION.md

## AeroDB Phase 2B — Replication Vision

### Status

* This document is **authoritative**
* It defines **intent, scope, and boundaries**
* No implementation details appear here
* Phase-1 and MVCC semantics are **frozen and inviolable**

---

## 1. Purpose of Replication in AeroDB

Replication exists to extend AeroDB from a **single-node, correctness-complete database** into a **multi-node system** without weakening any existing guarantees.

Replication in AeroDB is not a scaling shortcut.
It is a **durability, availability, and correctness extension**.

The primary goal is **state continuity**, not throughput.

---

## 2. Non-Negotiable Continuity

Replication must preserve **all existing guarantees**:

### Phase-1 (unchanged)

* No acknowledged write is ever lost
* WAL is the sole durability authority
* Recovery is deterministic
* Corruption is detected, never repaired silently
* Queries are bounded and deterministic
* Snapshot, checkpoint, backup, restore semantics remain intact

### Phase-2 MVCC (unchanged)

* CommitId total order is authoritative
* Visibility is deterministic and snapshot-based
* Write atomicity is absolute
* GC rules are proof-based and WAL-governed
* MVCC semantics are **frozen**

Replication **consumes** these properties.
It does not reinterpret them.

---

## 3. Replication Philosophy

Replication in AeroDB is:

* **Deterministic** — same WAL ⇒ same state everywhere
* **Explicit** — no hidden quorum rules or timing tricks
* **Fail-stop** — uncertainty results in refusal, not guesses
* **History-preserving** — replicas never rewrite or “heal” history
* **MVCC-aware** — version semantics are preserved exactly

Replication correctness is valued **above availability**.

---

## 4. What Replication Means in AeroDB

At a high level, replication introduces:

* **A single Primary**

  * Sole authority for accepting writes
  * Sole assigner of CommitIds
* **One or more Replicas**

  * Receive ordered state from the Primary
  * Never generate new history
* **Deterministic State Propagation**

  * WAL-based replication
  * Explicit snapshot transfer when needed

Replication is **state replication**, not command replication.

---

## 5. Explicit Non-Goals

Replication in this phase does **not** attempt to provide:

* Multi-leader writes
* Automatic leader election
* Write availability during primary loss
* Eventual consistency
* Conflict resolution
* Byzantine fault tolerance

If a behavior is not specified, it does not exist.

---

## 6. Replication Safety Model

Replication must obey the following high-level safety rules:

1. **Single Writer Rule**
   Only one node may acknowledge writes at any time.

2. **CommitId Authority Rule**
   CommitIds are assigned **only** by the Primary.

3. **History Prefix Rule**
   A Replica’s state must always be a prefix of the Primary’s history.

4. **No Guessing Rule**
   If ordering, completeness, or authority is uncertain → replication halts.

5. **No Invisible Divergence Rule**
   Divergence must be detectable and fatal, never silent.

---

## 7. Replication and Durability

Replication does **not** redefine durability.

* A write is durable when:

  * WAL fsync semantics are satisfied **per policy**
* Replication may **strengthen** durability guarantees
* Replication must never weaken them

Acknowledgement semantics will be defined explicitly later.

---

## 8. Replication and MVCC

Replication must preserve:

* CommitId ordering
* Version immutability
* Snapshot isolation
* GC eligibility rules

A Replica must produce **identical visibility outcomes** for the same read view boundary.

MVCC correctness is replicated, not recomputed.

---

## 9. Crash & Failure Perspective

Replication must be correct under:

* Primary crash
* Replica crash
* Network partition
* Partial WAL transfer
* Snapshot transfer interruption

There are **no undefined states**.
Every failure must map to a specified outcome.

---

## 10. Design Deferrals

This document intentionally does **not** define:

* WAL shipping mechanics
* Snapshot vs log transfer rules
* Replica read permissions
* Failure handling algorithms
* Promotion or leadership changes

Those belong in subsequent replication documents.

---

## 11. Vision Summary

Replication in AeroDB aims to:

* Extend correctness across nodes
* Preserve determinism end-to-end
* Make failure explicit and survivable
* Avoid all heuristic behavior
* Remain compatible with frozen MVCC semantics

> **Replication is not an optimization.
> It is correctness, extended in space.**

