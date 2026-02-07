# BOOT.md — AeroDB Boot & Startup Contract (Phase 0)

This document defines the **authoritative startup, recovery, and shutdown behavior** of AeroDB.

It is a governing specification.

If implementation behavior conflicts with this document, the implementation is wrong.

This applies to:

- `main.rs`
- Recovery Manager
- API Layer
- CLI
- Config Loader

No component may violate this contract.

---

## 1. Principles

AeroDB boot is designed around the following non-negotiable principles:

- Storage is the source of truth.
- WAL is the recovery authority.
- Indexes are derived state.
- Startup must be deterministic.
- Corruption must halt the system.
- No partial availability.
- No background recovery.
- No speculative serving.

The system is either:

- Fully consistent and serving  
or  
- Completely offline

There is no intermediate state.

---

## 2. High-Level Boot Phases

Startup proceeds in **strict sequence**:

1. Configuration Load
2. Schema Load
3. Recovery
4. Index Rebuild
5. Verification
6. API Activation

No phase may be skipped or reordered.

---

## 3. Detailed Boot Sequence

### 3.1 Configuration Load

Performed by Config Loader.

Steps:

1. Read config file
2. Apply defaults
3. Validate constraints
4. Reject unsafe values

Failures:

- Any invalid configuration → process exits immediately

No files are opened before config validation completes.

---

### 3.2 Schema Load

Performed before any WAL or storage access.

Steps:

1. Load all schemas from:


```

<data_dir>/metadata/schemas/

```

2. Parse schema JSON
3. Validate schema format
4. Register schemas in memory

Failures:

- Missing schema file
- Malformed schema
- Duplicate schema versions

Result:

→ FATAL: `AERO_RECOVERY_SCHEMA_MISSING`

Startup stops.

---

### 3.3 Recovery Phase

Performed by Recovery Manager.

This phase ALWAYS runs, even after clean shutdown.

Steps:

1. Open WAL reader
2. Open storage files
3. Replay WAL from byte 0 sequentially

For each WAL record:

- Validate checksum
- Apply record via `storage.apply_wal_record`

Rules:

- Any WAL corruption → FATAL
- No partial replay
- No skipping entries
- No heuristics

Same WAL must always produce the same storage state.

---

### 3.4 Index Rebuild

Indexes are rebuilt only AFTER WAL replay completes.

Steps:

1. Sequentially scan storage
2. Ignore tombstones
3. Extract indexed fields
4. Populate in-memory BTreeMaps

Rules:

- Indexes are derived state
- Indexes are never persisted
- Corruption during rebuild → FATAL

---

### 3.5 Consistency Verification

Performed after index rebuild.

Verifier must:

- Sequentially scan storage
- Validate checksum on every record
- Ensure all schema references exist

Failures:

- Storage corruption
- Unknown schema ID
- Unknown schema version

Result:

→ FATAL: `AERO_RECOVERY_VERIFICATION_FAILED`

---

### 3.6 Clean Shutdown Marker Handling

Location:


```

<data_dir>/clean_shutdown

```

Behavior:

- Marker presence does NOT skip recovery
- Marker is informational only in Phase 0
- Marker removed after successful boot

Clean shutdown marker does NOT affect replay behavior.

WAL is always replayed fully.

---

### 3.7 API Activation

Only after ALL previous phases succeed:

1. Global execution lock initialized
2. API handlers registered
3. System enters serving state

Before this point:

- No requests accepted
- No partial availability

---

## 4. Serving State

System is considered "online" only after:

- WAL replay completed
- Index rebuild completed
- Verification completed
- API layer initialized

Only then may queries or writes occur.

---

## 5. Shutdown Semantics

Graceful shutdown:

1. Stop accepting API requests
2. Wait for in-flight operation to complete
3. Write `clean_shutdown` marker
4. Exit process

No background flushes.
No delayed cleanup.

Crash shutdown:

- No marker written
- Recovery runs normally on next startup

---

## 6. Fatal Error Policy

Any of the following causes immediate process termination:

- WAL corruption
- Storage corruption
- Schema inconsistency
- Verification failure
- Index rebuild failure
- Invalid configuration

AeroDB does NOT attempt:

- partial recovery
- degraded mode
- read-only fallback

Failure is explicit and terminal.

Operator intervention is required.

---

## 7. Determinism Guarantees

Given identical:

- WAL
- Storage files
- Schemas
- Config

AeroDB must:

- Produce identical in-memory state
- Build identical indexes
- Serve identical query results

No timestamps, randomness, or environment-dependent behavior may affect boot.

---

## 8. Phase-0 Limitations

Phase 0 intentionally does NOT include:

- Checkpointing
- WAL truncation
- Fast startup
- Parallel recovery
- Incremental index rebuild

Every startup performs full replay and full rebuild.

---

## 9. Authority

This document governs:

- Recovery Manager
- main.rs
- API startup
- CLI start command

Violations of this contract are considered correctness bugs.