## Purpose

This document defines the query model, syntax, semantics, and enforcement rules
for aerodb in Phase 0.

Queries in aerodb are:
- Explicit
- Deterministic
- Bounded
- Schema-version-specific

Any query that violates these properties is rejected before execution.

---

## Query Philosophy

aerodb does not attempt to be expressive.
It attempts to be predictable.

Queries are designed to:
- Be statically analyzable
- Have provable upper bounds
- Produce deterministic results
- Never guess user intent

If a query cannot be safely reasoned about, it is forbidden.

---

## Query Scope (Phase 0)

### Supported
- Single-collection queries
- Equality and bounded range predicates
- Explicit sorting
- Explicit limits
- Deterministic execution plans

### Explicitly Not Supported
- Joins
- Aggregations
- Subqueries
- Cross-collection queries
- Full-text search
- Geospatial queries
- Vector search
- Ad-hoc projections
- Server-side functions

---

## Query Structure

All queries must conform to the following structure:

```json
{
  "collection": "<string>",
  "schema_id": "<string>",
  "schema_version": "<string>",
  "filter": { ... },
  "sort": [ ... ],
  "limit": <integer>
}
````

### Required Fields

| Field            | Required | Description                     |
| ---------------- | -------- | ------------------------------- |
| `collection`     | Yes      | Target collection name          |
| `schema_id`      | Yes      | Schema identifier               |
| `schema_version` | Yes      | Explicit schema version         |
| `filter`         | Yes      | Predicate object (may be empty) |
| `limit`          | Yes      | Maximum number of documents     |

There are **no defaults**.
Missing fields cause query rejection.

---

## Filter Semantics

### Filter Object

The `filter` field is a JSON object describing predicates.

Example:

```json
{
  "email": "user@example.com",
  "age": { "$gte": 18, "$lte": 30 }
}
```

---

### Supported Predicate Types

#### Equality Predicate

```json
{ "field": "value" }
```

Rules:

* Field must be indexed
* Field must exist in schema
* Exact type match required

---

#### Range Predicate

```json
{ "field": { "$gte": 10, "$lte": 20 } }
```

Rules:

* Field must be indexed
* At least one bound required
* Must be paired with an explicit `limit`
* Only numeric fields allowed

---

### Predicate Combination Rules

* All predicates are combined using logical AND
* OR conditions are forbidden
* Nested predicates are forbidden

---

### Forbidden Filters

The following cause immediate rejection:

* Filters on non-indexed fields
* Empty filter with no primary key
* OR conditions
* Regex or pattern matching
* Functions or expressions
* Implicit type conversion

---

## Primary Key Queries

### `_id` Lookup

Example:

```json
{
  "filter": { "_id": "abc123" },
  "limit": 1
}
```

Rules:

* `_id` equality queries are always bounded
* `limit` must be `1`
* `_id` must be indexed (mandatory)

---

## Sorting Semantics

### Sort Structure

```json
"sort": [
  { "field": "age", "order": "asc" }
]
```

### Sort Rules

* Sort fields must be indexed
* Sort order must be explicit (`asc` or `desc`)
* Multi-field sort is forbidden in Phase 0
* Sorting without an index is forbidden

---

## Limit Semantics

### Limit Rules

* `limit` is mandatory
* `limit` must be a positive integer
* Maximum allowed limit is implementation-defined
* Queries without `limit` are rejected

Limits are part of query safety guarantees.

---

## Query Boundedness Rules

A query is considered **bounded** if:

1. It uses only indexed fields
2. It includes a mandatory `limit`
3. The planner can compute an upper bound on:

   * Documents scanned
   * Memory usage
   * Execution time

Queries that fail boundedness checks are rejected.

---

## Query Planning Rules

### Deterministic Planner Requirements

* Planner uses rule-based index selection
* No statistics-driven heuristics
* No adaptive optimization
* Same inputs → same plan

### Index Selection Priority

1. Primary key equality
2. Indexed equality
3. Indexed range with limit

If no valid index applies → reject query.

---

## Execution Semantics

### Execution Guarantees

* Single-threaded execution
* Stable iteration order
* Deterministic result ordering
* No runtime plan changes

### Result Ordering

Results are ordered by:

* Index traversal order, or
* Primary key order if applicable

There is no implicit ordering.

---

## Schema Enforcement During Queries

* All returned documents must match:

  * `schema_id`
  * `schema_version`
* Documents with different versions are excluded
* Cross-version reads are forbidden

Queries without explicit schema version are rejected.

---

## Error Handling

### Query Errors

| Error Code               | Condition                           |
| ------------------------ | ----------------------------------- |
| `QUERY_INVALID`          | Malformed query structure           |
| `SCHEMA_REQUIRED`        | Missing schema fields               |
| `UNKNOWN_SCHEMA`         | Schema ID not found                 |
| `UNKNOWN_SCHEMA_VERSION` | Schema version not found            |
| `UNBOUNDED_QUERY`        | Cannot prove bounded execution      |
| `UNINDEXED_FIELD`        | Filter or sort on non-indexed field |
| `LIMIT_REQUIRED`         | Missing or invalid limit            |
| `SORT_NOT_INDEXED`       | Sort field not indexed              |

Errors are deterministic and explicit.

---

## Explain Plan Requirement

Every query must support an explain plan.

Explain output includes:

* Selected index
* Predicate evaluation order
* Estimated bounds
* Rejection reason (if applicable)

Explain plans are human-readable and deterministic.

---

## Forbidden Query Behaviors

The following are strictly forbidden:

* Implicit full scans
* Implicit sorting
* Implicit limits
* Guessing user intent
* Adaptive re-planning
* Partial execution

Any such behavior is a bug.

---

## Invariants Enforced by Query System

| Invariant | Enforcement              |
| --------- | ------------------------ |
| Q1        | Mandatory bounds + limit |
| Q2        | No implicit scans        |
| Q3        | No guessing              |
| T1        | Deterministic planning   |
| T2        | Deterministic execution  |
| F1        | Fail loudly              |
| F3        | Deterministic errors     |

---

## Phase 0 Trade-offs (Intentional)

* Expressiveness is limited
* Some valid workloads are rejected
* Clients must structure data carefully
* Safety > flexibility

These trade-offs are deliberate.

---

## Final Statement

Queries are executable contracts.

If aerodb cannot prove a query is safe,
it refuses to run it.

This is not a limitation.
It is the foundation of predictability.
