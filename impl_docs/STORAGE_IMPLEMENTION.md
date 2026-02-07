# Document Storage Implementation

## Summary

Implemented the Document Storage subsystem for aerodb Phase 0 at `src/storage/`, fully compliant with `STORAGE.md`.

## Module Structure

```
src/storage/
├── mod.rs       # Public exports
├── errors.rs    # Error codes (AERO_STORAGE_*, AERO_DATA_CORRUPTION)
├── checksum.rs  # CRC32 checksums
├── record.rs    # Document record format (STORAGE.md §6.2)
├── writer.rs    # Append-only with fsync & WAL replay hook
└── reader.rs    # Sequential scan & corruption detection
```

## Record Format (per STORAGE.md §6.2)

| Field | Size | Description |
|---|---|---|
| Record Length | u32 LE | Total record size |
| Document ID | string | Collection:ID composite |
| Schema ID | string | Schema identifier |
| Schema Version | string | Version identifier |
| Tombstone Flag | u8 | 1 = Deleted, 0 = Live |
| Payload | variable | Document body (empty if tombstone) |
| Checksum | u32 LE | CRC32 over all previous bytes |

## Invariants Enforced

| Invariant | Enforcement |
|---|---|
| D2 | Checksums verified on every read |
| C1 | Full document state (no deltas) |
| K2 | Halt-on-corruption (FATAL error) |
| R1 | Writes occur after WAL fsync (via `writer.rs`) |
| R3 | Idempotent replay via `apply_wal_record` |

## Error Codes (per ERRORS.md)

| Code | Severity | Description |
|---|---|---|
| `AERO_STORAGE_WRITE_FAILED` | ERROR | Write/Fsync failure |
| `AERO_STORAGE_READ_FAILED` | ERROR | IO error during read |
| `AERO_DATA_CORRUPTION` | FATAL | Checksum mismatch or invalid format |

## Test Results

All 70 tests passed (31 Storage + 39 WAL).

Key tests implemented:
- `test_write_and_read_back`: Basic roundtrip
- `test_tombstone_write`: Delete semantics
- `test_overwrite_semantics`: Latest record wins
- `test_corruption_detected`: Bitflip detection (FATAL)
- `test_apply_wal_record`: Recovery hook
- `test_replay_idempotency`: Multiple replays = stable state

## Key Design Decisions

1.  **Append-Only**: No in-place updates. `documents.dat` only grows.
2.  **Latest Wins**: In-memory index maps `collection:id` to the latest file offset.
3.  **Fatal Corruption**: Any checksum mismatch halts the system immediately.
4.  **Composite Keys**: Document ID stored as `collection_id:document_id`.
5.  **WAL Driven**: Storage writes happen strictly after WAL fsyncs.
