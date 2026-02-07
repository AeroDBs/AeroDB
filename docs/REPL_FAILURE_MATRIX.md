# REPLICATION_FAILURE_MATRIX.md

## AeroDB — Replication Failure & Partition Matrix

### Status

* This document is **authoritative**
* It enumerates **every meaningful replication failure**
* Every failure maps to **exactly one correct outcome**
* No implementation details appear here
* Phase-1 and MVCC semantics are **frozen and preserved**

---

## 1. Purpose of the Failure Matrix

Replication correctness depends on one rule:

> **There must be no ambiguous outcomes under failure.**

This document defines:

* What can fail
* When it can fail
* What state may exist at failure time
* What recovery outcome is allowed

If a failure is not described here, it is **illegal**.

---

## 2. Failure Dimensions

Replication failures are classified along four axes:

1. **Node failure** (Primary or Replica crash)
2. **Network failure** (partition, delay, duplication)
3. **State transfer failure** (WAL or snapshot)
4. **Corruption failure** (data integrity)

Each failure is analyzed independently and in combination.

---

## 3. Primary-Side Failures

### 3.1 Primary Crash Before WAL Commit

**Failure Point**

* Crash before WAL fsync of a commit

**State at Failure**

* No durable commit
* No authoritative history extension

**Required Outcome**

* Commit does not exist
* Replicas never observe it
* Recovery is identical to Phase-1 behavior

---

### 3.2 Primary Crash After WAL Commit, Before Replication

**Failure Point**

* Commit durable on Primary
* WAL not yet shipped to Replicas

**State at Failure**

* Commit exists only on Primary

**Required Outcome**

* Commit is durable
* Replicas will receive it after Primary recovery
* No Replica may invent or infer it

Durability is not weakened by replication.

---

### 3.3 Primary Crash During WAL Shipping

**Failure Point**

* WAL partially transferred

**State at Failure**

* Replica may have:

  * Some records
  * Or none

**Required Outcome**

* Replica detects incomplete transfer
* Replica halts replication
* WAL transfer resumes explicitly

Partial history must not be applied silently.

---

## 4. Replica-Side Failures

### 4.1 Replica Crash Before WAL Append

**Failure Point**

* Crash before WAL record durability

**State at Failure**

* Record not durable

**Required Outcome**

* Record is lost on Replica
* Retransmission required
* No inconsistency introduced

---

### 4.2 Replica Crash After WAL Append

**Failure Point**

* WAL record durably appended
* Replay incomplete

**State at Failure**

* WAL is authoritative

**Required Outcome**

* Recovery replays WAL deterministically
* Replica state advances correctly

---

### 4.3 Replica Crash During Snapshot Installation

**Failure Point**

* Snapshot partially applied

**State at Failure**

* Incomplete snapshot

**Required Outcome**

* Snapshot is discarded
* Replica remains in prior valid state
* No partial state survives

---

## 5. Network Failures

### 5.1 Network Partition (Primary ↔ Replica)

**Failure Point**

* WAL transfer interrupted

**State at Failure**

* Replica lags

**Required Outcome**

* Replica stops applying WAL
* Reads may continue only if safe
* Writes remain Primary-only

No speculative catch-up is allowed.

---

### 5.2 Message Duplication or Reordering

**Failure Point**

* WAL records duplicated or reordered in transit

**State at Failure**

* Replica detects mismatch

**Required Outcome**

* Replica rejects invalid order
* Replication halts explicitly

Transport unreliability must not affect correctness.

---

## 6. WAL Integrity Failures

### 6.1 Corrupted WAL Record in Transit

**Failure Point**

* Checksum mismatch

**State at Failure**

* Record unreadable

**Required Outcome**

* Replica rejects record
* Replication halts
* Operator intervention required

No repair attempts are allowed.

---

### 6.2 Missing WAL Record (Gap)

**Failure Point**

* WAL sequence discontinuity

**State at Failure**

* History incomplete

**Required Outcome**

* Replica halts
* Enters `ReplicationHalted`
* No reads or writes allowed

---

## 7. Snapshot-Related Failures

### 7.1 Invalid Snapshot Transfer

**Failure Point**

* Snapshot integrity check fails

**Required Outcome**

* Snapshot rejected
* Replica remains unchanged
* Replication must restart

---

### 7.2 Snapshot Boundary Mismatch

**Failure Point**

* Snapshot boundary conflicts with WAL history

**Required Outcome**

* Fatal error
* Replica must not continue

---

## 8. Divergence Detection

### 8.1 Replica History Diverges from Primary

**Failure Point**

* Replica WAL is not a prefix of Primary WAL

**Required Outcome**

* Fatal correctness violation
* Replication halts
* Manual intervention required

Divergence must never be auto-healed.

---

## 9. Combined Failures

### 9.1 Crash + Network Partition

**Failure Point**

* Replica crashes while partitioned

**Required Outcome**

* Replica recovers locally
* Remains behind
* Resumes replication explicitly

---

### 9.2 Crash During Recovery

**Failure Point**

* Crash during recovery or replay

**Required Outcome**

* Recovery restarts
* Deterministic outcome preserved

---

## 10. Forbidden Recovery Outcomes

Recovery must **never** result in:

* Partial commits becoming visible
* Reordered commits
* Healed divergence
* Implicit authority assumption
* Mixed snapshot + WAL states

Any such state is a fatal error.

---

## 11. Determinism Guarantee

For any failure scenario:

* Outcome is uniquely determined
* No heuristics are involved
* No timing assumptions are made

Replication failure handling is **purely state-driven**.

---

## 12. Failure Matrix Summary

Replication failure handling guarantees:

* No data loss
* No silent divergence
* No speculative recovery
* Explicit halting on uncertainty

> **If replication cannot be proven correct, it must stop.**
