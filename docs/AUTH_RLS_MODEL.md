# AUTH_RLS_MODEL.md — Row-Level Security Model

## Status
- Phase: **8**
- Authority: **Normative**
- Depends on: AUTH_ARCHITECTURE.md
- Date: 2026-02-06

---

## 1. Purpose

This document defines AeroDB's **Row-Level Security (RLS)** model, which provides fine-grained access control at the document level.

---

## 2. Core Concept

RLS automatically filters data based on the authenticated user's identity. Users only see and modify data they are authorized to access.

```
Query: SELECT * FROM posts
User: alice (id: 123)
RLS Policy: owner_id = auth.user_id

Effective Query: SELECT * FROM posts WHERE owner_id = '123'
```

---

## 3. RLS Context

Every request carries an RLS context:

```rust
pub struct RlsContext {
    /// The authenticated user's ID (None if anonymous)
    pub user_id: Option<Uuid>,
    
    /// Whether the request is authenticated
    pub is_authenticated: bool,
    
    /// Whether using service role (bypasses RLS)
    pub is_service_role: bool,
    
    /// Custom claims from JWT (for advanced policies)
    pub claims: HashMap<String, serde_json::Value>,
}
```

---

## 4. Policy Types

### 4.1 Ownership Policy (Default)

The default policy enforces document ownership:

```rust
pub struct OwnershipPolicy {
    /// Field name containing the owner ID
    pub owner_field: String,  // Default: "owner_id"
}
```

**Behavior:**
- Read: Filter by `{owner_field} = user_id`
- Insert: Automatically set `{owner_field} = user_id`
- Update: Only allow if `{owner_field} = user_id`
- Delete: Only allow if `{owner_field} = user_id`

### 4.2 Public Read Policy

Allows public reads, owner-only writes:

```rust
pub struct PublicReadPolicy {
    pub owner_field: String,
}
```

**Behavior:**
- Read: No filter (all rows visible)
- Write: Restricted to owner

### 4.3 Custom Predicate Policy

Advanced policies using custom predicates:

```rust
pub struct PredicatePolicy {
    pub read_predicate: Option<FilterExpr>,
    pub write_predicate: Option<FilterExpr>,
}
```

---

## 5. Policy Assignment

Policies are assigned per collection in the schema:

```json
{
  "collection": "posts",
  "schema": { ... },
  "rls": {
    "enabled": true,
    "policy": "ownership",
    "owner_field": "author_id"
  }
}
```

---

## 6. Enforcement Points

### 6.1 Query Planning (Primary)

RLS filters are injected at query planning time:

```rust
impl RlsEnforcer for OwnershipEnforcer {
    fn enforce_read(&self, query: &Query, ctx: &RlsContext) -> Result<Query, RlsError> {
        if ctx.is_service_role {
            return Ok(query.clone());  // Bypass for service role
        }
        
        let user_id = ctx.user_id.ok_or(RlsError::AuthenticationRequired)?;
        
        // Inject ownership filter
        let filter = Filter::eq(self.owner_field.clone(), Value::Uuid(user_id));
        Ok(query.with_additional_filter(filter))
    }
}
```

### 6.2 Write Validation

Write operations are validated before execution:

```rust
fn enforce_write(&self, doc: &Document, ctx: &RlsContext) -> Result<(), RlsError> {
    if ctx.is_service_role {
        return Ok(());
    }
    
    let user_id = ctx.user_id.ok_or(RlsError::AuthenticationRequired)?;
    let doc_owner = doc.get(&self.owner_field)
        .and_then(|v| v.as_uuid())
        .ok_or(RlsError::MissingOwnerField)?;
    
    if doc_owner != user_id {
        return Err(RlsError::Unauthorized);
    }
    
    Ok(())
}
```

---

## 7. Service Role Bypass

API keys with `service_role` can bypass RLS:

```rust
pub enum ApiKeyRole {
    Anon,          // No auth, RLS enforced
    User,          // User-level, RLS enforced  
    ServiceRole,   // RLS bypassed
}
```

> [!CAUTION]
> Service role keys should only be used server-side. Never expose in client applications.

---

## 8. Determinism Guarantee

RLS enforcement is **deterministic**:

- Same query + same user + same policy = same filtered query
- No randomness in filter generation
- No time-dependent policy evaluation

---

## 9. Observability

RLS decisions are observable:

```rust
pub enum RlsEvent {
    PolicyApplied { collection: String, user_id: Uuid, policy: String },
    AccessDenied { collection: String, user_id: Uuid, reason: String },
    ServiceRoleBypass { collection: String },
}
```

---

## 10. Error Handling

RLS failures are explicit and informative:

```rust
pub enum RlsError {
    /// User must be authenticated to access this resource
    AuthenticationRequired,
    
    /// User is not authorized to access this document
    Unauthorized,
    
    /// Collection does not have owner field
    MissingOwnerField,
    
    /// RLS policy configuration is invalid
    InvalidPolicy(String),
}
```

---

## 11. Invariants

| ID | Invariant |
|----|-----------|
| **RLS-1** | Anonymous requests MUST be rejected for RLS-enabled collections (unless public policy) |
| **RLS-2** | Users MUST NOT read documents outside their policy scope |
| **RLS-3** | Users MUST NOT write documents outside their policy scope |
| **RLS-4** | Service role MUST explicitly bypass RLS (no silent bypass) |
| **RLS-5** | RLS filter injection MUST be deterministic |

---

END OF DOCUMENT
