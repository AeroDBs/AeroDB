# AeroDB Source Code Analysis Report

## Executive Summary

This document provides a comprehensive analysis of the AeroDB backend source code (`/home/snigdha/aerodb/src`) to identify all hard-coded functions and compare them against the current dashboard implementation (`/home/snigdha/aerodb/dashboard`) to determine missing features.

## Backend Module Structure

AeroDB backend consists of **25 modules** organized into distinct BaaS features:

```
src/
├── api/              # Core CRUD operations (Phase 0)
├── auth/             # Authentication & Authorization (Phase 8)
├── backup/           # Database backup system
├── checkpoint/       # Checkpoint management
├── cli/              # CLI interface
├── dx/               # Developer experience utilities
├── executor/         # Query executor
├── file_storage/     # S3-compatible file storage (Phase 11)
├── functions/        # Serverless functions (Phase 12)
├── http_server/      # HTTP server infrastructure
├── index/            # Index management
├── mvcc/             # Multi-version concurrency control
├── observability/    # Audit, metrics, logging
├── performance/      # Performance monitoring
├── planner/          # Query planner
├── promotion/        # Replica promotion (Phase 6)
├── realtime/         # WebSocket subscriptions (Phase 10)
├── recovery/         # Crash recovery
├── replication/      # Data replication (Phase 5-6)
├── rest_api/         # Auto-generated REST API (Phase 9)
├── restore/          # Database restore
├── schema/           # Schema validation
├── snapshot/         # Snapshot management
├── storage/          # Storage layer
└── wal/              # Write-ahead logging
```

---

## Hard-Coded Functions by Module

### 1. **API Module** (`src/api/`)

**Core Operations:**
- `insert()` - Insert document with schema validation
- `update()` - Update existing document
- `delete()` - Delete document (tombstone)
- `query()` - Query with filters, pagination, sorting
- `explain()` - Query execution plan

**Request Flow:**
1. Schema validation via `SchemaValidator`
2. WAL append via `WalWriter`
3. Storage write via `StorageWriter`
4. Index update via `IndexManager`

**Supported Filter Operators:**
- `$eq` - Equality
- `$gt`, `$gte` - Greater than (or equal)
- `$lt`, `$lte` - Less than (or equal)

---

### 2. **Auth Module** (`src/auth/`)

**User Management:**
- `signup()` - User registration with password hashing
- `login()` - Authentication with JWT generation
- `refresh()` - Refresh access token
- `logout()` - Session invalidation
- `get_user()` - Get user by ID
- `update_user()` - Update user profile
- `change_password()` - Change password with validation
- `forgot_password()` - Request password reset (sends email)
- `reset_password()` - Reset password using token

**Session Management:**
- `create_session()` - Create new user session
- `validate_session()` - Validate session token
- `revoke_session()` - Invalidate session
- `cleanup_expired()` - Remove expired sessions

**JWT (JSON Web Tokens):**
- `generate_access_token()` - Create short-lived access token
- `generate_refresh_token()` - Create long-lived refresh token
- `validate_token()` - Verify JWT signature and expiration
- `extract_claims()` - Parse JWT claims

**RLS (Row-Level Security):**
- `create_policy()` - Define RLS policy
- `evaluate_policy()` - Check if operation allowed
- `apply_filter()` - Add RLS filters to queries
- Policy operations: `select`, `insert`, `update`, `delete`

**Email System:**
- `send_verification_email()` - Email verification link
- `send_password_reset_email()` - Password reset link
- `send_welcome_email()` - Welcome message

**Password Policy:**
- Minimum length enforcement
- Complexity requirements (uppercase, lowercase, numbers, symbols)
- Password hashing with Argon2

---

### 3. **REST API Module** (`src/rest_api/`)

**Auto-Generated Endpoints:**
- `GET /api/:collection` - List records with filtering
- `GET /api/:collection/:id` - Get single record
- `POST /api/:collection` - Insert record
- `PATCH /api/:collection/:id` - Update record
- `DELETE /api/:collection/:id` - Delete record

**Query Features:**
- `filter` - Field-based filtering
- `select` - Field selection/projection
- `order` - Sorting (asc/desc)
- `limit` - Pagination limit
- `offset` - Pagination offset

**RLS Integration:**
- Automatic RLS policy enforcement on all operations
- User context extraction from JWT
- Policy-based filtering applied to queries

---

### 4. **File Storage Module** (`src/file_storage/`)

**Bucket Operations:**
- `create_bucket()` - Create storage bucket
- `delete_bucket()` - Remove bucket
- `list_buckets()` - Get all buckets
- `get_bucket_info()` - Bucket metadata
- `update_bucket_config()` - Modify bucket settings (public/private)

**File Operations:**
- `upload_file()` - Upload file with metadata
- `download_file()` - Retrieve file
- `delete_file()` - Remove file
- `list_files()` - List files in bucket/path
- `get_file_metadata()` - File info (size, mime, created_at)
- `copy_file()` - Copy file within/across buckets
- `move_file()` - Move/rename file

**Access Control:**
- `check_permission()` - Verify user can access file
- `generate_signed_url()` - Temporary access URL with expiration
- `get_public_url()` - Public file URL (for public buckets)

**Storage Backends:**
- `LocalBackend` - Filesystem-based storage
- Extensible `StorageBackend` trait for S3/GCS/Azure

---

### 5. **Functions Module** (`src/functions/`)

**Function Management:**
- `register_function()` - Deploy new function
- `update_function()` - Update function code/config
- `delete_function()` - Remove function
- `list_functions()` - Get all functions
- `get_function()` - Function details

**Invocation:**
- `invoke_sync()` - Synchronous invocation
- `invoke_async()` - Asynchronous invocation
- `invoke_with_context()` - Invoke with DB/auth context

**Runtime:**
- `WasmtimeRuntime` - WebAssembly execution
- Environment variables injection
- Resource limits (CPU, memory, timeout)

**Triggers:**
- `HTTP` - HTTP endpoint triggers
- `Database` - Insert/update/delete triggers
- `Cron` - Scheduled execution

**Scheduler:**
- `schedule_cron()` - Set up cron job
- `cancel_schedule()` - Remove cron job
- `list_schedules()` - Get all scheduled functions

---

### 6. **Realtime Module** (`src/realtime/`)

**WebSocket Server:**
- `start_server()` - Start WebSocket server
- `handle_connection()` - New client connection
- `send_message()` - Send to specific client
- `broadcast()` - Send to all clients in channel

**Subscriptions:**
- `subscribe()` - Subscribe to channel/table
- `unsubscribe()` - Cancel subscription
- `list_subscriptions()` - Get active subscriptions
- `filter_subscription()` - Add filters to subscription

**Event System:**
- `publish_event()` - Publish event to channel
- `database_event()` - Convert DB change to event
- `broadcast_event()` - Custom channel event

**Presence:**
- `track_presence()` - Track user online status
- `get_presence()` - Get users in channel
- `remove_presence()` - User left

**Event Log:**
- `append_event()` - Log event (deterministic)
- `read_events()` - Replay events
- `cleanup_old_events()` - Remove old events

---

### 7. **Observability Module** (`src/observability/`)

**Audit Logging:**
- `log_action()` - Log user action (who, what, when)
- `query_audit_log()` - Search audit logs
- Tracks: logins, queries, mutations, permission changes

**Metrics:**
- `record_metric()` - Record metric value
- `get_metrics()` - Query metric data
- Built-in metrics: request_count, latency, error_rate, db_size

**Logger:**
- `log_debug()`, `log_info()`, `log_warn()`, `log_error()` - Structured logging
- `set_log_level()` - Configure verbosity
- `get_logs()` - Query logs with filters

**Events:**
- System event tracking (startup, shutdown, errors)
- Performance event tracking

---

### 8. **Backup Module** (`src/backup/`)

**Backup Operations:**
- `create_backup()` - Full database backup
- `list_backups()` - Get all backups
- `get_backup_info()` - Backup metadata
- `delete_backup()` - Remove backup

**Backup Format:**
- WAL snapshot + incremental WAL segments
- Compressed backup files
- Metadata (timestamp, size, WAL position)

---

### 9. **Snapshot Module** (`src/snapshot/`)

**Snapshot Management:**
- `create_snapshot()` - Create point-in-time snapshot
- `list_snapshots()` - Get all snapshots
- `delete_snapshot()` - Remove snapshot
- `restore_from_snapshot()` - Restore database state

---

### 10. **Cluster & Replication Module** (`src/replication/`, `src/promotion/`)

**Replication:**
- `setup_replication()` - Configure replica
- `start_replication()` - Begin sync
- `stop_replication()` - Pause replication
- `get_replication_status()` - Lag, position
- `list_replicas()` - Get all replicas

**Promotion (Failover):**
- `promote_replica()` - Promote replica to primary
- `demote_primary()` - Graceful primary demotion
- `check_promotion_eligibility()` - Verify replica ready

---

## Dashboard Current Implementation

### **Services Implemented** (`dashboard/src/services/`)

| Service | Functions Implemented | Status |
|---------|----------------------|---------|
| **Auth** | `signIn`, `signUp`, `signOut`, `refreshToken`, `getUsers`, `getUser`, `createUser`, `updateUser`, `deleteUser`, `getSessions`, `revokeSession`, `getRLSPolicies`, `createRLSPolicy`, `deleteRLSPolicy`, `toggleRLSPolicy` | ✅ Comprehensive |
| **Database** | `getTables`, `getTableSchema`, `getTableData`, `executeQuery`, `insertRow`, `updateRow`, `deleteRow`, `createTable`, `dropTable`, `getStatistics` | ✅ Good Coverage |
| **Storage** | `getBuckets`, `getBucket`, `createBucket`, `deleteBucket`, `updateBucket`, `listFiles`, `getFile`, `uploadFile`, `deleteFile`, `moveFile`, `createSignedUrl`, `getPublicUrl`, `getBucketStats` | ✅ Comprehensive |
| **Functions** | `getFunctions`, `getFunction`, `createFunction`, `updateFunction`, `deleteFunction`, `invokeFunction`, `getFunctionLogs`, `getInvocations`, `getFunctionStats` | ✅ Good Coverage |
| **Realtime** | `getSubscriptions`, `getUserSubscriptions`, `getChannelSubscriptions`, `broadcast`, `disconnectSubscription`, `getRealtimeStats`, `getWebSocketUrl` | ⚠️ Limited (WebSocket client not implemented) |
| **Backup** | `createBackup`, `listBackups`, `deleteBackup`, `restoreBackup`, `getBackupStatus` | ✅ Basic Coverage |
| **Cluster** | `getNodes`, `getNodeHealth`, `addNode`, `removeNode`, `getClusterMetrics` | ⚠️ Limited |
| **Observability** | `getAuditLogs`, `getMetrics`, `getSystemLogs`, `getPerformanceMetrics` | ⚠️ Limited |

---

## Missing Dashboard Features

> [!IMPORTANT]
> The following features exist in the backend but are **NOT** implemented or **PARTIALLY** implemented in the dashboard.

### 🔴 **Critical Missing Features**

#### **1. Auth Module - Password Management**
- ❌ **Forgot Password UI** - Backend has `forgot_password()` 
- ❌ **Reset Password Page** - Backend has `reset_password()`
- ❌ **Email Verification UI** - Backend has email verification system
- ❌ **Change Password Form** - Only for authenticated users

#### **2. Auth Module - User Management**
- ❌ **User Roles Management** - Backend supports roles but no UI
- ❌ **User Metadata Editor** - Backend stores metadata but no UI to edit
- ❌ **Password Policy Display** - No UI showing password requirements

#### **3. Auth Module - RLS Advanced Features**
- ❌ **RLS Policy Testing** - Backend can evaluate policies, no UI to test
- ❌ **RLS Policy Templates** - Common policies (user owns record, public read)
- ❌ **RLS Debugger** - Show why query was denied

#### **4. REST API Module**
- ❌ **REST API Explorer** - Backend auto-generates APIs, no UI to explore
- ❌ **API Key Management** - Backend can use API keys, no UI
- ❌ **OpenAPI/Swagger docs** - Backend has schema, no auto-docs

#### **5. File Storage - Advanced Features**
- ❌ **File Permissions UI** - Backend has permission system
- ❌ **File Preview** - Images, PDFs, videos
- ❌ **Bulk Upload** - Upload multiple files
- ❌ **Folder Management** - Backend supports paths, no folder UI
- ❌ **File Search** - Search within bucket
- ❌ **Signed URL Management** - View/manage active signed URLs
- ❌ **Storage Quota Management** - Per-bucket limits

---

### 🟡 **Important Missing Features**

#### **6. Functions - Advanced Management**
- ❌ **Function Code Editor** - Inline code editing
- ❌ **Function Templates** - Pre-built functions
- ❌ **Environment Variables UI** - Manage env vars
- ❌ **Function Testing** - Test with sample payloads
- ❌ **Trigger Configuration UI** - Visual trigger setup (cron, DB, HTTP)
- ❌ **Function Versioning** - Deploy/rollback versions
- ❌ **Cold Start Metrics** - Show function warmup times
- ❌ **Memory/CPU Usage** - Real-time resource monitoring

#### **7. Realtime - WebSocket Client**
- ❌ **Live WebSocket Connection** - Connect to backend WS
- ❌ **Channel Browser** - View active channels
- ❌ **Message Inspector** - View messages in real-time
- ❌ **Presence Visualization** - Show online users
- ❌ **Subscription Builder** - Visual subscription editor
- ❌ **Event Log Viewer** - View event history

#### **8. Observability - Enhanced Monitoring**
- ❌ **Real-Time Metrics Dashboard** - Live charts
- ❌ **Custom Metric Queries** - Filter/aggregate metrics
- ❌ **Alert Configuration** - Set up alerts on thresholds
- ❌ **Log Streaming** - Live log tail
- ❌ **Log Search** - Full-text search in logs
- ❌ **Audit Log Filtering** - Advanced filters (user, action, date)
- ❌ **Performance Profiler** - Query performance analysis
- ❌ **Slow Query Log** - Identify slow queries

---

### 🟢 **Nice-to-Have Missing Features**

#### **9. Backup - Automation**
- ❌ **Scheduled Backups** - Cron-based automatic backups
- ❌ **Backup Policies** - Retention policies (keep last N)
- ❌ **Incremental Backups** - Backend supports, no UI
- ❌ **Backup Verification** - Test restore
- ❌ **Cloud Backup** - S3/GCS backup destination

#### **10. Snapshot - Management**
- ❌ **Snapshot Browser** - View all snapshots
- ❌ **Snapshot Comparison** - Diff between snapshots
- ❌ **Automate Snapshot Creation** - Before dangerous operations

#### **11. Cluster - Monitoring & Management**
- ❌ **Cluster Topology View** - Visual node diagram
- ❌ **Node Health Dashboard** - CPU, memory, disk per node
- ❌ **Replication Lag Monitor** - Real-time lag metrics
- ❌ **Failover Wizard** - Step-by-step promotion
- ❌ **Node Configuration** - Edit node settings
- ❌ **Add Replica Wizard** - Add new replica step-by-step

#### **12. Database - Advanced Query Tools**
- ❌ **Visual Query Builder** - Drag-drop query builder
- ❌ **Query History** - Save/recall past queries
- ❌ **Saved Queries** - Named queries
- ❌ **Schema Visualization** - ER diagram
- ❌ **Data Export** - CSV/JSON export
- ❌ **Data Import** - Upload CSV/JSON

#### **13. Developer Experience**
- ❌ **API Playground** - Test API calls from dashboard
- ❌ **SDK Code Generator** - Generate client code
- ❌ **Webhook Management** - Backend can trigger webhooks
- ❌ **CLI Integration** - Run CLI commands from dashboard

---

## Summary Statistics

| Category | Backend Functions | Dashboard Functions | Coverage |
|----------|------------------|---------------------|----------|
| **Auth** | 25+ | 16 | 64% |
| **Database** | 30+ | 10 | 33% |
| **Storage** | 20+ | 13 | 65% |
| **Functions** | 15+ | 9 | 60% |
| **Realtime** | 15+ | 7 | 47% |
| **Observability** | 20+ | 4 | 20% |
| **Backup** | 10+ | 5 | 50% |
| **Cluster** | 15+ | 5 | 33% |
| **REST API** | 10+ | 0 | 0% |

**Overall Coverage: ~42%**

---

## Recommendations

### **Phase 1: Critical Features (Next 2-4 weeks)**
1. ✅ Complete Auth flows (password reset, email verification)
2. ✅ File storage permissions UI
3. ✅ WebSocket client for Realtime
4. ✅ REST API explorer

### **Phase 2: Important Features (1-2 months)**
1. ✅ Function code editor + testing
2. ✅ Enhanced observability (live metrics, log search)
3. ✅ Cluster monitoring dashboard
4. ✅ Backup automation

### **Phase 3: Nice-to-Have (2-4 months)**
1. Visual query builder
2. API playground
3. Function templates
4. SDK code generator
5. Advanced RLS debugger

---

## Hard-Coded Backend Functions Summary

### **Total Hard-Coded Functions: ~200+**

Breakdown by module:
- **API**: 5 core operations
- **Auth**: 25+ functions (user, session, JWT, RLS, email)
- **REST API**: 10+ auto-generated endpoints
- **File Storage**: 20+ file/bucket operations
- **Functions**: 15+ function management + invocation
- **Realtime**: 15+ WebSocket, subscriptions, presence
- **Observability**: 20+ audit, metrics, logging
- **Backup/Snapshot**: 15+ backup/snapshot operations
- **Cluster/Replication**: 15+ cluster management
- **Other modules**: ~50+ (recovery, WAL, storage, schema, etc.)

---

## Conclusion

The AeroDB backend is **feature-rich** with comprehensive BaaS functionality. However, the dashboard currently exposes only **~42%** of backend capabilities. The most critical gaps are:

1. **Auth**: Password reset/email verification flows
2. **REST API**: No exploration/documentation UI
3. **File Storage**: Missing permissions and advanced features
4. **Realtime**: No WebSocket client integration
5. **Observability**: Limited monitoring/logging UI
6. **Cluster**: Minimal replication/failover management

Prioritizing **Phase 1 features** would significantly improve the dashboard's utility and bring it closer to feature parity with the backend.
