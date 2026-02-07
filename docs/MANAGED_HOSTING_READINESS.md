# Phase 15: Managed Hosting & Multi-Tenancy - Readiness Criteria

## Freeze Checklist

Phase 15 is ready to freeze when **all items below** are complete:

---

## 1. Control Plane API

- [ ] **Tenant Management**
  - [ ] POST /v1/tenants (create tenant)
  - [ ] GET /v1/tenants (list tenants)
  - [ ] GET /v1/tenants/{id} (get tenant details)
  - [ ] DELETE /v1/tenants/{id} (delete tenant)
  - [ ] PATCH /v1/tenants/{id} (update tenant config)

- [ ] **Provisioning**
  - [ ] Schema-per-tenant: < 5 seconds
  - [ ] Database-per-tenant: < 30 seconds
  - [ ] Cluster-per-tenant: < 5 minutes
  - [ ] Automatic DNS setup (tenant-name.aerodb.com)

- [ ] **Quota Enforcement**
  - [ ] Storage limits (reject writes if exceeded)
  - [ ] API request limits (rate limiting per tenant)
  - [ ] File storage limits
  - [ ] Real-time connection limits

---

## 2. Isolation Mechanisms

- [ ] **Schema-per-Tenant**
  - [ ] RLS policy: `tenant_id = current_setting('app.tenant_id')`
  - [ ] Connection pools tagged with tenant_id
  - [ ] Middleware sets `app.tenant_id` on every request
  - [ ] Zero cross-tenant data leakage (verified by audit)

- [ ] **Database-per-Tenant**
  - [ ] Separate Postgres processes
  - [ ] VPC isolation (if cloud-hosted)
  - [ ] No shared tables

- [ ] **Cluster-per-Tenant**
  - [ ] Dedicated VMs/containers
  - [ ] Separate replication cluster
  - [ ] No shared hardware

---

## 3. Billing & Metering

- [ ] **Usage Tracking**
  - [ ] API requests (count, endpoint, status code)
  - [ ] Storage used (documents, file storage)
  - [ ] Egress bandwidth
  - [ ] Real-time connections (peak, average)

- [ ] **Billing API**
  - [ ] GET /v1/tenants/{id}/usage (current month)
  - [ ] GET /v1/tenants/{id}/usage/{month} (historical)
  - [ ] Breakdown by resource type

- [ ] **Invoice Generation**
  - [ ] Monthly invoices (CSV, JSON, PDF)
  - [ ] Stripe integration (optional)

---

## 4. Admin Dashboard Integration

- [ ] **Tenant List**
  - [ ] Table with name, plan, created_at, storage_used
  - [ ] Search by name
  - [ ] Filter by plan (Free, Pro, Enterprise)

- [ ] **Tenant Details**
  - [ ] Current usage metrics
  - [ ] Quota limits
  - [ ] Edit quota button
  - [ ] Delete tenant (with confirmation)

- [ ] **Usage Charts**
  - [ ] API requests over time
  - [ ] Storage growth
  - [ ] Real-time connections

---

## 5. Testing

- [ ] **Unit Tests**
  - [ ] Tenant provisioning logic
  - [ ] Quota enforcement (storage, API requests)
  - [ ] RLS policy generation

- [ ] **Integration Tests**
  - [ ] Create schema-per-tenant → insert data → verify isolation
  - [ ] Create database-per-tenant → verify separate processes
  - [ ] Exceed quota → verify rejection

- [ ] **Load Tests**
  - [ ] 1000 tenants (schema-per-tenant)
  - [ ] 100 tenants (database-per-tenant)
  - [ ] API latency < 100ms (p95) with multi-tenancy

---

## 6. Security

- [ ] **Tenant Isolation Audit**
  - [ ] Penetration test: Tenant A cannot read Tenant B's data
  - [ ] SQL injection tests (parameterized queries only)
  - [ ] RLS bypass tests (no `SECURITY DEFINER` functions)

- [ ] **API Authentication**
  - [ ] Platform key required for control plane API
  - [ ] Tenants cannot access other tenants' data
  - [ ] Admin users cannot access tenant data without permission

---

## 7. Performance

- [ ] **Provisioning Time**
  - [ ] Schema-per-tenant: < 5s
  - [ ] Database-per-tenant: < 30s

- [ ] **Query Latency**
  - [ ] Schema-per-tenant: No overhead vs single-tenant
  - [ ] Database-per-tenant: <10ms overhead

---

## 8. Documentation

- [ ] **Managed Hosting Guide**
  - [ ] How to create a tenant
  - [ ] Isolation models comparison
  - [ ] Pricing calculator

- [ ] **API Reference**
  - [ ] Control Plane API (Tenant CRUD)
  - [ ] Billing API
  - [ ] Examples (curl, JS, Python)

---

## 9. SLA & Uptime

- [ ] **Monitoring**
  - [ ] Healthcheck per tenant (/health endpoint)
  - [ ] Alert if tenant unreachable for > 1 minute
  - [ ] Alert if quota exceeded

- [ ] **SLA**
  - [ ] 99.9% uptime guarantee (Free tier: best effort)
  - [ ] Pro/Enterprise: < 5 min downtime/month

---

## Sign-Off

Phase 15 is **frozen** when:

1. All checklist items complete
2. Multi-tenancy tested with 100+ tenants
3. Zero cross-tenant data leakage
4. Billing API functional
5. Control Plane API documented

**Frozen on**: [DATE]

**Approved by**: [NAME]
