# Phase 11: Bucket Model

**Document Type:** Data Model Specification  
**Phase:** 11 - File Storage  
**Status:** Active

---

## Overview

**Buckets** are top-level containers for files. They define:
- **Access policy** (public/private/authenticated)
- **MIME type restrictions** (e.g., only images)
- **Size limits** (e.g., max 5MB per file)
- **Ownership** (who can manage the bucket)

---

## Bucket Structure

```rust
pub struct Bucket {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Option<Uuid>,
    pub policy: BucketPolicy,
    pub config: BucketConfig,
    pub created_at: DateTime<UTC>,
    pub updated_at: DateTime<Utc>,
}

pub enum BucketPolicy {
    Public,        // Anyone read,owner write
    Private,       // Owner only
    Authenticated, // Any authed user
}

pub struct BucketConfig {
    pub allowed_mime_types: Vec<String>,  // Empty = allow all
    pub max_file_size: u64,                // Bytes, 0 = unlimited
}
```

---

## Field Specifications

### id
- **Type:** UUID v4
- **Purpose:** Primary key, immutable
- **Generation:** Server-side on creation

### name
- **Type:** String
- **Constraints:**
  - Unique across all buckets
  - 3-63 characters
  - Lowercase alphanumeric + hyphens
  - Cannot start/end with hyphen
- **Regex:** `^[a-z0-9][a-z0-9-]*[a-z0-9]$`
  
**Valid Names:**
```
✅ avatars
✅ user-uploads
✅ public-docs-2026
❌ Avatars  (uppercase)
❌ av      (too short)
❌ -avatars (starts with hyphen)
❌ user_uploads (underscore)
```

### owner_id
- **Type:** UUID (nullable)
- **Purpose:** Who can manage this bucket
- **Null Semantics:**
  - `NULL` = system bucket (no single owner)
  - `Some(user_id)` = user-owned bucket

### policy
- **Type:** Enum (Public, Private, Authenticated)
- **Default:** Private (secure by default)
- **Immutable:** No (can be changed, but dangerous)

### allowed_mime_types
- **Type:** String array
- **Default:** `[]` (empty = allow all)
- **Format:** MIME type patterns
  - Exact: `image/jpeg`
  - Wildcard: `image/*`
  - Multiple: `["image/jpeg", "image/png", "image/webp"]`

**Examples:**
```rust
// Only JPEG/PNG images
allowed_mime_types: vec!["image/jpeg", "image/png"]

// Any image
allowed_mime_types: vec!["image/*"]

// PDFs and Word docs
allowed_mime_types: vec!["application/pdf", "application/vnd.openxmlformats-officedocument.wordprocessingml.document"]

// Allow all
allowed_mime_types: vec![]
```

**Enforcement:**
Upload request with `Content-Type: video/mp4` to bucket allowing only `image/*`:
```
→ 415 Unsupported Media Type
```

### max_file_size
- **Type:** u64 (bytes)
- **Default:** 0 (unlimited, but global limit still applies)
- **Purpose:** Prevent accidentally large uploads

**Examples:**
```rust
max_file_size: 5_242_880       // 5 MB
max_file_size: 104_857_600     // 100 MB
max_file_size: 0               // Unlimited (use global limit)
```

**Enforcement:**
Upload 10MB file to bucket with 5MB limit:
```
→ 413 Payload Too Large
```

---

## Bucket Operations

### Create Bucket

**Request:**
```http
POST /storage/v1/bucket
Authorization: Bearer <JWT>
Content-Type: application/json

{
  "name": "avatars",
  "policy": "private",
  "allowed_mime_types": ["image/jpeg", "image/png"],
  "max_file_size": 5242880
}
```

**Response:**
```http
HTTP/1.1 201 Created
Content-Type: application/json

{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "avatars",
  "owner_id": "789e0123-e45b-67d8-9012-345678901234",
  "policy": "private",
  "config": {
    "allowed_mime_types": ["image/jpeg", "image/png"],
    "max_file_size": 5242880
  },
  "created_at": "2026-02-06T09:00:00Z",
  "updated_at": "2026-02-06T09:00:00Z"
}
```

**Errors:**
- `409 Conflict` - Bucket name already exists
- `400 Bad Request` - Invalid name format
- `401 Unauthorized` - No authentication

---

### List Buckets

**Request:**
```http
GET /storage/v1/bucket
Authorization: Bearer <JWT>
```

**Response:**
```http
HTTP/1.1 200 OK
Content-Type: application/json

[
  {
    "id": "550e8400-...",
    "name": "avatars",
    "policy": "private",
    "file_count": 42,
    "total_size": 1234567
  },
  {
    "id": "660f9500-...",
    "name": "public-docs",
    "policy": "public",
    "file_count": 128,
    "total_size": 9876543
  }
]
```

**Filtering:**
- Owner's buckets: Filter by `owner_id = current_user_id()`
- Public buckets: Filter by `policy = 'public'`
- Service role: See all buckets

---

### Get Bucket

**Request:**
```http
GET /storage/v1/bucket/avatars
Authorization: Bearer <JWT>
```

**Response:**
```http
HTTP/1.1 200 OK
Content-Type: application/json

{
  "id": "550e8400-...",
  "name": "avatars",
  "owner_id": "789e0123-...",
  "policy": "private",
  "config": {
    "allowed_mime_types": ["image/jpeg", "image/png"],
    "max_file_size": 5242880
  },
  "file_count": 42,
  "total_size": 1234567,
  "created_at": "2026-02-06T09:00:00Z"
}
```

---

### Update Bucket

**Request:**
```http
PATCH /storage/v1/bucket/avatars
Authorization: Bearer <JWT>
Content-Type: application/json

{
  "max_file_size": 10485760  // Increase to 10MB
}
```

**Response:**
```http
HTTP/1.1 200 OK
Content-Type: application/json

{
  "id": "550e8400-...",
  "config": {
    "max_file_size": 10485760
  },
  "updated_at": "2026-02-06T09:15:00Z"
}
```

**Authorization:**
- Must be bucket owner
- Service role can update any bucket

---

### Delete Bucket

**Request:**
```http
DELETE /storage/v1/bucket/avatars
Authorization: Bearer <JWT>
```

**Success (Empty Bucket):**
```http
HTTP/1.1 204 No Content
```

**Error (Non-Empty Bucket):**
```http
HTTP/1.1 409 Conflict
Content-Type: application/json

{
  "error": "409 Conflict",
  "message": "Bucket not empty (42 files)",
  "code": "BUCKET_NOT_EMPTY"
}
```

**Force Delete (Future):**
```http
DELETE /storage/v1/bucket/avatars?force=true
→ Deletes bucket AND all files
```

---

## Validation Rules

### Name Validation
```rust
pub fn validate_bucket_name(name: &str) -> Result<()> {
    if name.len() < 3 || name.len() > 63 {
        return Err(StorageError::InvalidBucketName("Length must be 3-63"));
    }
    
    let regex = Regex::new(r"^[a-z0-9][a-z0-9-]*[a-z0-9]$").unwrap();
    if !regex.is_match(name) {
        return Err(StorageError::InvalidBucketName("Invalid characters"));
    }
    
    Ok(())
}
```

### MIME Validation
```rust
pub fn is_mime_allowed(&self, content_type: &str) -> bool {
    if self.config.allowed_mime_types.is_empty() {
        return true;  // Allow all
    }
    
    for pattern in &self.config.allowed_mime_types {
        if pattern.ends_with("/*") {
            // Wildcard: image/* matches image/jpeg
            let prefix = &pattern[..pattern.len() - 2];
            if content_type.starts_with(prefix) {
                return true;
            }
        } else {
            // Exact match
            if content_type == pattern {
                return true;
            }
        }
    }
    
    false
}
```

### Size Validation
```rust
pub fn is_size_allowed(&self, size: u64) -> bool {
    if self.config.max_file_size == 0 {
        return true;  // Unlimited (global limit still applies)
    }
    
    size <= self.config.max_file_size
}
```

---

## Bucket Lifecycle

### Creation Flow
```
1. Client POST /storage/v1/bucket
2. Server validates name (unique, format)
3. Server creates bucket with owner_id = current_user
4. Server inserts into storage_buckets table
5. Server returns bucket details
```

### Deletion Flow
```
1. Client DELETE /storage/v1/bucket/avatars
2. Server checks if empty (file_count = 0)
3. If not empty → 409 Conflict
4. If empty → Delete from storage_buckets
5. Cascade delete metadata (foreign key)
```

---

## Default Buckets

Some buckets may be created by default:

```rust
// System bucket for internal use
Bucket {
    name: "system",
    owner_id: None,
    policy: BucketPolicy::Private,
    config: BucketConfig {
        allowed_mime_types: vec![],
        max_file_size: 0,
    },
}
```

---

## Invariants

### BKT-I1: Name Uniqueness
> **No two buckets have the same name**

**Enforcement:** UNIQUE constraint on `name` field

### BKT-I2: Policy Consistency
> **Bucket policy matches file access behavior**

**Verification:** Permission tests for each policy type

### BKT-I3: Config Validation
> **MIME and size limits enforced on upload**

**Enforcement:** Pre-upload validation checks

---

## Examples

### Image Storage Bucket
```json
{
  "name": "user-avatars",
  "policy": "private",
  "allowed_mime_types": ["image/jpeg", "image/png", "image/webp"],
  "max_file_size": 5242880
}
```

### Document Archive (Public)
```json
{
  "name": "public-docs",
  "policy": "public",
  "allowed_mime_types": ["application/pdf"],
  "max_file_size": 10485760
}
```

### Team Workspace
```json
{
  "name": "team-shared",
  "policy": "authenticated",
  "allowed_mime_types": [],
  "max_file_size": 104857600
}
```
