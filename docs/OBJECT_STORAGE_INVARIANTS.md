# Phase 11: File Storage Invariants

**Document Type:** Invariants Specification  
**Phase:** 11 - File Storage  
**Status:** Active

---

## Core Invariants

### FS-I1: Metadata Consistency
> **File exists on disk ⟺ Metadata exists in database**

**Formal Statement:**
```
∀ file f, bucket b:
  exists_on_disk(f, b) ⟺ exists_in_db(metadata(f, b))
```

**Implications:**
- Upload MUST be atomic: file written AND metadata inserted
- Delete MUST be atomic: file removed AND metadata deleted
- Orphan detection: scheduled job finds files without metadata or metadata without files

**Violation Consequences:**
- Orphan files waste storage
- Orphan metadata causes 404 on download
- Both violate user expectations

**Enforcement:**
1. Atomic operations via transaction or two-phase commit
2. Background cleanup job runs every 1 hour
3. Metadata queries check file existence before returning

---

### FS-I2: Atomic Operations
> **Upload/Delete succeed completely or not at all**

**Formal Statement:**
```
Operation(upload | delete) → (complete ∧ consistent) ∨ (rollback ∧ unchanged)
```

**Upload Atomicity:**
```
BEGIN
  1. Write file to disk → disk_result
  2. Insert metadata to DB → db_result
  IF disk_result = OK ∧ db_result = OK:
    COMMIT
  ELSE:
    DELETE file from disk (if written)
    ROLLBACK DB transaction
    RETURN error
END
```

**Delete Atomicity:**
```
BEGIN
  1. Load metadata from DB → metadata
  2. Delete metadata from DB → db_result
  3. Delete file from disk → disk_result
  IF db_result = OK ∧ disk_result = OK:
    COMMIT
  ELSE:
    ROLLBACK DB transaction
    RETURN error (metadata may still exist)
END
```

**Enforcement:**
- Transactional metadata operations
- Cleanup on partial failure
- Idempotent retry logic

---

### FS-I3: RLS Enforcement
> **All file access goes through RLS check**

**Formal Statement:**
```
∀ operation op, file f, user u:
  perform(op, f, u) → first_check_rls(u, f.bucket) = allow
```

**Policy Types:**
```rust
Public:         allow if true
Authenticated:  allow if u.is_authenticated
Private:        allow if u.id = f.owner_id
Service:        allow if u.role = service_role
```

**Enforcement Points:**
1. **Upload**: Check write permission to bucket
2. **Download**: Check read permission to bucket
3. **Delete**: Check delete permission to bucket
4. **List**: Filter results by read permission

**Bypass Mechanism:**
Only service role can bypass RLS. This is explicit:
```rust
if context.can_bypass_rls() {
    // Service role - bypass RLS
} else {
    // Apply RLS based on bucket policy
}
```

**Violation Consequences:**
- Data leakage if not enforced
- Unauthorized modification/deletion
- Security breach

---

### FS-I4: Path Uniqueness
> **No two objects in same bucket have same path**

**Formal Statement:**
```
∀ objects o1, o2 in bucket b:
  o1.path = o2.path → o1.id = o2.id
```

**Implications:**
- Bucket + path is unique key
- Upload to existing path overwrites (future: versioning)
- List results have no duplicates

**Enforcement:**
```sql
CREATE UNIQUE INDEX idx_storage_objects_bucket_path 
ON storage_objects(bucket_id, path);
```

**Violation Consequences:**
- Ambiguous file resolution
- Undefined behavior on download
- Data loss on concurrent upload

---

## Failure Invariants

### FS-F1: Upload Failure Cleanup
> **Failed upload cleans up partial file**

**Guarantee:**
```
upload_fails(f) → ¬exists_on_disk(f) ∧ ¬exists_in_db(metadata(f))
```

**Failure Scenarios:**
1. **Disk write fails**: No metadata inserted
2. **Metadata insert fails**: File deleted from disk
3. **Network timeout**: Partial file deleted, no metadata
4. **Invalid MIME type**: Validation before any write
5. **Size limit exceeded**: Validation before any write

**Cleanup Strategy:**
```rust
fn upload_with_cleanup(path, data, context) -> Result<Object> {
    // Validate BEFORE writing
    validate_size(data.len())?;
    validate_mime(data.content_type)?;
    
    // Atomic operation
    let file_path = write_to_disk(path, data)?;
    match insert_metadata(path, context) {
        Ok(obj) => Ok(obj),
        Err(e) => {
            delete_from_disk(file_path); // Cleanup
            Err(e)
        }
    }
}
```

---

### FS-F2: Delete Failure Safe
> **Delete failure leaves system in consistent state**

**Guarantee:**
```
delete_fails(f) → 
  (exists_on_disk(f) ∧ exists_in_db(metadata(f))) ∨
  (¬exists_on_disk(f) ∧ ¬exists_in_db(metadata(f)))
```

**Failure Scenarios:**
1. **Metadata delete fails**: File still exists, metadata exists (retry)
2. **File delete fails**: Metadata removed, file orphaned (cleanup job handles)
3. **File not found**: Metadata removed (idempotent)

**Recovery Strategy:**
- If metadata delete fails: Entire operation fails, nothing changes
- If file delete fails after metadata removed: Orphan cleanup job removes file
- Idempotent: Deleting non-existent file succeeds

---

## Orphan Detection Invariants

### FS-O1: Bounded Orphan Window
> **Orphans detected within cleanup interval**

**Formal Statement:**
```
orphan_created(f) → detected(f) within cleanup_interval
```

**Default:** cleanup_interval = 1 hour

**Detection Logic:**
```sql
-- Files without metadata
SELECT path FROM filesystem 
WHERE path NOT IN (SELECT path FROM storage_objects);

-- Metadata without files  
SELECT path FROM storage_objects
WHERE NOT exists_on_filesystem(bucket_id || '/' || path);
```

---

### FS-O2: Cleanup Determinism
> **Cleanup job is idempotent**

**Guarantee:**
```
cleanup() → cleanup() produces same result
```

**Enforcement:**
- Read-only scan
- Explicit delete confirmations
- Logged actions for audit

---

## Signed URL Invariants

### FS-S1: Time-Limited Access
> **Expired signed URLs are rejected**

**Formal Statement:**
```
∀ url u: 
  now() > u.expires_at → access(u) = REJECTED
```

**Enforcement:**
```rust
fn verify_signed_url(url: &SignedUrl) -> Result<()> {
    if Utc::now() > url.expires_at {
        return Err(StorageError::UrlExpired);
    }
    verify_signature(url)?;
    Ok(())
}
```

---

### FS-S2: Signature Integrity
> **Only validly signed URLs accepted**

**Formal Statement:**
```
∀ url u:
  access(u) = ALLOWED → signature_valid(u, secret)
```

**Signature Generation:**
```
HMAC-SHA256(secret, bucket || path || expires_at)
```

**Enforcement:**
- Constant-time comparison
- Secret rotation support (future)
- No access without valid signature

---

## Invariant Testing Matrix

| Invariant | Unit Test | Integration Test | Stress Test |
|-----------|-----------|------------------|-------------|
| FS-I1 | ✅ Metadata sync | ✅ E2E upload | ✅ 10k files |
| FS-I2 | ✅ Atomic ops | ✅ Failure injection | ✅ Concurrent ops |
| FS-I3 | ✅ RLS enforcement | ✅ Multi-user | ✅ 1k users |
| FS-I4 | ✅ Path collision | ✅ Concurrent upload | ✅ Rapid overwrites |
| FS-F1 | ✅ Cleanup on error | ✅ Network timeout | - |
| FS-F2 | ✅ Delete idempotency | ✅ Partial failure | - |
| FS-O1 | ✅ Orphan detection | ✅ Cleanup job | - |
| FS-S1 | ✅ Expiration check | ✅ Time simulation | - |
| FS-S2 | ✅ Signature verify | ✅ Tamper detection | - |

---

## Invariant Monitoring

### Runtime Checks
- Metadata consistency check on startup
- Orphan count metric
- Failed upload cleanup counter

### Alerts
- Orphan count > 100 → WARNING
- Metadata-file mismatch > 10 → CRITICAL
- RLS bypass attempts → SECURITY
