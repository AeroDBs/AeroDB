# Schema Validator Implementation

## Summary

Implemented the Schema Validator subsystem for aerodb Phase 0 at `src/schema/`, fully compliant with `SCHEMA.md`.

## Module Structure

```
src/schema/
├── mod.rs       # Public exports
├── errors.rs    # Error codes (AERO_SCHEMA_*, AERO_UNKNOWN_*)
├── types.rs     # FieldType, FieldDef, Schema struct
├── loader.rs    # Loading from metadata/schemas/, in-memory registry
└── validator.rs # Validation logic with no implicit coercion
```

## Schema Definition (per SCHEMA.md)

### Supported Types (Phase 0)
- `string`: UTF-8 string
- `int`: 64-bit signed integer
- `bool`: Boolean
- `float`: 64-bit floating point
- `object`: Nested object with explicit field definitions
- `array`: Homogeneous array with single element type

### Rules
- **Mandatory**: Every write must specify `schema_id` and `schema_version`.
- **Immutable**: Once registered, a schema version cannot change.
- **Strict**: No nulls (Phase 0), no extra fields, no type casting.
- **Primary Key**: `_id` is required, unique, and immutable.

## Invariants Enforced

| Invariant | Enforcement |
|---|---|
| S1 | Schema mandatory on all writes |
| S2 | Validation before WAL append |
| S3 | Explicit version binding (loader registry) |
| S4 | Violations abort writes (REJECT severity) |
| Q3 | No guessing or coercion (strict type checks) |

## Error Codes (per ERRORS.md)

| Code | Severity | Description |
|---|---|---|
| `AERO_SCHEMA_REQUIRED` | REJECT | No schema specified |
| `AERO_UNKNOWN_SCHEMA` | REJECT | Schema ID not found |
| `AERO_UNKNOWN_SCHEMA_VERSION` | REJECT | Version not found |
| `AERO_SCHEMA_VALIDATION_FAILED` | REJECT | Type mismatch, extra fields, etc. |
| `AERO_SCHEMA_IMMUTABLE` | REJECT | Attempt to overwrite schema |
| `AERO_RECOVERY_SCHEMA_MISSING` | FATAL | Schema file missing at startup |

## Test Results

All 99 tests passed (29 Schema + 31 Storage + 39 WAL).

Key tests implemented:
- `test_valid_document_passes`: Happy path
- `test_extra_field_fails`: No undeclared fields
- `test_type_mismatch_fails`: Strict typing
- `test_nested_object_validation`: Recursive validation
- `test_id_immutable_on_update`: Identity preservation
- `test_null_rejected`: Strict non-null policy

## Key Design Decisions

1.  **Validation before WAL**: Bad data never hits the log.
2.  **Deterministic**: Validation depends only on the document and loaded schema.
3.  **No Type Coercion**: `123` is not `"123"`.
4.  **Implicit Rejection**: Missing fields (if required) or extra fields always reject.
