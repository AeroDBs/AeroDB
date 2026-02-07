# REPLICATION_READINESS.md

## AeroDB — Phase 2 Replication Readiness Checklist

### Status

* This document tracks **replication implementation completeness**
* All items must be verified before replication is production-ready
* Semantic freeze is enforced

---

## 1. Semantic Freeze Declaration

> **Phase-1 semantics are FROZEN.**
>
> **MVCC semantics are FROZEN.**
>
> Replication CONSUMES existing correctness.
> Any deviation is a correctness bug.

---

## 2. Core Implementation Status

### REPLICATION-01: Roles & Authority ✅

- [x] ReplicationRole (Primary, Replica)
- [x] ReplicationState state machine
- [x] HaltReason enumeration
- [x] Authority enforcement (write admission, commit authority)
- [x] Dual-primary detection

### REPLICATION-02: WAL Shipping ✅

- [x] WalPosition tracking
- [x] WalSender (Primary-side)
- [x] WalReceiver (Replica-side)
- [x] Prefix validation (Replica_WAL == Prefix(Primary_WAL))
- [x] Gap detection → ReplicationHalted

### REPLICATION-03: Snapshot Transfer ✅

- [x] SnapshotMetadata
- [x] SnapshotReceiver lifecycle
- [x] Eligibility checks (Primary-only, complete, validated)
- [x] Atomic installation
- [x] WAL resume after boundary

### REPLICATION-04: Replica Reads ✅

- [x] ReadEligibility enum
- [x] ReplicaReadAdmission
- [x] Boundary check: R.read_upper_bound ≤ C_replica
- [x] Refusal paths (halted, gap, snapshot, recovery)

### REPLICATION-05: Failure Matrix ✅

- [x] ReplicationCrashPoint enumeration
- [x] FailureOutcome mapping
- [x] FailureState tracking
- [x] All crash points from FAILURE_MATRIX.md

### REPLICATION-06: Recovery ✅

- [x] PrimaryRecovery (WAL/MVCC verification)
- [x] ReplicaRecovery (prefix/snapshot verification)
- [x] RecoveryValidation enum
- [x] Halt on uncertainty

### REPLICATION-07: Compatibility ✅

- [x] Phase1Compatibility assertions
- [x] MvccCompatibility assertions
- [x] CompatibilityCheck verification

---

## 3. Test Coverage

| Module | Tests |
|--------|-------|
| role | 9 |
| authority | 8 |
| errors | 2 |
| wal_sender | 5 |
| wal_receiver | 7 |
| snapshot_transfer | 8 |
| replica_reads | 10 |
| failure_matrix | 9 |
| recovery | 10 |
| compatibility | 6 |

**Total: 74+ replication tests**

---

## 4. Phase-1 Compatibility Guarantees

Per REPLICATION_COMPATIBILITY.md §2:

- [x] WAL remains sole durability authority
- [x] fsync semantics unchanged
- [x] WAL replay rules identical
- [x] Storage invariants intact
- [x] Query engine unchanged

---

## 5. MVCC Compatibility Guarantees

Per REPLICATION_COMPATIBILITY.md §3:

- [x] CommitIds globally ordered
- [x] CommitIds immutable
- [x] CommitIds only from Primary
- [x] Visibility semantics unchanged
- [x] GC rules unchanged

---

## 6. Invariant Summary

From REPLICATION_INVARIANTS.md:

| # | Invariant | Status |
|---|-----------|--------|
| 1 | Single-Writer | ✅ Enforced |
| 2 | Commit Authority | ✅ Enforced |
| 3 | Prefix Validity | ✅ Enforced |
| 4 | Gap Detection | ✅ Enforced |
| 5 | Fail-Stop | ✅ Enforced |

---

## 7. Readiness Certification

- [x] All REPLICATION-01 through REPLICATION-07 implemented
- [x] All tests pass
- [x] Phase-1 semantics preserved
- [x] MVCC semantics preserved
- [x] Fail-stop on uncertainty

> **Replication adds nodes, not meanings.**
