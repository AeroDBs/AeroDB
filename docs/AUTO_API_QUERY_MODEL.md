# Phase 9: Query Model

**Document Type:** Technical Specification  
**Phase:** 9 - Auto-Generated REST API  
**Status:** Active

---

## Overview

This document specifies how REST query parameters translate to AeroDB AST.

---

## Query Parameter Syntax

### Filtering

| Syntax | Operator | Example |
|--------|----------|---------|
| `field=eq.value` | Equals | `status=eq.active` |
| `field=neq.value` | Not equals | `status=neq.deleted` |
| `field=gt.value` | Greater than | `age=gt.18` |
| `field=gte.value` | Greater or equal | `age=gte.18` |
| `field=lt.value` | Less than | `price=lt.100` |
| `field=lte.value` | Less or equal | `price=lte.100` |
| `field=like.pattern` | Pattern match | `name=like.*son` |
| `field=in.(a,b,c)` | In list | `status=in.(active,pending)` |
| `field=is.null` | Is null | `deleted_at=is.null` |

### Sorting

```
?order=field.asc
?order=field.desc
?order=created_at.desc,name.asc
```

### Pagination

```
?limit=20
?offset=40
```

### Field Selection

```
?select=id,name,email
?select=*
```

---

## Translation Pipeline

```
Query String → QueryParams → FilterSet → AeroDB AST
```

### Example

```
GET /rest/v1/posts?author_id=eq.123&status=in.(draft,published)&order=created_at.desc&limit=10
```

Translates to:

```rust
QueryParams {
    select: None,  // All fields
    filters: vec![
        FilterExpr { field: "author_id", op: Eq, value: "123" },
        FilterExpr { field: "status", op: In, value: ["draft", "published"] },
    ],
    order: vec![
        OrderBy { field: "created_at", ascending: false },
    ],
    limit: 10,
    offset: 0,
}
```

---

## Bounds Enforcement

### Invariant Q1: All queries must be bounded

| Condition | Action |
|-----------|--------|
| No limit specified | Apply DEFAULT_LIMIT (100) |
| Limit > MAX_LIMIT (1000) | Return 400 error |
| Limit = 0 | Return empty result |

---

## RLS Filter Injection

Before query execution, RLS filter is injected:

```rust
// Original filters
filters: [status=eq.active]

// After RLS injection (ownership policy)
filters: [status=eq.active, owner_id=eq.{user_id}]
```

This is transparent to the client.
