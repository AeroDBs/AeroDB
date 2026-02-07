# Query Executor Implementation

## Summary

Implemented the Query Executor subsystem for aerodb Phase 0 at `src/executor/`, fully compliant with `QUERY.md` and `ERRORS.md`.

## Module Structure

```
src/executor/
├── mod.rs       # Public exports
├── errors.rs    # Error codes (AERO_EXECUTION_*, AERO_DATA_CORRUPTION)
├── result.rs    # ResultDocument, ExecutionResult
├── executor.rs  # Main execution logic (offsets -> read -> filter -> sort)
├── filters.rs   # Strict predicate evaluation
└── sorter.rs    # Deterministic stable sorting
```

## Execution Flow (Strict Order)

The execution pipeline is rigid and deterministic. No steps can be reordered or skipped.

1.  **Index Lookup**: Resolve `chosen_index` from the plan to a list of candidate storage offsets.
2.  **Storage Read**: Read `DocumentRecord` from storage at each offset.
    - **CRITICAL**: Checksum is validated immediately handling `AERO_DATA_CORRUPTION` as FATAL.
    - Tombstones are skipped.
3.  **Schema Filtering**:
    - Check if `record.schema_id` matches plan.
    - Check if `record.schema_version` matches plan.
    - Mismatch = exclude (silent skip, not error).
4.  **Predicate Filtering**:
    - Apply all filters (logical AND).
    - Exact type matching only (no coercion).
    - Nulls never match.
5.  **Sorting**:
    - If `sort` is present, sort candidate documents using stable sort.
    - Ranking: Null < Bool < Number < String < Array < Object.
6.  **Limit Application**:
    - Truncate result list to `limit`.
7.  **Result Construction**: Return `ExecutionResult`.

## Invariants Enforced

| Invariant | Enforcement |
|---|---|
| T2 | Deterministic execution (stable sort, strict order) |
| D2 | Checksum validation on every read |
| F1 | Fail loudly on corruption (AERO_DATA_CORRUPTION) |
| Q1 | Limit enforced physically |
| Q2 | Only indexed documents are candidates |

## Error Codes (per ERRORS.md)

| Code | Severity | Description |
|---|---|---|
| `AERO_EXECUTION_FAILED` | ERROR | General failure |
| `AERO_DATA_CORRUPTION` | FATAL | Checksum mismatch during read |
| `AERO_EXECUTION_LIMIT` | ERROR | Resource limit exceeded (not yet used) |

## Test Results

All 148 tests passed (24 Executor + 25 Planner + 29 Schema + 31 Storage + 39 WAL).

Key executor tests:
- `test_pk_lookup_execution`: Verifies end-to-end ID lookup.
- `test_indexed_range_with_limit`: Verifies constraints.
- `test_corruption_halts_execution`: Verifies FATAL error on bad checksum.
- `test_schema_mismatch_excluded`: Verifies multi-version coexistence safety.
- `test_deterministic_ordering`: Verifies stable sort.
