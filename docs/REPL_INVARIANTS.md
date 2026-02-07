# REPLICATION_INVARIANTS.md

## AeroDB Phase 2B — Replication Invariants

### Status

* This document is **authoritative**
* All invariants herein are **non-negotiable**
* Phase-1 and MVCC invariants remain **fully in force**
* Any behavior violating an invariant is a **fatal correctness bug**
* No implementation details appear here

---

## 1. Invariants Preserved from Earlier Phases (Unchanged)

Replication **must not weaken, reinterpret, or bypass** any existing invariant.

### 1.1 Phase-1 Durability Invariants (UNCHANGED)

* A write acknowledged by the system is never lost
* WAL fsync semantics remain the definition of durability
* No background, deferred, or “eventual” durability is permitted
* Replication does not redefine what “durable” means

If replication is disabled, behavior must be **identical** to Phase-1.

---

### 1.2 Phase-1 Recovery Invariants (UNCHANGED)

* Recovery is deterministic
* WAL replay produces exactly one valid state
* Corruption is detected and halts recovery
* No state is guessed, skipped, or healed

Replication must not introduce recovery ambiguity.

---

### 1.3 MVCC Invariants (UNCHANGED)

Replication must preserve **all** MVCC guarantees:

* CommitId total order is authoritative
* CommitIds are immutable and WAL-derived
* Visibility is snapshot-based and deterministic
* Version immutability is absolute
* GC is proof-based and WAL-governed
* MVCC semantics are frozen

Replication **consumes** MVCC — it does not reinterpret it.

---

## 2. Core Replication Invariants (NEW)

These invariants define what replication **is allowed to be**.

---

### 2.1 Single-Writer Invariant

At any moment in time:

* **Exactly one node** may acknowledge writes
* That node is called the **Primary**
* All other nodes are **Replicas**

No two nodes may acknowledge writes concurrently.

If authority is unclear → writes must be rejected.

---

### 2.2 Commit Authority Invariant

* CommitIds are assigned **only** by the Primary
* Replicas must never:

  * Generate CommitIds
  * Renumber CommitIds
  * Compress CommitIds
  * Infer CommitIds

If a Replica observes a CommitId not issued by the Primary → **fatal error**.

---

### 2.3 History Prefix Invariant

For any Replica `R` and Primary `P`:

```
History(R) ⊆ Prefix(History(P))
```

Meaning:

* A Replica’s WAL history must be a **prefix** of the Primary’s history
* Replicas must never diverge, fork, or reorder history
* Replicas may lag, but never invent

Divergence is a fatal condition.

---

### 2.4 Deterministic State Reconstruction Invariant

Given:

* The same WAL prefix
* The same snapshot (if any)

All nodes must reconstruct **identical state**, including:

* Version chains
* MVCC metadata
* GC state
* Snapshot boundaries

Replication must not introduce nondeterminism.

---

### 2.5 No Invisible Divergence Invariant

If replication state differs between nodes:

* The difference must be **detectable**
* The system must fail explicitly
* Silent divergence is forbidden

“No news” is not correctness.

---

### 2.6 Replica Read Safety Invariant

A Replica may serve reads **only if**:

* The read view is ≤ the Replica’s applied CommitId
* MVCC visibility rules can be satisfied locally
* No future commits are implied or assumed

If safety cannot be proven → reads must be refused.

---

### 2.7 WAL Completeness Invariant

Replication must ensure:

* WAL records are transferred **in order**
* WAL records are transferred **without gaps**
* Partial WAL application is detectable

If a WAL gap exists → Replica must stop.

---

### 2.8 Snapshot Authority Invariant

Snapshots used for replication must:

* Represent a valid MVCC cut
* Be self-contained
* Carry explicit commit boundaries
* Never depend on implicit WAL state

Snapshot + WAL replay must be sufficient to reconstruct state.

---

### 2.9 Replication Crash Safety Invariant

At any crash point:

* A node must recover into a state that satisfies **all invariants**
* Partial replication state must be:

  * Either fully applied
  * Or fully discarded

Replication must not create “in-between” states.

---

### 2.10 No Heuristic Progress Invariant

Replication must never:

* Guess leader status
* Assume liveness
* Rely on timing, heartbeats, or clocks
* Auto-resolve conflicts

If progress cannot be proven → replication halts.

---

## 3. Explicitly Forbidden Replication Behaviors

Replication must **never**:

* Allow two Primaries concurrently
* Allow Replicas to acknowledge writes
* Reorder WAL records
* Skip WAL records
* “Catch up” by inference
* Heal divergence automatically
* Expose uncommitted or speculative state
* Downgrade durability semantics

Any such behavior is a correctness violation.

---

## 4. Invariant Enforcement Philosophy

Replication invariants are enforced by:

* WAL structure
* Explicit state transitions
* Recovery logic
* Crash testing

Not by:

* Monitoring
* Background reconciliation
* Operational assumptions

---

## 5. Invariant Summary

Replication in AeroDB guarantees:

* Single authoritative history
* Deterministic replication
* MVCC-correct reads
* Explicit failure on uncertainty
* No hidden or heuristic behavior

> **Replication extends correctness — it never negotiates it.**

