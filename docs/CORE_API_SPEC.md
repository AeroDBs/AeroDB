This document defines the **authoritative client-facing API** for AeroDB Phase 0.

It governs:

- API Layer implementation
- CLI behavior
- Integration tests
- Client expectations

If implementation behavior conflicts with this document, the implementation is wrong.

This API is intentionally minimal, explicit, and deterministic.

No undocumented behavior is allowed.

---

## 1. Core Principles

The AeroDB API follows these non-negotiable rules:

- All operations are synchronous.
- All requests are serialized (global execution lock).
- All errors are explicit.
- No implicit defaults.
- No hidden metadata.
- No automatic ID generation.
- No partial success.

Every request either:

- fully succeeds  
or  
- fully fails  

There is no intermediate state.

---

## 2. Transport

Phase 0 uses **JSON over stdin/stdout** (via CLI or embedded API).

There is no HTTP server in Phase 0.

All requests and responses are JSON objects.

Encoding: UTF-8.

---

## 3. Request Envelope

Every request MUST be a JSON object with:

```

{
"op": "<operation>",
...
}

```

If `op` is missing or unknown:

→ `AERO_QUERY_INVALID`

No additional top-level fields are allowed unless explicitly documented.

---

## 4. Supported Operations

Phase 0 supports exactly:

- insert
- update
- delete
- query
- explain

No other operations exist.

---

## 5. Insert

### Request

```

{
"op": "insert",
"schema_id": "user",
"schema_version": "v1",
"document": {
"_id": "123",
"name": "alice",
"age": 30
}
}

```

### Rules

- `_id` is mandatory
- Schema must exist
- Schema version must exist
- Document must fully validate

### Success Response

```

{
"status": "ok",
"data": []
}

```

Insert does NOT return the document.

---

## 6. Update

### Request

```

{
"op": "update",
"schema_id": "user",
"schema_version": "v1",
"document": {
"_id": "123",
"name": "alice",
"age": 31
}
}

```

### Rules

- Full document replacement only
- `_id` must match existing document
- `_id` is immutable
- Same validation rules as insert

### Success Response

```

{
"status": "ok",
"data": []
}

```

No partial updates exist in Phase 0.

---

## 7. Delete

### Request

```

{
"op": "delete",
"schema_id": "user",
"schema_version": "v1",
"_id": "123"
}

```

### Rules

- `_id` required
- Delete writes tombstone
- Silent success if document does not exist

### Success Response

```

{
"status": "ok",
"data": []
}

```

---

## 8. Query

### Request

```

{
"op": "query",
"schema_id": "user",
"schema_version": "v1",
"filter": {
"age": 30
},
"sort": "name",
"limit": 10
}

```

### Required Fields

- schema_id
- schema_version
- filter
- limit

### Optional Fields

- sort

---

### Filter Rules

- AND only
- Equality or indexed range
- No OR
- No functions
- No expressions
- All fields must be indexed

---

### Limit Rules

- Must be present
- Must be > 0

Missing or invalid limit:

→ `AERO_QUERY_LIMIT_REQUIRED`

---

### Sort Rules

- Must reference indexed field
- Optional
- Stable deterministic ordering

Unindexed sort:

→ `AERO_QUERY_SORT_NOT_INDEXED`

---

### Success Response

```

{
"status": "ok",
"data": [
{
"_id": "123",
"name": "alice",
"age": 30
}
]
}

```

Returned documents are ordered and schema-valid.

---

## 9. Explain

Explain uses the same input as query:

```

{
"op": "explain",
"schema_id": "user",
"schema_version": "v1",
"filter": {
"age": 30
},
"sort": "name",
"limit": 10
}

```

### Response

```

{
"status": "ok",
"data": {
"chosen_index": "age",
"scan_type": "EQ",
"predicates": ["age = 30"],
"sort": "name",
"limit": 10,
"bounded": true
}
}

```

Explain output must be deterministic.

---

## 10. Error Response Format

All errors use:

```

{
"status": "error",
"code": "AERO_QUERY_UNBOUNDED",
"message": "Range query requires explicit limit"
}

```

Rules:

- code must match ERRORS.md
- message is human-readable
- no stack traces
- no internal details

---

## 11. Error Propagation

API Layer must:

- pass subsystem errors directly
- not remap codes
- not downgrade severity
- not suppress failures

Corruption errors are always FATAL.

---

## 12. Determinism

API must NOT:

- add timestamps
- add request IDs
- generate document IDs
- inject metadata
- reorder results

Given same input and same database state:

API must return identical output.

---

## 13. Serialization

All API calls are serialized via global execution lock.

Only one request executes at any time.

Concurrent requests block.

---

## 14. Phase-0 Limitations

Explicitly unsupported:

- joins
- aggregations
- projections
- partial updates
- transactions
- pagination
- streaming
- subscriptions
- bulk writes

Any attempt results in `AERO_QUERY_INVALID`.

---

## 15. Authority

This document governs:

- API Layer
- CLI request formats
- Client expectations
- Integration tests

Any deviation is a correctness bug.
