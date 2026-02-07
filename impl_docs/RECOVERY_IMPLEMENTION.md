# Recovery Manager Implementation

## Summary

Implemented the Recovery Manager subsystem for aerodb Phase 0 at `src/recovery/`, fully compliant with `WAL.md`, `STORAGE.md`, and `ERRORS.md`.

## Module Structure

```
src/recovery/
├── mod.rs       # Public exports
├── errors.rs    # Error codes (All FATAL)
├── replay.rs    # Sequential WAL replay
├── verifier.rs  # Post-replay consistency check
└── startup.rs   # Full recovery orchestration
```

## Startup Sequence (Strict Order)

The startup sequence is rigid and enforced by `RecoveryManager::recover`:

1.  **Clean Shutdown Check**: Check for `clean_shutdown` marker.
2.  **WAL Replay**: Replay WAL from byte 0 (always required in Phase 0).
    - Validates checksums on every record.
    - Applies records to storage idempotently.
    - Aborts immediately on `AERO_WAL_CORRUPTION`.
3.  **Index Rebuild**: `index.rebuild_from_storage()` called after replay.
4.  **Consistency Verification**:
    - Scans storage sequentially.
    - Validates storage checksums.
    - Verifies all schema IDs and versions exist in the registry.
    - Aborts on verification failure.
5.  **Marker Cleanup**: Remove `clean_shutdown` marker.

## Invariants Enforced

| Invariant | Enforcement |
|---|---|
| R1 | WAL is the single source of truth for recovery |
| R2 | Sequential replay from byte 0 |
| K2 | Halt-on-corruption (WAL or Storage) |
| S3 | Verified schema references |
| D2 | Checksum validation on every read (WAL & Storage) |

## Error Codes (per ERRORS.md)

All recovery errors are **FATAL**.

| Code | Description |
|---|---|
| `AERO_WAL_CORRUPTION` | Checksum mismatch in WAL |
| `AERO_STORAGE_CORRUPTION` | Checksum mismatch in Storage |
| `AERO_RECOVERY_SCHEMA_MISSING` | Unknown schema ID/version in data |
| `AERO_RECOVERY_VERIFICATION_FAILED` | Inconsistency detected |
| `AERO_RECOVERY_FAILED` | General failure (fs, permissions) |

## Test Results

All 164 tests passed (16 Recovery + 24 Executor + 25 Planner + 29 Schema + 31 Storage + 39 WAL).

Key recovery tests:
- `test_full_recovery`: Verifies end-to-end restoration.
- `test_replay_idempotency`: Verifies re-running replay is safe.
- `test_corruption_aborts_replay`: Verifies immediate halt on bad WAL.
- `test_storage_corruption_aborts`: Verifies verification catches bad storage.
- `test_clean_shutdown_marker`: Verifies marker handling.
- `test_index_rebuilt_after_replay`: Verifies index derived state update.
