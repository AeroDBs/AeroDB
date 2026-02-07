# Phase 9: REST API Invariants

**Document Type:** Normative Specification  
**Phase:** 9 - Auto-Generated REST API  
**Status:** Active

---

## Query Invariants

### REST-Q1: Bounded Queries

> All queries MUST have a limit. Unbounded queries are rejected with 400.

```rust
if params.limit == 0 || params.limit > MAX_LIMIT {
    return Err(RestError::LimitExceeded);
}
```

### REST-Q2: Deterministic Results

> Same query params + same data → Same JSON response.

Ordering is stable. If no `order` specified, results ordered by primary key.

### REST-Q3: No Guessing

> Invalid query syntax returns 400, not best-effort interpretation.

```
?age=gt.abc  → 400 (invalid number)
?status=unknown.value → 400 (unknown operator)
```

---

## Security Invariants

### REST-S1: RLS Enforcement

> RLS filters are applied BEFORE query execution, not after.

Filter injection happens in handler, before any data access.

### REST-S2: No Silent Bypass

> Missing auth → 401, not anonymous access.

```rust
if context.user_id.is_none() && !context.is_service_role {
    return Err(RestError::AuthenticationRequired);
}
```

### REST-S3: Service Role Explicit

> Service role requires explicit header, not just valid JWT.

```
Authorization: Bearer <jwt>
X-Service-Role: true
```

---

## Response Invariants

### REST-R1: Consistent Structure

> All list responses have same shape.

```json
{
  "data": [...],
  "count": 10,
  "limit": 20,
  "offset": 0
}
```

### REST-R2: Error Structure

> All errors have same shape.

```json
{
  "error": {
    "code": "string",
    "message": "string",
    "status": 400
  }
}
```

---

## Performance Invariants

### REST-P1: No N+1 Queries

> Relations (when implemented) use batch loading.

### REST-P2: Index Hints

> Filter fields should have indexes for performance.

Warning logged if filtering on non-indexed field.
