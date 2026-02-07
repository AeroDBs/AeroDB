# Phase 11: File Storage Testing Strategy

**Document Type:** Testing Strategy  
**Phase:** 11 - File Storage  
**Status:** Active

---

## Test Coverage Goals

| Component | Target | Focus Areas |
|-----------|--------|-------------|
| Bucket operations | 95% | CRUD, validation, concurrency |
| File CRUD | 95% | Upload, download, delete, move, copy |
| Permissions | 98% | RLS enforcement, public/private, signed URLs |
| Signed URLs | 90% | Generation, validation, expiration |
| Backend abstraction | 90% | Local FS, future S3 |
| Metadata storage | 95% | DB integration, consistency |

---

## Unit Tests

### Bucket Management Tests
```rust
#[test]
fn test_create_bucket_with_policy()
fn test_duplicate_bucket_name_errors()
fn test_delete_non_empty_bucket_fails()
fn test_bucket_mime_type_validation()
fn test_bucket_size_limit_enforcement()
fn test_bucket_owner_assignment()
```

### File Operations Tests
```rust
#[test]
fn test_upload_to_public_bucket()
fn test_upload_to_private_bucket_as_owner()
fn test_upload_to_private_bucket_unauthorized()
fn test_download_respects_rls()
fn test_delete_requires_ownership()
fn test_file_metadata_stored_in_db()
fn test_atomic_upload_failure_cleanup()
fn test_checksum_verification()
```

### Permission Tests
```rust
#[test]
fn test_public_bucket_anonymous_read()
fn test_private_bucket_owner_only()
fn test_authenticated_bucket_any_user()
fn test_rls_context_integration()
fn test_service_role_bypass()
```

### Signed URL Tests
```rust
#[test]
fn test_signed_url_generation()
fn test_signed_url_expiration()
fn test_invalid_signature_rejected()
fn test_expired_url_rejected()
fn test_url_single_use_enforcement() // future
```

---

## Integration Tests

### End-to-End Scenarios
1. **Complete Upload Flow**
   - Create bucket → Upload file → Verify metadata in DB → Download → Verify content

2. **Permission Enforcement**
   - Upload as user A → Attempt download as user B → Verify rejection
   - Upload as user A → Download as user A → Verify success

3. **Signed URL Flow**
   - Generate signed URL → Access anonymously → Verify content
   - Wait for expiration → Access again → Verify rejection

4. **Concurrent Operations**
   - Multiple uploads to same bucket
   - Upload during bucket deletion
   - Delete during download

5. **Failure Recovery**
   - Upload with network interruption → Verify cleanup
   - Delete with missing file → Verify metadata removal
   - Orphan cleanup job execution

---

## Stress Tests

### Volume Testing
- 10k files in single bucket
- 1k concurrent uploads
- 100MB file upload/download
- Rapid create/delete cycles

### Metadata Consistency
- Verify file exists ⟺ metadata exists
- Concurrent metadata queries
- Transaction rollback handling

---

## Failure Injection

| Scenario | Expected Behavior |
|----------|-------------------|
| Disk full during upload | Error 507, partial file cleaned up |
| Database down | Error 500, no orphan files |
| Network timeout | Graceful timeout, cleanup |
| Invalid MIME type | Error 415 before write |
| Exceeds size limit | Error 413 before write |
| Missing bucket | Error 404, no file created |

---

## RLS Testing

### Ownership-Based Access
```rust
#[test]
fn test_file_owner_can_read()
fn test_non_owner_cannot_read_private()
fn test_service_role_bypasses_rls()
fn test_rls_filters_list_results()
```

### Policy Enforcement
```rust
#[test]
fn test_public_bucket_any_read()
fn test_authenticated_bucket_authed_only()
fn test_private_bucket_owner_only()
```

---

## Performance Benchmarks

### Target Metrics
- Upload throughput: > 50 MB/s (local FS)
- Download throughput: > 100 MB/s (local FS)
- Metadata query: < 10ms (p99)
- Signed URL generation: < 5ms

### Benchmark Tests
```rust
#[bench]
fn bench_upload_1mb_file()
fn bench_download_1mb_file()
fn bench_generate_signed_url()
fn bench_list_1000_files()
```

---

## Coverage Validation

Run with coverage:
```bash
cargo tarpaulin --lib --packages aerodb --out Lcov -- file_storage::
```

**Acceptance:** > 90% line coverage, > 85% branch coverage

---

## Test Data Management

### Fixtures
- Sample files (text, binary, large)
- Test user contexts (owner, non-owner, anonymous, service role)
- Bucket configurations (public, private, authenticated, MIME filters)

### Cleanup
All tests use `TempDir` for isolation. No shared state.

---

## Invariant Validation

Each test category validates specific invariants:

- **FS-I1** (Metadata Consistency): Integration tests verify file ⟺ metadata
- **FS-I2** (Atomic Operations): Failure injection tests verify cleanup
- **FS-I3** (RLS Enforcement): Permission tests verify blocking
- **FS-I4** (Path Uniqueness): Bucket tests verify collision detection
