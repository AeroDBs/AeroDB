# Phase 8: Failure Model

**Document Type:** Normative Specification  
**Phase:** 8 - Authentication & Authorization  
**Status:** Active

---

## Overview

This document specifies authentication failure handling with fail-closed semantics.

---

## Fail-Closed Principle

> **AUTH-F1:** On any authentication error, the system MUST deny access. No fallback to anonymous access.

---

## Error Categories

### Authentication Errors (4xx)

| Error | HTTP | Response | Action |
|-------|------|----------|--------|
| Invalid credentials | 401 | `invalid_credentials` | Log attempt |
| Token expired | 401 | `token_expired` | Prompt refresh |
| Token invalid | 401 | `invalid_token` | Clear client state |
| Session revoked | 401 | `session_revoked` | Force re-auth |
| Missing auth | 401 | `authentication_required` | Prompt login |

### Authorization Errors (4xx)

| Error | HTTP | Response | Action |
|-------|------|----------|--------|
| RLS violation | 403 | `access_denied` | Log violation |
| Resource not owned | 403 | `forbidden` | Log violation |
| Insufficient role | 403 | `insufficient_permissions` | Log violation |

### Server Errors (5xx)

| Error | HTTP | Response | Action |
|-------|------|----------|--------|
| Crypto failure | 500 | `internal_error` | Alert ops |
| DB unavailable | 503 | `service_unavailable` | Retry with backoff |
| Token signing failed | 500 | `internal_error` | Alert ops |

---

## Error Response Format

```json
{
  "error": {
    "code": "invalid_credentials",
    "message": "Invalid email or password",
    "status": 401
  }
}
```

### Response Invariants

- **AUTH-E1:** Error messages never reveal which field was wrong
- **AUTH-E2:** No timing differences between valid/invalid users
- **AUTH-E3:** Rate limiting applied before detailed validation

---

## Rate Limiting

### Limits

| Endpoint | Window | Max Attempts |
|----------|--------|--------------|
| `/auth/login` | 15 min | 5 per email |
| `/auth/signup` | 1 hour | 3 per IP |
| `/auth/forgot-password` | 1 hour | 3 per email |

### Response

```json
{
  "error": {
    "code": "rate_limit_exceeded",
    "message": "Too many attempts. Try again later.",
    "status": 429,
    "retry_after": 900
  }
}
```

---

## Failure Logging

### Required Fields

```json
{
  "event": "auth_failure",
  "error_code": "invalid_credentials",
  "email_hash": "sha256(email)",
  "ip_address": "x.x.x.x",
  "user_agent": "...",
  "timestamp": "2026-02-06T00:00:00Z"
}
```

### Privacy Invariants

- **AUTH-L1:** Full email never logged (use hash)
- **AUTH-L2:** Passwords never logged
- **AUTH-L3:** IP logged for security, subject to retention policy

---

## Recovery Procedures

### Account Lockout

After 10 failed attempts in 1 hour:

1. Account temporarily locked (30 min)
2. Email notification sent to user
3. Admin can manually unlock

### Brute Force Detection

Pattern detection triggers:

1. Alert to security monitoring
2. CAPTCHA requirement
3. IP-level rate limiting

---

## Determinism

Authentication failures are **deterministic**:

- Same input → Same error code
- No randomized delays in error paths
- Timing attacks mitigated via constant-time comparison
