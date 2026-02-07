# DX_OBSERVABILITY_PRINCIPLES.md — Phase 4 Observability Principles

This document defines the authoritative observability surface of AeroDB Phase 1.

Observability exists to ensure that:

- operators can understand system state
- failures are diagnosable
- recovery is auditable
- performance regressions are visible

If implementation behavior conflicts with this document, the implementation is wrong.

Observability is explicit and synchronous.

No background telemetry.

No opaque metrics.

---

## 1. Principles

AeroDB observability follows strict rules:

1. Explicit over implicit
2. Deterministic over heuristic
3. Operator-visible over internal-only
4. Failures are loud
5. No hidden background reporting
6. No sampling
7. No aggregation that hides raw values

All metrics are exact.

All events are logged.

---

## 2. Observability Surfaces

Phase 1 exposes observability through:

1. Startup logs (stdout/stderr)
2. Runtime event logs (stdout)
3. `aerodb stats` command
4. Exit codes
5. Explicit corruption messages

There is no HTTP metrics endpoint in Phase 1.

---

## 3. Startup Logging

On every startup, AeroDB MUST log:

```

AERODB_STARTUP_BEGIN

```

Followed by:

### Configuration

```

CONFIG_LOADED
data_dir=<path>
wal_sync_mode=fsync
max_wal_size_bytes=<value>
max_memory_bytes=<value>

```

---

### Schema Load

```

SCHEMAS_LOADED
schema_count=<n>

```

---

### Recovery

```

RECOVERY_BEGIN

```

Then:

```

WAL_REPLAY_BEGIN
wal_records=<count>

```

After replay:

```

WAL_REPLAY_COMPLETE

```

---

### Index Rebuild

```

INDEX_REBUILD_BEGIN

```

Then:

```

INDEX_REBUILD_COMPLETE
index_count=<n>

```

---

### Verification

```

VERIFICATION_BEGIN

```

Then:

```

VERIFICATION_COMPLETE

```

---

### Serving

```

AERODB_SERVING

```

Only after this log may the system accept requests.

---

## 4. Runtime Event Logging

Each operation MUST log:

### Writes

```

WRITE_BEGIN op=insert|update|delete id=<doc_id>
WRITE_COMMIT

```

---

### Queries

```

QUERY_BEGIN
QUERY_COMPLETE rows=<n>

```

---

### Explain

```

EXPLAIN_BEGIN
EXPLAIN_COMPLETE

```

---

Logs are synchronous.

No buffering.

---

## 5. Stats Command

Operators may query live state:

```

aerodb stats --config aerodb.json

````

Returns JSON:

```json
{
  "documents": 1234,
  "schemas": 2,
  "indexes": 3,
  "wal_bytes": 45678,
  "snapshot_count": 1,
  "last_checkpoint": "2026-02-04T11:30:00Z",
  "recovery_duration_ms": 812,
  "uptime_seconds": 120
}
````

---

### Metric Definitions

| Field                | Meaning             |
| -------------------- | ------------------- |
| documents            | Live document count |
| schemas              | Registered schemas  |
| indexes              | Active indexes      |
| wal_bytes            | Current WAL size    |
| snapshot_count       | Valid snapshots     |
| last_checkpoint      | Timestamp           |
| recovery_duration_ms | Last boot recovery  |
| uptime_seconds       | Since last start    |

All values are exact.

---

## 6. Corruption Visibility

On any corruption:

* explicit log printed
* error code returned
* process exits

Example:

```
FATAL AERO_DATA_CORRUPTION
wal_entry=128
checksum_expected=abcd
checksum_found=dead
```

No retry.

No masking.

---

## 7. Checkpoint Observability

Checkpoint emits:

```
CHECKPOINT_BEGIN
SNAPSHOT_CREATED id=<snapshot_id>
WAL_TRUNCATED
CHECKPOINT_COMPLETE
```

Any failure emits:

```
CHECKPOINT_FAILED reason=<error>
```

---

## 8. Backup Observability

Backup emits:

```
BACKUP_BEGIN
BACKUP_COMPLETE output=<path>
```

Failure:

```
BACKUP_FAILED
```

---

## 9. Restore Observability

Restore emits:

```
RESTORE_BEGIN
RESTORE_COMPLETE
```

Failure:

```
RESTORE_FAILED
```

---

## 10. Determinism

Observability must not introduce:

* timestamps inside internal state
* nondeterministic ordering
* randomness

Logs may include wall-clock timestamps, but never affect execution.

---

## 11. Phase-1 Limitations

Phase 1 does NOT include:

* Prometheus
* OpenTelemetry
* structured logging frameworks
* tracing

These belong to Phase 2+.

---

## 12. Authority

This document governs:

* startup logging
* runtime logs
* stats output
* corruption visibility
* checkpoint reporting
* backup reporting
* restore reporting

Violations are correctness bugs.
