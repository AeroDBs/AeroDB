# AUTH_VISION.md — Authentication & Authorization Vision

## Status
- Phase: **8**
- Authority: **Normative**
- Depends on: Phases 0–7 (frozen)
- Date: 2026-02-06

---

## 1. Purpose

This document defines the **vision, philosophy, and goals** for AeroDB's authentication and authorization layer. Phase 8 transforms AeroDB from a database engine into an application backend by adding user authentication and data access control.

---

## 2. Core Philosophy

### 2.1 Explicit Over Magic

Unlike platforms that auto-provision users or auto-discover OAuth providers, AeroDB authentication is **explicit**:

- User creation requires explicit API calls
- OAuth providers require explicit configuration
- No background user cleanup or auto-merge

### 2.2 Fail-Closed Security

Authentication and authorization follow a **fail-closed** model:

- Invalid credentials → immediate rejection (no timing leaks)
- Missing auth context → request rejected
- RLS violation → query fails (never bypasses silently)

### 2.3 Observable & Auditable

All authentication events are logged to the audit trail:

- Signup attempts (success/failure)
- Login attempts (success/failure)
- Token refresh operations
- Password changes
- RLS enforcement decisions

### 2.4 Deterministic Behavior

Given identical inputs, authentication produces identical outputs:

- Same credentials + same state → same decision
- Token validation is stateless (verifiable from token alone)
- RLS filters produce deterministic query modifications

---

## 3. Goals

### 3.1 User Authentication
Enable applications to authenticate end users via:
- Email + password (primary method)
- OAuth 2.0 providers (Google, GitHub, GitLab)
- Magic links (passwordless email)

### 3.2 Session Management
Provide secure, stateless sessions via:
- Short-lived JWT access tokens (15 minutes)
- Long-lived refresh tokens (30 days, stored in DB)
- Explicit token rotation on refresh

### 3.3 Row-Level Security (RLS)
Enable fine-grained data access control via:
- User context injection at query time
- Ownership-based access patterns
- Custom RLS policies per collection

### 3.4 API Keys
Enable machine-to-machine authentication via:
- Project-level API keys with configurable permissions
- Per-key rate limiting and audit logging

---

## 4. Non-Goals (Deferred to Later Phases)

- Multi-factor authentication (MFA)
- SAML/LDAP enterprise SSO
- Advanced permission roles (RBAC beyond owner/admin)
- User impersonation

---

## 5. Philosophy Alignment

| AeroDB Principle | Phase 8 Alignment |
|-----------------|-------------------|
| **Correctness** | Fail-closed authentication; passwords never logged |
| **Determinism** | Same credentials → same result; stateless JWT |
| **Explicitness** | No auto-provisioning; explicit user creation |
| **Observability** | All auth events in audit log |
| **No Magic** | No hidden provider discovery or auto-OAuth |

---

## 6. Success Criteria

Phase 8 is successful when:

1. Users can sign up, log in, and manage sessions
2. JWTs are verifiable without database lookup
3. RLS enforces data isolation between users
4. All auth operations are auditable
5. No Phase 0-7 behavior is modified

---

END OF DOCUMENT
