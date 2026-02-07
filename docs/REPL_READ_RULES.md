# REPLICATION_READ_SEMANTICS.md

## AeroDB — Replica Read Semantics & MVCC Safety

### Status

* This document is **authoritative**
* It defines **exactly what reads Replicas may or may not serve**
* All rules are **safety-first and fail-stop**
* No implementation details appear here
* Phase-1 and MVCC semantics are **frozen and assumed correct**

---

## 1. Purpose of Replica Read Semantics

Replication introduces a fundamental question:

> **When is it safe for a Replica to answer a read?**

This document answers that question **without weakening**:

* MVCC visibility rules
* Deterministic behavior
* Crash safety
* Phase-1 correctness guarantees

Replica reads are an **optional capability**, not a requirement.

---

## 2. Foundational Principle

> **A Replica may serve a read if and only if it can prove that the result is identical to what the Primary would return for the same read view.**

If this proof cannot be made, the read **must be refused**.

---

## 3. Replica Read Modes (Conceptual)

Replication defines **exactly two conceptual read modes**.

### 3.1 Primary Reads

* Served by the Primary
* Always authoritative
* Always up-to-date
* Governed by standard MVCC rules

Primary reads are unchanged by replication.

---

### 3.2 Replica Reads

* Served by a Replica
* Always **potentially stale**
* Must obey strict safety rules
* Must never imply future visibility

Replica reads are optional and explicitly constrained.

---

## 4. Replica Read Eligibility Rule (Formal)

A Replica may serve a read **if and only if** all of the following hold:

1. The Replica is in state `ReplicaActive`
2. The Replica’s applied WAL prefix ends at commit `C_replica`
3. The requested read view `R` satisfies:

   ```
   R.read_upper_bound ≤ C_replica
   ```
4. All MVCC metadata required for the read view exists locally
5. No WAL gaps or replication errors are present

Failure of **any** condition → read must be refused.

---

## 5. Read View Handling on Replicas

### 5.1 Read View Creation

Replica read views:

* Are created locally
* Must use:

  ```
  read_upper_bound = C_replica
  ```
* Must not reference Primary state
* Must not infer future commits

Replica read views are **never speculative**.

---

### 5.2 Read View Stability

Once created:

* The read view is immutable
* Subsequent WAL application must not affect it
* Reads observe a stable snapshot

Replica reads obey **exactly the same MVCC_VISIBILITY rules**.

---

## 6. Visibility Guarantees on Replicas

For any key `K`:

* The Replica must select the visible version using:

  * The same visibility resolver as the Primary
  * The same commit identity rules
* Results must be **bit-for-bit identical** to a Primary read at the same boundary

If this cannot be guaranteed → read must be refused.

---

## 7. Range Queries on Replicas

Range queries served by Replicas must:

* Use **one shared read view**
* Never mix visibility boundaries
* Never observe partial replication progress

If WAL advances during the range scan:

* The scan must continue using the original read view
* WAL progress must not affect results

---

## 8. Read Refusal Conditions (Mandatory)

A Replica **must refuse reads** if:

* Replication is halted
* WAL gaps are detected
* Snapshot installation is incomplete
* Replica is mid-recovery
* Replica state is uncertain
* Read view boundary cannot be proven safe

Refusal is **not a failure** — it is correctness enforcement.

---

## 9. Crash & Restart Semantics

After a crash:

* Replica must re-validate:

  * WAL prefix integrity
  * Snapshot boundary (if any)
* Replica must not serve reads until validation completes

No “best-effort” reads are allowed during recovery.

---

## 10. Interaction with GC

Replica reads must respect GC rules:

* Versions required by read view must exist
* GC must never remove versions visible to replica reads
* Snapshot retention rules apply equally on Replicas

GC safety is **identical** on Primary and Replica.

---

## 11. Explicitly Forbidden Replica Read Behaviors

Replica reads must **never**:

* Serve reads ahead of applied WAL
* Guess Primary commit state
* Use wall-clock time
* Use “last known good” shortcuts
* Relax MVCC visibility rules
* Mask replication errors

Any of these constitutes a correctness violation.

---

## 12. Read Semantics Summary

Replica read semantics guarantee:

* Deterministic, MVCC-correct results
* Explicit staleness boundaries
* Fail-stop behavior on uncertainty
* No semantic drift from Primary behavior

> **A Replica may lag.
> It may never lie.**
