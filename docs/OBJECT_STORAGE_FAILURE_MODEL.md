# Phase 11: File Storage Failure Model

**Document Type:** Failure Model  
**Phase:** 11 - File Storage  
**Status:** Active

---

## Error Classification

### Client Errors (4xx)

#### 400 Bad Request
**Causes:**
- Mal formed upload request
- Invalid path characters
- Missing required headers

**Example:**
```
POST /storage/v1/object/avatars/../etc/passwd
→ 400: Invalid path (contains ..)
```

**Recovery:** Client must fix request

---

#### 403 Forbidden
**Causes:**
- RLS policy denies access
- User not bucket owner (private bucket)
- Invalid signed URL signature

**Example:**
```
GET /storage/v1/object/private_bucket/file.txt
Authorization: Bearer <user_b_token>
→ 403: Access denied (bucket owner is user_a)
```

**Recovery:** Authenticate as authorized user or request signed URL

---

#### 404 Not Found
**Causes:**
- Bucket doesn't exist
- File doesn't exist
- Metadata exists but file missing (orphan)

**Example:**
```
GET /storage/v1/object/nonexistent/file.txt
→ 404: Bucket not found
```

**Recovery:** Verify bucket/path or create bucket

**Special Case - Orphan Metadata:**
```
Metadata in DB: ✓
File on disk: ✗
→ 404 + log warning + trigger cleanup job
```

---

#### 409 Conflict
**Causes:**
- Bucket name already exists
- Bucket not empty (on delete)

**Example:**
```
POST /storage/v1/bucket {"name": "avatars"}
→ 409: Bucket already exists
```

**Recovery:** Choose different name or delete existing bucket

---

#### 413 Payload Too Large
**Causes:**
- File exceeds bucket `max_file_size`
- File exceeds global `storage.max_file_size`

**Example:**
```
POST /storage/v1/object/avatars/huge.jpg
Content-Length: 150000000  (150MB)
Bucket limit: 5MB
→ 413: File too large (150MB, max 5MB)
```

**Timing:** Validation BEFORE disk write (fail fast)

**Recovery:** Resize file or request limit increase

---

#### 415 Unsupported Media Type
**Causes:**
- MIME type not in bucket's `allowed_mime_types`

**Example:**
```
POST /storage/v1/object/images/doc.pdf
Content-Type: application/pdf
Bucket allows: image/*
→ 415: Invalid MIME type (application/pdf not allowed)
```

**Timing:** Validation BEFORE disk write

**Recovery:** Convert file or upload to different bucket

---

### Server Errors (5xx)

#### 500 Internal Server Error
**Causes:**
- Database connection failure
- Unexpected panic
- Checksum mismatch (data corruption)

**Example:**
```
GET /storage/v1/object/docs/file.txt
(Downloaded file checksum != stored checksum)
→ 500: Checksum mismatch (data corruption detected)
```

**Recovery:** 
- Retry (transient errors)
- Alert ops team (persistent errors)
- Re-upload file (corruption)

---

#### 503 Service Unavailable
**Causes:**
- Database maintenance
- Backend storage unavailable
- Server overloaded

**Example:**
```
POST /storage/v1/object/uploads/file.txt
(Connection pool exhausted)
→ 503: Service temporarily unavailable
Retry-After: 30
```

**Recovery:** Retry with exponential backoff

---

#### 507 Insufficient Storage
**Causes:**
- Disk full
- Quota exceeded (future)

**Example:**
```
POST /storage/v1/object/large_files/huge.bin
→ 507: Insufficient storage space
```

**Timing:** Detected during disk write

**Recovery:**
- Clean up old files
- Request storage increase
- Upgrade tier (future)

---

## Failure Scenarios

### 1. Upload Atomic Failure

#### Scenario: Disk write succeeds, DB insert fails
```
1. Write file to disk → OK
2. Insert metadata to DB → ERROR (connection lost)
```

**State:**
- File on disk: ✓
- Metadata in DB: ✗

**Recovery:**
```rust
async fn upload(path, data) -> Result<Object> {
    let file_path = write_to_disk(path, data)?;
    
    match insert_metadata(path, data.len()) {
        Ok(obj) => Ok(obj),
        Err(e) => {
            // Cleanup orphan file
            let _ = delete_from_disk(&file_path);
            Err(e)
        }
    }
}
```

**Invariant:** FS-F1 (Upload Failure Cleanup)

**Detection:** No metadata means upload failed, file cleaned up

---

### 2. Delete Partial Failure

#### Scenario: Metadata deleted, file delete fails
```
1. Delete metadata from DB → OK
2. Delete file from disk → ERROR (permission denied)
```

**State:**
- File on disk: ✓ (orphan)
- Metadata in DB: ✗

**Recovery:**
```
Orphan cleanup job (runs every hour):
  1. Scan filesystem
  2. Find files without metadata
  3. Delete orphan files
  4. Log cleanup actions
```

**Invariant:** FS-O1 (Bounded Orphan Window)

**User Impact:** Delete operation succeeds (from their perspective)

**Ops Impact:** Orphan file consumes disk space until cleanup

---

### 3. Concurrent Upload Collision

#### Scenario: Two clients upload to same path
```
Client A: POST /storage/v1/object/avatars/user123.jpg
Client B: POST /storage/v1/object/avatars/user123.jpg
```

**Behavior:**
- Both write to disk (last write wins)
- DB transaction serializes metadata insert
- Second insert fails with unique constraint violation

**Result:**
```
Client A: 200 OK
Client B: 409 Conflict (path already exists)
```

**Future:** Versioning to keep both uploads

---

### 4. Download During Delete

#### Scenario: Client downloads while another client deletes
```
Time  Client A (Download)        Client B (Delete)
T1    GET /avatars/user123.jpg
T2                                DELETE /avatars/user123.jpg
T3    (Read file from disk)
```

**Race Outcomes:**

| Timing | Result |
|--------|--------|
| Delete before read | 404 (expected) |
| Delete during read | Partial download or error |
| Read completes before delete | 200 OK (expected) |

**Mitigation:** File locking (future) or versioning

**Current:** Best-effort, no guarantees during concurrent access

---

### 5. Signed URL Expiration Race

#### Scenario: URL expires during upload
```
T1: Client gets signed URL (expires at T10)
T2-T9: Client uploads large file
T10: URL expires
T11: Upload completes
```

**Behavior:**
- Upload started with valid URL → Allow completion
- Server checks signature at START, not END

**Alternative:** Strict expiration (reject mid-upload)

**Decision:** Allow completion for better UX (current)

---

## Cascading Failures

### Database Unavailable
**Impact:**
- All operations fail (no metadata)
- Existing downloads may still work (file exists)

**Mitigation:**
- DB connection pooling with retry
- Circuit breaker pattern
- Graceful degradation (read-only mode)

---

### Storage Backend Unavailable
**Impact:**
- Uploads fail immediately
- Downloads fail
- Metadata operations still work

**Mitigation:**
- Health check before operations
- Multiple storage backends (future)
- Cached file serving (future CDN)

---

### Authentication Service Down
**Impact:**
- New uploads/downloads blocked
- Signed URLs still work (JWT self-contained)
- Service role operations still work

**Mitigation:**
- JWT caching
- Service role bypass for critical ops

---

## Orphan Cleanup Job

### Detection Logic
```sql
-- Find orphan files (file exists, no metadata)
SELECT path FROM enumerate_filesystem()
WHERE path NOT IN (
  SELECT bucket_id || '/' || path 
  FROM storage_objects
);

-- Find orphan metadata (metadata exists, no file)
SELECT bucket_id, path 
FROM storage_objects
WHERE NOT file_exists(bucket_id || '/' || path);
```

### Cleanup Actions
```rust
async fn cleanup_orphans() {
    // Orphan files: Delete from disk
    for file in detect_orphan_files() {
        delete_from_disk(&file)?;
        log_cleanup("orphan_file", &file);
    }
    
    // Orphan metadata: Delete from DB
    for metadata in detect_orphan_metadata() {
        delete_from_db(&metadata.id)?;
        log_cleanup("orphan_metadata", &metadata.path);
    }
}
```

### Scheduling
- **Frequency:** Every 1 hour
- **Grace period:** 5 minutes (allow recent uploads to complete)
- **Max cleanup per run:** 1000 files (prevent overload)

---

## Error Logging

### Structured Logs
```json
{
  "level": "ERROR",
  "service": "file_storage",
  "operation": "upload",
  "error": "507 Insufficient Storage",
  "bucket": "large_files",
  "path": "video.mp4",
  "size_bytes": 104857600,
  "user_id": "uuid",
  "request_id": "uuid",
  "timestamp": "2026-02-06T09:00:00Z"
}
```

### Metrics
- `storage_upload_errors_total{error="413"}`
- `storage_download_errors_total{error="404"}`
- `storage_orphan_files_cleaned_total`
- `storage_orphan_metadata_cleaned_total`

### Alerts
- **CRITICAL:** Orphan count > 1000
- **WARNING:** Error rate > 5% for 5 minutes
- **INFO:** Cleanup job completed

---

## Client Retry Strategy

### Recommended Approach
```javascript
async function uploadWithRetry(file, maxRetries = 3) {
  for (let attempt = 0; attempt < maxRetries; attempt++) {
    try {
      return await uploadFile(file);
    } catch (error) {
      if (error.status >= 400 && error.status < 500) {
        // Client error - don't retry
        throw error;
      }
      
      if (attempt === maxRetries - 1) {
        throw error;
      }
      
      // Exponential backoff: 2^attempt seconds
      await sleep(1000 * Math.pow(2, attempt));
    }
  }
}
```

### Idempotency
- Upload: NOT idempotent (creates new version)
- Download: Idempotent (read-only)
- Delete: Idempotent (delete nonexistent = success)
