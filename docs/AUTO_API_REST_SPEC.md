# AUTO_API_REST_SPEC.md — REST API Generator Specification

## Status
- Phase: **9**
- Authority: **Normative**
- Depends on: Phase 8 (Authentication & RLS)
- Date: 2026-02-06

---

## 1. Purpose

This document defines the specification for AeroDB's auto-generated REST API layer, which provides CRUD operations for all collections based on their schemas.

---

## 2. Design Philosophy

### 2.1 Schema-Driven

REST endpoints are **generated from schema definitions**:
- No manual endpoint registration required
- Schema changes automatically update available endpoints
- Explicit reload required (no auto-watch)

### 2.2 Deterministic Query Translation

REST queries translate to AeroDB operations deterministically:
- Same URL + query params → same AeroDB query
- No hidden optimization or caching
- Bounded queries enforced (no unbounded scans)

### 2.3 RLS Integration

All REST endpoints enforce Row-Level Security:
- Authenticated requests carry RLS context
- Filters are injected before execution
- Service role can bypass RLS

---

## 3. Endpoint Structure

### 3.1 Collection Endpoints

For each collection `{collection}`:

| Method | Path | Description |
|--------|------|-------------|
| GET | `/rest/v1/{collection}` | List records (with filters) |
| GET | `/rest/v1/{collection}/{id}` | Get single record |
| POST | `/rest/v1/{collection}` | Insert record(s) |
| PATCH | `/rest/v1/{collection}/{id}` | Update record |
| DELETE | `/rest/v1/{collection}/{id}` | Delete record |

### 3.2 Authentication

All endpoints require authentication via:
- `Authorization: Bearer <access_token>` header
- `apikey: <api_key>` header (for service role)

---

## 4. Query Parameters

### 4.1 Filtering

| Operator | Syntax | Description |
|----------|--------|-------------|
| eq | `?field=eq.value` | Equals |
| neq | `?field=neq.value` | Not equals |
| gt | `?field=gt.10` | Greater than |
| gte | `?field=gte.10` | Greater than or equal |
| lt | `?field=lt.10` | Less than |
| lte | `?field=lte.10` | Less than or equal |
| like | `?field=like.*pattern*` | Pattern match |
| in | `?field=in.(a,b,c)` | In list |

### 4.2 Sorting

```
?order=field.asc
?order=field.desc
?order=field1.asc,field2.desc
```

### 4.3 Pagination

```
?limit=20
?offset=0
```

**Default limit:** 100
**Maximum limit:** 1000

### 4.4 Field Selection

```
?select=id,name,email
?select=*  (all fields)
```

---

## 5. Request/Response Format

### 5.1 List Response

```json
{
  "data": [...],
  "count": 42,
  "limit": 20,
  "offset": 0
}
```

### 5.2 Single Record Response

```json
{
  "data": { ... }
}
```

### 5.3 Insert Request

```json
{
  "field1": "value1",
  "field2": "value2"
}
```

Or batch insert:
```json
[
  { "field1": "value1" },
  { "field1": "value2" }
]
```

### 5.4 Error Response

```json
{
  "error": "Description of error",
  "code": 400
}
```

---

## 6. HTTP Status Codes

| Code | Description |
|------|-------------|
| 200 | Success |
| 201 | Created |
| 400 | Bad request (invalid query) |
| 401 | Unauthorized (missing/invalid auth) |
| 403 | Forbidden (RLS violation) |
| 404 | Not found |
| 409 | Conflict (duplicate key) |
| 500 | Internal error |

---

## 7. Module Structure

```
src/rest_api/
├── mod.rs           # Module exports
├── server.rs        # HTTP server setup (axum)
├── generator.rs     # Schema → endpoint mapping
├── parser.rs        # Query parameter parsing
├── filter.rs        # Filter AST generation
├── handler.rs       # Request handlers
├── response.rs      # Response formatting
└── errors.rs        # HTTP error types
```

---

## 8. Invariants

| ID | Invariant |
|----|-----------|
| **REST-1** | Unbounded queries MUST be rejected (limit required) |
| **REST-2** | RLS MUST be enforced on all operations |
| **REST-3** | Query translation MUST be deterministic |
| **REST-4** | Schema changes require explicit reload |
| **REST-5** | All errors MUST map to appropriate HTTP codes |

---

END OF DOCUMENT
