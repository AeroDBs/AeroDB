# Phase 9: Failure Model

**Document Type:** Technical Specification  
**Phase:** 9 - Auto-Generated REST API  
**Status:** Active

---

## Overview

This document specifies HTTP error mapping for REST API failures.

---

## Error Mapping

### Client Errors (4xx)

| Error | HTTP | Code | When |
|-------|------|------|------|
| Invalid query param | 400 | `invalid_query_param` | Bad syntax |
| Limit exceeded | 400 | `limit_exceeded` | limit > 1000 |
| Invalid filter value | 400 | `invalid_filter` | Type mismatch |
| Missing auth | 401 | `authentication_required` | No token |
| Invalid token | 401 | `invalid_token` | Bad JWT |
| Token expired | 401 | `token_expired` | JWT expired |
| RLS violation | 403 | `access_denied` | Not authorized |
| Not found | 404 | `not_found` | No record |
| Validation error | 422 | `validation_error` | Schema violation |

### Server Errors (5xx)

| Error | HTTP | Code | When |
|-------|------|------|------|
| Internal error | 500 | `internal_error` | Unexpected |
| DB unavailable | 503 | `service_unavailable` | Connection fail |

---

## Error Response Format

```json
{
  "error": {
    "code": "invalid_query_param",
    "message": "Invalid limit value: must be a number",
    "status": 400,
    "details": {
      "param": "limit",
      "value": "abc"
    }
  }
}
```

---

## Error Logging

All 4xx/5xx responses logged with:
- Request ID
- Endpoint
- Error code
- User ID (if authenticated)
- Response time

---

## Determinism

Error responses are deterministic:
- Same invalid input → Same error code
- No randomized error messages
