# Phase 11: File Storage Readiness Checklist

**Document Type:** Readiness Checklist  
**Phase:** 11 - File Storage  
**Status:** In Progress

---

## Documentation Checklist

- [x] OBJECT_STORAGE_VISION.md - Goals and philosophy
- [x] OBJECT_STORAGE_ARCHITECTURE.md - Backend abstraction design
- [x] OBJECT_STORAGE_BUCKET_MODEL.md - Bucket policies and validation
- [x] OBJECT_STORAGE_ACCESS_MODEL.md - Signed URLs and RLS
- [x] OBJECT_STORAGE_METADATA_MODEL.md - File metadata in AeroDB
- [x] OBJECT_STORAGE_INVARIANTS.md - Storage invariants (FS-I1, FS-I2, FS-I3, FS-I4)
- [x] OBJECT_STORAGE_FAILURE_MODEL.md - Failure handling and recovery
- [x] OBJECT_STORAGE_TESTING_STRATEGY.md - Test coverage plan
- [x] OBJECT_STORAGE_READINESS.md - Freeze criteria (this document)

**Status:** ✅ All 9 documents complete

---

## Implementation Checklist

### Core Modules
- [ ] `src/file_storage/mod.rs` - Module entry point
- [ ] `src/file_storage/errors.rs` - Storage error types
- [ ] `src/file_storage/bucket.rs` - Bucket management
- [ ] `src/file_storage/file.rs` - File CRUD operations
- [ ] `src/file_storage/permissions.rs` - RLS integration
- [ ] `src/file_storage/backend.rs` - Storage backend trait
- [ ] `src/file_storage/local.rs` - Local filesystem backend
- [ ] `src/file_storage/signed_url.rs` - Signed URL generation/validation

### API Endpoints
- [ ] POST `/storage/v1/object/{bucket}/{path}` - Upload file
- [ ] GET `/storage/v1/object/{bucket}/{path}` - Download file
- [ ] DELETE `/storage/v1/object/{bucket}/{path}` - Delete file
- [ ] POST `/storage/v1/object/copy` - Copy file (future)
- [ ] POST `/storage/v1/object/move` - Move file (future)
- [ ] POST `/storage/v1/bucket` - Create bucket
- [ ] GET `/storage/v1/bucket` - List buckets
- [ ] DELETE `/storage/v1/bucket/{name}` - Delete bucket

### Metadata Storage
- [ ] `storage_objects` collection schema
- [ ] `storage_buckets` collection schema
- [ ] Executor integration for metadata queries
- [ ] Atomic file + metadata operations
- [ ] Orphan cleanup background job

### Permissions
- [ ] Public bucket policy (anyone can read)
- [ ] Private bucket policy (owner only)
- [ ] Authenticated bucket policy (any authed user)
- [ ] Per-file RLS context integration
- [ ] Signed URL HMAC generation
- [ ] Signed URL expiration enforcement
- [ ] Service role bypass

---

## Test Checklist

### Unit Tests
- [ ] Bucket creation and validation (> 10 tests)
- [ ] File upload/download/delete (> 15 tests)
- [ ] Permission enforcement (> 10 tests)
- [ ] Signed URL generation/validation (> 8 tests)
- [ ] Local backend operations (> 8 tests)
- [ ] Error handling (> 10 tests)

**Target:** > 60 unit tests, > 90% coverage

### Integration Tests
- [ ] End-to-end upload → metadata → download
- [ ] RLS enforcement with real auth contexts
- [ ] Concurrent file operations
- [ ] Metadata consistency checks
- [ ] Bucket deletion with files

**Target:** > 15 integration tests

### Stress Tests
- [ ] 10k files in single bucket
- [ ] 100MB file upload/download
- [ ] 1k concurrent uploads
- [ ] Rapid create/delete cycles

---

## Invariant Validation

Each invariant must have tests proving it holds:

| Invariant | Test Coverage | Status |
|-----------|---------------|--------|
| FS-I1: Metadata Consistency | Integration tests | ⬜ |
| FS-I2: Atomic Operations | Failure injection tests | ⬜ |
| FS-I3: RLS Enforcement | Permission tests | ⬜ |
| FS-I4: Path Uniqueness | Bucket tests | ⬜ |
| FS-F1: Upload Failure Cleanup | Error tests | ⬜ |
| FS-F2: Delete Failure Safe | Error tests | ⬜ |

---

## Performance Benchmarks

| Metric | Target | Actual | Status |
|--------|--------|--------|--------|
| Upload (1MB, local FS) | > 50 MB/s | - | ⬜ |
| Download (1MB, local FS) | > 100 MB/s | - | ⬜ |
| Metadata query (p99) | < 10ms | - | ⬜ |
| Signed URL generation | < 5ms | - | ⬜ |
| List 1000 files | < 50ms | - | ⬜ |

---

## Integration Points

### AeroDB Core
- [ ] Executor integration for metadata storage
- [ ] WAL integration for atomic operations (future)
- [ ] RLS context from auth module
- [ ] Error mapping to HTTP status codes

### REST API
- [ ] Storage endpoints registered
- [ ] Multipart upload support (future)
- [ ] Range request support (future)

### Authentication
- [ ] RLS context passed to all operations
- [ ] Service role bypass tested
- [ ] Signed URL JWT validation

---

## Deployment Checklist

### Configuration
- [ ] `storage.backend` = "local" | "s3" (future)
- [ ] `storage.local.root_dir` path
- [ ] `storage.max_file_size` global limit
- [ ] `storage.cleanup_interval` for orphans

### Migration
- [ ] Create `storage_objects` schema
- [ ] Create `storage_buckets` schema
- [ ] Index on `bucket_id` + `path`

### Monitoring
- [ ] Metrics: upload/download throughput
- [ ] Metrics: storage usage per bucket
- [ ] Metrics: orphan file count
- [ ] Logs: file operations with user context

---

## Freeze Criteria

Phase 11 can be frozen when:

1. ✅ All 9 documentation files complete and peer-reviewed
2. ⬜ All 8 core modules implemented
3. ⬜ All 8 API endpoints working
4. ⬜ Metadata stored in AeroDB via executor
5. ⬜ All 6 invariants have test coverage
6. ⬜ > 60 unit tests passing
7. ⬜ > 15 integration tests passing
8. ⬜ Performance benchmarks meet targets
9. ⬜ RLS enforcement verified
10. ⬜ Orphan cleanup job implemented

**Current Status:** 🔴 NOT READY

---

## Known Limitations (Deferred)

- S3-compatible backend (future Phase 11.1)
- Multipart uploads for large files (>100MB)
- Resumable uploads
- CDN integration
- Image transformations/thumbnails
- Storage quota enforcement per user/project
- Signed URL single-use tokens

These are explicitly deferred to keep Phase 11 focused on core file storage.

---

## Risk Assessment

| Risk | Mitigation | Owner |
|------|------------|-------|
| Metadata-file inconsistency | Atomic ops + cleanup job | Core team |
| Orphan files on crash | Background cleanup + WAL integration | Core team |
| Signed URL security | HMAC with rotation | Security team |
| Large file memory usage | Streaming upload/download | Performance team |
| Concurrent access races | File locking or versioning | Core team |

---

## Sign-Off Required

- [ ] Lead Engineer: Core implementation review
- [ ] Security: RLS and signed URL review
- [ ] QA: Test coverage and edge cases
- [ ] Docs: All 9 documents complete and accurate
- [ ] Performance: Benchmarks meet targets
