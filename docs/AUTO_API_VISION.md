# Phase 9: REST API Vision

**Document Type:** Vision Statement  
**Phase:** 9 - Auto-Generated REST API  
**Status:** Active

---

## Goal

Provide automatic REST API generation from database schema, inspired by PostgREST, while preserving AeroDB's determinism and explicit control principles.

---

## Philosophy

### Core Principles

1. **Schema-Driven:** API endpoints derived from schema, not hand-coded
2. **Deterministic:** Same query params → Same results
3. **Bounded:** All queries must be bounded (no unbounded scans)
4. **Secure:** RLS enforcement at query level, not middleware
5. **Explicit:** No hidden magic, predictable behavior

### Non-Goals

- GraphQL (Phase 17+)
- Auto-migrations on schema change
- Automatic relationship detection

---

## Target Experience

```bash
# After creating a schema, REST endpoints are available immediately
POST /rest/v1/posts
GET /rest/v1/posts?author_id=eq.123&order=created_at.desc&limit=10
```

---

## Success Criteria

1. Zero-config REST API from any schema
2. Full CRUD operations
3. PostgREST-compatible query syntax
4. RLS enforcement on all endpoints
5. <10ms overhead vs direct queries
