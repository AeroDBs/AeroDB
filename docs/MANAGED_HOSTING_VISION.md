# Phase 15: Managed Hosting & Multi-Tenancy  - Vision

## Purpose

Provide **managed AeroDB hosting** with **multi-tenant isolation**, enabling SaaS applications to scale from prototype to production without infrastructure concerns.

## Philosophy

### Control Plane for Tenant Lifecycle

Managed hosting introduces a **control plane** that orchestrates tenant databases:
- **Provisioning**: Create isolated database instances per tenant
- **Scaling**: Auto-scale resources based on usage
- **Billing**: Track usage, enforce quotas
- **Upgrades**: Rolling updates without downtime

### Isolation Models

Three isolation strategies:

1. **Schema-per-Tenant** (Shared Database)
   - All tenants in one DB, separate schemas (`tenant_123.users`)
   - Lowest cost, highest density
   - RLS ensures data isolation

2. **Database-per-Tenant** (Dedicated Database)
   - Each tenant gets isolated database process
   - Higher cost, stronger isolation
   - Suitable for compliance (HIPAA, SOC2)

3. **Cluster-per-Tenant** (Enterprise)
   - Dedicated cluster (authority + replicas)
   - Highest cost, maximum isolation
   - For high-volume or regulated workloads

---

## Core Capabilities

### 1. Tenant Provisioning

API to create new tenants:

```bash
curl -X POST https://control.aerodb.com/v1/tenants \
  -H "Authorization: Bearer PLATFORM_KEY" \
  -d '{"name": "acme-corp", "plan": "pro", "region": "us-east-1"}'
```

Response:
```json
{
  "tenant_id": "ten_abc123",
  "database_url": "https://acme-corp.aerodb.com",
  "created_at": "2024-08-15T12:00:00Z"
}
```

### 2. Resource Quotas

Enforce limits per plan:

| Resource       | Free   | Pro      | Enterprise |
|----------------|--------|----------|------------|
| Storage        | 500 MB | 100 GB   | Unlimited  |
| API Requests   | 10k/mo | 1M/mo    | Unlimited  |
| File Storage   | 1 GB   | 100 GB   | Unlimited  |
| Realtime Conns | 100    | 10,000   | Unlimited  |

Quota enforcement at **database level** (Phase 7 Control Plane integration).

### 3. Metrics & Billing

Track usage per tenant:
- API requests (count, latency)
- Storage used (documents, files)
- Egress bandwidth
- Real-time connections

Billing calculated monthly, exposed via admin API.

### 4. Tenant Management Dashboard

SaaS providers get admin panel:
- List all tenants
- Create/delete tenants
- View usage metrics
- Manage quotas

---

## Isolation Guarantees

### Schema-per-Tenant

**Invariant**: Tenant A **cannot read/write** Tenant B's data.

**Enforcement**:
- RLS policies: `WHERE tenant_id = current_tenant()`
- Connection pool tagged with `tenant_id`
- Middleware injects `SET LOCAL app.tenant_id = 'ten_123'`

### Database-per-Tenant

**Invariant**: Tenant A's database **is not aware** of Tenant B's existence.

**Enforcement**:
- Separate Postgres processes
- No shared tables, no foreign keys across tenants
- Network isolation (VPC per tenant)

### Cluster-per-Tenant

**Invariant**: Tenant A's cluster **does not share hardware** with Tenant B.

**Enforcement**:
- Dedicated VM instances
- No multi-tenancy in compute layer

---

## Deployment Architecture

```
┌──────────────────────────────────────────────────────────┐
│              Control Plane (Global)                      │
│  ┌──────────────┐  ┌────────────────┐  ┌──────────────┐ │
│  │ Provisioning │  │ Billing API    │  │ Metrics API  │ │
│  └──────────────┘  └────────────────┘  └──────────────┘ │
└──────────────────────┬───────────────────────────────────┘
                       │
         ┌─────────────┼─────────────┐
         ↓             ↓             ↓
┌────────────┐  ┌────────────┐  ┌────────────┐
│ Tenant A   │  │ Tenant B   │  │ Tenant C   │
│ (Schema)   │  │ (Database) │  │ (Cluster)  │
│            │  │            │  │            │
│ aerodb.com │  │ acme.aero  │  │ bigco.aero │
└────────────┘  └────────────┘  └────────────┘
```

---

## Success Criteria

Managed hosting is successful if:

1. **Tenant creation < 30 seconds** (schema-per-tenant)
2. **Zero cross-tenant data leakage** (100% RLS coverage)
3. **99.9% uptime SLA** (per tenant)
4. **Cost-efficient**: < $10/month per tenant (schema-per-tenant, low traffic)

---

## Non-Goals

- **Multi-region active-active**: Single region per tenant (Phase 2 replication is single-leader)
- **Automatic backups**: Users must configure backup schedules (Phase 1 backup/restore)
- **Custom domains**: Tenants use `tenant-name.aerodb.com`

---

## Prior Art

Inspired by:
- **Supabase Hosted**: Database-per-project, auto-pause on idle
- **PlanetScale**: Schema-per-tenant, branching for dev/prod
- **Heroku Postgres**: Managed Postgres with add-ons
- **Neon**: Serverless Postgres with instant provisioning

Differentiator: **Built-in multi-tenancy** - RLS + control plane are integrated.
