# Phase 8: Token Model

**Document Type:** Normative Specification  
**Phase:** 8 - Authentication & Authorization  
**Status:** Active

---

## Overview

This document specifies JWT access token design, claims structure, and token rotation for AeroDB authentication.

---

## Token Types

| Type | Storage | Lifetime | Purpose |
|------|---------|----------|---------|
| Access Token | Stateless (JWT) | 15 min | API authentication |
| Refresh Token | Stateful (DB) | 30 days | Access token renewal |
| API Key | Stateful (DB) | Unlimited | Service authentication |

---

## JWT Access Token

### Algorithm

- **Algorithm:** HS256 (HMAC-SHA256)
- **Future:** RS256 for asymmetric verification

### Claims Structure

```json
{
  "sub": "user-uuid",
  "email": "user@example.com",
  "iat": 1707192000,
  "exp": 1707192900,
  "aud": "aerodb",
  "iss": "aerodb-auth",
  "role": "authenticated"
}
```

### Standard Claims

| Claim | Type | Description |
|-------|------|-------------|
| `sub` | string | User ID (UUID) |
| `email` | string | User email |
| `iat` | number | Issued at (Unix timestamp) |
| `exp` | number | Expiration (Unix timestamp) |
| `aud` | string | Audience (always "aerodb") |
| `iss` | string | Issuer (always "aerodb-auth") |
| `role` | string | User role for RLS |

### Custom Claims

| Claim | Type | Description |
|-------|------|-------------|
| `app_metadata` | object | Server-controlled data |
| `user_metadata` | object | User-controlled data |

---

## Token Invariants

- **AUTH-T1:** Access tokens are stateless (no DB lookup required)
- **AUTH-T2:** Secrets never appear in JWT payload
- **AUTH-T3:** JWT signature validated before claims
- **AUTH-T4:** Expired tokens rejected without exception
- **AUTH-T5:** `exp` claim is mandatory

---

## Token Rotation

### Access Token Rotation

```
Client → Refresh Request → Server
        ↓
        Validate refresh token (DB lookup)
        ↓
        Issue new access token (stateless)
        ↓
        Issue new refresh token (rotate)
        ↓
Client ← New tokens ← Server
```

### Rotation Rules

1. New access token issued on every refresh
2. New refresh token issued on every refresh (rotation)
3. Old refresh token invalidated immediately
4. Refresh reuse triggers session revocation

---

## Token Validation

### Validation Order

1. Parse JWT structure (reject malformed)
2. Validate signature (reject if invalid)
3. Check `exp` claim (reject if expired)
4. Check `iss` and `aud` (reject if mismatch)
5. Extract claims for RLS context

### Error Responses

| Condition | HTTP Status | Error |
|-----------|-------------|-------|
| Malformed token | 401 | `invalid_token` |
| Invalid signature | 401 | `invalid_token` |
| Expired token | 401 | `token_expired` |
| Wrong issuer/audience | 401 | `invalid_token` |

---

## Secret Management

### JWT Secret

- **Generation:** Cryptographically random (256-bit minimum)
- **Storage:** Environment variable or secrets manager
- **Rotation:** Supported via key versioning (future)

### Secret Invariants

- **AUTH-K1:** Secret never logged or exposed in errors
- **AUTH-K2:** Secret rotation does not invalidate existing tokens
- **AUTH-K3:** Minimum secret length: 32 bytes

---

## API Key Tokens

### Structure

```
aero_sk_live_xxxxxxxxxxxxxxxxxxxx
```

- Prefix: `aero_sk_live_` (live) or `aero_sk_test_` (test)
- Body: 32 random bytes, base64url encoded

### API Key Claims

API keys map to a special context:

```rust
RlsContext {
    user_id: None,
    is_service_role: true,  // Bypasses RLS
}
```
