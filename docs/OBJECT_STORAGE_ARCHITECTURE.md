# Phase 11: File Storage Architecture

**Document Type:** Technical Architecture  
**Phase:** 11 - File Storage  
**Status:** Active

---

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Client Applications                       │
│                   (REST API, SDKs, Admin UI)                     │
└────────────────────────────┬────────────────────────────────────┘
                             │
                  ┌──────────▼──────────┐
                  │    REST API Layer   │
                  │  (Phase 9)          │
                  └──────────┬──────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
┌───────▼────────┐  ┌────────▼────────┐  ┌───────▼────────┐
│   Auth/RLS     │  │  File Storage   │  │   Database     │
│   (Phase 8)    │  │    Service      │  │   (Executor)   │
└────────────────┘  └─────────┬───────┘  └────────────────┘
                              │
                    ┌─────────┴─────────┐
                    │                   │
            ┌───────▼──────┐   ┌────────▼────────┐
            │   Storage    │   │    Metadata     │
            │   Backend    │   │   Collections   │
            │(FS, S3, etc) │   │ (storage_*)     │
            └──────────────┘   └─────────────────┘
```

---

## Component Architecture

### 1. File Storage Service

**Location:** `src/file_storage/`

**Responsibilities:**
- Coordinate file operations (upload, download, delete)
- Enforce bucket policies and RLS
- Validate MIME types and size limits
- Generate/verify signed URLs
- Ensure metadata-file consistency

**Does NOT:**
- Store metadata directly (uses Database Executor)
- Manage WebSocket connections
- Handle authentication (delegates to Auth module)

---

### 2. Storage Backend Abstraction

```rust
pub trait StorageBackend {
    async fn write(&self, path: &str, data: &[u8]) -> Result<()>;
    async fn read(&self, path: &str) -> Result<Vec<u8>>;
    async fn delete(&self, path: &str) -> Result<()>;
    async fn exists(&self, path: &str) -> Result<bool>;
    async fn size(&self, path: &str) -> Result<u64>;
}
```

**Implementations:**

#### Local Filesystem (`src/file_storage/local.rs`)
```rust
pub struct LocalBackend {
    root_dir: PathBuf,  // e.g., /var/lib/aerodb/storage
}
```

**Path Mapping:**
```
Bucket ID: 550e8400-e29b-41d4-a716-446655440000
File path: users/123/avatar.jpg
Physical: /var/lib/aerodb/storage/550e8400.../users/123/avatar.jpg
```

#### S3-Compatible Backend (Future: `src/file_storage/s3.rs`)
```rust
pub struct S3Backend {
    endpoint: String,
    bucket: String,
    credentials: Credentials,
}
```

**Benefits of Abstraction:**
- Switch backends without code changes
- Test with local, deploy with S3
- Multiple backends per environment

---

### 3. Metadata Storage

**Collections:** `storage_buckets`, `storage_objects`

**Integration:**
```rust
// FileStorage does NOT directly access collections
// Instead, uses Database Executor
pub struct FileStorage {
    backend: Box<dyn StorageBackend>,
    executor: Arc<Executor>,  // ⬅️ For metadata queries
    permission_checker: PermissionChecker,
}
```

**Metadata Operations:**
```rust
// Insert metadata (transactional)
let query = Insert::new("storage_objects")
    .values(metadata_as_document)?;
executor.execute(query, &rls_context)?;

// Query metadata
let query = Select::new("storage_objects")
    .filter(Eq::new("bucket_id", bucket_id))?;
let results = executor.execute(query, &rls_context)?;
```

**Why Executor Integration:**
- Metadata participates in MVCC
- Replication includes metadata
- RLS applied to metadata queries
- Transactional consistency

---

## Module Structure

```
src/file_storage/
├── mod.rs              # Module entry, exports
├── errors.rs           # Storage error types
├── bucket.rs           # Bucket management (CRUD)
├── file.rs             # File operations (upload, download, delete)
├── permissions.rs      # RLS enforcement, policy checks
├── backend.rs          # StorageBackend trait
├── local.rs            # Local filesystem implementation
├── s3.rs               # S3-compatible implementation (future)
└── signed_url.rs       # Signed URL generation/verification
```

---

## Data Flow

### Upload Flow

```
1. Client → REST API
   POST /storage/v1/object/avatars/user123.jpg
   Headers: Authorization, Content-Type
   Body: <file bytes>

2. REST API → Auth Module
   Extract JWT → RlsContext

3. REST API → File Storage Service
   upload(bucket="avatars", path="user123.jpg", data, context)

4. File Storage Service:
   a. Load bucket from metadata (via Executor)
   b. Check permissions (PermissionChecker)
   c. Validate MIME type (bucket.allowed_mime_types)
   d. Validate size (bucket.max_file_size)
   e. Calculate checksum (SHA-256)
   
5. File Storage Service → Storage Backend
   backend.write(physical_path, data)
   
6. File Storage Service → Database Executor
   executor.execute(INSERT INTO storage_objects ...)
   
7. If step 6 fails:
   backend.delete(physical_path)  // Cleanup
   
8. File Storage Service → REST API
   Return StorageObject

9. REST API → Client
   201 Created {id, bucket, path, size, ...}
```

**Atomicity Guarantee:**
- File written to disk
- Metadata inserted to DB
- If either fails, both rollback

---

### Download Flow

```
1. Client → REST API
   GET /storage/v1/object/avatars/user123.jpg
   Headers: Authorization

2. REST API → Auth Module
   Extract JWT → RlsContext

3. REST API → File Storage Service
   download(bucket="avatars", path="user123.jpg", context)

4. File Storage Service → Database Executor
   Query metadata: SELECT * FROM storage_objects WHERE ...
   
5. File Storage Service:
   a. Check permissions (PermissionChecker.check_read)
   
6. File Storage Service → Storage Backend
   data = backend.read(physical_path)
   
7. File Storage Service:
   a. (Optional) Verify checksum
   
8. File Storage Service → REST API
   Return (data, content_type)

9. REST API → Client
   200 OK
   Headers: Content-Type, Content-Length
   Body: <file bytes>
```

---

### Delete Flow

```
1. Client → REST API
   DELETE /storage/v1/object/avatars/user123.jpg

2. Auth → RlsContext

3. File Storage Service:
   a. Query metadata
   b. Check permissions (owner only)
   c. Delete metadata from DB
   d. Delete file from disk

4. If disk delete fails:
   - Metadata already removed
   - Orphan file handled by cleanup job
   
5. Return 204 No Content
```

---

### Signed URL Flow

**Generation:**
```
1. Client → REST API
   POST /storage/v1/object/sign/avatars/user123.jpg?expires_in=3600

2. Auth → RlsContext (must be authenticated)

3. File Storage Service:
   a. Check user can access file
   b. Generate HMAC signature
   c. Create URL with token + expiration

4. Return signed URL to client
```

**Usage:**
```
1. Client (unauthenticated) → REST API
   GET /storage/v1/object/avatars/user123.jpg?token=...&expires=...

2. File Storage Service:
   a. Extract token, expires from query params
   b. Verify expiration (now < expires)
   c. Verify signature (constant-time comparison)
   d. If valid, serve file (bypass RLS)

3. Return file bytes
```

---

## Integration Points

### With Authentication (Phase 8)

```rust
// Every storage operation requires RLS context
let context = extract_rls_context(&request)?;

// Service role bypass
if context.can_bypass_rls {
    // Skip permission checks (admin operations)
}
```

---

### With REST API (Phase 9)

**Endpoints Added:**
```
POST   /storage/v1/bucket
GET    /storage/v1/bucket
DELETE /storage/v1/bucket/{name}

POST   /storage/v1/object/{bucket}/{path}
GET    /storage/v1/object/{bucket}/{path}
DELETE /storage/v1/object/{bucket}/{path}

POST   /storage/v1/object/sign/{bucket}/{path}
```

---

### With Real-Time (Phase 10)

**Events Emitted:**
```rust
// After successful upload
emit_event(Event::StorageObjectCreated {
    bucket_id,
    path,
    owner_id,
    timestamp: Utc::now(),
});

// Clients can subscribe
subscription.filter("storage.object.created", {bucket: "avatars"})
```

---

### With Executor (Core)

**Metadata Queries:**
```rust
// All metadata operations go through Executor
let query = Select::new("storage_objects")
    .filter(Eq::new("bucket_id", bucket_id))?;
    
let results = executor.execute(query, &rls_context)?;
```

**Why Not Direct Collection Access:**
- RLS enforcement
- MVCC visibility
- Transaction support
- Replication

---

## Orphan Cleanup Job

**Architecture:**
```rust
pub struct OrphanCleanupJob {
    backend: Arc<dyn StorageBackend>,
    executor: Arc<Executor>,
    interval: Duration,  // Default: 1 hour
}

impl OrphanCleanupJob {
    pub async fn run(&self) {
        loop {
            sleep(self.interval).await;
            
            // Find orphan files (file exists, no metadata)
            let orphan_files = self.find_orphan_files().await?;
            for file in orphan_files {
                self.backend.delete(&file)?;
                log::warn!("Cleaned orphan file: {}", file);
            }
            
            // Find orphan metadata (metadata exists, no file)
            let orphan_metadata = self.find_orphan_metadata().await?;
            for metadata_id in orphan_metadata {
                self.executor.execute(
                    Delete::new("storage_objects")
                        .filter(Eq::new("id", metadata_id))
                )?;
                log::warn!("Cleaned orphan metadata: {}", metadata_id);
            }
        }
    }
}
```

**Scheduling:** Background thread, started with AeroDB instance

---

## Error Handling

**Error Propagation:**
```rust
pub async fn upload(
    bucket: &str,
    path: &str,
    data: Vec<u8>,
    context: &RlsContext,
) -> Result<StorageObject, StorageError> {
    // Each operation can fail with specific error
    let bucket_meta = self.load_bucket(bucket)?;            // NotFound
    self.check_permissions(&bucket_meta, context)?;         // Unauthorized
    self.validate_mime(&bucket_meta, &content_type)?;       // UnsupportedMediaType
    self.validate_size(&bucket_meta, data.len())?;          // PayloadTooLarge
    
    let physical_path = self.backend.write(path, &data)?;   // IoError
    
    match self.insert_metadata(path, ...) {
        Ok(obj) => Ok(obj),
        Err(e) => {
            self.backend.delete(&physical_path)?;           // Cleanup
            Err(e)
        }
    }
}
```

**HTTP Mapping:**
```rust
impl From<StorageError> for HttpStatus {
    fn from(err: StorageError) -> HttpStatus {
        match err {
            StorageError::NotFound => 404,
            StorageError::Unauthorized => 403,
            StorageError::UnsupportedMediaType => 415,
            StorageError::PayloadTooLarge => 413,
            StorageError::IoError(_) => 500,
            ...
        }
    }
}
```

---

## Performance Considerations

### Streaming (Future)
Current: Load entire file into memory
```rust
let data = backend.read(path)?;  // Loads all bytes
```

Future: Stream for large files
```rust
let stream = backend.read_stream(path)?;
while let Some(chunk) = stream.next().await {
    // Send chunk to client
}
```

### Caching (Future)
- Metadata cache (reduce DB queries)
- File content cache (CDN integration)
- Checksum cache (avoid recalculation)

### Concurrency
- File operations are NOT serialized
- Concurrent uploads to different files: OK
- Concurrent uploads to same file: Last write wins (current), versioning (future)

---

## Security Architecture

### Defense in Depth
1. **Authentication:** JWT validation (Phase 8)
2. **Authorization:** Bucket policy + RLS enforcement
3. **Validation:** MIME type, size limits (fail fast)
4. **Isolation:** Physical paths use bucket ID (prevent path traversal)
5. **Signed URLs:** Time-limited, HMAC-signed
6. **Audit:** All operations logged with user context

### Threat Mitigation

| Threat | Mitigation |
|--------|------------|
| Path traversal | Validate path (no `..`), use bucket ID in physical path |
| Unauthorized access | RLS enforcement on every operation |
| Large file DoS | Size limits enforced before write |
| Malicious MIME types | MIME type validation (future: antivirus scan) |
| Signed URL abuse | Expiration + signature verification |

---

## Observability

### Metrics
```
storage_upload_total{bucket, status}
storage_download_total{bucket, status}
storage_upload_bytes_total{bucket}
storage_download_bytes_total{bucket}
storage_orphan_files_total
storage_backend_latency_seconds{operation, backend}
```

### Logs
```json
{
  "level": "INFO",
  "service": "file_storage",
  "operation": "upload",
  "bucket": "avatars",
  "path": "user123.jpg",
  "size": 12345,
  "user_id": "uuid",
  "duration_ms": 45
}
```

### Alerts
- Orphan count > 1000
- Upload error rate > 5%
- Backend latency > 1s (p99)
