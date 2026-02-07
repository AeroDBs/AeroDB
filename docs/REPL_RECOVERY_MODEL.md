# REPLICATION_RECOVERY.md

## AeroDB — Replication Recovery & Restart Semantics

### Status

* This document is **authoritative**
* It defines how replication resumes **after crashes or restarts**
* All behavior is **fail-stop and deterministic**
* No implementation details appear here
* Phase-1 and MVCC semantics are **frozen and preserved**

---

## 1. Purpose of Replication Recovery

Replication recovery exists to answer one question only:

> **After a crash or restart, is it safe for this node to participate in replication?**

If safety cannot be proven, the node must **refuse to operate**.

Recovery prioritizes **correctness over availability**.

---

## 2. Recovery Entry Conditions

On startup, a node must determine:

* Its configured role (Primary or Replica)
* The integrity of its local state
* Whether replication can resume safely

No implicit role inference is permitted.

---

## 3. Primary Recovery Semantics

### 3.1 Primary Restart Preconditions

A node configured as Primary must verify:

1. Local WAL integrity
2. WAL completeness (no gaps, no corruption)
3. MVCC state consistency
4. Snapshot correctness (if present)

Failure of any check → **startup aborts**.

---

### 3.2 Commit Authority Reassertion

After recovery:

* The Primary reasserts commit authority
* CommitId assignment resumes strictly after the last durable CommitId
* No CommitId renumbering or rewriting is allowed

The Primary must **not** assume replicas are synchronized.

---

### 3.3 Write Admission After Recovery

The Primary may accept writes **only if**:

* Local recovery completed successfully
* No replication invariant violations are detected
* Authority configuration is unchanged

Replication state does **not** gate Primary write authority unless explicitly configured.

---

## 4. Replica Recovery Semantics

### 4.1 Replica Restart Preconditions

A Replica must verify:

1. WAL integrity
2. WAL prefix validity
3. Snapshot boundary correctness (if present)
4. MVCC metadata consistency

Failure → Replica enters `ReplicationHalted`.

---

### 4.2 Replica History Validation

A Replica must validate that:

* Its WAL is a valid prefix of the Primary’s WAL
* No local WAL records exist beyond the Primary’s history
* No divergence is present

If validation cannot be performed → replication must not resume.

---

### 4.3 Resuming WAL Application

A Replica may resume WAL replay **only if**:

* WAL continuity is provable
* Next expected WAL record is known
* No gaps exist

Otherwise, explicit operator intervention is required.

---

## 5. Snapshot Interaction During Recovery

### 5.1 Recovery from Snapshot

If a snapshot exists:

* Snapshot is loaded first
* Snapshot commit boundary is established
* WAL replay resumes strictly after boundary

Snapshot correctness is mandatory.

---

### 5.2 Snapshot Invalidation

If snapshot integrity cannot be proven:

* Snapshot must be discarded
* Replica must restart bootstrap explicitly

No partial reuse is allowed.

---

## 6. Crash During Recovery

If a crash occurs:

* Recovery restarts from the beginning
* No partial recovery state persists
* Deterministic outcome is preserved

Recovery is idempotent.

---

## 7. Authority Ambiguity Handling

If a node detects:

* Conflicting role configuration
* Unclear Primary identity
* Divergent histories

Then:

* Node must refuse to operate
* Node enters `ReplicationHalted`
* No reads or writes are allowed

Correctness requires explicit resolution.

---

## 8. Interaction with Read Semantics

After recovery:

* Primary may serve reads immediately
* Replica may serve reads **only after**:

  * WAL prefix validation
  * Snapshot/WAL consistency validation

Reads during uncertainty are forbidden.

---

## 9. Explicitly Forbidden Recovery Behaviors

Replication recovery must **never**:

* Guess missing WAL records
* Auto-truncate history
* Heal divergence
* Assume liveness of other nodes
* Resume partially validated state

Any such behavior is a correctness violation.

---

## 10. Determinism Guarantee

Given identical local state:

* Recovery outcome is identical
* No timing or ordering effects exist
* Restart behavior is reproducible

Replication recovery is a pure function of persisted state.

---

## 11. Recovery Summary

Replication recovery guarantees:

* Safe restart
* Explicit validation
* Deterministic resumption
* Fail-stop on uncertainty

> **A node that cannot prove safety must refuse to run.**
