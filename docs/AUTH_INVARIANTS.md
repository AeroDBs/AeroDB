# AUTH_INVARIANTS.md — Authentication Invariants

## Status
- Phase: **8**
- Authority: **Normative**
- Depends on: AUTH_VISION.md, AUTH_ARCHITECTURE.md
- Date: 2026-02-06

---

## 1. Purpose

This document defines **inviolable invariants** for Phase 8 authentication and authorization. These invariants MUST be enforced by implementation and verified by tests.

---

## 2. Security Invariants

### AUTH-S1: Passwords Never Logged

> **Plaintext passwords MUST never appear in logs, traces, or error messages.**

```rust
// FORBIDDEN
log::error!("Login failed for password: {}", password);

// ALLOWED
log::error!("Login failed for user: {}", email);
```

### AUTH-S2: Passwords Never Stored Plaintext

> **Passwords MUST only be stored as cryptographic hashes using Argon2id.**

### AUTH-S3: Constant-Time Comparison

> **All secret comparisons MUST use constant-time algorithms to prevent timing attacks.**

```rust
use subtle::ConstantTimeEq;

fn verify_token(provided: &[u8], stored: &[u8]) -> bool {
    provided.ct_eq(stored).into()
}
```

### AUTH-S4: Fail Closed

> **Any authentication validation failure MUST result in denial, never silent acceptance.**

```rust
// FORBIDDEN
if let Some(user) = maybe_user {
    // proceed
}
// Falls through silently if user is None

// REQUIRED
let user = maybe_user.ok_or(AuthError::InvalidCredentials)?;
```

---

## 3. Session Invariants

### AUTH-SS1: Refresh Token Rotation

> **Each refresh token MUST be single-use. Using a refresh token invalidates it.**

### AUTH-SS2: Session Expiration

> **Sessions MUST expire at their stated expiration time. No implicit extension.**

### AUTH-SS3: Logout Invalidation

> **Logout MUST immediately invalidate the session. Subsequent requests MUST fail.**

---

## 4. JWT Invariants

### AUTH-JWT1: Stateless Validation

> **Access tokens MUST be validatable without database lookup.**

Validation requires only:
- The token itself
- The signing secret
- Current time (for expiration)

### AUTH-JWT2: Short Expiration

> **Access tokens MUST expire within 15 minutes of issuance.**

### AUTH-JWT3: No Secret in Token

> **Tokens MUST NOT contain passwords, secrets, or sensitive data.**

---

## 5. RLS Invariants

### AUTH-RLS1: No Silent Bypass

> **RLS MUST NOT be silently bypassed. Bypass requires explicit service role.**

### AUTH-RLS2: Query Injection Determinism

> **Same query + same user + same policy MUST produce identical filtered query.**

### AUTH-RLS3: Write Validation

> **All writes MUST be validated against RLS policy before execution.**

### AUTH-RLS4: Filter Before Execution

> **RLS filters MUST be applied at planning time, before any data is accessed.**

---

## 6. Observability Invariants

### AUTH-O1: All Events Logged

> **All authentication attempts (success and failure) MUST be logged to audit trail.**

### AUTH-O2: No Sensitive Data in Logs

> **Logs MUST NOT contain passwords, tokens, or secrets.**

Allowed: user ID, email, timestamp, success/failure
Forbidden: password, JWT, refresh token, API key

---

## 7. Integration Invariants

### AUTH-I1: No Core Modification

> **Phase 8 MUST NOT modify any Phase 0-7 behavior.**

### AUTH-I2: No WAL Records

> **Authentication does NOT write to WAL for auth operations.**

Auth state is stored in regular collections (`_users`, `_sessions`), not special WAL records.

### AUTH-I3: Planner Extension Only

> **RLS extends the planner via composition, not modification.**

The core planner is wrapped, not changed.

---

## 8. Enforcement Matrix

| Invariant | Enforcement Location | Test File |
|-----------|---------------------|-----------|
| AUTH-S1 | All logging calls | `auth_security_tests.rs` |
| AUTH-S2 | `crypto.rs` | `crypto_tests.rs` |
| AUTH-S3 | `crypto.rs`, `jwt.rs` | `timing_tests.rs` |
| AUTH-S4 | All auth handlers | `auth_denial_tests.rs` |
| AUTH-SS1 | `session.rs` | `session_tests.rs` |
| AUTH-SS2 | `session.rs` | `expiration_tests.rs` |
| AUTH-SS3 | `session.rs`, `api.rs` | `logout_tests.rs` |
| AUTH-JWT1 | `jwt.rs` | `jwt_tests.rs` |
| AUTH-JWT2 | `jwt.rs` | `jwt_tests.rs` |
| AUTH-JWT3 | `jwt.rs` | `jwt_tests.rs` |
| AUTH-RLS1 | `rls.rs` | `rls_bypass_tests.rs` |
| AUTH-RLS2 | `rls.rs` | `rls_determinism_tests.rs` |
| AUTH-RLS3 | `rls.rs` | `rls_write_tests.rs` |
| AUTH-RLS4 | `rls.rs` | `rls_planning_tests.rs` |

---

## 9. Violation Response

If any invariant is violated:

1. The operation MUST fail with an explicit error
2. The violation MUST be logged (without exposing secrets)
3. The invariant ID MUST be referenced in the error
4. No partial or "best-effort" execution is allowed

---

END OF DOCUMENT
