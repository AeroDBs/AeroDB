This document defines the authoritative lifecycle of an AeroDB instance.

It governs:

- CLI behavior
- Process startup
- Shutdown semantics
- Crash recovery
- Restart guarantees
- Operator expectations

If implementation behavior conflicts with this document, the implementation is wrong.

AeroDB lifecycle is explicit, deterministic, and conservative.

---

## 1. Lifecycle States

An AeroDB instance exists in exactly one of the following states:

1. UNINITIALIZED  
2. INITIALIZING  
3. RECOVERING  
4. VERIFYING  
5. SERVING  
6. SHUTTING_DOWN  
7. TERMINATED  

Transitions are strictly ordered.

There are no parallel states.

---

## 2. UNINITIALIZED

The system is UNINITIALIZED when:

- `data_dir` does not exist
- or contains no AeroDB metadata

Allowed action:

```

aerodb init

```

Behavior:

- Creates directory structure
- Initializes WAL and storage files
- Creates metadata directories
- Does NOT start serving

No queries or writes allowed.

---

## 3. INITIALIZING

Entered on:

```

aerodb start

```

Steps:

1. Load configuration
2. Validate config
3. Load schemas

Failure at any step → TERMINATED.

No WAL or storage access before schema load completes.

---

## 4. RECOVERING

Entered after INITIALIZING succeeds.

Performed by Recovery Manager.

Steps:

1. Replay WAL from byte 0
2. Apply records to storage
3. Rebuild indexes

Rules:

- WAL corruption → TERMINATED
- Storage corruption → TERMINATED
- No partial replay
- No degraded mode

Same WAL must always produce same state.

---

## 5. VERIFYING

Entered after RECOVERING.

Steps:

1. Sequentially scan storage
2. Validate checksums
3. Verify schema references

Failure → TERMINATED.

This guarantees internal consistency before serving.

---

## 6. SERVING

Entered only after VERIFYING succeeds.

Behavior:

- API becomes available
- Global execution lock active
- Queries and writes allowed
- All operations serialized

System remains SERVING until explicit shutdown or crash.

---

## 7. SHUTTING_DOWN

Entered on:

- SIGTERM
- CLI stop
- controlled exit

Steps:

1. Stop accepting API requests
2. Wait for in-flight operation
3. Write `clean_shutdown` marker
4. Exit process

No background cleanup.

---

## 8. TERMINATED

Final state.

Process exits.

Causes:

- Fatal error
- Crash
- Operator shutdown

On next start:

- Full recovery always runs

---

## 9. Crash Semantics

Crash includes:

- power loss
- SIGKILL
- panic
- segmentation fault

Behavior:

- clean_shutdown marker absent
- WAL replay required
- storage verified
- indexes rebuilt

Crash does NOT corrupt correctness unless disk data is corrupted.

---

## 10. Restart Semantics

Restart always executes:

1. INITIALIZING
2. RECOVERING
3. VERIFYING
4. SERVING

Clean shutdown does NOT skip recovery in Phase 0.

Fast-start is forbidden.

---

## 11. Upgrade Semantics (Phase 0)

Phase 0 does NOT support:

- in-place upgrades
- rolling upgrades
- schema migrations

Binary upgrades require:

1. Full shutdown
2. Replace executable
3. Start

Backward compatibility is not guaranteed.

---

## 12. Operator Responsibilities

Operators must:

- maintain backups
- monitor disk health
- respond to FATAL errors
- restore from backup if corruption occurs

AeroDB does not auto-repair.

---

## 13. Determinism Guarantees

Given identical:

- WAL
- storage
- schemas
- config

AeroDB must:

- reach SERVING
- expose identical data
- return identical query results

No lifecycle step may depend on time or environment.

---

## 14. Phase-0 Limitations

Explicitly unsupported:

- hot reload
- online upgrades
- partial recovery
- read-only mode
- degraded serving

Failure is terminal.

---

## 15. Authority

This document governs:

- CLI commands
- Recovery Manager
- main.rs
- operator expectations

Violations are correctness bugs.
