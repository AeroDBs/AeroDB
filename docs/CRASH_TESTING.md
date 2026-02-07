# CRASH_TESTING.md — AeroDB Crash Injection & Failure Validation (Phase 1)

This document defines the authoritative crash testing methodology for AeroDB Phase 1.

Crash testing validates:

- WAL durability
- storage atomicity
- recovery determinism
- checkpoint correctness
- snapshot safety
- backup/restore survivability

If implementation behavior conflicts with this document, the implementation is wrong.

Crash testing is mandatory before production usage.

---

## 1. Principles

Crash testing must obey:

1. Deterministic reproduction
2. Explicit kill points
3. Zero partial recovery
4. Exact post-crash validation
5. No silent failures

Crashes are intentional and controlled.

---

## 2. Crash Types

The following crash modes MUST be tested:

| Crash Type | Description |
|-----------|-------------|
| SIGKILL | Immediate process termination |
| Power Loss | Simulated abrupt filesystem stop |
| Panic | Rust panic |
| Disk Error | Forced IO failure |

Each crash type must be validated independently.

---

## 3. Kill Points

Crash injection must be supported at the following points:

### WAL

- after record append
- before fsync
- after fsync

---

### Storage

- before document write
- after write, before checksum
- after checksum

---

### Index

- during rebuild
- during update

---

### Snapshot

- during storage copy
- before manifest write
- after manifest write

---

### Checkpoint

- after snapshot
- before WAL truncation
- after WAL truncation

---

### Restore

- after extraction
- before directory swap
- after directory swap

---

## 4. Crash Injection Mechanism

Testing harness must support:

```

AERODB_CRASH_POINT=<symbolic_name>

```

Example:

```

AERODB_CRASH_POINT=wal_after_fsync

```

When set:

- process terminates immediately at that point

Crash points must be deterministic and reproducible.

---

## 5. Required Test Scenarios

### 5.1 WAL Durability

Procedure:

1. Insert document
2. Crash after WAL fsync
3. Restart

Expected:

- document exists

---

### 5.2 WAL Pre-Fsync Crash

1. Insert document
2. Crash before WAL fsync
3. Restart

Expected:

- document does NOT exist

---

### 5.3 Storage Crash

1. Insert document
2. Crash during storage write
3. Restart

Expected:

- either old or new document
- never corrupted state

---

### 5.4 Index Rebuild Crash

1. Populate data
2. Crash during index rebuild
3. Restart

Expected:

- recovery completes
- indexes rebuilt cleanly

---

### 5.5 Snapshot Crash

Crash during snapshot creation.

Expected:

- snapshot ignored
- WAL recovery used

---

### 5.6 Checkpoint Crash

Crash after snapshot but before WAL truncation.

Expected:

- snapshot used
- WAL replayed

---

### 5.7 Restore Crash

Crash during restore.

Expected:

- either original or restored data_dir exists
- never partial mix

---

## 6. Post-Crash Validation

After each crash:

Must verify:

- storage checksums valid
- schemas loaded
- indexes rebuilt
- queries deterministic
- no partial documents

All invariants must hold.

---

## 7. Automation

Crash tests must be automated via:

- subprocess execution
- environment variables
- filesystem inspection

Tests belong in:

```

tests/crash/

```

---

## 8. Failure Criteria

Any of the following is unacceptable:

- corrupted document state
- missing acknowledged writes
- partial records
- inconsistent indexes
- silent recovery
- non-deterministic results

Any violation is a blocking defect.

---

## 9. Phase-1 Limitations

Crash testing does NOT include:

- network failures
- distributed faults
- replica divergence

These belong to Phase 2.

---

## 10. Authority

This document governs:

- crash injection
- recovery validation
- checkpoint correctness
- snapshot reliability
- restore survivability

Violations are correctness bugs.
