# Phase 9: REST API Architecture

**Document Type:** Technical Architecture  
**Phase:** 9 - Auto-Generated REST API  
**Status:** Active

---

## Overview

AeroREST is a PostgREST-inspired REST API layer that auto-generates endpoints from AeroDB schemas.

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                     HTTP Request                                 │
│              GET /rest/v1/posts?limit=10                        │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     RestServer (Axum)                            │
│  - Route matching                                                │
│  - JWT extraction → RlsContext                                   │
│  - Service role detection                                        │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     QueryParser                                  │
│  - Parse query params                                            │
│  - Build FilterSet                                               │
│  - Extract pagination                                            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     RestHandler                                  │
│  - Apply RLS filter                                              │
│  - Execute CRUD operation                                        │
│  - Format response                                               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     AeroDB Executor                              │
│  - Query planning                                                │
│  - MVCC read/write                                               │
│  - WAL logging                                                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## Module Structure

```
src/rest_api/
├── mod.rs           # Module entry, exports
├── errors.rs        # HTTP error codes
├── parser.rs        # Query parameter parsing
├── filter.rs        # Filter expression AST
├── response.rs      # JSON response formatting
├── handler.rs       # CRUD operations + RLS
├── server.rs        # Axum HTTP server
└── generator.rs     # Schema → endpoint mapping
```

---

## Request Flow

1. **Route Match:** `/rest/v1/{collection}` → handler
2. **Auth Extract:** JWT → RlsContext
3. **Parse Query:** Query params → QueryParams
4. **Apply RLS:** Inject ownership filter
5. **Execute:** CRUD via handler
6. **Format:** Records → JSON response

---

## Integration Points

| Component | Integration |
|-----------|-------------|
| Auth (Phase 8) | JWT validation, RlsContext |
| Planner | Query bounds checking |
| Executor | CRUD operations |
| WAL | Write durability |
