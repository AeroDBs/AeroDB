# Phase 9: Schema Model

**Document Type:** Technical Specification  
**Phase:** 9 - Auto-Generated REST API  
**Status:** Active

---

## Overview

This document specifies how schema introspection generates REST endpoints.

---

## Schema Introspection

### Source

Schemas read from AeroDB `schemas` collection at startup.

### Schema Structure

```json
{
  "name": "posts",
  "fields": [
    { "name": "id", "type": "uuid", "primary": true },
    { "name": "title", "type": "string", "required": true },
    { "name": "author_id", "type": "uuid", "required": true },
    { "name": "created_at", "type": "datetime" }
  ],
  "rls_policy": {
    "type": "ownership",
    "owner_field": "author_id"
  }
}
```

---

## Endpoint Generation

### Per-Schema Endpoints

For schema `posts`:

| Method | Endpoint | Handler |
|--------|----------|---------|
| GET | `/rest/v1/posts` | list |
| GET | `/rest/v1/posts/{id}` | get |
| POST | `/rest/v1/posts` | insert |
| PATCH | `/rest/v1/posts/{id}` | update |
| DELETE | `/rest/v1/posts/{id}` | delete |

### Endpoint Registry

```rust
pub struct EndpointRegistry {
    endpoints: HashMap<String, SchemaEndpoint>,
}

pub struct SchemaEndpoint {
    collection: String,
    fields: Vec<FieldDef>,
    rls_policy: RlsPolicy,
}
```

---

## Reload Strategy

### Explicit Reload (No Auto-Watch)

```bash
POST /admin/reload-schemas
```

Triggers:
1. Re-read schemas from DB
2. Rebuild endpoint registry
3. Return success/failure

### Invariant

Schema changes do NOT auto-apply. Explicit reload required.

---

## Validation

### Field Type Validation

| Type | JSON Type | Validation |
|------|-----------|------------|
| uuid | string | UUID format |
| string | string | Max length |
| number | number | Range |
| boolean | boolean | - |
| datetime | string | ISO 8601 |
| json | object/array | Valid JSON |

### Required Fields

On insert, required fields must be present or error 400.
