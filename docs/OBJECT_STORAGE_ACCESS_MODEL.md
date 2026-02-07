# Phase 11: File Storage Access Model

**Document Type:** Access Control Specification  
**Phase:** 11 - File Storage  
**Status:** Active

---

## Access Control Philosophy

File storage uses **bucket-level policies** + **RLS context integration** to control access. This provides a simpler model than per-file ACLs while maintaining security.

---

## Bucket Policies

### Policy Types

#### 1. Public
**Rule:** Anyone can **read**, only owner can **write/delete**

**Use Cases:**
- Public documentation
- Marketing assets
- Open-source project files
- Blog images

**Example:**
```rust
Bucket {
    name: "public_docs",
    policy: BucketPolicy::Public,
    ...
}
```

**Access Matrix:**

| Operation | Anonymous | Authenticated | Owner | Service Role |
|-----------|-----------|---------------|-------|--------------|
| Read | ✅ | ✅ | ✅ | ✅ |
| Write | ❌ | ❌ | ✅ | ✅ |
| Delete | ❌ | ❌ | ✅ | ✅ |

---

#### 2. Private
**Rule:** Only **owner** can read/write/delete

**Use Cases:**
- User-uploaded files
- Private documents
- Personal photos
- Sensitive data

**Example:**
```rust
Bucket {
    name: "user_uploads",
    policy: BucketPolicy::Private,
    ...
}
```

**Access Matrix:**

| Operation | Anonymous | Authenticated | Owner | Service Role |
|-----------|-----------|---------------|-------|--------------|
| Read | ❌ | ❌ | ✅ | ✅ |
| Write | ❌ | ❌ | ✅ | ✅ |
| Delete | ❌ | ❌ | ✅ | ✅ |

---

#### 3. Authenticated
**Rule:** Any **authenticated user** can read/write, owner can delete

**Use Cases:**
- Shared team folders
- Collaborative workspaces
- Internal company resources

**Example:**
```rust
Bucket {
    name: "team_shared",
    policy: BucketPolicy::Authenticated,
    ...
}
```

**Access Matrix:**

| Operation | Anonymous | Authenticated | Owner | Service Role |
|-----------|-----------|---------------|-------|--------------|
| Read | ❌ | ✅ | ✅ | ✅ |
| Write | ❌ | ✅ | ✅ | ✅ |
| Delete | ❌ | ❌ | ✅ | ✅ |

---

## RLS Context Integration

Every file operation requires an **RLS context** (from Phase 8 auth):

```rust
pub struct RlsContext {
    pub user_id: Option<Uuid>,
    pub is_authenticated: bool,
    pub can_bypass_rls: bool,  // Service role
}
```

### Permission Check Flow

```rust
pub fn check_read(&self, bucket: &Bucket, context: &RlsContext) -> Result<()> {
    // Service role bypasses all checks
    if context.can_bypass_rls {
        return Ok(());
    }
    
    match bucket.policy {
        BucketPolicy::Public => {
            // Anyone can read
            Ok(())
        }
        BucketPolicy::Authenticated => {
            if context.is_authenticated {
                Ok(())
            } else {
                Err(StorageError::Unauthorized)
            }
        }
        BucketPolicy::Private => {
            // Must be owner
            if let Some(owner_id) = &bucket.owner_id {
                if Some(owner_id) == context.user_id.as_ref() {
                    Ok(())
                } else {
                    Err(StorageError::Unauthorized)
                }
            } else {
                // System bucket, only service role
                Err(StorageError::Unauthorized)
            }
        }
    }
}
```

---

## Signed URLs

### Purpose
Provide **temporary, pre-authenticated access** without requiring a session.

### Use Cases
1. **Direct browser upload** (bypass API server)
2. **Share files with unauthenticated users**
3. **Embed images in emails** (public link)
4. **Third-party integrations** (temporary access)

### Generation

**Request:**
```http
POST /storage/v1/object/sign/avatars/user123.jpg?expires_in=3600
Authorization: Bearer <JWT>
```

**Response:**
```json
{
  "url": "https://api.aerodb.io/storage/v1/object/avatars/user123.jpg?token=eyJhbGc...&expires=1675123456",
  "expires_at": "2026-02-06T10:00:00Z"
}
```

### Signature Algorithm

```rust
pub fn generate_signed_url(
    bucket: &str,
    path: &str,
    expires_at: DateTime<Utc>,
    secret: &str,
) -> String {
    // Message: bucket + path + expiration timestamp
    let message = format!("{}/{}/{}", bucket, path, expires_at.timestamp());
    
    // HMAC-SHA256 signature
    let mut hasher = Sha256::new();
    hasher.update(secret);
    hasher.update(message.as_bytes());
    let signature = format!("{:x}", hasher.finalize());
    
    // URL with signature
    format!(
        "/{}/{}?token={}&expires={}",
        bucket, path, signature, expires_at.timestamp()
    )
}
```

### Verification

```rust
pub fn verify_signed_url(
    url: &str,
    secret: &str,
) -> Result<()> {
    let params = parse_query_params(url)?;
    let token = params.get("token")?;
    let expires = params.get("expires")?.parse::<i64>()?;
    
    // Check expiration
    let expires_at = DateTime::from_timestamp(expires, 0)?;
    if Utc::now() > expires_at {
        return Err(StorageError::UrlExpired);
    }
    
    // Verify signature
    let expected = generate_signature(bucket, path, expires_at, secret);
    
    // Constant-time comparison (prevent timing attacks)
    if constant_time_eq(token, &expected) {
        Ok(())
    } else {
        Err(StorageError::InvalidSignature)
    }
}
```

### Security Properties

| Property | Implementation |
|----------|----------------|
| Time-limited | Expires at timestamp checked on every access |
| Tamper-proof | HMAC signature prevents modification |
| Single-path | Signature tied to specific bucket + path |
| Revocable | Secret rotation invalidates all URLs (future) |
| No replay (future) | Nonce or single-use tokens |

---

## Service Role Bypass

**Service role** is a special administrative role that bypasses all RLS checks.

### When to Use
- Background jobs (orphan cleanup, migrations)
- Admin operations (bulk delete, audit)
- System-level access (backups, replication)

### How to Bypass
```rust
let service_context = RlsContext {
    user_id: None,
    is_authenticated: true,
    can_bypass_rls: true,  // ⬅️ Service role flag
};

storage.upload("private_bucket", "file.txt", data, &service_context)?;
// Succeeds even if service_context.user_id != bucket.owner_id
```

### Security
- JWT must have `role=service` claim
- Only issued to trusted backend services
- Never exposed to clients
- Logged explicitly for audit

---

## Access Control Examples

### Example 1: Public Read, Owner Write

**Scenario:** Blog images

```rust
Bucket {
    name: "blog_images",
    policy: BucketPolicy::Public,
    owner_id: Some(admin_user_id),
}
```

**Behavior:**
- Anonymous users: Can download
- Authenticated users: Can download
- Admin user: Can upload, download, delete
- Other users: Can download only

---

### Example 2: Authenticated Collaboration

**Scenario:** Team document folder

```rust
Bucket {
    name: "team_docs",
    policy: BucketPolicy::Authenticated,
    owner_id: Some(team_lead_id),
}
```

**Behavior:**
- Anonymous users: Blocked
- Any authenticated user: Can upload, download
- Team lead: Can upload, download, delete
- Other team members: Can upload, download (not delete)

---

### Example 3: Private User Uploads

**Scenario:** User avatars

```rust
Bucket {
    name: "user_avatars",
    policy: BucketPolicy::Private,
    owner_id: None,  // Per-file ownership
}
```

**Per-File Ownership:**
```rust
Object {
    id: "...",
    bucket_id: "user_avatars",
    path: "user123.jpg",
    owner_id: Some(user123_id),  // ⬅️ File owner
}
```

**Behavior:**
- User 123: Can upload, download, delete `user123.jpg`
- User 456: Blocked from `user123.jpg`
- Service role: Full access to all

---

### Example 4: Temporary Guest Access

**Scenario:** Share photo with friend (no account)

```rust
// 1. Owner generates signed URL (valid 24 hours)
let url = storage.generate_signed_url(
    "user_avatars",
    "user123.jpg",
    Utc::now() + Duration::hours(24),
)?;

// 2. Send URL to friend
send_email(friend_email, url);

// 3. Friend accesses without login
GET https://api.aerodb.io/storage/v1/object/user_avatars/user123.jpg?token=...&expires=...
→ 200 OK (signature valid, not expired)
```

---

## Permission Denial Responses

### 403 Forbidden (RLS Denial)
```http
HTTP/1.1 403 Forbidden
Content-Type: application/json

{
  "error": "403 Forbidden",
  "message": "Access denied: bucket policy does not allow this operation",
  "code": "STORAGE_UNAUTHORIZED"
}
```

### 401 Unauthorized (No Auth)
```http
HTTP/1.1 401 Unauthorized
WWW-Authenticate: Bearer

{
  "error": "401 Unauthorized",
  "message": "Authentication required",
  "code": "AUTH_REQUIRED"
}
```

### 410 Gone (Expired Signed URL)
```http
HTTP/1.1 410 Gone
Content-Type: application/json

{
  "error": "410 Gone",
  "message": "Signed URL expired at 2026-02-06T09:00:00Z",
  "code": "URL_EXPIRED"
}
```

---

## Future Enhancements

### Per-File ACLs
Allow fine-grained access control per file (not just bucket-level):

```rust
Object {
    acl: vec![
        AccessRule { user_id: "user456", permission: "read" },
        AccessRule { group_id: "team_a", permission: "write" },
    ],
}
```

### Signed URL Single-Use
Prevent replay attacks with nonce:

```rust
SignedUrl {
    token: "...",
    nonce: "one-time-uuid",
    expires_at: "...",
}
```

On first use, mark nonce as consumed.

### Secret Rotation
Rotate HMAC secret to invalidate old signed URLs:

```rust
Config {
    current_secret: "new-secret",
    previous_secret: Some("old-secret"),  // Grace period
}
```

Verify with both secrets, generate with current only.
