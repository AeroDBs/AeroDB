# Phase 8: Testing Strategy

**Document Type:** Normative Specification  
**Phase:** 8 - Authentication & Authorization  
**Status:** Active

---

## Overview

This document specifies the testing strategy for Phase 8 authentication components.

---

## Test Categories

### Unit Tests

| Component | Test File | Coverage Target |
|-----------|-----------|-----------------|
| crypto.rs | crypto_tests | 100% |
| jwt.rs | jwt_tests | 100% |
| session.rs | session_tests | 95% |
| user.rs | user_tests | 95% |
| rls.rs | rls_tests | 100% |
| api.rs | api_tests | 90% |

### Integration Tests

| Scenario | Location | Dependencies |
|----------|----------|--------------|
| Full auth flow | tests/auth_integration.rs | In-memory DB |
| RLS enforcement | tests/rls_integration.rs | In-memory DB |
| Token lifecycle | tests/token_integration.rs | None |

---

## Critical Test Scenarios

### Authentication

1. **Signup Flow**
   - Valid registration succeeds
   - Duplicate email rejected
   - Weak password rejected
   - Email format validated

2. **Login Flow**
   - Valid credentials return tokens
   - Invalid password returns 401
   - Non-existent user returns 401 (same error)
   - Timing attack protection

3. **Token Refresh**
   - Valid refresh returns new tokens
   - Expired refresh rejected
   - Reused refresh revokes session

4. **Logout**
   - Session invalidated
   - Refresh token invalidated
   - Access token still valid until expiry (stateless)

### Authorization (RLS)

1. **Read Filtering**
   - Users see only owned records
   - Service role sees all records
   - Anonymous denied (fail-closed)

2. **Write Validation**
   - Users can create owned records
   - Users cannot modify others' records
   - Owner field auto-set on insert

3. **Policy Types**
   - Ownership policy enforced
   - Public read policy allows anonymous reads
   - Custom policies rejected (not implemented)

---

## Security Tests

### Invariant Tests

| Invariant | Test |
|-----------|------|
| AUTH-1 | Passwords never logged |
| AUTH-2 | Argon2id for password hashing |
| AUTH-3 | Constant-time password comparison |
| AUTH-T1 | JWT stateless validation |
| AUTH-T4 | Expired tokens rejected |
| AUTH-R1 | Refresh reuse = revocation |

### Penetration Tests

1. **Timing attacks** - Response time constant
2. **SQL injection** - Parameterized queries
3. **Token forgery** - Signature validation
4. **Session fixation** - New session on login

---

## Test Data

### Test Users

```rust
const TEST_EMAIL: &str = "test@example.com";
const TEST_PASSWORD: &str = "ValidP@ssw0rd123";
const WEAK_PASSWORD: &str = "123";
```

### Test Tokens

```rust
// Generated at test time, not hardcoded
let (access, refresh) = auth.login(email, password)?;
```

---

## Coverage Requirements

| Component | Minimum | Target |
|-----------|---------|--------|
| crypto.rs | 100% | 100% |
| jwt.rs | 95% | 100% |
| session.rs | 90% | 95% |
| user.rs | 90% | 95% |
| rls.rs | 95% | 100% |
| api.rs | 85% | 90% |
| **Overall** | **90%** | **95%** |

---

## CI/CD Integration

### Pre-Merge

```bash
cargo test auth:: --lib
cargo test --test auth_integration
```

### Post-Merge

```bash
cargo test --all
cargo audit
```

### Security Scan

Weekly automated security scan for:
- Dependency vulnerabilities
- Credential leaks
- Timing attack patterns
