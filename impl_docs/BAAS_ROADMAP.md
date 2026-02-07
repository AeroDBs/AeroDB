# AeroDB BaaS Transformation Roadmap

**Document Type:** Strategic Planning  
**Current State:** Phase 7 Complete (Control Plane)  
**Target State:** Backend-as-a-Service Platform  
**Date:** 2026-02-06

---

## Executive Summary

This document provides a comprehensive roadmap for transforming AeroDB from a correctness-first database engine into a Backend-as-a-Service (BaaS) platform comparable to Supabase, while preserving AeroDB's core philosophy of determinism, correctness, and explicit control.

**Key Challenge:** Supabase prioritizes developer velocity and "magic" (auto-generated APIs, automatic features), while AeroDB prioritizes correctness and explicit control. The transformation must reconcile these philosophies.

**Recommendation:** Create "AeroDB BaaS" as an **optional layer** on top of the core database, maintaining strict separation between the correctness kernel (Phases 0-7) and the BaaS convenience layer (Phase 8+).

---

## Current State Analysis (Phase 0-7)

### ✅ What AeroDB Already Has

#### Core Database Engine
- ✅ WAL-backed durability with crash safety
- ✅ MVCC snapshot isolation
- ✅ B-tree indexes
- ✅ Schema validation
- ✅ Deterministic query planning and execution
- ✅ Checkpoint and backup/restore

#### Replication & High Availability
- ✅ Single-writer replication (Phase 5)
- ✅ Replica reads with visibility guarantees
- ✅ Explicit promotion with safety validation (Phase 6)
- ✅ Crash-safe authority transfer (with blockers to resolve)

#### Observability & Control
- ✅ Explanation engine (Phase 4)
- ✅ Read-only observability API (Phase 4)
- ✅ Control plane with operator commands (Phase 7)
- ✅ Audit logging (Phase 7)
- ✅ Structured logging and metrics

#### Developer Interface
- ✅ CLI for all operations
- ✅ Error taxonomy with deterministic errors
- ✅ Query explain plans

---

## ❌ What's Missing for BaaS

Compared to Supabase, AeroDB lacks:

### 1. Authentication & Authorization
- ❌ User authentication (email, social logins, magic links)
- ❌ Session management and JWT tokens
- ❌ Row-Level Security (RLS) or equivalent
- ❌ API key management
- ❌ OAuth provider integrations

### 2. Auto-Generated APIs
- ❌ REST API auto-generated from schema
- ❌ GraphQL API
- ❌ Real-time subscriptions (WebSocket)
- ❌ API filtering, pagination, sorting

### 3. File Storage
- ❌ File upload/download API
- ❌ S3-compatible storage
- ❌ Permission-based file access
- ❌ CDN integration

### 4. Serverless Functions
- ❌ Edge functions / serverless compute
- ❌ Database webhooks / triggers
- ❌ Scheduled functions (cron jobs)

### 5. Developer Experience
- ❌ Web-based admin dashboard
- ❌ Client SDKs (JavaScript, Python, Go, etc.)
- ❌ Database migrations tooling
- ❌ Local development environment
- ❌ CLI for deployment and management

### 6. Hosting & Infrastructure
- ❌ Multi-tenant isolation
- ❌ Project provisioning
- ❌ Usage metering and billing
- ❌ Backup scheduling and management
- ❌ Connection pooling

---

## Phase 8: Authentication & Authorization (CRITICAL)

**Priority:** HIGHEST (foundational for BaaS)  
**Estimated Effort:** 3-4 months  
**Philosophy Alignment:** Medium (requires balancing convenience with explicitness)

### 8.1 Authentication Service

**Component:** `AeroAuth` (inspired by Supabase's GoTrue)

#### Features to Implement

##### 8.1.1 User Management
- **User Registration:**
  - Email + password
  - Email verification flow
  - Password strength requirements (explicit, no hidden rules)
- **User Login:**
  - Email + password authentication
  - Session creation with JWT tokens
  - Refresh token mechanism
- **Password Management:**
  - Password reset via email
  - Password change for authenticated users
  - Password history (prevent reuse)

##### 8.1.2 Social Authentication (Optional Phase 8.2)
- OAuth 2.0 integration framework
- Providers: Google, GitHub, GitLab, Bitbucket
- Explicit provider configuration (no auto-discovery)

##### 8.1.3 Magic Links (Optional Phase 8.3)
- Passwordless authentication via email
- Time-limited, single-use tokens
- Explicit expiration and revocation

#### Implementation Approach

**Module:** `src/auth/`

```
src/auth/
├── mod.rs
├── user.rs          # User model and storage
├── session.rs       # Session management
├── jwt.rs           # JWT token generation/validation
├── crypto.rs        # Password hashing (bcrypt/Argon2)
├── email.rs         # Email sending (SMTP integration)
├── api.rs           # Auth API endpoints
└── errors.rs        # Auth-specific errors
```

**Key Design Decisions:**

1. **User Storage:**
   - Store users as documents in AeroDB (schema: `users` collection)
   - Schema includes: id, email, hashed_password, email_verified, created_at, updated_at
   - Use AeroDB's MVCC for versioning

2. **Session Storage:**
   - Sessions stored in AeroDB (schema: `sessions` collection)
   - Include: session_id, user_id, refresh_token, access_token, expires_at
   - Auto-expire via explicit TTL check (no background cleanup)

3. **JWT Tokens:**
   - Stateless JWT for access tokens (short-lived, 15 minutes)
   - Stateful refresh tokens (stored in DB, long-lived, 30 days)
   - Explicit token rotation on refresh

4. **AeroDB Philosophy Alignment:**
   - ✅ Explicit user creation (no auto-provisioning)
   - ✅ Deterministic auth flows (no timing-dependent behavior)
   - ✅ Fail-closed (invalid credentials → immediate rejection)
   - ✅ Observable (all auth events logged to audit trail)

#### API Endpoints

```
POST /auth/signup          # Register new user
POST /auth/login           # Authenticate user
POST /auth/logout          # Invalidate session
POST /auth/refresh         # Refresh access token
POST /auth/forgot-password # Request password reset
POST /auth/reset-password  # Reset password with token
GET  /auth/user            # Get current user info
PUT  /auth/user            # Update user profile
```

---

### 8.2 Authorization & Row-Level Security (RLS)

**Goal:** Fine-grained access control for data

#### RLS Design for AeroDB

Unlike PostgreSQL's built-in RLS, AeroDB must implement RLS explicitly:

**Option 1: Query-Level Enforcement (Recommended)**

- Inject `user_id` filter into all queries automatically
- Enforce at query planning stage (before execution)
- Fail-closed: queries without valid user context rejected

**Option 2: Schema-Level Ownership**

- Add `owner_id` field to all schemas
- Validate ownership at write time
- Filter reads by ownership at executor level

**Implementation:**

```rust
// src/auth/rls.rs

pub struct RLSContext {
    pub user_id: String,
    pub roles: Vec<String>,
}

pub trait RLSEnforcer {
    fn enforce_read(&self, query: &Query, context: &RLSContext) -> Result<Query>;
    fn enforce_write(&self, write: &WriteOp, context: &RLSContext) -> Result<()>;
}
```

**Integration with Existing Planner:**

- Extend `src/planner/planner.rs` to accept `RLSContext`
- Inject ownership filters before planning
- Deterministic (same user + query → same filtered query)

---

## Phase 9: Auto-Generated REST API

**Priority:** HIGH  
**Estimated Effort:** 2-3 months  
**Philosophy Alignment:** HIGH (can be explicit and deterministic)

### 9.1 REST API Generator

**Component:** `AeroREST` (inspired by PostgREST)

#### Features

##### 9.1.1 Automatic Endpoints from Schema

For each collection (schema), auto-generate:

```
GET    /rest/v1/{collection}           # List records (with filters)
GET    /rest/v1/{collection}/{id}      # Get single record
POST   /rest/v1/{collection}           # Insert record
PATCH  /rest/v1/{collection}/{id}      # Update record
DELETE /rest/v1/{collection}/{id}      # Delete record
```

##### 9.1.2 Query Parameters

- **Filtering:** `?field=value`, `?field=gt.10`, `?field=like.*search*`
- **Sorting:** `?order=created_at.desc`
- **Pagination:** `?limit=20&offset=0`
- **Field Selection:** `?select=id,name,email`

##### 9.1.3 Relations (Future)

- **Embedded:** `?select=*,author(*)`
- **Foreign Key Expansion:** Auto-join on explicit foreign keys

#### Implementation

**Module:** `src/rest_api/`

```
src/rest_api/
├── mod.rs
├── generator.rs     # Schema → endpoint mapping
├── parser.rs        # Query parameter parsing
├── handler.rs       # HTTP request handling
├── filter.rs        # Filter AST generation
├── response.rs      # JSON response formatting
└── errors.rs        # HTTP error mapping
```

**HTTP Server:**

- Use `axum` or `actix-web` for HTTP handling
- Integrate with existing `src/api/handler.rs` logic
- Reuse query planner for deterministic execution

**Key Design Decisions:**

1. **Schema Introspection:**
   - Read from `schemas` collection at startup
   - Generate endpoint registry in memory
   - Reload on schema changes (explicit reload, no auto-watch)

2. **Query Translation:**
   - Query params → AeroDB AST
   - Leverage existing planner for bounds checking
   - Reject unbounded queries (Q1 invariant preserved)

3. **Error Handling:**
   - Map AeroDB errors to HTTP status codes
   - 400: Invalid query (Q3: execution never guesses)
   - 403: RLS violation
   - 500: Internal error (with deterministic error ID)

---

## Phase 10: Real-Time Subscriptions

**Priority:** MEDIUM  
**Estimated Effort:** 3-4 months  
**Philosophy Alignment:** MEDIUM (requires balancing real-time with determinism)

### 10.1 Real-Time Engine

**Component:** `AeroRealtime` (inspired by Supabase Realtime)

#### Features

##### 10.1.1 Database Change Streams

- Subscribe to insert/update/delete events on collections
- Filter subscriptions by query predicates
- Broadcast changes to connected clients via WebSocket

##### 10.1.2 Broadcast Channels

- Pub/sub messaging across clients
- Channel-based isolation
- Explicit channel permissions

##### 10.1.3 Presence

- Track active users in a channel
- Heartbeat-based liveness detection
- Explicit join/leave events

#### Implementation Challenges

**Determinism Conflict:**

- AeroDB is deterministic, but real-time delivery is non-deterministic (network timing)
- **Resolution:** Separate "event generation" (deterministic) from "event delivery" (non-deterministic)

**Architecture:**

```
WAL Entries → Event Log (deterministic) → Real-Time Dispatcher (non-deterministic)
```

**Module:** `src/realtime/`

```
src/realtime/
├── mod.rs
├── event_log.rs     # WAL → Event transformation (deterministic)
├── dispatcher.rs    # WebSocket event delivery (non-deterministic)
├── subscription.rs  # Client subscription management
├── broadcast.rs     # Pub/sub channels
├── presence.rs      # User presence tracking
└── errors.rs
```

**Key Design Decisions:**

1. **Event Log as Source of Truth:**
   - Append-only event log derived from WAL
   - Each WAL entry → one or more events
   - Events are deterministic (same WAL → same events)

2. **Non-Deterministic Delivery:**
   - WebSocket dispatcher is explicitly non-deterministic
   - No guarantees on delivery order or timing
   - Clients must handle out-of-order events

3. **RLS Enforcement:**
   - Filter events by RLS rules before delivery
   - Clients only receive events they're authorized to see

---

## Phase 11: File Storage

**Priority:** MEDIUM  
**Estimated Effort:** 2 months  
**Philosophy Alignment:** HIGH (explicit permissions, S3-compatible)

### 11.1 Storage Service

**Component:** `AeroStorage`

#### Features

##### 11.1.1 File Operations

```
POST   /storage/v1/object/{bucket}/{path}      # Upload file
GET    /storage/v1/object/{bucket}/{path}      # Download file
DELETE /storage/v1/object/{bucket}/{path}      # Delete file
POST   /storage/v1/object/copy                 # Copy file
POST   /storage/v1/object/move                 # Move file
```

##### 11.1.2 Bucket Management

```
POST   /storage/v1/bucket           # Create bucket
GET    /storage/v1/bucket           # List buckets
DELETE /storage/v1/bucket/{name}    # Delete bucket
```

##### 11.1.3 Permissions

- Bucket-level policies (public/private)
- RLS integration (per-file permissions)
- Signed URLs for temporary access

#### Implementation

**Backend Options:**

1. **Local Filesystem:**
   - Simple, self-hosted
   - Store in `<data_dir>/storage/`
   - No external dependencies

2. **S3-Compatible:**
   - Use MinIO or AWS S3
   - Better scalability
   - CDN integration

**Recommended:** Start with local filesystem, add S3 support later.

**Module:** `src/storage/`

```
src/storage/
├── mod.rs
├── bucket.rs        # Bucket management
├── file.rs          # File operations
├── permissions.rs   # RLS integration
├── backend.rs       # Storage backend abstraction
├── local.rs         # Local filesystem backend
└── s3.rs            # S3-compatible backend (future)
```

**Metadata Storage:**

- Store file metadata in AeroDB (schema: `storage_objects`)
- Fields: bucket, path, size, content_type, owner_id, created_at, updated_at
- Separate metadata (in DB) from blobs (on disk/S3)

---

## Phase 12: Edge Functions

**Priority:** LOW  
**Estimated Effort:** 3-4 months  
**Philosophy Alignment:** LOW (serverless is inherently non-deterministic)

### 12.1 Serverless Functions

**Component:** `AeroFunctions` (inspired by Supabase Edge Functions)

#### Features

##### 12.1.1 Function Deployment

- Deploy TypeScript/JavaScript functions
- HTTP trigger or database event trigger
- Explicit function declaration (no auto-discovery)

##### 12.1.2 Runtime

- **Option 1:** Deno runtime (like Supabase)
  - Secure by default
  - TypeScript native
  - Web-standard APIs

- **Option 2:** WebAssembly (Wasm)
  - Language-agnostic
  - Sandboxed execution
  - Better performance

**Recommendation:** Deno (easier developer experience)

#### Implementation Challenges

**Non-Determinism:**

- Edge functions are inherently non-deterministic
- **Resolution:** Treat as separate layer, clearly document non-determinism

**Module:** `src/functions/`

```
src/functions/
├── mod.rs
├── runtime.rs       # Deno runtime integration
├── registry.rs      # Function registry
├── trigger.rs       # HTTP and DB triggers
├── executor.rs      # Function execution
└── errors.rs
```

**Integration:**

- Functions can call AeroDB via SDK
- Functions run outside core database (separate process)
- Failures are contained (fail-closed for DB, fail-open for functions)

---

## Phase 13: Admin Dashboard (Web UI)

**Priority:** HIGH  
**Estimated Effort:** 4-6 months  
**Philosophy Alignment:** HIGH (observability without semantic authority)

### 13.1 Web Dashboard

**Component:** `AeroDashboard`

#### Features

##### 13.1.1 Database Management

- View collections and schemas
- Browse data (paginated table view)
- Execute queries (SQL-like or filter-based)
- View explain plans
- Schema editor (create/modify schemas)

##### 13.1.2 Authentication Management

- User list and management
- Session monitoring
- Role management (future)

##### 13.1.3 Storage Management

- Browse buckets and files
- Upload/download files
- View storage usage

##### 13.1.4 Real-Time Monitoring

- Active connections
- Event throughput
- Subscription list

##### 13.1.5 Control Plane Integration

- Cluster topology view (from Phase 7)
- Promotion controls
- Replication lag monitoring
- System health dashboard

##### 13.1.6 Logs & Observability

- Structured log viewer
- Metrics dashboard
- Audit log viewer

#### Technology Stack

**Frontend:**

- **Framework:** React or Vue.js
- **UI Library:** Tailwind CSS + shadcn/ui (modern, accessible)
- **Charts:** Recharts or Chart.js
- **State Management:** React Query or Pinia

**Backend:**

- Reuse AeroDB REST API and Control Plane API
- Add dashboard-specific endpoints if needed

**Architecture:**

```
Browser → AeroDashboard (React) → REST API → AeroDB Core
                                 → Control Plane API
                                 → Realtime (WebSocket)
```

**Phase 4 Compliance:**

- Dashboard is read-only for data (write via explicit actions only)
- All actions logged to audit trail
- No semantic authority (just a view into the database)

---

## Phase 14: Client SDKs

**Priority:** HIGH  
**Estimated Effort:** 3-4 months per SDK  
**Philosophy Alignment:** HIGH (SDKs are thin clients)

### 14.1 JavaScript/TypeScript SDK

**Package:** `@aerodb/client`

#### Features

```typescript
import { createClient } from '@aerodb/client'

const aerodb = createClient({
  url: 'https://your-project.aerodb.io',
  apiKey: 'your-api-key'
})

// Authentication
const { user, error } = await aerodb.auth.signUp({
  email: 'user@example.com',
  password: 'password123'
})

const { data, error } = await aerodb.auth.signIn({
  email: 'user@example.com',
  password: 'password123'
})

// Database queries
const { data, error } = await aerodb
  .from('posts')
  .select('*')
  .eq('author_id', user.id)
  .order('created_at', { ascending: false })
  .limit(10)

// Real-time subscriptions
const subscription = aerodb
  .from('messages')
  .on('INSERT', (payload) => {
    console.log('New message:', payload.new)
  })
  .subscribe()

// Storage
const { data, error } = await aerodb.storage
  .from('avatars')
  .upload('user1.png', file)
```

#### Implementation

**Repository:** `sdks/javascript/`

```
sdks/javascript/
├── src/
│   ├── AeroDBClient.ts
│   ├── auth/
│   ├── database/
│   ├── realtime/
│   ├── storage/
│   └── functions/
├── tests/
├── package.json
└── README.md
```

**Key Requirements:**

- TypeScript-first (with JavaScript support)
- Tree-shakeable (ESM modules)
- Framework-agnostic (works with React, Vue, Svelte, etc.)
- Typed query builder
- Error handling (map HTTP errors to SDK errors)

---

### 14.2 Python SDK

**Package:** `aerodb-py`

#### Features

```python
from aerodb import create_client

aerodb = create_client(
    url='https://your-project.aerodb.io',
    api_key='your-api-key'
)

# Authentication
user = aerodb.auth.sign_up(email='user@example.com', password='password123')
session = aerodb.auth.sign_in(email='user@example.com', password='password123')

# Database queries
data = aerodb.table('posts').select('*').eq('author_id', user.id).execute()

# Storage
aerodb.storage.from_('avatars').upload('user1.png', file)
```

---

### 14.3 Go SDK (Future)

**Package:** `github.com/aerodb/aerodb-go`

---

## Phase 15: Managed Hosting & Multi-Tenancy

**Priority:** MEDIUM (for cloud offering)  
**Estimated Effort:** 6-12 months  
**Philosophy Alignment:** LOW (requires significant operational automation)

### 15.1 Multi-Tenant Architecture

#### Isolation Models

**Option 1: Database-Per-Tenant**

- Each project gets dedicated AeroDB instance
- Strong isolation (no shared data)
- Higher resource cost

**Option 2: Schema-Per-Tenant**

- Shared AeroDB instance, separate schemas per tenant
- Lower resource cost
- Requires RLS enforcement

**Recommendation:** Start with Database-Per-Tenant for simplicity and security.

#### Features

##### 15.1.1 Project Management

- Create/delete projects
- Project provisioning (spin up new AeroDB instance)
- Project isolation (network, filesystem, process)

##### 15.1.2 Resource Limits

- Connection limits per project
- Storage quotas
- Bandwidth limits
- Rate limiting

##### 15.1.3 Billing & Metering

- Usage tracking (storage, bandwidth, API calls)
- Tiered pricing (free, pro, enterprise)
- Billing integration (Stripe, etc.)

#### Infrastructure

**Containerization:**

- Docker containers for each AeroDB instance
- Orchestration via Kubernetes or Docker Swarm
- Auto-scaling (with limits per project)

**Networking:**

- Reverse proxy (Nginx or Tracie)
- TLS termination
- Project-specific subdomains (`project-id.aerodb.io`)

**Monitoring:**

- Prometheus for metrics
- Grafana for dashboards
- Loki for log aggregation

---

## Phase 16: Developer Tools

**Priority:** MEDIUM  
**Estimated Effort:** 2-3 months  

### 16.1 CLI Enhancements

**Package:** `aerodb-cli` (extend existing CLI)

#### New Commands

```bash
# Project management
aerodb init                    # Initialize new project
aerodb link                    # Link to remote project
aerodb status                  # Show project status

# Migrations
aerodb migrations new          # Create new migration
aerodb migrations up           # Apply migrations
aerodb migrations down         # Rollback migrations

# Functions
aerodb functions deploy        # Deploy edge functions
aerodb functions invoke        # Test function locally

# Database
aerodb db pull                 # Pull schema from remote
aerodb db push                 # Push local schema to remote
aerodb db seed                 # Run seed data

# Development
aerodb start                   # Start local dev environment
aerodb logs                    # Tail logs
```

---

### 16.2 Local Development Environment

**Package:** `aerodb-dev`

#### Features

- Local AeroDB instance (via Docker or binary)
- Hot-reload on schema changes
- Seed data management
- Local dashboard access

**Implementation:**

```bash
# Start local stack
aerodb start

# Starts:
# - AeroDB database (localhost:54321)
# - REST API (localhost:54322)
# - Dashboard (localhost:54323)
# - Realtime (localhost:54324)
```

---

## Summary Roadmap

### Phase Timeline

| Phase | Component | Priority | Effort | Completion |
|-------|-----------|----------|--------|------------|
| **Phase 8** | Authentication & RLS | 🔴 HIGHEST | 3-4 months | Q2 2026 |
| **Phase 9** | REST API Generator | 🔴 HIGH | 2-3 months | Q3 2026 |
| **Phase 10** | Real-Time Subscriptions | 🟡 MEDIUM | 3-4 months | Q4 2026 |
| **Phase 11** | File Storage | 🟡 MEDIUM | 2 months | Q4 2026 |
| **Phase 13** | Admin Dashboard | 🔴 HIGH | 4-6 months | Q1 2027 |
| **Phase 14** | Client SDKs (JS/TS) | 🔴 HIGH | 3-4 months | Q2 2027 |
| **Phase 14.2** | Python SDK | 🟡 MEDIUM | 2 months | Q3 2027 |
| **Phase 12** | Edge Functions | 🟢 LOW | 3-4 months | Q3 2027 |
| **Phase 15** | Managed Hosting | 🟡 MEDIUM | 6-12 months | Q4 2027 |
| **Phase 16** | Developer Tools | 🟡 MEDIUM | 2-3 months | Q1 2028 |

### Minimum Viable BaaS (MVB)

To compete with Supabase, you need **at minimum**:

1. ✅ **Phase 8:** Authentication & RLS
2. ✅ **Phase 9:** REST API Generator
3. ✅ **Phase 13:** Admin Dashboard
4. ✅ **Phase 14.1:** JavaScript SDK

**Timeframe:** ~12-15 months for MVB

---

## Philosophy Preservation Strategies

### Core Principle: BaaS as Optional Layer

```
┌─────────────────────────────────────┐
│   BaaS Layer (Phases 8-16)          │  ← Convenience, developer velocity
│   - Auth, REST API, Real-time       │  ← Can be disabled entirely
│   - Admin UI, SDKs, Functions       │
├─────────────────────────────────────┤
│   Correctness Kernel (Phases 0-7)   │  ← Frozen, deterministic
│   - WAL, MVCC, Replication          │  ← No compromises
│   - Control Plane, Observability    │
└─────────────────────────────────────┘
```

### Explicit Opt-In Model

- BaaS features are **disabled by default**
- Explicit configuration to enable: `baas.enabled = true`
- Each BaaS component can be enabled/disabled independently

### Determinism Boundaries

**Deterministic (Kernel):**
- Database operations (CRUD, queries)
- Replication and promotion
- Recovery and crash handling

**Non-Deterministic (BaaS):**
- Real-time event delivery timing
- Edge function execution timing
- HTTP request ordering

**Separated:** Clear documentation of determinism boundaries.

### Fail-Closed Enforcement

- Invalid auth → 401 (never allow)
- Unbounded queries → 400 (never execute)
- RLS violations → 403 (never bypass)

---

## Development Strategy

### 1. Specification-First (AeroDB Way)

Before implementing each phase:

1. Write normative specification (e.g., `PHASE8_AUTH_SPEC.md`)
2. Define invariants (e.g., `AUTH-1: Passwords never logged`)
3. Get review and approval
4. **Then** implement

### 2. Separate Repository Structure

**Option A:** Monorepo

```
aerodb/
├── core/           # Phases 0-7 (frozen)
├── baas/           # Phases 8+ (active development)
│   ├── auth/
│   ├── rest_api/
│   ├── realtime/
│   └── ...
└── sdks/           # Client SDKs
```

**Option B:** Separate Repos

- `aerodb/aerodb` - Core database (Phases 0-7)
- `aerodb/aerodb-baas` - BaaS layer (Phases 8+)
- `aerodb/aerodb-js` - JavaScript SDK

**Recommendation:** Monorepo for easier development, clear phase boundaries internally.

### 3. Testing Strategy

**BaaS Tests Must:**

- Not weaken core invariants
- Be separate from core tests
- Include integration tests with mock clients
- Verify RLS enforcement

---

## Critical Decisions Required

### Decision 1: Philosophy Trade-Off

**Question:** How much "magic" is acceptable?

**Options:**

A. **Strict Mode (Default):**
   - No auto-generated APIs (explicit endpoint registration)
   - No auto-migrations (explicit schema changes)
   - No auto-scaling (explicit resource allocation)
   - **Pro:** Preserves AeroDB philosophy
   - **Con:** Higher developer friction

B. **Convenience Mode (Opt-In):**
   - Auto-generated APIs from schema
   - Auto-apply migrations on deploy
   - Auto-scale within limits
   - **Pro:** Better developer experience
   - **Con:** Deviates from AeroDB philosophy

**Recommendation:** **Hybrid Approach**
- Default: Strict mode (explicit everything)
- Opt-in: Convenience mode (via `baas.auto_generate = true`)
- Document trade-offs clearly

---

### Decision 2: RLS Implementation

**Question:** How to implement Row-Level Security?

**Options:**

A. **Query Injection (Recommended):**
   - Inject filters at query planning stage
   - Preserves deterministic planner
   - Transparent to users

B. **Middleware Layer:**
   - Enforce RLS in REST API layer
   - Simpler to implement
   - Less secure (can bypass via direct DB access)

**Recommendation:** Query Injection (Option A) for security.

---

### Decision 3: Real-Time Architecture

**Question:** How to reconcile real-time with determinism?

**Options:**

A. **Event Log + Non-Deterministic Delivery:**
   - Event log is deterministic (derived from WAL)
   - Delivery is non-deterministic (WebSocket timing)
   - **Pro:** Preserves core determinism
   - **Con:** Complex architecture

B. **Polling-Based (No WebSocket):**
   - Clients poll for changes
   - Fully deterministic
   - **Pro:** Simpler, deterministic
   - **Con:** Higher latency, more resource usage

**Recommendation:** Option A (Event Log) for real-time UX.

---

## Immediate Next Steps

### Step 1: Resolve Phase 6 Blockers

Before starting BaaS work, **must resolve**:

1. ❌ Durable authority marker (Phase 6 blocker)
2. ❌ Disk-level crash tests (Phase 6 blocker)

**Timeframe:** 2-4 weeks

---

### Step 2: Write Phase 8 Specification

Create authoritative specification documents:

1. `docs/PHASE8_AUTH_VISION.md`
2. `docs/PHASE8_AUTH_ARCHITECTURE.md`
3. `docs/PHASE8_RLS_MODEL.md`
4. `docs/PHASE8_INVARIANTS.md`

**Timeframe:** 2-3 weeks

---

### Step 3: Prototype Authentication

Build minimal auth prototype:

- User registration and login
- JWT token generation
- Session management
- Integration with existing CLI

**Timeframe:** 4-6 weeks

---

### Step 4: Community Feedback

- Share roadmap with potential users
- Gather feedback on priority and features
- Adjust roadmap based on feedback

---

## Conclusion

Transforming AeroDB into a BaaS platform is **feasible** but requires careful planning to preserve AeroDB's core philosophy. The key is to treat BaaS features as an **optional layer** on top of the correctness kernel, with clear separation and explicit opt-in.

**Total Timeframe:** 18-24 months for full-featured BaaS platform  
**Minimum Viable BaaS:** 12-15 months (Auth, REST API, Dashboard, JS SDK)

**Critical Success Factors:**

1. ✅ Maintain phase-based development discipline
2. ✅ Write specifications before implementation
3. ✅ Preserve core determinism invariants
4. ✅ Clear documentation of determinism boundaries
5. ✅ Explicit opt-in for all BaaS features

**Recommendation:** Start with **Phase 8 (Authentication)** after resolving Phase 6 blockers. This is the foundation for all other BaaS features and has the highest ROI.

---

**Document Version:** 1.0  
**Author:** Antigravity AI  
**Date:** 2026-02-06
