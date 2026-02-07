# Phase 14: Client SDKs - Readiness Criteria

## Freeze Checklist

Phase 14 is ready to freeze when **all items below** are complete:

---

## 1. JavaScript/TypeScript SDK (@aerodb/client)

- [ ] **Core Features**
  - [ ] AeroDBClient class with all sub-clients
  - [ ] AuthClient (signUp, signIn, signOut, getUser, onAuthStateChange)
  - [ ] QueryBuilder (select, eq, neq, gt, gte, lt, lte, like, order, limit, offset)
  - [ ] PostgrestClient (from method)
  - [ ] RealtimeClient (channel, subscribe, unsubscribe)
  - [ ] RealtimeChannel (on, off event handlers)
  - [ ] StorageClient (from, upload, download, delete)
  - [ ] FunctionsClient (invoke)

- [ ] **Type Safety**
  - [ ] Full TypeScript types for all methods
  - [ ] Generic type parameter for `from<T>()`
  - [ ] Union types for filter operators
  - [ ] Result type (`{ data, error }`) for all async methods

- [ ] **Error Handling**
  - [ ] No exceptions thrown (all use Result type)
  - [ ] AeroDBError interface with message, status, code
  - [ ] Network errors caught and returned as error
  - [ ] 401 triggers token refresh automatically

- [ ] **Build**
  - [ ] ESM and CJS outputs (`dist/index.js`, `dist/index.cjs`)
  - [ ] TypeScript declarations (`dist/index.d.ts`)
  - [ ] Tree-shakeable (side-effect free)
  - [ ] Bundle size < 50KB (gzipped)

---

## 2. Python SDK (aerodb-py)

- [ ] **Core Features**
  - [ ] `AeroDBClient` class
  - [ ] `AuthClient` (sign_up, sign_in, sign_out, get_user)
  - [ ] `QueryBuilder` (select, eq, neq, gt, gte, lt, lte, like, order, limit, offset)
  - [ ] `PostgrestClient` (from_ method, avoid keyword clash)
  - [ ] `RealtimeClient` (async WebSocket)
  - [ ] `StorageClient` (upload, download, delete)
  - [ ] `FunctionsClient` (invoke)

- [ ] **Type Hints**
  - [ ] All methods have type annotations
  - [ ] Generic `TypeVar` for `from_[T]()`
  - [ ] Result type: `AeroDBResponse[T]`
  - [ ] mypy passes with strict mode

- [ ] **Async Support**
  - [ ] All I/O methods are `async def`
  - [ ] Uses `aiohttp` for HTTP
  - [ ] Works with asyncio event loop

- [ ] **Pandas Integration**
  - [ ] `to_dataframe()` method on query results
  - [ ] Automatic type inference (str, int, datetime)

---

## 3. Testing

- [ ] **Unit Tests (JS/TS)**
  - [ ] QueryBuilder (all filter methods)
  - [ ] AuthClient (signIn, signOut)
  - [ ] RealtimeChannel (subscribe, on)
  - [ ] 90%+ line coverage

- [ ] **Unit Tests (Python)**
  - [ ] QueryBuilder (all filter methods)
  - [ ] AuthClient (sign_in, sign_out)
  - [ ] async methods work with asyncio
  - [ ] 90%+ line coverage

- [ ] **Integration Tests**
  - [ ] Test against real AeroDB instance
  - [ ] CRUD operations (insert, select, update, delete)
  - [ ] Real-time subscriptions (receive events)
  - [ ] Storage uploads/downloads

- [ ] **Mock API Tests**
  - [ ] MSW for JS/TS (Mock Service Worker)
  - [ ] `responses` library for Python
  - [ ] All endpoints mocked

---

## 4. Documentation

- [ ] **README.md**
  - [ ] Installation instructions
  - [ ] Quick start example
  - [ ] API reference (brief)
  - [ ] Links to full docs

- [ ] **API Documentation**
  - [ ] TSDoc comments for all public methods (JS/TS)
  - [ ] Docstrings for all public methods (Python)
  - [ ] Generated docs (TypeDoc for JS, Sphinx for Python)

- [ ] **Examples**
  - [ ] CRUD operations
  - [ ] Authentication flow
  - [ ] Real-time subscriptions
  - [ ] File uploads

- [ ] **Migration Guide**
  - [ ] From raw fetch() to SDK
  - [ ] Breaking changes between versions

---

## 5. Publishing

- [ ] **npm Package (@aerodb/client)**
  - [ ] Published to https://www.npmjs.com/package/@aerodb/client
  - [ ] Semantic versioning (1.0.0 for initial release)
  - [ ] `latest` tag points to stable version
  - [ ] `next` tag for beta releases

- [ ] **PyPI Package (aerodb-py)**
  - [ ] Published to https://pypi.org/project/aerodb-py/
  - [ ] Semantic versioning
  - [ ] Wheel and source distribution

- [ ] **GitHub Releases**
  - [ ] CHANGELOG.md kept up to date
  - [ ] GitHub release for each version
  - [ ] Tagged commits (v1.0.0, v1.1.0, etc.)

---

## 6. Security

- [ ] **Token Storage**
  - [ ] Access token in memory by default (not localStorage)
  - [ ] Refresh token in httpOnly cookie (if supported)
  - [ ] Tokens never logged to console/errors

- [ ] **HTTPS Enforcement**
  - [ ] SDK refuses to connect to http:// URLs (except localhost)
  - [ ] Warning if connecting to insecure endpoint

---

## 7. Performance

- [ ] **Bundle Size**
  - [ ] JS/TS: < 50KB gzipped
  - [ ] Tree-shakeable (unused exports removed)

- [ ] **Dependency Count**
  - [ ] JS/TS: Zero dependencies (use native fetch, WebSocket)
  - [ ] Python: Minimal dependencies (aiohttp, typing-extensions)

- [ ] **Request Overhead**
  - [ ] < 5ms overhead over raw fetch()

---

## 8. Backward Compatibility

- [ ] **Versioning Policy**
  - [ ] Breaking changes require major version bump
  - [ ] Deprecation warnings for 1 minor version before removal
  - [ ] Support for AeroDB API v1.x

---

## Sign-Off

Phase 14 is **frozen** when:

1. All checklist items complete
2. JS/TS SDK published to npm
3. Python SDK published to PyPI
4. All tests pass (unit + integration)
5. Documentation complete (README + API docs)

**Frozen on**: [DATE]

**Approved by**: [NAME]
