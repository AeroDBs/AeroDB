## Purpose

This document defines the Write-Ahead Log (WAL) design for aerodb.

The WAL is the **authoritative durability mechanism** in Phase 0.
No acknowledged write exists unless it is fully persisted in the WAL.

This document is binding.
Any implementation that violates these rules is incorrect.

---

## WAL Design Principles

The WAL in aerodb prioritizes:

- Durability over throughput
- Determinism over optimization
- Simplicity over cleverness
- Explicit failure over silent recovery

The WAL is intentionally conservative.
Phase 0 does not attempt to optimize write performance.

---

## Authoritative Role of the WAL

The WAL is:

- The **single source of truth** for crash recovery
- Required for every acknowledged write
- Fully replayed on every startup in Phase 0

The WAL is **never bypassed**.

---

## WAL Scope (Phase 0)

### Included in WAL
- Document INSERT operations
- Document UPDATE operations
- Document DELETE operations
- Schema version reference per document
- Operation sequence number

### Explicitly Excluded from WAL
- Index mutations
- Schema creation or migration
- Configuration changes
- Checkpoints
- Metadata updates

Indexes are derived state and are rebuilt from storage.

---

## WAL File Layout

```

data_dir/
└── wal/
└── wal.log

```

### File Properties
- Append-only
- Single file
- Never truncated in Phase 0
- Opened with exclusive write access

---

## WAL Record Ordering

- WAL records are strictly ordered by a monotonically increasing sequence number
- Sequence numbers start at `1`
- Sequence numbers never repeat or rewind

Sequence numbers define the **total order of writes**.

---

## WAL Record Structure

Each WAL record is self-contained and self-describing.

```

+------------------------+
| Record Length (u32)    |
+------------------------+
| Record Type (u8)       |
+------------------------+
| Sequence Number (u64)  |
+------------------------+
| Payload (variable)     |
+------------------------+
| Checksum (u32)         |
+------------------------+

```

### Field Definitions

| Field | Description |
|------|-------------|
| Record Length | Total byte length of the record |
| Record Type | INSERT / UPDATE / DELETE |
| Sequence Number | Global monotonic operation ID |
| Payload | Operation-specific data |
| Checksum | CRC32 or equivalent over entire record except checksum |

---

## WAL Payload Format

### Common Payload Fields

Every WAL payload MUST include:

- Collection identifier
- Document primary key
- Schema version identifier
- Full document body (post-operation state)

### Payload Rules

- WAL records always store the **full document state**
- No delta encoding
- No partial updates
- Deletes are represented as tombstone records

This guarantees deterministic replay.

---

## WAL Record Types

### INSERT

Represents insertion of a new document.

Rules:
- Document must fully conform to schema
- Full document body stored in payload

---

### UPDATE

Represents replacement of an existing document.

Rules:
- WAL stores the **entire new document**
- Partial updates are forbidden
- Schema version must be explicit

---

### DELETE

Represents deletion of a document.

Rules:
- WAL stores a tombstone payload
- Document primary key remains referenced
- Schema version included for validation

---

## Write Rules (Critical)

### WAL Write Sequence

For every write operation:

1. Construct WAL record
2. Append record to `wal.log`
3. Flush WAL to disk using `fsync`
4. Only after fsync may the operation proceed
5. Only after storage write completes may the operation be acknowledged

**Acknowledgment before fsync is forbidden.**

---

## fsync Policy (Phase 0)

- Every WAL append is followed by `fsync`
- No batching
- No group commit
- No async durability

This is intentional.

---

## WAL Integrity Guarantees

### Checksum Enforcement

- Every WAL record includes a checksum
- Checksum covers:
  - Record header
  - Payload
  - Sequence number

Any checksum mismatch is corruption.

---

## WAL Corruption Policy

### Phase 0 Rule: Zero Tolerance

If **any** WAL corruption is detected:

- Startup halts immediately
- No partial replay
- No skipping records
- No repair attempts

The database refuses to start.

This is mandatory.

---

## WAL Replay Rules

### Replay Order

- WAL records are replayed strictly in sequence number order
- Replay always starts from the first record
- Replay is single-threaded

---

### Replay Behavior

For each WAL record:

1. Validate checksum
2. Validate record structure
3. Validate schema version existence
4. Apply document state to document storage
5. If record is DELETE, write tombstone

If any step fails → recovery halts.

---

## WAL and Idempotency

### Replay Idempotency Rule

Applying the same WAL record twice must result in the same final state.

This is achieved by:
- Full-document replacement
- Primary-key-based overwrite semantics

No record may depend on previous application state.

---

## WAL and Document Storage Interaction

- WAL durability precedes storage mutation
- Storage writes are considered secondary
- On crash:
  - WAL is authoritative
  - Storage is reconciled to WAL state

Indexes are rebuilt after replay completes.

---

## WAL and Recovery Determinism

Recovery must be:

- Deterministic
- Repeatable
- Independent of timing or environment

Given the same WAL file, recovery must always produce the same storage state.

---

## WAL Growth Policy (Phase 0)

### Phase 0 Rule

- WAL grows without bound
- WAL is never truncated
- WAL is never compacted

This is a conscious trade-off.

---

## WAL and Clean Shutdown

### Clean Shutdown Marker

- On clean shutdown, aerodb writes a shutdown marker to metadata
- WAL contents remain unchanged
- WAL is not truncated

On restart, WAL is still fully replayed.

---

## Forbidden WAL Behaviors

The following are explicitly forbidden:

- Skipping WAL entries
- Partial WAL replay
- WAL batching without fsync
- Delta-based WAL entries
- WAL-driven index mutation
- Silent corruption handling

Any of these violate core invariants.

---

## Invariants Enforced by WAL

| Invariant | Enforcement |
|----------|-------------|
| D1 | fsync before acknowledgment |
| R1 | WAL precedes all storage writes |
| R2 | Sequential deterministic replay |
| R3 | Explicit recovery success/failure |
| K1 | Checksums on every record |
| K2 | Halt-on-corruption policy |
| C1 | Full-document atomicity |

---

## Phase 0 Limitations (Intentional)

- No checkpointing
- No WAL truncation
- No WAL compression
- No encryption-at-rest
- No replication hooks

All intentionally deferred.

---

## Final Statement

The WAL is the foundation of aerodb’s reliability.

If WAL correctness is compromised:
- Data safety is compromised
- Recovery is compromised
- Trust is compromised

There are no acceptable shortcuts.

The WAL must remain boring, strict, and absolute.
