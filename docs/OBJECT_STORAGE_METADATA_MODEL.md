# Phase 11: File Storage Metadata Model

**Document Type:** Data Model Specification  
**Phase:** 11 - File Storage  
**Status:** Active

---

## Overview

File **metadata** is stored in AeroDB collections, while **file contents** live on the storage backend (local FS or S3). This separation enables:
- Queryable metadata via SQL/REST API
- MVCC visibility for metadata
- Transactional consistency
- Replication of metadata
- RLS enforcement on metadata queries

---

## Collections

### storage_buckets

Stores bucket definitions and policies.

```sql
CREATE COLLECTION storage_buckets (
    id           TEXT PRIMARY KEY,      -- Bucket ID (UUID)
    name         TEXT UNIQUE NOT NULL,  -- Bucket name (user-facing)
    owner_id     TEXT,                  -- Owner user ID (NULL = system)
    policy       TEXT NOT NULL,         -- "public" | "private" | "authenticated"
    allowed_mime_types TEXT[],          -- Empty array = allow all
    max_file_size      BIGINT NOT NULL, -- Bytes, 0 = unlimited
    created_at   TIMESTAMP NOT NULL,
    updated_at   TIMESTAMP NOT NULL
);

CREATE INDEX idx_buckets_name ON storage_buckets(name);
CREATE INDEX idx_buckets_owner ON storage_buckets(owner_id);
```

**Example Row:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "avatars",
  "owner_id": null,
  "policy": "private",
  "allowed_mime_types": ["image/jpeg", "image/png", "image/webp"],
  "max_file_size": 5242880,
  "created_at": "2026-02-06T09:00:00Z",
  "updated_at": "2026-02-06T09:00:00Z"
}
```

---

### storage_objects

Stores file metadata (NOT the file contents).

```sql
CREATE COLLECTION storage_objects (
    id            TEXT PRIMARY KEY,       -- Object ID (UUID)
    bucket_id     TEXT NOT NULL           -- Foreign key to storage_buckets.id
                  REFERENCES storage_buckets(id) ON DELETE CASCADE,
    path          TEXT NOT NULL,          -- Path within bucket (e.g., "users/123/avatar.jpg")
    size          BIGINT NOT NULL,        -- File size in bytes
    content_type  TEXT NOT NULL,          -- MIME type (e.g., "image/jpeg")
    checksum      TEXT NOT NULL,          -- SHA-256 hash of contents
    owner_id      TEXT,                   -- Owner user ID (for RLS)
    metadata      JSONB DEFAULT '{}',     -- Custom metadata (extensible)
    created_at    TIMESTAMP NOT NULL,
    updated_at    TIMESTAMP NOT NULL,
    
    CONSTRAINT uniq_bucket_path UNIQUE (bucket_id, path)
);

CREATE INDEX idx_objects_bucket ON storage_objects(bucket_id);
CREATE INDEX idx_objects_owner ON storage_objects(owner_id);
CREATE INDEX idx_objects_path ON storage_objects(bucket_id, path);
CREATE INDEX idx_objects_created ON storage_objects(created_at);
```

**Example Row:**
```json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "bucket_id": "550e8400-e29b-41d4-a716-446655440000",
  "path": "users/456/avatar.jpg",
  "size": 12345,
  "content_type": "image/jpeg",
  "checksum": "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a",
  "owner_id": "789e0123-e45b-67d8-9012-345678901234",
  "metadata": {
    "width": 512,
    "height": 512,
    "uploaded_from": "ios_app"
  },
  "created_at": "2026-02-06T09:00:00Z",
  "updated_at": "2026-02-06T09:00:00Z"
}
```

---

## Metadata Fields

### id (Object ID)
- **Type:** UUID
- **Purpose:** Primary key, unique across all objects
- **Generation:** Server-side on upload

### bucket_id
- **Type:** UUID (foreign key)
- **Purpose:** Links object to bucket
- **Constraint:** Must exist in `storage_buckets`
- **Cascade:** Delete bucket → Delete all objects

### path
- **Type:** String
- **Purpose:** File path within bucket
- **Format:** `folder/subfolder/filename.ext`
- **Uniqueness:** (bucket_id, path) is unique
- **Validation:**
  - No leading/trailing slashes
  - No `..` (path traversal)
  - Max length: 1024 characters

**Example Paths:**
```
✅ users/123/avatar.jpg
✅ documents/2026/Q1/report.pdf
✅ images/products/laptop-hero.webp
❌ /absolute/path.jpg  (leading slash)
❌ ../etc/passwd       (path traversal)
❌ ../../evil.txt      (path traversal)
```

### size
- **Type:** BIGINT (64-bit integer)
- **Purpose:** File size in bytes
- **Range:** 0 to 2^63 - 1
- **Usage:**
  - Display to user ("12.3 KB")
  - Quota enforcement (future)
  - Storage billing (future)

### content_type
- **Type:** String (MIME type)
- **Purpose:** File type
- **Examples:**
  - `image/jpeg`
  - `application/pdf`
  - `video/mp4`
- **Validation:** Must match bucket's `allowed_mime_types`

### checksum
- **Type:** String (hex-encoded SHA-256)
- **Purpose:** Verify file integrity
- **Length:** 64 characters
- **Calculation:**
  ```rust
  use sha2::{Sha256, Digest};
  let mut hasher = Sha256::new();
  hasher.update(&file_contents);
  let checksum = format!("{:x}", hasher.finalize());
  ```
- **Usage:**
  - Detect corruption
  - Deduplication (future)
  - Content-addressable storage (future)

### owner_id
- **Type:** UUID (user ID)
- **Purpose:** RLS enforcement (who uploaded the file)
- **Nullable:** Yes (system-uploaded files have NULL owner)
- **RLS Query:**
  ```sql
  SELECT * FROM storage_objects 
  WHERE owner_id = current_user_id();
  ```

### metadata (JSONB)
- **Type:** JSON object
- **Purpose:** Extensible custom metadata
- **Schema:** Unstructured (application-defined)
- **Examples:**
  ```json
  {
    "width": 1920,
    "height": 1080,
    "camera": "iPhone 13",
    "location": "SF",
    "tags": ["vacation", "beach"]
  }
  ```
- **Queryable:**
  ```sql
  SELECT * FROM storage_objects 
  WHERE metadata->>'camera' = 'iPhone 13';
  ```

### created_at / updated_at
- **Type:** TIMESTAMP (UTC)
- **Purpose:** Audit trail
- **Auto-managed:** Server sets on insert/update

---

## Filesystem Mapping

Metadata path → Filesystem path:

```
Metadata: bucket_id="550e84...", path="users/123/avatar.jpg"
Filesystem: <storage_root>/550e8400-e29b-41d4-a716-446655440000/users/123/avatar.jpg
```

**Separation Benefits:**
- Bucket rename doesn't move files
- Path collision handled at DB level
- Filesystem layout is implementation detail

---

## Invariants (Metadata-Specific)

### MET-I1: Referential Integrity
> **Every object references a valid bucket**

**Enforcement:**
```sql
FOREIGN KEY (bucket_id) REFERENCES storage_buckets(id) ON DELETE CASCADE
```

**Implication:** Deleting bucket deletes all objects

---

### MET-I2: Path Uniqueness
> **No two objects in same bucket have same path**

**Enforcement:**
```sql
CONSTRAINT uniq_bucket_path UNIQUE (bucket_id, path)
```

**Implication:** Upload to existing path fails with 409 Conflict

---

### MET-I3: Checksum Match
> **Checksum in metadata matches file on disk**

**Verification:**
```rust
async fn verify_checksum(obj: &StorageObject) -> Result<()> {
    let file_data = read_from_disk(&obj.path)?;
    let actual = StorageObject::calculate_checksum(&file_data);
    
    if actual != obj.checksum {
        return Err(StorageError::ChecksumMismatch);
    }
    
    Ok(())
}
```

**When to verify:**
- On download (optional, performance tradeoff)
- Health check (periodic)
- Migration (S3 upload)

---

## Querying Metadata

### List Files in Bucket
```sql
SELECT id, path, size, content_type, created_at 
FROM storage_objects 
WHERE bucket_id = ?
ORDER BY created_at DESC
LIMIT 100;
```

### Find Large Files
```sql
SELECT bucket_id, path, size 
FROM storage_objects 
WHERE size > 10485760  -- 10MB
ORDER BY size DESC;
```

### Search by MIME Type
```sql
SELECT * FROM storage_objects 
WHERE content_type LIKE 'image/%';
```

### User's Files (RLS Applied)
```sql
SELECT * FROM storage_objects 
WHERE owner_id = current_user_id();
```

### Custom Metadata Search
```sql
SELECT * FROM storage_objects 
WHERE metadata->>'camera' = 'iPhone 13'
  AND metadata->>'location' = 'SF';
```

---

## Storage Usage Analytics

### Bucket Storage
```sql
SELECT 
  b.name AS bucket,
  COUNT(o.id) AS file_count,
  SUM(o.size) AS total_bytes,
  SUM(o.size) / 1024 / 1024 AS total_mb
FROM storage_buckets b
LEFT JOIN storage_objects o ON b.id = o.bucket_id
GROUP BY b.id, b.name
ORDER BY total_bytes DESC;
```

### User Storage Quota
```sql
SELECT 
  owner_id,
  COUNT(*) AS file_count,
  SUM(size) AS total_bytes
FROM storage_objects
WHERE owner_id IS NOT NULL
GROUP BY owner_id
HAVING SUM(size) > 1073741824  -- 1GB
ORDER BY total_bytes DESC;
```

---

## Metadata Events (Real-Time)

File operations emit events to AeroDB event log:

```json
{
  "type": "storage.object.created",
  "object_id": "123e4567-...",
  "bucket": "avatars",
  "path": "users/456/avatar.jpg",
  "size": 12345,
  "content_type": "image/jpeg",
  "owner_id": "789e0123-...",
  "timestamp": "2026-02-06T09:00:00Z"
}
```

Event types:
- `storage.object.created`
- `storage.object.updated` (overwrite)
- `storage.object.deleted`
- `storage.bucket.created`
- `storage.bucket.deleted`

Clients can subscribe:
```javascript
const subscription = supabase
  .channel('storage-changes')
  .on('storage.object.created', { bucket: 'avatars' }, handleUpload)
  .subscribe();
```

---

## Migration Strategy

### From No Metadata to Metadata
```sql
-- Scan filesystem, create metadata for existing files
INSERT INTO storage_objects (id, bucket_id, path, size, content_type, checksum)
SELECT 
  gen_random_uuid(),
  bucket_id_from_path(file_path),
  relative_path(file_path),
  file_size(file_path),
  detect_mime_type(file_path),
  sha256(file_path)
FROM enumerate_filesystem();
```

### Metadata Versioning
Future: Add `version` field to track overwrite history
```sql
ALTER TABLE storage_objects ADD COLUMN version INTEGER DEFAULT 1;
```
