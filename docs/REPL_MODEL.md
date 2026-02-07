# REPLICATION_MODEL.md

## AeroDB — Replication Authority & Role Model

### Status

* This document is **authoritative**
* It defines **what replication roles exist and what they are allowed to do**
* It defines **authority boundaries and illegal states**
* No implementation details appear here
* Phase-1 and MVCC semantics are **frozen and assumed correct**

---

## 1. Purpose of the Replication Model

The replication model defines:

* **Who is allowed to create history**
* **Who is allowed to consume history**
* **What states are legal vs illegal**
* **When the system must refuse to operate**

Replication is fundamentally about **authority over history**, not data movement.

---

## 2. Fundamental Roles

Replication introduces **exactly two roles**.

### 2.1 Primary

The **Primary** is the sole authority for:

* Accepting client writes
* Assigning `CommitId`
* Appending authoritative WAL records
* Defining the global history order

The Primary is the **only node** allowed to create new history.

---

### 2.2 Replica

A **Replica** is a node that:

* Receives history from the Primary
* Applies history deterministically
* Never creates new history
* Never assigns `CommitId`

A Replica is **purely derivative**.

---

## 3. Authority Invariants (Restated Precisely)

### 3.1 Single-Writer Authority Invariant

At any moment:

* **Exactly one node** may act as Primary
* All other nodes must behave as Replicas

If two nodes believe they are Primary → **system must halt writes**.

---

### 3.2 Commit Authority Invariant

* Only the Primary may assign `CommitId`
* Replicas must reject:

  * Local commit attempts
  * WAL entries that imply new CommitIds not issued by the Primary

Commit authority is **not transferable implicitly**.

---

## 4. Role State Machine (Conceptual)

Each node exists in **one** of the following states.

### 4.1 `Uninitialized`

* No authoritative history exists
* Node has no WAL authority
* Node must not accept traffic

This state exists only before bootstrap.

---

### 4.2 `PrimaryActive`

* Node is the sole write authority
* Node may:

  * Accept writes
  * Assign CommitIds
  * Emit WAL records
* Node must:

  * Reject attempts to follow another Primary

---

### 4.3 `ReplicaActive`

* Node follows a specific Primary
* Node may:

  * Apply WAL records
  * Serve reads (only if allowed later)
* Node must:

  * Reject all writes
  * Reject CommitId assignment

---

### 4.4 `ReplicationHalted`

* Node has detected an invariant violation
* Examples:

  * WAL gap detected
  * Divergent history detected
  * Authority ambiguity detected

In this state:

* No reads
* No writes
* Explicit operator intervention required

---

## 5. Illegal States (Must Never Exist)

The following states are **forbidden**:

### ❌ Dual Primary

Two nodes accepting writes concurrently.

### ❌ Replica with Commit Authority

A Replica assigning CommitIds or acknowledging writes.

### ❌ History Fork

Replica history that is not a prefix of Primary history.

### ❌ Implicit Promotion

A Replica becoming Primary without explicit, external reconfiguration.

Any of these conditions → **fatal correctness violation**.

---

## 6. Authority Determination Rules

Replication **does not** include:

* Leader election
* Consensus
* Heartbeats
* Leases
* Timeouts

Authority is determined **externally** (operator, config, orchestration).

If authority is unclear → **the node must refuse to operate**.

---

## 7. Write Admission Rules

A write request:

* **May be accepted only if**:

  * Node is in `PrimaryActive`
  * WAL is writable
  * No authority ambiguity exists
* **Must be rejected if**:

  * Node is a Replica
  * Node is ReplicationHalted
  * Node cannot guarantee durability

Write rejection is safer than incorrect admission.

---

## 8. Read Admission Rules (High-Level)

At this level:

* Reads **may** be served by:

  * Primary
  * Replica (subject to later rules)
* Reads **must be refused** if:

  * Visibility cannot be proven correct
  * MVCC rules cannot be satisfied

Exact read rules are deferred to `REPLICATION_READ_SEMANTICS.md`.

---

## 9. Crash Perspective

After a crash:

* A node must re-establish its role explicitly
* A Replica must re-validate:

  * Its WAL prefix
  * Its snapshot boundary
* A Primary must ensure:

  * No other node is acting as Primary

Crash recovery **never infers authority**.

---

## 10. Explicit Non-Goals (Restated)

This model **does not** define:

* Automatic failover
* Split-brain resolution
* Dynamic membership
* Consensus protocols

These require a future phase.

---

## 11. Model Summary

The replication model enforces:

* Single authoritative history
* Explicit authority boundaries
* Fail-stop behavior on uncertainty
* MVCC-preserving replication

> **Replication correctness begins with authority clarity.
> If authority is ambiguous, correctness is impossible.**

