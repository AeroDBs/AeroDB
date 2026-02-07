# Phase 13: Admin Dashboard - Vision

## Purpose

The AeroDB Admin Dashboard provides **observability without semantic authority**. It is a read-heavy interface for monitoring, managing, and understanding the state of an AeroDB instance, while preserving the core principle that the database (not the UI) is the source of truth.

## Philosophy

### Observability, Not Control

The dashboard is explicitly **non-authoritative**:
- **Dashboard does not mutate core state** - Database modifications happen via REST API or direct CLI
- **Dashboard reflects reality** - It reads current state but doesn't define what that state should be
- **Dashboard can lag** - Eventual consistency is acceptable; real-time accuracy is not required
- **Dashboard can be wrong** - If UI and DB disagree, DB wins

### User Personas

1. **Database Administrators**: Monitor health, view schemas, inspect replication lag
2. **Developers**: Browse data, test queries, view logs, debug issues
3. **Platform Operators**: Manage users, view metrics, configure policies

## Core Tenets

### 1. Read-Heavy, Write-Light

The dashboard primarily **reads** state:
- View collections and schemas
- Browse data with pagination
- Inspect active sessions
- Monitor file storage usage
- View real-time subscriptions

Writes are limited to non-critical operations:
- Schema editor (generates migration SQL)
- User management (calls auth API)
- Policy configuration (updates via REST API)

### 2. No Hidden Mutations

Any write operation must:
- Be explicitly initiated by the user (button click)
- Show confirmation dialog for destructive actions
- Display the underlying API call being made
- Provide rollback instructions where applicable

### 3. Fail-Open for UI

Dashboard failures **never block database operations**:
- If dashboard crashes, database continues running
- If dashboard shows stale data, queries still execute correctly
- If dashboard denies access, API still accepts valid tokens

### 4. Technology Agnosticism

The dashboard communicates via **public APIs only**:
- REST API for data operations
- Control Plane API for cluster state
- Auth API for user management
- WebSocket API for real-time updates

No direct database access, no privileged backdoors.

---

## Feature Categories

### 1. Data Management
- **Table Browser**: Paginated view of collections with filters
- **SQL Console**: Execute queries, view results, see explain plans
- **Schema Editor**: Visual schema designer (generates migration SQL)
- **Import/Export**: Bulk data operations via REST API

### 2. Authentication & Authorization
- **User List**: View all users, their roles, last login
- **Session Monitor**: Active sessions, token expiry times
- **RLS Policy Viewer**: Inspect configured RLS rules per collection

### 3. File Storage
- **Bucket Explorer**: Browse buckets and objects
- **Upload Interface**: Drag-and-drop file upload
- **Storage Metrics**: Usage by bucket, quota warnings

### 4. Real-Time Monitoring
- **Active Subscriptions**: Who is subscribed to what
- **Event Throughput**: Events published per second
- **Connection List**: WebSocket connections, heartbeat status

### 5. Cluster Management (Phase 7 Integration)
- **Topology View**: Authority, replicas, replication lag
- **Promotion Controls**: Trigger failover (with confirmation)
- **WAL Viewer**: Inspect WAL entries, sequence numbers

### 6. Observability
- **Structured Logs**: Filter by level, timestamp, module
- **Metrics Dashboard**: Queries/sec, latency percentiles, error rate
- **Audit Log**: User actions, API calls, schema changes

---

## Design Principles

### Minimize Friction

- **Fast page loads**: Client-side rendering, code splitting
- **Responsive queries**: Pagination always required, limits enforced
- **Progressive disclosure**: Show summaries first, details on demand

### Explicit Over Implicit

- **No auto-refresh by default**: User controls when data refreshes
- **Clear timestamps**: All data shows "as of [timestamp]"
- **Visible staleness**: If data is cached, show cache age

### Graceful Degradation

- **Offline mode**: Cache last known state, show warning banner
- **Partial failures**: If metrics fail, still show data table
- **Network errors**: Retry with exponential backoff, show error state

---

## Non-Goals

The dashboard explicitly **does not**:

1. **Replace the CLI**: Bulk operations, migrations use CLI
2. **Enforce invariants**: Database enforces correctness, UI just reflects it
3. **Provide granular ACLs**: RLS is in the database, not the dashboard
4. **Store its own state**: No dashboard-specific database, all state is derived

---

## Security Model

### Authentication

- Must provide valid JWT (obtained from `/auth/login`)
- Service role token allows admin access
- User tokens see only RLS-filtered data

### RBAC (Future)

- `admin`: Full read/write access to all collections
- `developer`: Read-only access to data, can execute queries
- `viewer`: Read-only access to metrics and logs

### Audit Trail

Every dashboard action is logged:
- Timestamp, user ID, action type
- Affected resource (collection, bucket, user)
- Result (success/failure)

---

## Success Criteria

The dashboard is successful if:

1. **Self-explanatory**: New users understand data model within 5 minutes
2. **Fast enough**: Page load < 1s, query results < 500ms (p95)
3. **Non-intrusive**: Dashboard bugs don't affect database uptime
4. **API-aligned**: All features use public REST/Control Plane APIs

---

## Prior Art

Inspired by:
- **Supabase Dashboard**: Clean UI, table editor, SQL console
- **Postgres Admin Tools**: pgAdmin, TablePlus (schema visualization)
- **Grafana**: Metrics dashboards, time-series charts
- **Prisma Studio**: Relation-aware data browser

Differentiator: **Explicit non-authority** - Dashboard is a client, not a privileged layer.
