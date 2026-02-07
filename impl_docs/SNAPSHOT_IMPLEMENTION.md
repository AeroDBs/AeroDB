# Snapshot Subsystem Implementation

## Summary

Successfully implemented the Snapshot subsystem for AeroDB Phase 1, providing point-in-time durable copies of database state per SNAPSHOT.md specification.

## Files Created

| File | Description |
|------|-------------|
| [errors.rs](file:///home/snigdha/aerodb/src/snapshot/errors.rs) | Error types: `AERO_SNAPSHOT_FAILED`, `AERO_SNAPSHOT_IO`, `AERO_SNAPSHOT_MANIFEST` |
| [checksum.rs](file:///home/snigdha/aerodb/src/snapshot/checksum.rs) | CRC32 file checksum computation and formatting |
| [manifest.rs](file:///home/snigdha/aerodb/src/snapshot/manifest.rs) | Manifest struct with JSON serialization per §3.3 |
| [creator.rs](file:///home/snigdha/aerodb/src/snapshot/creator.rs) | Core snapshot creation algorithm per §4 |
| [mod.rs](file:///home/snigdha/aerodb/src/snapshot/mod.rs) | `SnapshotManager` public API and module exports |

## Files Modified

| File | Change |
|------|--------|
| [lib.rs](file:///home/snigdha/aerodb/src/lib.rs) | Added `pub mod snapshot` |
| [Cargo.toml](file:///home/snigdha/aerodb/Cargo.toml) | Added `chrono = "0.4"` dependency |

---

## Public API

```rust
pub struct SnapshotManager;

impl SnapshotManager {
    pub fn create_snapshot(
        data_dir: &Path,
        storage_path: &Path,
        schema_dir: &Path,
        wal: &WalWriter,
        lock: &GlobalExecutionLock,
    ) -> Result<SnapshotId, SnapshotError>;
}
```

---

## Algorithm Compliance (SNAPSHOT.md §4)

| Step | Specification | Implementation |
|------|---------------|----------------|
| 1 | Acquire global execution lock | `GlobalExecutionLock` marker required |
| 2 | fsync WAL | WAL writer passed, confirmed last write fsynced |
| 3 | Create snapshot directory | `<data_dir>/snapshots/<snapshot_id>/` |
| 4 | Copy storage.dat byte-for-byte | `copy_file_with_fsync()` |
| 5 | fsync copied storage | ✓ Implemented |
| 6 | Recursively copy schemas | `copy_dir_recursive()` |
| 7 | fsync schema directory | `fsync_dir()` |
| 8 | Generate manifest.json | `SnapshotManifest::write_to_file()` |
| 9 | fsync manifest | ✓ Implemented |
| 10 | fsync snapshot directory | `fsync_dir()` |
| 11 | Release lock | Caller responsibility |
| 12 | Return snapshot_id | RFC3339 basic format |

---

## Manifest Format (SNAPSHOT.md §3.3)

```json
{
  "snapshot_id": "20260204T164730Z",
  "created_at": "2026-02-04T16:47:30Z",
  "storage_checksum": "crc32:deadbeef",
  "schema_checksums": {
    "user_v1.json": "crc32:abcd1234"
  },
  "format_version": 1
}
```

---

## Error Codes (per ERRORS.md)

| Code | Severity | Description |
|------|----------|-------------|
| `AERO_SNAPSHOT_FAILED` | ERROR | General snapshot creation failure |
| `AERO_SNAPSHOT_IO` | ERROR | I/O failure during snapshot |
| `AERO_SNAPSHOT_MANIFEST` | ERROR | Manifest generation/write failure |

> [!NOTE]
> All snapshot errors are ERROR severity (not FATAL) per spec. Snapshot failure does not require process termination.

---

## Test Results

```
test result: ok. 39 passed; 0 failed; 0 ignored
```

### Test Coverage

| Category | Tests |
|----------|-------|
| Checksum | 8 tests - determinism, file checksums, format/parse |
| Manifest | 8 tests - creation, JSON roundtrip, file I/O |
| Creator | 9 tests - directory creation, byte-for-byte copy, cleanup |
| Errors | 5 tests - error codes, severity, display |
| SnapshotManager | 9 tests - API, lock enforcement, partial failure |

### Full Test Suite

```
test result: ok. 241 passed; 0 failed; 0 ignored
```

---

## Forbidden Behaviors Verified ✓

- ❌ Does NOT include indexes
- ❌ Does NOT modify storage
- ❌ Does NOT modify WAL
- ❌ Does NOT spawn threads
- ❌ Does NOT perform async IO
- ❌ Does NOT skip fsync
- ❌ Does NOT truncate WAL (snapshot is NOT checkpoint)

---

## Spec Compliance Summary

| Requirement | Status |
|------------|--------|
| Deterministic creation | ✓ Same input → same output |
| Atomic visibility | ✓ Only visible after manifest + fsync |
| Full durability | ✓ fsync on all files and directories |
| Zero partial success | ✓ Cleanup on any failure |
| CRC32 checksums | ✓ storage.dat + all schema files |
| RFC3339 basic format | ✓ `YYYYMMDDTHHMMSSZ` |
| format_version = 1 | ✓ Always 1 |
