# Phase 11: File Storage Vision

**Document Type:** Vision Statement  
**Phase:** 11 - File Storage  
**Status:** Active

---

## Goal

Provide **S3-compatible file storage** with **RLS-based access control**, enabling applications to store and retrieve files (images, documents, videos) with the same permission model and correctness guarantees as database records.

---

## Core Philosophy

### 1. **Metadata in Database**
File metadata lives in AeroDB collections, enjoying all the benefits of AeroDB:
- Transactional consistency
- MVCC visibility  
- Replication to followers
- Queryable via SQL/REST API

**Example:**
```sql
SELECT * FROM storage_objects 
WHERE bucket_id = 'avatars' 
  AND owner_id = current_user_id()
  AND size < 1000000;
```

### 2. **RLS for Files**
Files use the **same permission model** as collections:
- Public buckets (anyone can read)
- Private buckets (owner only)
- Authenticated buckets (any authenticated user)
- Per-file owner_id for fine-grained control

**Invariant:** No file access without RLS check

### 3. **Backend Abstraction**
Storage backend is pluggable:
- **Local filesystem** (Phase 11)
- **S3-compatible** (future Phase 11.1)
- **CDN integration** (future)

Application code is **backend-agnostic**

### 4. **Signed URLs**
Time-limited, pre-authenticated URLs for:
- Sharing files with unauthenticated users
- Direct browser upload/download (bypass API)
- Temporary access without session

**Example:**
```
https://api.aerodb.io/storage/v1/object/sign/avatars/user123.jpg
  ?token=eyJhbGc...
  &expires=1675123456
```

### 5. **Explicit Buckets**
No implicit storage locations. Applications must:
1. Create bucket with policy
2. Upload to bucket
3. Manage bucket lifecycle

**Anti-Pattern:** Auto-create buckets on first upload (implicit, surprising)

---

## Design Principles

### Simplicity Over Features
**Include:**
- Upload, download, delete
- Public/private buckets
- Signed URLs
- MIME type validation
- Size limits

**Exclude (defer):**
- Image transformations/thumbnails
- Video transcoding
- Multipart uploads
- Resumable uploads
- Object versioning
- CDN integration
- Replication across regions

**Rationale:** Focus on core file storage. Transformations are a separate phase.

---

### Fail-Safe Defaults
**Buckets default to private** (secure by default)
```rust
BucketConfig::default().policy == BucketPolicy::Private
```

**Uploads validate before writing** (fail fast)
```rust
// 1. Check size BEFORE disk I/O
if file.size > bucket.max_file_size {
    return Err(413);
}

// 2. Check MIME BEFORE disk I/O
if !bucket.is_mime_allowed(file.content_type) {
    return Err(415);
}

// 3. THEN write to disk
```

---

### Metadata Consistency
**Invariant FS-I1:** File exists ⟺ Metadata exists

**Implementation:**
- Atomic operations (file + metadata)
- Background cleanup job for orphans
- Health check on startup

**Why it matters:**
- Users expect metadata queries to reflect reality
- Download 404 on phantom metadata is confusing
- Orphan files waste storage

---

## Success Criteria

Phase 11 is successful when:

### Functional
1. ✅ Upload files up to 100MB
2. ✅ Download files with RLS enforcement
3. ✅ Delete files atomically (file + metadata)
4. ✅ Generate signed URLs valid for 1 hour
5. ✅ Public buckets allow anonymous read
6. ✅ Private buckets enforce ownership
7. ✅ MIME type filtering works
8. ✅ Size limits enforced

### Non-Functional
1. ✅ Upload throughput > 50 MB/s (local FS, p50)
2. ✅ Download throughput > 100 MB/s (local FS, p50)
3. ✅ Metadata query latency < 10ms (p99)
4. ✅ Orphan files cleaned within 1 hour
5. ✅ No data loss on crash (atomic operations)

### Security
1. ✅ All file access goes through RLS
2. ✅ Signed URLs expire correctly
3. ✅ Invalid signatures rejected
4. ✅ Service role bypass is explicit

---

## Integration with AeroDB

### Authentication (Phase 8)
```rust
// Every storage operation gets RLS context
let context = extract_rls_context(&request)?;
storage.upload(bucket, path, data, &context)?;
```

### REST API (Phase 9)
```
POST   /storage/v1/object/{bucket}/{path}  - Upload
GET    /storage/v1/object/{bucket}/{path}  - Download
DELETE /storage/v1/object/{bucket}/{path}  - Delete
POST   /storage/v1/bucket                  - Create bucket
```

### Real-Time (Phase 10)
File events emitted to event log:
```json
{
  "type": "storage.object.created",
  "bucket": "avatars",
  "path": "user123.jpg",
  "owner_id": "uuid",
  "timestamp": "2026-02-06T09:00:00Z"
}
```

Clients can subscribe to storage events.

---

## Examples

### Upload Avatar
```bash
curl -X POST \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: image/jpeg" \
  --data-binary @avatar.jpg \
  https://api.aerodb.io/storage/v1/object/avatars/user123.jpg
```

**Response:**
```json
{
  "id": "uuid",
  "bucket": "avatars",
  "path": "user123.jpg",
  "size": 12345,
  "content_type": "image/jpeg",
  "owner_id": "uuid",
  "created_at": "2026-02-06T09:00:00Z"
}
```

---

### Create Public Bucket
```bash
curl -X POST \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "public_docs",
    "policy": "public",
    "allowed_mime_types": ["application/pdf"],
    "max_file_size": 10485760
  }' \
  https://api.aerodb.io/storage/v1/bucket
```

---

### Generate Signed URL
```bash
curl -X POST \
  -H "Authorization: Bearer $JWT" \
  https://api.aerodb.io/storage/v1/object/sign/avatars/user123.jpg?expires_in=3600
```

**Response:**
```json
{
  "url": "https://api.aerodb.io/storage/v1/object/sign/avatars/user123.jpg?token=eyJhbGc...&expires=1675123456",
  "expires_at": "2026-02-06T10:00:00Z"
}
```

---

## Future Phases

### Phase 11.1: S3-Compatible Backend
- Configure S3 endpoint, bucket, credentials
- Transparent to application code
- Same metadata-in-DB approach

### Phase 11.2: Image Transformations
- Resize, crop, format conversion
- On-the-fly or pre-computed
- Cached results

### Phase 11.3: CDN Integration
- Edge caching for public files
- Signed URLs with CDN
- Cache invalidation

### Phase 11.4: Versioning
- Keep multiple versions of same path
- Delete = soft delete (mark version)
- Restore previous versions

---

## Explicit Non-Goals

**What Phase 11 does NOT do:**

### No File Locking
Files can be overwritten concurrently. Last write wins.
**Future:** Optimistic concurrency control with ETag

### No Streaming Uploads
Entire file must fit in memory during upload.
**Future:** Multipart/resumable uploads for large files

### No Automatic Backups
Files are NOT included in AeroDB snapshots/WAL.
**Future:** S3 lifecycle policies or external backup

### No Search
Cannot full-text search file contents.
**Use:** Store extracted text in AeroDB collection

### No Access Logs
No per-file access audit trail.
**Future:** Integrate with Phase 8 audit log

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Orphan files consume disk | Cleanup job runs hourly |
| Large files OOM server | Size limits enforced |
| Malicious uploads (virus) | Antivirus scan (future) |
| Signed URL abuse | Expiration + rate limiting |
| Concurrent access races | Documented, versioning (future) |
| Metadata-file desync | Atomic ops + health checks |

---

## Stakeholders

- **Application developers:** Simple file upload API
- **End users:** Fast, reliable file access
- **Security team:** RLS enforcement, signed URLs
- **Ops team:** Observable, reliable storage
- **AeroDB core:** Consistent with database philosophy
