# CHECKPOINT.md — AeroDB Checkpointing Specification (Phase 1)

This document defines the authoritative checkpointing behavior for AeroDB Phase 1.

Checkpointing bounds:

- WAL growth
- recovery time
- snapshot cadence

If implementation behavior conflicts with this document, the implementation is wrong.

Checkpointing exists to make AeroDB operationally viable.

Correctness is prioritized over speed.

---

## 1. Principles

Checkpointing must satisfy:

- Atomicity
- Durability
- Determinism
- No partial success
- Explicit failure

Checkpointing integrates snapshots with WAL truncation.

---

## 2. Definitions

### Snapshot

A durable copy of storage and schemas per SNAPSHOT.md.

### Checkpoint

A successful snapshot PLUS WAL truncation.

Checkpoint = Snapshot + WAL Reset.

---

## 3. Trigger Conditions

Phase 1 checkpointing is manual only.

Triggered via:

```

aerodb checkpoint

```

No automatic or background checkpoints.

---

## 4. Checkpoint Algorithm (Strict Order)

Checkpoint execution MUST follow:

1. Acquire global execution lock
2. fsync WAL
3. Create snapshot (per SNAPSHOT.md)
4. fsync snapshot
5. Write checkpoint manifest
6. Truncate WAL to zero
7. fsync WAL directory
8. Release global execution lock

Any failure aborts checkpoint.

---

## 5. Checkpoint Manifest

Location:

```

<data_dir>/checkpoint.json

````

Example:

```json
{
  "snapshot_id": "20260204T113000Z",
  "created_at": "2026-02-04T11:30:00Z",
  "wal_truncated": true,
  "format_version": 1
}
````

Written AFTER snapshot fsync and BEFORE WAL truncation.

---

## 6. WAL Truncation Rules

After snapshot success:

* WAL file deleted or truncated
* new WAL starts empty
* sequence numbers reset to 1

Truncation must be atomic.

If truncation fails:

* checkpoint fails
* snapshot remains
* WAL untouched

---

## 7. Crash During Checkpoint

### Case 1 — crash before manifest written

→ snapshot ignored
→ WAL replay from previous state

---

### Case 2 — crash after manifest but before WAL truncation

→ snapshot used
→ WAL replayed from beginning

Safe but slower.

---

### Case 3 — crash after WAL truncation

→ snapshot used
→ WAL empty

Fast recovery.

---

No scenario causes data loss.

---

## 8. Startup Behavior

On startup:

1. Detect checkpoint.json
2. Load referenced snapshot
3. Replay WAL (if any)
4. Rebuild indexes
5. Verify consistency

Checkpoint.json without snapshot is FATAL.

Snapshot without checkpoint.json is ignored.

---

## 9. Corruption Policy

Any corruption in:

* snapshot
* checkpoint.json
* WAL

Results in:

* abort startup
* operator intervention required

No auto-repair.

---

## 10. Phase-1 Limitations

Checkpointing does NOT:

* run automatically
* run in background
* throttle writes
* support incremental snapshots

All checkpoints are blocking and synchronous.

---

## 11. Determinism Guarantees

Given identical:

* snapshot
* WAL
* schemas

Recovery must produce identical state.

Checkpointing introduces no non-determinism.

---

## 12. Authority

This document governs:

* aerodb checkpoint
* WAL truncation
* startup recovery logic
* snapshot promotion

Violations are correctness bugs.
