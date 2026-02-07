# Index Manager Implementation

## Summary

Implemented the Index Manager subsystem for aerodb Phase 0 at `src/index/`. Indexes are strict, in-memory, BTreeMap-based structures consistent with `QUERY.md` and `STORAGE.md`.

## Module Structure

```
src/index/
├── mod.rs       # Public exports
├── errors.rs    # Error codes (All FATAL)
├── btree.rs     # Deterministic BTreeMap<IndexKey, Vec<Offset>>
└── manager.rs   # Index lifecycle (rebuild, write, delete, lookup)
```

## Functional Capabilities

### 1. In-Memory Indexes
- **Structure**: `BTreeMap<IndexKey, Vec<StorageOffset>>`
- **Keys**: `Bool < Int < Float < String` (Deterministic ordering)
- **Offsets**: Always sorted ascending in the value vector.
- **Persistence**: None (rebuilt on startup).

### 2. Startup Rebuild
- **`rebuild_from_storage`**:
    - Scans storage sequentially.
    - Extracts indexed fields from documents.
    - Populates in-memory trees.
    - Aborts strictly on `AERO_DATA_CORRUPTION`.

### 3. Runtime Updates
- **`apply_write`**: Updates index after a successful storage write.
- **`apply_delete`**: Removes document from all indexes after a tombstone write.

### 4. Lookup API
- **`lookup_eq`**: Exact match, returns sorted offsets.
- **`lookup_range`**: Range match (min/max), returns sorted offsets.
- **Filtering**: None (handled by Executor).

## Invariants Enforced

| Invariant | Enforcement |
|---|---|
| R1 | Index is derived state, rebuilt from storage |
| K2 | Halt-on-corruption during rebuild |
| D2 | Checksum validation during scan |
| T1 | Deterministic ordering (BTreeMap + sorted offsets) |

## Error Codes (per ERRORS.md)

All index errors are **FATAL**.

| Code | Description |
|---|---|
| `AERO_INDEX_BUILD_FAILED` | General build failure |
| `AERO_DATA_CORRUPTION` | Checksum mismatch during rebuild |

## Test Results

All 180 tests passed (16 Index + 16 Recovery + ...).

Key index tests:
- `test_rebuild_from_storage`: Verifies indexes are correctly populated.
- `test_delete_removes_index_entry`: Verifies removals works.
- `test_lookup_range_deterministic`: Verifies stable output order.
- `test_corruption_during_rebuild_halts`: Verifies safety.
- `test_tombstones_ignored`: Verifies correct handling of deletions during rebuild.
