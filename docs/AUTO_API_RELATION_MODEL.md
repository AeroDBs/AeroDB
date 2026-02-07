# Phase 9: Relation Model

**Document Type:** Technical Specification  
**Phase:** 9 - Auto-Generated REST API  
**Status:** Deferred

---

## Overview

This document specifies foreign key expansion and embedded relations (future feature).

---

## Relation Types

### Embedded Relations

```
GET /rest/v1/posts?select=*,author(*)
```

Returns:

```json
{
  "id": "post-1",
  "title": "Hello",
  "author": {
    "id": "user-1",
    "name": "Alice"
  }
}
```

### Many-to-One

```
posts.author_id → users.id
```

### One-to-Many

```
users.id ← posts.author_id
```

---

## Syntax

### Select with Relations

```
?select=*,author(id,name)
?select=*,author(*),comments(*)
```

### Nested Relations

```
?select=*,author(id,company(*))
```

---

## Implementation Status

**Status: DEFERRED**

This feature requires:
1. Foreign key metadata in schemas
2. Query planning for joins
3. N+1 query prevention

Will be implemented in future iteration.

---

## Current Behavior

Relations in select are **ignored** silently.

```
?select=*,author(*)
# Returns: * fields only, author ignored
```
