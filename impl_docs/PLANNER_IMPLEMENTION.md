# Query Planner Implementation

## Summary

Implemented the Query Planner subsystem for aerodb Phase 0 at `src/planner/`, fully compliant with `QUERY.md`.

## Module Structure

```
src/planner/
├── mod.rs       # Public exports
├── errors.rs    # Error codes (AERO_QUERY_*)
├── ast.rs       # Query, Predicate, FilterOp, SortSpec
├── bounds.rs    # Static boundedness analysis (must pass before planning)
├── planner.rs   # Deterministic plan generation logic
└── explain.rs   # Human-readable explain output
```

## Query Lifecycle

1.  **Parse**: Query JSON -> `Query` AST (not implemented in this phase, AST is constructed programmatically).
2.  **Validate Schema**: Check `schema_id` and `schema_version` against registry.
3.  **Prove Boundedness**: `BoundednessAnalyzer` checks if execution is safe.
    - Limit > 0 required.
    - All filters must be on indexed fields.
    - Sort field must be indexed.
4.  **Select Index**: `QueryPlanner` strictly orders choices:
    1.  Primary Key (`_id`) = `PK_LOOKUP`
    2.  Indexed Equality = `INDEX_EQ`
    3.  Indexed Range = `INDEX_RANGE`
    *   Ties broken by lexicographically smallest field name.
5.  **Generate Plan**: Produce immutable `QueryPlan`.

## Boundedness Rules (per QUERY.md §205)

A query is bounded if and only if:
- `limit` is present and positive.
- All filter predicates reference indexed fields.
- The sort field (if present) is indexed.
- No OR conditions (Phase 0).
- No functions or expressions.

Violations result in `AERO_QUERY_UNBOUNDED`, `AERO_QUERY_UNINDEXED_FIELD`, or `AERO_QUERY_LIMIT_REQUIRED`.

## Index Selection Priority

The planner is **rule-based and deterministic**:

1.  **Primary Key Equality**: `_id = "..."` (O(1) lookup).
2.  **Indexed Equality**: `field = "..."`.
3.  **Indexed Range**: `field >= ...` with limit.

If multiple indexes qualify for the same priority (e.g., `a=1 AND b=2`), the field name occurring first alphabetically is chosen (`a`).

## Error Codes (per ERRORS.md)

| Code | Severity | Description |
|---|---|---|
| `AERO_QUERY_INVALID` | REJECT | Malformed query structure |
| `AERO_QUERY_UNBOUNDED` | REJECT | Impossible to prove safety |
| `AERO_QUERY_UNINDEXED_FIELD` | REJECT | Filter/sort on non-indexed field |
| `AERO_QUERY_LIMIT_REQUIRED` | REJECT | Missing `limit` |
| `AERO_QUERY_SORT_NOT_INDEXED`| REJECT | Sort field not indexed |
| `AERO_SCHEMA_VERSION_REQUIRED`| REJECT | Missing schema version |

## Test Results

All 124 tests passed (25 Planner + 29 Schema + 31 Storage + 39 WAL).

Key planner tests:
- `test_pk_equality_plan`: Correctly selects PK scan.
- `test_indexed_equality_plan`: Selects EQ scan.
- `test_indexed_range_plan`: Selects RANGE scan.
- `test_lexicographic_index_selection`: Deterministic tie-breaking.
- `test_missing_limit_rejected`: Enforces safety.
- `test_unindexed_filter_rejected`: Prevent full scans.
- `test_explain_deterministic`: Explain output stability.
