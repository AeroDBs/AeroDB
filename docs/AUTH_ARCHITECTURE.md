# AUTH_ARCHITECTURE.md — Authentication Architecture

## Status
- Phase: **8**
- Authority: **Normative**
- Depends on: AUTH_VISION.md
- Date: 2026-02-06

---

## 1. Overview

This document defines the **technical architecture** for AeroDB's authentication and authorization system.

---

## 2. Module Structure

```
src/auth/
├── mod.rs           # Module exports and public API
├── user.rs          # User model, storage, and CRUD
├── session.rs       # Session and refresh token management
├── jwt.rs           # JWT generation, validation, claims
├── crypto.rs        # Password hashing (Argon2id)
├── email.rs         # Email sending abstraction
├── api.rs           # HTTP API endpoints
├── rls.rs           # Row-Level Security enforcement
├── api_key.rs       # API key management
└── errors.rs        # Auth-specific error types
```

---

## 3. Data Models

### 3.1 User Model

Users are stored as AeroDB documents in the `_users` collection:

```rust
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub email_verified: bool,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}
```

**Constraints:**
- Email must be unique (enforced by unique index)
- Password hash uses Argon2id (never stored plaintext)
- ID is UUIDv4 (generated on creation)

### 3.2 Session Model

Sessions are stored in the `_sessions` collection:

```rust
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub refresh_token_hash: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
}
```

### 3.3 JWT Claims

Access tokens contain:

```rust
pub struct JwtClaims {
    pub sub: Uuid,           // User ID
    pub email: String,       // User email
    pub iat: i64,            // Issued at
    pub exp: i64,            // Expires at (15 min)
    pub aud: String,         // Audience (project ID)
    pub iss: String,         // Issuer (aerodb)
}
```

---

## 4. Security Architecture

### 4.1 Password Hashing

```rust
// src/auth/crypto.rs
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::SaltString;

pub fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
    let parsed = PasswordHash::new(hash)?;
    Ok(Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
}
```

### 4.2 JWT Signing

- Algorithm: HS256 (HMAC-SHA256)
- Secret: 256-bit random key (configured per deployment)
- Access token TTL: 15 minutes
- Refresh token TTL: 30 days

### 4.3 Refresh Token Storage

Refresh tokens are:
1. Generated as 256-bit random values
2. Hashed with SHA-256 before storage
3. Returned to client as base64-encoded raw value
4. Validated by hashing client-provided value and comparing

---

## 5. Row-Level Security (RLS)

### 5.1 RLS Context

Every authenticated request carries an RLS context:

```rust
pub struct RlsContext {
    pub user_id: Uuid,
    pub is_authenticated: bool,
    pub is_service_role: bool,  // API key with service role
}
```

### 5.2 RLS Enforcement Points

RLS is enforced at **query planning time**, not execution:

```rust
// src/auth/rls.rs
pub trait RlsEnforcer {
    /// Inject RLS filters into a read query
    fn enforce_read(&self, query: &Query, ctx: &RlsContext) -> Result<Query, RlsError>;
    
    /// Validate write operation against RLS policy
    fn enforce_write(&self, doc: &Document, ctx: &RlsContext) -> Result<(), RlsError>;
}
```

### 5.3 Default RLS Policy

Unless overridden, the default policy is:
- Users can only read/write documents where `owner_id == user_id`
- Service role keys bypass RLS (explicit opt-in)

---

## 6. API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/auth/signup` | Register new user |
| POST | `/auth/login` | Authenticate user |
| POST | `/auth/logout` | Invalidate session |
| POST | `/auth/refresh` | Refresh access token |
| POST | `/auth/forgot-password` | Request password reset |
| POST | `/auth/reset-password` | Reset password with token |
| GET | `/auth/user` | Get current user info |
| PUT | `/auth/user` | Update user profile |

---

## 7. Integration with Core

### 7.1 No Core Modifications

Phase 8 MUST NOT modify:
- WAL format or behavior
- MVCC mechanics
- Replication protocol
- Core query execution

### 7.2 Planner Extension

RLS extends the planner via composition:

```rust
// RLS wraps the existing planner
pub struct RlsPlanner<P: Planner> {
    inner: P,
    enforcer: Box<dyn RlsEnforcer>,
}

impl<P: Planner> Planner for RlsPlanner<P> {
    fn plan(&self, query: &Query, ctx: &RlsContext) -> Result<Plan, PlanError> {
        let filtered_query = self.enforcer.enforce_read(query, ctx)?;
        self.inner.plan(&filtered_query)
    }
}
```

---

## 8. Observability

All authentication events emit structured logs:

```rust
pub enum AuthEvent {
    SignupAttempt { email: String, success: bool },
    LoginAttempt { email: String, success: bool },
    TokenRefresh { user_id: Uuid },
    PasswordReset { user_id: Uuid },
    RlsViolation { user_id: Uuid, collection: String },
}
```

---

## 9. Configuration

```toml
[auth]
enabled = true
jwt_secret = "your-256-bit-secret"
jwt_access_ttl_seconds = 900       # 15 minutes
jwt_refresh_ttl_days = 30

[auth.password]
min_length = 8
require_uppercase = false
require_number = false

[auth.email]
smtp_host = "smtp.example.com"
smtp_port = 587
from_address = "noreply@example.com"
```

---

## 10. Constraints

Phase 8 MUST:
- Never log passwords (even hashed)
- Never store plaintext passwords
- Always use constant-time comparison for secrets
- Fail closed on any validation error

Phase 8 MUST NOT:
- Modify Phase 0-7 behavior
- Add WAL records for auth operations
- Introduce background cleanup threads

---

END OF DOCUMENT
