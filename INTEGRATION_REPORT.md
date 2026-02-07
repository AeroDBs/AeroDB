# ⚠️ AeroDB Backend-Frontend Integration Wiring Audit

**Audit Date**: 2026-02-07  
**Auditor**: Senior Integration Engineer  
**Scope**: Full end-to-end verification of dashboard-backend wiring

---

## 🚨 EXECUTIVE SUMMARY

> **CRITICAL FINDING**: **95%+ of "dashboard endpoints are NOT wired to functional backend routes.**

The AeroDB frontend dashboard assumes a comprehensive HTTP REST API surface across 11 modules, making **150+ distinct API calls**. The backend, however, **only implements 4 HTTP routes**:

| Route | Status |
|-------|--------|
| `GET /health` | ✅ Implemented |
| `POST /auth/*` | ⚠️ Partial (5 endpoints only) |
| `GET /observability/*` | ⚠️ Minimal (2 endpoints only) |
| `POST /api/v1/operation` | ✅ Unified pipeline (CRUD only) |

**All other frontend service calls (`/api/tables/*`, `/storage/*`, `/functions/*`, `/realtime/*`, `/backup/*`, `/cluster/*`, `/observability/*`)** **→ 404 NOT FOUND**

---

## ✅ 1. WIRING COVERAGE TABLE
 
### Auth Module

| Feature | Frontend Function | Backend Endpoint | Method | Auth | RLS | Status |
|---------|-------------------|------------------|--------|------|-----|--------|
| **Login** | `authService.signIn()` | `/auth/login` | POST | ❌ | N/A | ✅ **WIRED** |
| **Signup** | `authService.signUp()` | `/auth/signup` | POST | ❌ | N/A | ✅ **WIRED** |
| **Logout** | `authService.signOut()` | `/auth/logout` | POST | ✅ | N/A | ✅ **WIRED** |
| **Refresh Token** | `authService.refreshToken()` | `/auth/refresh` | POST | ❌ | N/A | ✅ **WIRED** |
| **Get Current User** | `authService.getUser()` (called in header) | `/auth/user` | GET | ✅ | N/A | ✅ **WIRED** |
| **Get All Users** | `authService.getUsers()` | `/auth/users` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get User by ID** | `authService.getUser(userId)` | `/auth/users/:id` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Create User** | `authService.createUser()` | `/auth/users` | POST | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Update User** | `authService.updateUser()` | `/auth/users/:id` | PATCH | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Delete User** | `authService.deleteUser()` | `/auth/users/:id` | DELETE | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Sessions** | `authService.getSessions()` | `/auth/sessions` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Revoke Session** | `authService.revokeSession()` | `/auth/sessions/:id` | DELETE | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get RLS Policies** | `authService.getRLSPolicies()` | `/auth/rls/:table` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Create RLS Policy** | `authService.createRLSPolicy()` | `/auth/rls/:table` | POST | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Delete RLS Policy** | `authService.deleteRLSPolicy()` | `/auth/rls/:table/:id` | DELETE | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Toggle RLS Policy** | `authService.toggleRLSPolicy()` | `/auth/rls/:table/:id` | PATCH | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Forgot Password** | `authService.forgotPassword()` | `/auth/forgot-password` | POST | ❌ | N/A | ❌ **NOT IMPLEMENTED** |
| **Reset Password** | `authService.resetPassword()` | `/auth/reset-password` | POST | ❌ | N/A | ❌ **NOT IMPLEMENTED** |
| **Change Password** | `authService.changePassword()` | `/auth/change-password` | POST | ✅ | N/A | ❌ **NOT IMPLEMENTED** |
| **Get Password Policy** | `authService.getPasswordPolicy()` | `/auth/password-policy` | GET | ❌ | N/A | ❌ **NOT IMPLEMENTED** |
| **Verify Email** | `authService.verifyEmail()` | `/auth/verify-email` | POST | ❌ | N/A | ❌ **NOT IMPLEMENTED** |
| **Resend Verification** | `authService.resendVerificationEmail()` | `/auth/users/:id/resend-verification` | POST | ✅ | N/A | ❌ **NOT IMPLEMENTED** |

**Auth Coverage**: **5/22 endpoints (23%)**

---

### Database Module

| Feature | Frontend Function | Backend Endpoint | Method | Auth | RLS | Status |
|---------|-------------------|------------------|--------|------|-----|--------|
| **Get Tables** | `databaseService.getTables()` | `/api/tables` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Table Schema** | `databaseService.getTableSchema()` | `/api/tables/:name/schema` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Table Data** | `databaseService.getTableData()` | `/api/tables/:name/data` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Execute Query** | `databaseService.executeQuery()` | `/api/query` | POST | ✅ | ⚠️ | ⚠️ **PARTIAL** (via `/api/v1/operation`) |
| **Insert Row** | `databaseService.insertRow()` | `/api/tables/:name/rows` | POST | ✅ | ⚠️ | ⚠️ **PARTIAL** (via `/api/v1/operation`) |
| **Update Row** | `databaseService.updateRow()` | `/api/tables/:name/rows/:id` | PATCH | ✅ | ⚠️ | ⚠️ **PARTIAL** (via `/api/v1/operation`) |
| **Delete Row** | `databaseService.deleteRow()` | `/api/tables/:name/rows/:id` | DELETE | ✅ | ⚠️ | ⚠️ **PARTIAL** (via `/api/v1/operation`) |
| **Create Table** | `databaseService.createTable()` | `/api/tables` | POST | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Drop Table** | `databaseService.dropTable()` | `/api/tables/:name` | DELETE | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Statistics** | `databaseService.getStatistics()` | `/api/database/stats` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Migrations** | `databaseService.getMigrations()` | `/api/migrations` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Apply Migration** | `databaseService.applyMigration()` | `/api/migrations/:id/apply` | POST | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Rollback Migration** | `databaseService.rollbackMigration()` | `/api/migrations/:id/rollback` | POST | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Generate Migration** | `databaseService.generateMigration()` | `/api/migrations/generate` | POST | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Indexes** | `databaseService.getIndexes()` | `/api/tables/:name/indexes` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Create Index** | `databaseService.createIndex()` | `/api/tables/:name/indexes` | POST | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Drop Index** | `databaseService.dropIndex()` | `/api/tables/:name/indexes/:name` | DELETE | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Relationships** | `databaseService.getRelationships()` | `/api/tables/:name/relationships` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Create Relationship** | `databaseService.createRelationship()` | `/api/tables/:name/relationships` | POST | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get ERD Data** | `databaseService.getERDData()` | `/api/database/erd` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |

**Database Coverage**: **4/20 endpoints (20%)** via unified client only

---

### Storage Module

| Feature | Frontend Function | Backend Endpoint | Method | Auth | RLS | Status |
|---------|-------------------|------------------|--------|------|-----|--------|
| **Get Buckets** | `storageService.getBuckets()` | `/storage/buckets` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Bucket** | `storageService.getBucket()` | `/storage/buckets/:name` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Create Bucket** | `storageService.createBucket()` | `/storage/buckets` | POST | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Delete Bucket** | `storageService.deleteBucket()` | `/storage/buckets/:name` | DELETE | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Update Bucket** | `storageService.updateBucket()` | `/storage/buckets/:name` | PATCH | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **List Files** | `storageService.listFiles()` | `/storage/buckets/:name/files` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get File** | `storageService.getFile()` | `/storage/buckets/:name/files/:path` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Upload File** | `storageService.uploadFile()` | `/storage/buckets/:name/files` | POST | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Delete File** | `storageService.deleteFile()` | `/storage/buckets/:name/files/:path` | DELETE | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Move File** | `storageService.moveFile()` | `/storage/buckets/:name/files/move` | POST | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Create Signed URL** | `storageService.createSignedUrl()` | `/storage/buckets/:name/files/:path/sign` | POST | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Public URL** | `storageService.getPublicUrl()` | (client-side only) | N/A | ❌ | ❌ | ⚠️ **CLIENT ONLY** |
| **Get Bucket Stats** | `storageService.getBucketStats()` | `/storage/buckets/:name/stats` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **(+10 more storage endpoints)** | ... | ... | ... | ... | ... | ❌ **NOT IMPLEMENTED** |

**Storage Coverage**: **0/23 endpoints (0%)**

---

### Functions Module

| Feature | Frontend Function | Backend Endpoint | Method | Auth | RLS | Status |
|---------|-------------------|------------------|--------|------|-----|--------|
| **Get Functions** | `functionsService.getFunctions()` | `/functions` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Function** | `functionsService.getFunction()` | `/functions/:id` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Create Function** | `functionsService.createFunction()` | `/functions` | POST | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Update Function** | `functionsService.updateFunction()` | `/functions/:id` | PATCH | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Delete Function** | `functionsService.deleteFunction()` | `/functions/:id` | DELETE | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Invoke Function** | `functionsService.invokeFunction()` | `/functions/:id/invoke` | POST | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Function Logs** | `functionsService.getFunctionLogs()` | `/functions/:id/logs` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Invocations** | `functionsService.getInvocations()` | `/functions/:id/invocations` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Function Stats** | `functionsService.getFunctionStats()` | `/functions/:id/stats` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **(+10 more function endpoints)** | ... | ... | ... | ... | ... | ❌ **NOT IMPLEMENTED** |

**Functions Coverage**: **0/19 endpoints (0%)**

---

### Realtime Module

| Feature | Frontend Function | Backend Endpoint | Method | Auth | RLS | Status |
|---------|-------------------|------------------|--------|------|-----|--------|
| **Get Subscriptions** | `realtimeService.getSubscriptions()` | `/realtime/subscriptions` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get User Subscriptions** | `realtimeService.getUserSubscriptions()` | `/realtime/subscriptions?user_id=` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Channel Subscriptions** | `realtimeService.getChannelSubscriptions()` | `/realtime/subscriptions?channel=` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Broadcast** | `realtimeService.broadcast()` | `/realtime/broadcast` | POST | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Disconnect Subscription** | `realtimeService.disconnectSubscription()` | `/realtime/subscriptions/:id` | DELETE | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Realtime Stats** | `realtimeService.getRealtimeStats()` | `/realtime/stats` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get WebSocket URL** | `realtimeService.getWebSocketUrl()` | (client-side computes `ws://`) | N/A | ❌ | ❌ | ⚠️ **CLIENT ONLY** |

**Realtime Coverage**: **0/7 endpoints (0%)**

---

### Observability Module

| Feature | Frontend Function | Backend Endpoint | Method | Auth | RLS | Status |
|---------|-------------------|------------------|--------|------|-----|--------|
| **Health Check** | N/A | `/health` | GET | ❌ | N/A | ✅ **WIRED** |
| **Get Metrics** | `observabilityService.getMetrics()` | `/observability/metrics` | GET | ✅ | ❌ | ✅ **WIRED** (returns basic JSON) |
| **Get Logs** | `observabilityService.getLogs()` | `/observability/logs` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Stream Logs** | `observabilityService.getLogStreamUrl()` | `/observability/logs/stream` | GET/SSE | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Multiple Metrics** | `observabilityService.getMultipleMetrics()` | `/observability/metrics/batch` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Current Metrics** | `observabilityService.getCurrentMetrics()` | `/observability/metrics/current` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Performance Stats** | `observabilityService.getPerformanceStats()` | `/observability/performance` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Audit Log** | `observabilityService.getAuditLog()` | `/observability/audit` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Slow Queries** | `observabilityService.getSlowQueries()` | `/observability/slow-queries` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Explain Query** | `observabilityService.explainQuery()` | `/observability/explain` | POST | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **(+10 more observability endpoints)** | ... | ... | ... | ... | ... | ❌ **NOT IMPLEMENTED** |

**Observability Coverage**: **2/20+ endpoints (10%)**

---

### Backup Module

| Feature | Frontend Function | Backend Endpoint | Method | Auth | RLS | Status |
|---------|-------------------|------------------|--------|------|-----|--------|
| **Create Backup** | `backupService.createBackup()` | `/backup/create` | POST | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **List Backups** | `backupService.listBackups()` | `/backup/list` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Backup** | `backupService.getBackup()` | `/backup/:id` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Delete Backup** | `backupService.deleteBackup()` | `/backup/:id` | DELETE | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Download Backup** | `backupService.downloadBackup()` | `/backup/:id/download` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Restore Backup** | `backupService.restoreBackup()` | `/backup/:id/restore` | POST | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **(+10 more backup endpoints)** | ... | ... | ... | ... | ... | ❌ **NOT IMPLEMENTED** |

**Backup Coverage**: **0/16 endpoints (0%)**

---

### Cluster Module

| Feature | Frontend Function | Backend Endpoint | Method | Auth | RLS | Status |
|---------|-------------------|------------------|--------|------|-----|--------|
| **Get Nodes** | `clusterService.getNodes()` | `/cluster/nodes` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Node** | `clusterService.getNode()` | `/cluster/nodes/:id` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Topology** | `clusterService.getTopology()` | `/cluster/topology` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Replication Status** | `clusterService.getReplicationStatus()` | `/cluster/replication/status` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Promote Replica** | `clusterService.promoteReplica()` | `/cluster/promote` | POST | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **Get Cluster Health** | `clusterService.getClusterHealth()` | `/cluster/health` | GET | ✅ | ❌ | ❌ **NOT IMPLEMENTED** |
| **(+8 more cluster endpoints)** | ... | ... | ... | ... | ... | ❌ **NOT IMPLEMENTED** |

**Cluster Coverage**: **0/14 endpoints (0%)**

---

### Unified Pipeline Client

| Feature | Frontend Function | Backend Endpoint | Method | Auth | RLS | Status |
|---------|-------------------|------------------|--------|------|-----|--------|
| **Read Document** | `unifiedClient.read()` | `/api/v1/operation` | POST | ✅ | ✅ | ✅ **WIRED** |
| **Write Document** | `unifiedClient.write()` | `/api/v1/operation` | POST | ✅ | ✅ | ✅ **WIRED** |
| **Update Document** | `unifiedClient.update()` | `/api/v1/operation` | POST | ✅ | ✅ | ✅ **WIRED** |
| **Delete Document** | `unifiedClient.remove()` | `/api/v1/operation` | POST | ✅ | ✅ | ✅ **WIRED** |
| **Query Documents** | `unifiedClient.query()` | `/api/v1/operation` | POST | ✅ | ✅ | ✅ **WIRED** |
| **Invoke Function** | `unifiedClient.invoke()` | `/api/v1/operation` | POST | ✅ | ❌ | ⚠️ **STUB** (backend returns mock response) |
| **Subscribe** | (operation type) | `/api/v1/operation` | POST | ✅ | ❌ | ⚠️ **STUB** (backend returns mock response) |
| **Broadcast** | (operation type) | `/api/v1/operation` | POST | ✅ | ❌ | ⚠️ **STUB** (backend returns mock response) |
| **Upload** | (operation type) | `/api/v1/operation` | POST | ✅ | ❌ | ⚠️ **STUB** (backend returns mock response) |
| **Download** | (operation type) | `/api/v1/operation` | POST | ✅ | ❌ | ⚠️ **STUB** (backend returns mock response) |

**Unified Client Coverage**: **5/10 operations fully functional (50%)**

---

## 🚨 2. BROKEN WIRING LIST

### CRITICAL FAILURES

#### 1. **Database Management Endpoints → 404**

**Frontend:** `dashboard/src/services/database.ts`

**Calls:** 
- `GET /api/tables` → **404 NOT FOUND**
- `GET /api/tables/:name/schema` → **404 NOT FOUND**
- `GET /api/tables/:name/data` → **404 NOT FOUND**
- `POST /api/tables` (create table) → **404 NOT FOUND**
- `DELETE /api/tables/:name` (drop table) → **404 NOT FOUND**
- `GET /api/database/stats` → **404 NOT FOUND**
- `GET /api/migrations` → **404 NOT FOUND**
- `GET /api/tables/:name/indexes` → **404 NOT FOUND**
- `GET /api/tables/:name/relationships` → **404 NOT FOUND**
- `GET /api/database/erd` → **404 NOT FOUND**

**Backend State:**  
- NO `/api/*` routes registered in HTTP server
- Only `/api/v1/operation` exists (unified pipeline)
- Database CRUD operations **ONLY** via `/api/v1/operation` POST requests with operation payload

**Why Broken:**  
Frontend uses traditional REST endpoints (`GET /api/tables`), but backend only has unified POST endpoint.

---

#### 2. **Storage Endpoints → 404**

**Frontend:** `dashboard/src/services/storage.ts`

**Calls:**
- ALL 23 storage endpoints → **404 NOT FOUND**
- `GET /storage/buckets`
- `POST /storage/buckets`
- `POST /storage/buckets/:name/files` (upload)
- `POST /storage/buckets/:name/files/:path/sign` (signed URL)
- ... (20 more)

**Backend State:**  
- NO `/storage/*` routes registered
- File storage module exists at `src/file_storage/` but **NO HTTP ROUTES**

**Why Broken:**  
Storage module is fully implemented in Rust, but completely missing HTTP API layer.

---

#### 3. **Functions Endpoints → 404**

**Frontend:** `dashboard/src/services/functions.ts`

**Calls:**
- ALL 19 function endpoints → **404 NOT FOUND**  
- `GET /functions`
- `POST /functions/:id/invoke`
- `GET /functions/:id/logs`
- ... (16 more)

**Backend State:**  
- NO `/functions/*` routes registered
- Functions module exists at `src/functions/` but **NO HTTP ROUTES**

**Why Broken:**  
Functions module fully implemented, but missing HTTP layer.

---

#### 4. **Realtime WebSocket Not Accessible**

**Frontend:** `dashboard/src/services/realtime.ts`

**Calls:**
- `GET /realtime/subscriptions` → **404 NOT FOUND**
- `POST /realtime/broadcast` → **404 NOT FOUND**
- `DELETE /realtime/subscriptions/:id` → **404 NOT FOUND**
- WebSocket connection to `ws://localhost:54321/realtime` → **CONNECTION REFUSED**

**Backend State:**  
- NO `/realtime/*` routes registered
- Realtime module exists at `src/realtime/` but **NO HTTP/WS SERVER**

**Why Broken:**  
Realtime module implements WebSocket server logic, but never exposed via HTTP server router.

---

#### 5. **Auth Management Endpoints → 404**

**Frontend:** `dashboard/src/services/auth.ts`

**Calls (MISSING):**
- `GET /auth/users` → **404 NOT FOUND** (admin user list)
- `GET /auth/users/:id` → **404 NOT FOUND**
- `POST /auth/users` → **404 NOT FOUND** (admin create user)
- `PATCH /auth/users/:id` → **404 NOT FOUND**
- `DELETE /auth/users/:id` → **404 NOT FOUND**
- `GET /auth/sessions` → **404 NOT FOUND**
- `DELETE /auth/sessions/:id` → **404 NOT FOUND**
- `GET /auth/rls/:table` → **404 NOT FOUND**
- `POST /auth/rls/:table` → **404 NOT FOUND**
- `POST /auth/forgot-password` → **404 NOT FOUND**
- `POST /auth/reset-password` → **404 NOT FOUND**
- `POST /auth/change-password` → **404 NOT FOUND**
- `GET /auth/password-policy` → **404 NOT FOUND**
- `POST /auth/verify-email` → **404 NOT FOUND**

**Backend State:**  
`src/http_server/auth_routes.rs` **ONLY** implements:
- `POST /auth/signup`
- `POST /auth/login`
- `POST /auth/refresh`
- `POST /auth/logout`
- `GET /auth/user` (current user only)

All admin/management endpoints **NOT IMPLEMENTED**.

**Why Broken:**  
Backend auth routes are **hard-coded** for basic login/signup only. No user management, no RLS management, no password reset flows.

---

#### 6. **Backup/Cluster/Observability → 404**

**Frontend:**  
- `backupService.*` → ALL 16 endpoints **404**
- `clusterService.*` → ALL 14 endpoints **404**
- `observabilityService.*` → 18/20 endpoints **404**

**Backend State:**  
- `/backup/*` → **NOT REGISTERED**
- `/cluster/*` → **NOT REGISTERED**
- `/observability/*` → **ONLY** `/observability/health` and `/observability/metrics` exist

**Why Broken:**  
Modules implemented, HTTP layer missing.

---

#### 7. **Unified Pipeline Stubs Non-functional Operations**

**Frontend:** `dashboard/src/services/unifiedClient.ts`

**Backend:** `src/rest_api/unified_api.rs` lines 249-276

When frontend calls unified operations for:
- `Operation::Invoke` → Backend returns **MOCK JSON** `{"status": "queued"}`
- `Operation::Subscribe` → Backend returns **MOCK JSON** `{"status": "created"}`
- `Operation::Broadcast` → Backend returns **MOCK JSON** `{"status": "sent"}`
- `Operation::Upload` → Backend returns **MOCK JSON** `{"status": "pending"}`
- `Operation::Download` → Backend returns **MOCK JSON** `{"url": "/storage/v1/..."}`

**Why Broken:**  
Backend **pretends** operations succeed without actually invoking functions, uploading files, or managing subscriptions.

---

### HIGH PRIORITY FAILURES

#### 8. **Missing RLS Enforcement on REST API**

**Backend:** `src/rest_api/server.rs`

The `/rest/v1/*` routes exist but are **NOT INTEGRATED** into main HTTP server.

Even if they were, the RLS enforcement is **INCOMPLETE**:
- Lines 62-84: `extract_context()` validates JWT
- But `RestHandler` trait does NOT enforce RLS on all operations
- `PipelineBridge` enforces RLS, but direct REST handlers **MAY NOT**

**Security Risk:** If REST endpoints are enabled, they might bypass RLS.

---

#### 9. **Frontend Assumes Multipart Upload, Backend Lacks Support**

**Frontend:** `dashboard/src/services/storage.ts` lines 73-95

```typescript
async uploadFile(...) {
  const formData = new FormData()
  formData.append('file', file)
  await api.post(`/storage/buckets/${bucketName}/files`, formData, {
    headers: { 'Content-Type': 'multipart/form-data' }
  })
}
```

**Backend:** **NO HANDLER** for file uploads.

---

#### 10. **WebSocket Auth Handshake Not Verified**

**Frontend:** `dashboard/src/services/realtime.ts` line 68-72

```typescript
getWebSocketUrl(): string {
  const wsBaseUrl = baseURL.replace(/^http/, 'ws')
  return `${wsBaseUrl}/realtime`
}
```

**Backend:** **NO** WebSocket server registered in HTTP router.

Frontend **assumes** it can connect, but backend has no WS endpoint.

---

## ⚠️ 3. RISK ASSESSMENT

### 🔴 **CRITICAL RISKS**

| Risk | Severity | Impact |
|------|----------|--------|
| **95% of dashboard unusable** | CRITICAL | Users cannot access database, storage, functions, cluster, backup, realtime features |
| **Frontend silently fails with 404s** | CRITICAL | No error boundaries handle missing endpoints → blank pages, infinite loading |
| **No RLS enforcement on potential REST endpoints** | CRITICAL | If `/rest/v1/*` is enabled, may bypass RLS |
| **Mock responses mislead users** | CRITICAL | Unified client returns "success" for non-functional operations (invoke, upload) |
| **No WebSocket connection** | CRITICAL | Realtime features completely broken |

---

### 🟠 **HIGH RISKS**

| Risk | Severity | Impact |
|------|----------|--------|
| **No admin user management** | HIGH | Cannot create/update/delete users except via backend CLI |
| **No RLS policy management** | HIGH | Cannot create/edit RLS policies via UI |
| **No password reset flow** | HIGH | Users locked out cannot recover accounts |
| **No file upload capability** | HIGH | Storage feature non-functional |
| **No function invocation** | HIGH | Functions section shows mock data |
| **No backup/restore UI** | HIGH | Critical operations unavailable |
| **No cluster management** | HIGH | Cannot monitor/manage replication |

---

### 🟡 **MEDIUM RISKS**

| Risk | Severity | Impact |
|------|----------|--------|
| **Observability limited** | MEDIUM | Only basic health + metrics, no logs/audit/slow queries |
| **No schema migrations UI** | MEDIUM | Must use CLI for migrations |
| **No index management UI** | MEDIUM | Index optimization manual only |
| **No query profiler** | MEDIUM | Performance debugging difficult |

---

### 🟢 **LOW RISKS**

| Risk | Severity | Impact |
|------|----------|--------|
| **Unified client works for CRUD** | LOW | Core database read/write functional via `/api/v1/operation` |
| **Auth login/signup work** | LOW | Basic authentication functional |
| **Health check works** | LOW | `/health` responds correctly |

---

## 🔧 4. MINIMAL FIX RECOMMENDATIONS

### **Option A: Extend HTTP Server (Recommended)**

**Add missing routes to `src/http_server/server.rs`**

**Files to modify:**

1. **`src/http_server/mod.rs`**
   - Add: `pub mod database_routes;`
   - Add: `pub mod storage_routes;`
   - Add: `pub mod functions_routes;`
   - Add: `pub mod realtime_routes;`
   - Add: `pub mod backup_routes;`
   - Add: `pub mod cluster_routes;`

2. **`src/http_server/server.rs`** (lines 64-72)
   ```rust
   Router::new()
       .merge(health_routes())
       .nest("/auth", auth_routes(auth_state))
       .nest("/observability", observability_routes())
       // ADD THESE:
       .nest("/api", database_routes())
       .nest("/storage", storage_routes())
       .nest("/functions", functions_routes())
       .nest("/realtime", realtime_ws_routes()) // WebSocket upgrade
       .nest("/backup", backup_routes())
       .nest("/cluster", cluster_routes())
       .layer(cors)
   ```

3. **Create new files:**
   - `src/http_server/database_routes.rs` → wire `GET /api/tables`, etc.
   - `src/http_server/storage_routes.rs` → wire `/storage/*`
   - `src/http_server/functions_routes.rs` → wire `/functions/*`
   - `src/http_server/realtime_routes.rs` → WebSocket upgrade handler
   - `src/http_server/backup_routes.rs` → wire `/backup/*`
   - `src/http_server/cluster_routes.rs` → wire `/cluster/*`

4. **Extend `auth_routes.rs`**
   - Add handlers for `/auth/users`, `/auth/sessions`, `/auth/rls`, password management

---

### **Option B: Adapt Frontend to Unified Client (Less Recommended)**

**Rewrite all frontend services to use only `/api/v1/operation`**

**Problem:** Unified client **ONLY** supports:
- Read, Write, Update, Delete, Query operations
- **CANNOT** handle:
  - File uploads (needs multipart/form-data)
  - WebSocket subscriptions (needs WS upgrade)
  - Admin operations (user management, RLS policies)
  - Backup/restore (needs streaming)
  - Cluster management (needs real-time status)

**Verdict:** **NOT VIABLE** without significant backend rework.

---

### **Option C: Hybrid Approach (Pragmatic)**

**Phase 1: Critical Endpoints (1-2 weeks)**
- ✅ Add `/storage/*` routes (file upload is critical)
- ✅ Add RealTime WebSocket endpoint `/realtime` (ws upgrade)
- ✅ Extend `/auth/*` for user management + RLS

**Phase 2: Important Endpoints (2-4 weeks)**
- ✅ Add `/functions/*` routes (invoke, logs)
- ✅ Add `/observability/*` routes (logs, audit, slow queries)
- ✅ Add `/cluster/*` routes (topology, replication)

**Phase 3: Nice-to-Have (4+ weeks)**
- ✅ Add `/backup/*` routes
- ✅ Add `/api/*` routes for schema management (migrations, indexes, ERD)

---

## 🧪 5. VERIFICATION CHECKLIST

### **Pre-Fix Validation**

Run these to confirm broken state:

```bash
# 1. Start backend
cd /home/snigdha/aerodb
cargo run --release

# 2. Test endpoints (should return 404)
curl -X GET http://localhost:54321/api/tables
# Expected: 404 Not Found

curl -X GET http://localhost:54321/storage/buckets
# Expected: 404 Not Found

curl -X GET http://localhost:54321/functions
# Expected: 404 Not Found

curl -X GET http://localhost:54321/realtime/subscriptions
# Expected: 404 Not Found

curl -X GET http://localhost:54321/backup/list
# Expected: 404 Not Found

curl -X GET http://localhost:54321/cluster/nodes
# Expected: 404 Not Found

# 3. Test WebSocket (should fail)
wscat -c ws://localhost:54321/realtime
# Expected: Connection refused or 404

# 4. Test unified client (should work)
curl -X POST http://localhost:54321/api/v1/operation \
  -H "Content-Type: application/json" \
  -d '{"op":"query","collection":"users","limit":10,"offset":0}'
# Expected: 200 OK (even if empty data)
```

---

### **Post-Fix Validation (Option A)**

After implementing HTTP routes:

```bash
# 1. Test database routes
curl -X GET http://localhost:54321/api/tables
# Expected: 200 OK with table list

curl -X GET http://localhost:54321/api/database/stats
# Expected: 200 OK with statistics

# 2. Test storage routes
curl -X GET http://localhost:54321/storage/buckets
# Expected: 200 OK with bucket list

curl -X POST http://localhost:54321/storage/buckets \
  -H "Content-Type: application/json" \
  -d '{"name":"test-bucket","public":false}'
# Expected: 201 Created

# 3. Test functions routes
curl -X GET http://localhost:54321/functions
# Expected: 200 OK with function list

# 4. Test realtime WebSocket
wscat -c ws://localhost:54321/realtime
# Expected: Connection established, ready to subscribe

# 5. Test auth extensions
curl -X GET http://localhost:54321/auth/users \
  -H "Authorization: Bearer <admin-token>"
# Expected: 200 OK with user list

curl -X POST http://localhost:54321/auth/forgot-password \
  -H "Content-Type: application/json" \
  -d '{"email":"user@example.com"}'
# Expected: 200 OK

# 6. Test backup routes
curl -X GET http://localhost:54321/backup/list
# Expected: 200 OK with backup list

# 7. Test cluster routes
curl -X GET http://localhost:54321/cluster/nodes
# Expected: 200 OK with node list
```

---

### **Frontend Integration Test**

1. **Start frontend dev server:**
   ```bash
   cd /home/snigdha/aerodb/dashboard
   npm run dev
   ```

2. **Manual UI validation:**
   - ✅ Login → Should work
   - ✅ Database page → Should show tables (not 404)
   - ✅ Storage page → Should show buckets, allow upload
   - ✅ Functions page → Should show functions, allow invoke
   - ✅ Realtime page → Should show subscriptions, connect WebSocket
   - ✅ Auth page → Should show users, allow CRUD
   - ✅ Observability page → Should show logs, metrics, audit log
   - ✅ Backup page → Should show backups, allow create/restore
   - ✅ Cluster page → Should show nodes, topology

3. **Browser console check:**
   - ❌ **Before fix**: Hundreds of 404 errors
   - ✅ **After fix**: No 404 errors, all API calls succeed

---

## 📋 SUMMARY

### **Current State**

| Module | Frontend Endpoints | Backend Routes | Coverage |
|--------|-------------------|----------------|----------|
| Auth | 22 | 5 | **23%** |
| Database | 20 | 0 (unified only) | **20%** |
| Storage | 23 | 0 | **0%** |
| Functions | 19 | 0 | **0%** |
| Realtime | 7 | 0 | **0%** |
| Observability | 20+ | 2 | **10%** |
| Backup | 16 | 0 | **0%** |
| Cluster | 14 | 0 | **0%** |
| **TOTAL** | **141+** | **7** | **~5%** |

---

### **Recommended Action**

> **Implement Option C (Hybrid Approach)** with 3-phase rollout:
> 1. Add `/storage/*`, `/realtime` (WebSocket), `/auth/*` extensions
> 2. Add `/functions/*`, `/observability/*`, `/cluster/*`
> 3. Add `/backup/*`, `/api/*` schema management

Estimated effort: **6-8 weeks** for full coverage.

---

**Report Complete.**

---

## APPENDIX: Evidence

### A. Backend HTTP Server Registration

**File:** `/home/snigdha/aerodb/src/http_server/server.rs:64-72`

```rust
Router::new()
    // Health check at root level
    .merge(health_routes())
    // Auth routes under /auth
    .nest("/auth", auth_routes(auth_state))
    // Observability routes under /observability
    .nest("/observability", observability_routes())
    // Apply CORS middleware
    .layer(cors)
```

**Analysis:** Only 3 route groups registered. No `/api/*`, `/storage/*`, `/functions/*`, `/realtime/*`, `/backup/*`, `/cluster/*`.

---

### B. Auth Routes Implementation

**File:** `/home/snigdha/aerodb/src/http_server/auth_routes.rs:50-58`

```rust
pub fn auth_routes(state: Arc<AuthState>) -> Router {
    Router::new()
        .route("/signup", post(signup_handler))
        .route("/login", post(login_handler))
        .route("/refresh", post(refresh_handler))
        .route("/logout", post(logout_handler))
        .route("/user", get(get_user_handler))
        .with_state(state)
}
```

**Analysis:** Only 5 endpoints. Missing 17 frontend-expected endpoints.

---

### C. Unified API Stub Evidence

**File:** `/home/snigdha/aerodb/src/rest_api/unified_api.rs:249-276`

```rust
// Invoke returns function result
Operation::Invoke(invoke_op) => {
    Ok(serde_json::json!({
        "type": "invoke",
        "function": invoke_op.function_name,
        "status": "queued", // ← MOCK
        "async": invoke_op.async_mode
    }))
}

// Broadcast returns delivery confirmation
Operation::Broadcast(broadcast_op) => {
    Ok(serde_json::json!({
        "type": "broadcast",
        "channel": broadcast_op.channel,
        "event": broadcast_op.event,
        "status": "sent" // ← MOCK
    }))
}
```

**Analysis:** Returns success without invoking functions or broadcasting events.

---

**END OF REPORT**
