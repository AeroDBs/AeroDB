# Phase 8: Session Model

**Document Type:** Normative Specification  
**Phase:** 8 - Authentication & Authorization  
**Status:** Active

---

## Overview

This document specifies the session lifecycle, refresh token mechanism, and TTL management for AeroDB authentication.

---

## Session Lifecycle

### States

```
CREATED → ACTIVE → REFRESHED → EXPIRED/REVOKED
```

| State | Description |
|-------|-------------|
| CREATED | Session initialized, tokens issued |
| ACTIVE | Valid session, access token usable |
| REFRESHED | Access token renewed, refresh token rotated |
| EXPIRED | TTL exceeded, requires re-authentication |
| REVOKED | Explicitly invalidated (logout, security) |

---

## Session Storage

### Schema: `sessions` collection

```json
{
  "id": "uuid",
  "user_id": "uuid",
  "refresh_token_hash": "string",
  "created_at": "datetime",
  "expires_at": "datetime",
  "last_refreshed_at": "datetime",
  "revoked": "boolean",
  "revoked_at": "datetime | null",
  "user_agent": "string | null",
  "ip_address": "string | null"
}
```

### Storage Invariants

- **AUTH-S1:** Refresh token stored as hash only (never plaintext)
- **AUTH-S2:** Session ID is cryptographically random (128-bit)
- **AUTH-S3:** One refresh token per session (rotated on use)

---

## Token Lifetimes

| Token Type | Default TTL | Configurable | Max |
|------------|-------------|--------------|-----|
| Access Token (JWT) | 15 minutes | Yes | 1 hour |
| Refresh Token | 30 days | Yes | 90 days |
| Session | 30 days | Yes | 90 days |

---

## Refresh Token Rotation

### Single-Use Refresh Tokens

Each refresh token is **single-use**. On refresh:

1. Validate incoming refresh token
2. Invalidate the old refresh token
3. Issue new access token + new refresh token
4. Update session with new refresh token hash

### Rotation Invariants

- **AUTH-R1:** Reuse of refresh token = session revocation (potential theft)
- **AUTH-R2:** New refresh token issued atomically with new access token
- **AUTH-R3:** Old refresh token rejected immediately after rotation

---

## Expiration Handling

### Explicit TTL Check

AeroDB uses **explicit TTL checks** (no background cleanup):

```rust
fn is_session_valid(session: &Session) -> bool {
    !session.revoked && Utc::now() < session.expires_at
}
```

### Cleanup Strategy

- Expired sessions remain in DB until explicit cleanup
- Cleanup via scheduled job or admin action (not implicit)
- Cleanup is idempotent and safe to run anytime

---

## Concurrency

### Multi-Device Sessions

- Users may have multiple concurrent sessions
- Each device gets independent session + refresh token
- Revoking one session does not affect others

### Race Conditions

- Concurrent refresh attempts: first wins, others get 401
- Protected by atomic compare-and-swap on refresh token hash

---

## Security Considerations

1. **Token Binding:** Optionally bind tokens to IP/User-Agent
2. **Anomaly Detection:** Log unusual session patterns
3. **Forced Logout:** Admin can revoke all user sessions
4. **Session Limits:** Configurable max sessions per user
