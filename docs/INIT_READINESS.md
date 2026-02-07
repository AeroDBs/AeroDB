# Phase 1 Final Readiness Checklist — AeroDB

This checklist defines the **non-negotiable completion criteria** for AeroDB Phase 1.

Phase 1 is considered complete **only if every item below is satisfied**.

If any item is unchecked, Phase 1 is incomplete.

---

## 1. Core Data Safety

### Write-Ahead Log (WAL)

- [x] Every acknowledged write is fsynced before success
- [x] WAL append order is strictly monotonic
- [x] WAL corruption is detected via checksum
- [x] WAL corruption halts recovery (FATAL)
- [x] WAL replay is deterministic
- [x] WAL truncation occurs **only** during checkpoint

---

### Storage Engine

- [x] Storage is append-only
- [x] Full-document writes only (no deltas)
- [x] Every record is checksummed
- [x] Corruption is detected on read
- [x] Corruption is never silently repaired
- [x] Tombstones correctly represent deletes
- [x] Reads never observe partial records

---

## 2. Schema Enforcement

- [x] Schemas are mandatory for all writes
- [x] Schema ID and version must be explicit
- [x] Schema versions are immutable
- [x] No implicit fields or coercion
- [x] Extra fields are rejected
- [x] Missing required fields are rejected
- [x] `_id` is required and immutable
- [x] Recovery fails if referenced schema is missing

---

## 3. Query Safety & Determinism

### Planner

- [x] All queries are proven bounded before execution
- [x] Queries without limit are rejected
- [x] Filters on non-indexed fields are rejected
- [x] Sorts on non-indexed fields are rejected
- [x] Planner output is deterministic
- [x] Tie-breaking rules are deterministic
- [x] Explain output is stable

---

### Executor

- [x] Execution follows plan exactly
- [x] No runtime heuristics
- [x] Deterministic result ordering
- [x] Stable sort only
- [x] Limit enforced physically
- [x] Schema version filtering enforced
- [x] Checksum validated on every read

---

## 4. Index Integrity

- [x] Indexes are derived state only
- [x] Indexes are rebuilt on startup
- [x] Index rebuild halts on corruption
- [x] Index ordering is deterministic
- [x] Tombstoned documents are excluded
- [x] No index persistence to disk

---

## 5. Recovery Correctness

- [x] Recovery always runs on startup
- [x] WAL replay starts at correct position
- [x] Replay is sequential and deterministic
- [x] Index rebuild runs after replay
- [x] Storage checksums verified post-replay
- [x] Schema references verified
- [x] Recovery success/failure is explicit
- [x] No partial recovery allowed

---

## 6. Snapshot

- [x] Snapshot is read-only
- [x] Snapshot fsyncs WAL before copy
- [x] Snapshot copies storage byte-for-byte
- [x] Snapshot copies schemas
- [x] Snapshot has a manifest
- [x] Manifest includes checksums
- [x] Partial snapshots are cleaned up
- [x] Snapshot does NOT truncate WAL

---

## 7. Checkpoint

- [x] Checkpoint acquires global execution lock
- [x] Checkpoint creates snapshot first
- [x] Checkpoint writes marker before truncation
- [x] WAL truncation is atomic
- [x] Crash before truncation preserves WAL
- [x] Crash after truncation recovers via snapshot
- [x] Checkpoint errors are non-fatal
- [x] Checkpoint ID equals snapshot ID

---

## 8. Backup

- [x] Backup uses latest snapshot
- [x] Backup includes WAL tail
- [x] Backup is deterministic
- [x] Backup has a manifest
- [x] No compression used
- [x] Backup is read-only
- [x] Partial backups are cleaned up
- [x] Backup errors do not affect serving

---

## 9. Restore

- [x] Restore is offline-only
- [x] Restore validates backup structure
- [x] Restore validates snapshot checksums
- [x] Restore validates WAL integrity
- [x] Restore uses atomic directory replacement
- [x] Original data preserved on failure
- [x] Restore never rebuilds indexes
- [x] Restore errors are FATAL

---

## 10. Observability

- [x] Structured JSON logs
- [x] Deterministic key ordering
- [x] Lifecycle events logged
- [x] WAL events logged
- [x] Snapshot / checkpoint events logged
- [x] Backup / restore events logged
- [x] Recovery events logged
- [x] Query lifecycle logged
- [x] Metrics counters monotonic
- [x] Observability has zero behavioral impact

---

## 11. Crash Testing

- [x] Crash injection via environment variable
- [x] Crash points implemented across subsystems
- [x] WAL crash scenarios tested
- [x] Storage mid-write crashes tested
- [x] Snapshot crashes tested
- [x] Checkpoint crashes tested
- [x] Backup crashes tested
- [x] Restore crashes tested
- [x] Recovery determinism verified
- [x] No flaky crash tests

---

## 12. Configuration Safety

- [x] Unsafe configurations rejected
- [x] WAL fsync cannot be disabled
- [x] Checksums cannot be disabled
- [x] Schemas cannot be bypassed
- [x] Configuration validated at startup
- [x] Configuration is immutable post-start

---

## 13. CLI & Boot

- [x] `init` creates correct layout
- [x] `start` runs full recovery before serving
- [x] `query` and `explain` require recovery
- [x] Clean shutdown marker handled correctly
- [x] Startup failure halts immediately
- [x] No background threads started implicitly

---

## 14. Tests & Discipline

- [x] All unit tests passing
- [x] All integration tests passing
- [x] All crash tests passing
- [x] No ignored tests
- [x] No flaky tests
- [x] Code matches governing docs
- [x] No undocumented behavior

---

## Phase 1 Exit Criteria

Phase 1 is complete **only if**:

- All checklist items are checked
- Crash testing passes consistently
- No invariants are violated
- System behavior matches documentation exactly

---

## What Phase 1 Guarantees

- No acknowledged write is lost
- Corruption is always detected
- Recovery is deterministic
- Backups are restorable
- Restore is atomic
- Queries are bounded and safe
- Behavior is explicit and explainable

---

END INIT_READINESS.md
