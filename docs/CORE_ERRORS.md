## Purpose

This document defines the error model for aerodb.

Errors in aerodb are:
- Explicit
- Deterministic
- Classified
- Mapped to invariants

There are no generic or silent failures.

Every error must:
- Have a stable code
- Have a severity
- Be traceable to a violated invariant or rule
- Be explainable to humans

If an error cannot be classified, it must not exist.

---

## Error Philosophy

aerodb does not hide failures.

Errors are treated as part of the public API and operational contract.

The system prefers:
- Loud failure over silent degradation
- Deterministic errors over probabilistic behavior
- Explicit rejection over best-effort execution

---

## Error Categories

All errors belong to exactly one category:

| Category | Meaning |
|----------|---------|
| CONFIG | Startup / configuration invalid |
| SCHEMA | Schema definition or validation |
| QUERY | Query structure or safety |
| STORAGE | Persistent storage failures |
| WAL | Write-Ahead Log failures |
| RECOVERY | Crash recovery failures |
| CORRUPTION | Detected data corruption |
| EXECUTION | Runtime execution errors |
| INTERNAL | Logic violations (bugs) |

Categories are part of the error code.

---

## Severity Levels

Every error has a severity:

| Level | Meaning |
|------|--------|
| FATAL | aerodb must terminate |
| ERROR | Operation fails, server continues |
| REJECT | Client request rejected |
| BUG | Internal invariant violation |

Severity is explicit and machine-readable.

---

## Error Code Format

All error codes follow:

```

AERO_<CATEGORY>_<NAME>

````

Examples:
- `AERO_SCHEMA_VALIDATION_FAILED`
- `AERO_WAL_CORRUPTION`
- `AERO_QUERY_UNBOUNDED`

Codes are stable and never reused.

---

## CONFIG Errors

| Code | Severity | Description |
|------|----------|-------------|
| AERO_CONFIG_INVALID | FATAL | Configuration file malformed |
| AERO_CONFIG_UNSAFE | FATAL | Unsafe configuration detected |
| AERO_CONFIG_IMMUTABLE | FATAL | Attempt to change immutable config |

---

## SCHEMA Errors

| Code | Severity | Description |
|------|----------|-------------|
| AERO_SCHEMA_REQUIRED | REJECT | No schema provided |
| AERO_UNKNOWN_SCHEMA | REJECT | Schema ID not found |
| AERO_UNKNOWN_SCHEMA_VERSION | REJECT | Schema version not found |
| AERO_SCHEMA_VALIDATION_FAILED | REJECT | Document violates schema |
| AERO_SCHEMA_IMMUTABLE | REJECT | Attempt to modify schema |
| AERO_SCHEMA_VERSION_REQUIRED | REJECT | Missing schema_version in query |

---

## QUERY Errors

| Code | Severity | Description |
|------|----------|-------------|
| AERO_QUERY_INVALID | REJECT | Malformed query |
| AERO_QUERY_UNBOUNDED | REJECT | Query has no provable bounds |
| AERO_QUERY_UNINDEXED_FIELD | REJECT | Predicate on non-indexed field |
| AERO_QUERY_LIMIT_REQUIRED | REJECT | Missing or invalid limit |
| AERO_QUERY_SORT_NOT_INDEXED | REJECT | Sort field not indexed |
| AERO_QUERY_SCHEMA_MISMATCH | REJECT | Query schema version mismatch |

---

## STORAGE Errors

| Code | Severity | Description |
|------|----------|-------------|
| AERO_STORAGE_IO_ERROR | ERROR | Disk I/O failure |
| AERO_STORAGE_WRITE_FAILED | ERROR | Document write failed |
| AERO_STORAGE_READ_FAILED | ERROR | Document read failed |

---

## WAL Errors

| Code | Severity | Description |
|------|----------|-------------|
| AERO_WAL_APPEND_FAILED | ERROR | WAL write failed |
| AERO_WAL_FSYNC_FAILED | FATAL | WAL fsync failed |
| AERO_WAL_CORRUPTION | FATAL | WAL checksum failure |

---

## RECOVERY Errors

| Code | Severity | Description |
|------|----------|-------------|
| AERO_RECOVERY_FAILED | FATAL | Recovery did not complete |
| AERO_RECOVERY_SCHEMA_MISSING | FATAL | Schema missing during replay |
| AERO_RECOVERY_INCONSISTENT | FATAL | Storage inconsistency detected |

---

## CORRUPTION Errors

| Code | Severity | Description |
|------|----------|-------------|
| AERO_DATA_CORRUPTION | FATAL | Data checksum failure |
| AERO_INDEX_CORRUPTION | FATAL | Index rebuild failed |
| AERO_METADATA_CORRUPTION | FATAL | Metadata invalid |

---

## EXECUTION Errors

| Code | Severity | Description |
|------|----------|-------------|
| AERO_EXECUTION_FAILED | ERROR | Query execution failure |
| AERO_EXECUTION_TIMEOUT | ERROR | Execution exceeded limits |
| AERO_EXECUTION_LIMIT | REJECT | Memory or row limit exceeded |

---

## INTERNAL Errors (BUGS)

These represent invariant violations.

| Code | Severity | Description |
|------|----------|-------------|
| AERO_INTERNAL_INVARIANT | BUG | Invariant violation |
| AERO_INTERNAL_STATE | BUG | Impossible state |
| AERO_INTERNAL_PANIC | BUG | Unhandled panic |

These must:
- Crash the process
- Produce stack trace
- Be treated as critical bugs

---

## Error Payload Structure

Every error response must include:

```json
{
  "code": "AERO_QUERY_UNBOUNDED",
  "severity": "REJECT",
  "message": "Range query on non-indexed field",
  "invariant": "Q1",
  "details": {
    "field": "age"
  }
}
````

### Required Fields

* `code`
* `severity`
* `message`
* `invariant` (if applicable)

---

## Invariant Mapping

Errors must explicitly reference violated invariants when applicable.

Examples:

| Error                         | Invariant |
| ----------------------------- | --------- |
| AERO_QUERY_UNBOUNDED          | Q1        |
| AERO_SCHEMA_VALIDATION_FAILED | S2        |
| AERO_WAL_CORRUPTION           | K2        |
| AERO_DATA_CORRUPTION          | D2        |
| AERO_CONFIG_UNSAFE            | O2        |

---

## Operator vs Client Responsibility

### Client Errors (REJECT)

Handled by user:

* Query violations
* Schema violations
* Limits exceeded

### Operator Errors (FATAL)

Require human intervention:

* Corruption
* Recovery failure
* WAL failure
* Config failure

---

## Logging Requirements

All errors must be logged with:

* Timestamp
* Error code
* Severity
* Component
* Invariant (if any)
* Human-readable message

FATAL errors must also log:

* Last WAL sequence number
* Recovery state (if applicable)

---

## Forbidden Error Behaviors

The following are forbidden:

* Generic “unknown error”
* Silent retries
* Partial success masking
* Random error messages
* Context-free failures

Errors are contracts.

---

## Phase 0 Trade-offs

* Verbose errors preferred over minimal
* Crash on corruption
* Reject unsafe queries aggressively

These are intentional.

---

## Final Statement

Errors define operational truth.

If aerodb fails quietly,
it has already failed.

Every failure must be explicit,
classified,
and traceable to a rule or invariant.

This is not defensive programming.

This is infrastructure discipline.
