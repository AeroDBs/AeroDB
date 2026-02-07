## Purpose

This document defines the schema system for aerodb.

Schemas in aerodb are not optional validation helpers.
They are **first-class system artifacts** that define what data is allowed to exist.

If data does not conform to a schema, it does not exist.

This document is authoritative.
Any implementation that weakens these rules is incorrect.

---

## Schema Philosophy

aerodb treats schemas as:

- Mandatory
- Explicit
- Versioned
- Immutable once published
- Enforced at write time
- Referenced at read time

Schemas are designed to prevent:
- Silent data drift
- Implicit coercion
- Schema chaos over time
- Ambiguous reads

Flexibility is intentionally sacrificed for correctness.

---

## Schema Scope (Phase 0)

### Supported
- Per-collection schemas
- Strict field typing
- Required vs optional fields
- Explicit schema versions
- Full-document validation

### Explicitly Not Supported (Phase 0)
- Schemaless writes
- Partial schema enforcement
- Implicit type coercion
- Schema evolution automation
- Cross-version reads
- Field-level migrations
- Dynamic or computed fields

---

## Schema Identity

Each schema is uniquely identified by:

- `schema_id` (string, globally unique)
- `schema_version` (monotonic integer or semantic tag)

### Example
```

schema_id: "users"
schema_version: "v1"

```

The tuple `(schema_id, schema_version)` uniquely identifies a schema.

---

## Schema Storage

Schemas are stored on disk at:

```

metadata/schemas/schema_<schema_id>_<version>.json

````

### Rules
- One file per schema version
- Schema files are immutable once written
- Deleting or modifying a schema file is forbidden
- Missing schema files cause startup failure

---

## Schema File Format

Schemas are defined using a **strict JSON-based DSL**.

### Top-Level Structure

```json
{
  "schema_id": "users",
  "schema_version": "v1",
  "description": "User account records",
  "fields": {
    "_id": {
      "type": "string",
      "required": true
    },
    "email": {
      "type": "string",
      "required": true
    },
    "age": {
      "type": "int",
      "required": false
    }
  }
}
````

---

## Field Definition Rules

Each field definition must specify:

| Property   | Required | Description                   |
| ---------- | -------- | ----------------------------- |
| `type`     | Yes      | Field data type               |
| `required` | Yes      | Whether field must be present |

No other properties are allowed in Phase 0.

---

## Supported Field Types (Phase 0)

| Type     | Description                                      |
| -------- | ------------------------------------------------ |
| `string` | UTF-8 string                                     |
| `int`    | 64-bit signed integer                            |
| `bool`   | Boolean                                          |
| `float`  | 64-bit floating point                            |
| `object` | Nested object with its own schema                |
| `array`  | Homogeneous array (single declared element type) |

### Notes

* Arrays must declare a single element type
* Objects must declare nested field schemas explicitly
* No union types
* No polymorphism

---

## Primary Key Rules

### `_id` Field

* Every schema **must** define an `_id` field
* `_id` must be:

  * Required
  * Unique within the collection
  * Immutable after insertion

The `_id` field is the primary key.

---

## Schema Validation Rules

### Validation Timing

* Validation occurs **before WAL append**
* Invalid data must never enter WAL or storage

---

### Validation Semantics

A document is valid if and only if:

1. All required fields are present
2. No undeclared fields exist
3. Field types exactly match schema types
4. `_id` is present and valid
5. Schema version exists and is known

Failure of any rule causes write rejection.

---

## Forbidden Behaviors

The following are strictly forbidden:

* Missing required fields
* Extra undeclared fields
* Implicit type coercion
* Default values
* Null values unless explicitly allowed (Phase 0: null forbidden)
* Partial validation

---

## Schema Versioning Rules

### Immutability

Once a schema version is created:

* It must never change
* It must never be overwritten
* It must never be deleted

Any change requires a **new schema version**.

---

### Creating a New Version

A new schema version must:

* Have a new `schema_version` identifier
* Be stored as a new schema file
* Not affect existing documents

No automatic migration occurs in Phase 0.

---

## Write-Time Schema Binding

Every write operation must explicitly specify:

* `schema_id`
* `schema_version`

Example write payload:

```json
{
  "schema_id": "users",
  "schema_version": "v1",
  "document": {
    "_id": "u123",
    "email": "user@example.com",
    "age": 30
  }
}
```

Writes without explicit schema version are rejected.

---

## Read-Time Schema Rules

Phase 0 read rules are strict:

* Every query must specify exactly one schema version
* Documents with other schema versions are excluded
* Cross-version reads are forbidden

This ensures:

* Predictable result shapes
* Deterministic execution
* No implicit compatibility assumptions

---

## Schema and WAL Interaction

* WAL records include:

  * `schema_id`
  * `schema_version`
* WAL replay validates schema existence
* Missing schema during recovery causes startup failure

Schemas are part of durability guarantees.

---

## Schema and Index Interaction

* Indexes are defined per `(schema_id, schema_version)`
* Indexes never span multiple schema versions
* Index rebuild uses schema to validate stored documents

---

## Error Handling

### Schema-Related Errors

| Error Code                 | Condition                         |
| -------------------------- | --------------------------------- |
| `SCHEMA_REQUIRED`          | No schema specified               |
| `UNKNOWN_SCHEMA`           | Schema ID not found               |
| `UNKNOWN_SCHEMA_VERSION`   | Version not found                 |
| `SCHEMA_VALIDATION_FAILED` | Document violates schema          |
| `SCHEMA_IMMUTABLE`         | Attempt to modify existing schema |

All schema errors are fatal to the operation.

---

## Invariants Enforced by Schema System

| Invariant | Enforcement                    |
| --------- | ------------------------------ |
| S1        | Schema mandatory on all writes |
| S2        | Validation before WAL          |
| S3        | Explicit version binding       |
| S4        | Violations abort writes        |
| D3        | Reads return schema-valid data |
| Q3        | No guessing or coercion        |

---

## Phase 0 Trade-offs (Intentional)

* No automatic migrations
* No backward compatibility checks
* No schema evolution tooling
* No optional fields defaulting
* No flexible typing

These are deferred by design.

---

## Final Statement

Schemas are the contract between data and trust.

aerodb does not attempt to infer intent,
repair mistakes,
or adapt silently.

If data does not match the schema,
it is rejected.

This strictness is not a limitation.
It is the foundation of reliability.

