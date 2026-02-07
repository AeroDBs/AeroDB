# Phase 14: Client SDKs - Vision

## Purpose

Provide **language-specific SDKs** that abstract AeroDB's REST/WebSocket APIs into idiomatic, type-safe client libraries. SDKs enable developers to interact with AeroDB without manually crafting HTTP requests.

## Philosophy

### Thin Clients, Not ORMs

AeroDB SDKs are **thin wrappers** around the public APIs:
- **No business logic**: SDKs serialize/deserialize, validate, and execute HTTP requests
- **No schema generation**: Database schema is defined server-side, not in SDK
- **No query builders with infinite chaining**: Simple, predictable API surface

Contrast with ORMs (Prisma, Django ORM):
- ORMs define models client-side → AeroDB schemas are server-defined
- ORMs generate migrations → AeroDB migrations are SQL scripts
- ORMs abstract SQL → AeroDB exposes SQL-like query semantics

### API-First Design

SDKs **mirror the REST API structure**:
- `/rest/v1/users` → `client.from('users')`
- `/auth/login` → `client.auth.signIn()`
- `/storage/v1/object/{bucket}/{path}` → `client.storage.from(bucket).upload(path)`

No magic, no hidden behavior.

---

## Supported Languages

### Phase 14.1: JavaScript/TypeScript (@aerodb/client)

**Priority**: HIGHEST (web developers)

Features:
- TypeScript-first (types for all methods)
- ESM modules (tree-shakeable)
- Works in browser (Vite/Webpack) and Node.js
- WebSocket support for real-time

### Phase 14.2: Python (aerodb-py)

**Priority**: HIGH (data science, backend)

Features:
- Type hints (mypy compatible)
- Async/await support (asyncio)
- Works with Flask, FastAPI, Django
- Pandas integration (fetch → DataFrame)

### Future: Go, Rust, Dart

**Priority**: MEDIUM

Languages chosen based on user demand.

---

## Core Tenets

### 1. Type Safety

SDKs provide **full type inference**:

```typescript
// TypeScript SDK
const { data, error } = await client
  .from('users')
  .select('id, name, email')
  .eq('role', 'admin');

// `data` is typed as: { id: string; name: string; email: string }[]
// `error` is typed as: RestError | null
```

### 2. Explicit Error Handling

SDKs **never throw exceptions** (use Result type):

```typescript
// Good: Predictable
const { data, error } = await client.from('users').select('*');
if (error) {
  console.error(error.message);
} else {
  console.log(data);
}

// Bad: Hidden control flow
try {
  const data = await client.from('users').select('*'); // throws on error
} catch (error) {
  console.error(error);
}
```

Python equivalent:
```python
result = await client.from_('users').select('*').execute()
if result.error:
    print(result.error.message)
else:
    print(result.data)
```

### 3. Method Chaining (Fluent API)

Queries are built **declaratively**:

```typescript
const { data } = await client
  .from('posts')
  .select('title, author(*)')  // Embed relation
  .gte('created_at', '2024-01-01')
  .order('created_at', { ascending: false })
  .limit(10);
```

### 4. Real-Time Subscriptions

WebSocket API is **integrated seamlessly**:

```typescript
const channel = client.channel('posts')
  .on('INSERT', (payload) => {
    console.log('New post:', payload.new);
  })
  .subscribe();

// Later: unsubscribe
channel.unsubscribe();
```

---

## Non-Goals

The SDKs explicitly **do not**:

1. **Generate types from database schema**: Use code generation tools (e.g., `aerodb-codegen`) separately
2. **Manage migrations**: Use CLI (`aerodb migrations up`)
3. **Provide an ORM**: Use raw SQL for complex queries
4. **Abstract away HTTP**: Developers should understand it's just API calls
5. **Support offline-first**: No local cache, always hits server

---

## Feature Matrix

| Feature                  | JS/TS | Python | Go | Rust |
|--------------------------|-------|--------|----|------|
| Auth (signIn, signOut)   | ✅     | ✅      | 🔜 | 🔜   |
| CRUD (select, insert, etc.) | ✅     | ✅      | 🔜 | 🔜   |
| Real-Time (WebSocket)    | ✅     | ✅      | 🔜 | 🔜   |
| Storage (upload, download) | ✅     | ✅      | 🔜 | 🔜   |
| Functions (invoke)       | ✅     | ⚠️     | 🔜 | 🔜   |
| Typed Responses          | ✅     | ✅      | 🔜 | 🔜   |
| Error Handling (Result)  | ✅     | ✅      | 🔜 | 🔜   |

✅ = Implemented | ⚠️ = Partial | 🔜 = Planned

---

## Versioning Strategy

### Semver

SDKs follow semantic versioning:
- **Major**: Breaking API changes (e.g., rename `signIn` → `login`)
- **Minor**: New features (e.g., add `client.analytics`)
- **Patch**: Bug fixes

### API Version Compatibility

SDK version maps to AeroDB API version:
- `@aerodb/client@1.x` → `/rest/v1`, `/auth/v1`
- `@aerodb/client@2.x` → `/rest/v2` (future)

---

## Success Criteria

The SDKs are successful if:

1. **Zero config by default**: `const client = new AeroDBClient({ url: '...' })` is enough
2. **Self-documenting**: Type hints and autocomplete reveal API
3. **Thin**: SDK bundle size < 50KB (gzipped, JS/TS)
4. **Fast**: Minimal overhead (<5ms) over raw fetch()

---

## Prior Art

Inspired by:
- **Supabase JS**: Fluent API, real-time subscriptions
- **Prisma Client**: Type safety, result types
- **AWS SDK**: Service-oriented structure (client.auth, client.storage)
- **Firebase SDK**: Real-time listeners

Differentiator: **Explicit HTTP semantics** - SDKs are thin, no magic.
