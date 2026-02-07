# Phase 13: Admin Dashboard - Invariants

## Critical Invariants

### I1: Dashboard Cannot Break Database

**Invariant**: Dashboard failures **never** impact database availability or correctness.

**Rationale**: Dashboard is observability-only. Database operations must succeed regardless of dashboard state.

**Enforcement**:
- Dashboard runs in separate process/container from database
- Dashboard uses **public APIs only** (no direct DB access)
- Database does not wait for or depend on dashboard responses

**Test**: Kill dashboard process → database continues serving queries

---

### I2: Database is Source of Truth

**Invariant**: If dashboard and database disagree on state, **database wins**.

**Rationale**: Dashboard may cache stale data, but should never define authoritative state.

**Enforcement**:
- All writes go through REST/Auth APIs (which enforce invariants)
- Dashboard displays "as of [timestamp]" for cached data
- Refresh button always fetches latest from database

**Test**: Update data via CLI → dashboard shows stale data → refresh shows correct data

---

### I3: No Hidden Mutations

**Invariant**: Every write operation is **explicitly user-initiated** and **reversible** (or has confirmation).

**Rationale**: Users must understand what the dashboard is doing. No background mutations.

**Enforcement**:
- Destructive actions require confirmation dialog
- All mutations show API endpoint being called
- Read-only mode available (no write buttons shown)

**Test**: Click "Delete User" → confirmation dialog shows → cancel works

---

### I4: RLS Respected

**Invariant**: Dashboard **never bypasses RLS** unless using service role.

**Rationale**: Users should only see data they have permission to access.

**Enforcement**:
- User JWT passed to all API calls
- Service role token used only for admin pages (explicitly labeled)
- RLS policies enforced server-side (not in dashboard code)

**Test**: User A logs in → can see only their data → cannot see User B's data

---

### I5: API Version Compatibility

**Invariant**: Dashboard works with **any AeroDB version supporting the same API major version**.

**Rationale**: Dashboard and database can be upgraded independently.

**Enforcement**:
- Dashboard version check on startup (compares API version)
- Graceful degradation if API endpoint missing
- Feature flags for version-specific functionality

**Test**: Dashboard v1.2 + AeroDB API v1.0 → dashboard works, shows "upgrade for feature X"

---

## Operational Invariants

### O1: Pagination Always Required

**Invariant**: List endpoints **must** specify limit (dashboard never fetches unbounded data).

**Rationale**: Prevents OOM errors, aligns with AeroDB Query Invariant (Q1).

**Enforcement**:
- All `useQuery` hooks include `limit` parameter (default: 20)
- Server rejects queries without limit
- UI shows total count but loads pages lazily

**Test**: Open table with 10,000 rows → only 20 fetched → pagination works

---

### O2: Stale Data Indicated

**Invariant**: If data is cached/stale, dashboard shows **age** or **refresh button**.

**Rationale**: Users must know if they're viewing current state.

**Enforcement**:
- React Query `staleTime` set to 5 minutes
- UI shows "Updated 2 min ago" timestamp
- Red indicator if data > 10 minutes old

**Test**: Open page → wait 6 minutes → "Stale data" indicator appears

---

### O3: Error States Shown

**Invariant**: Network/API errors are **visible to the user** (not silent failures).

**Rationale**: Users need to know when dashboard is not reflecting reality.

**Enforcement**:
- React Query `onError` shows toast notification
- Failed components show error boundary with retry button
- Network offline → banner at top

**Test**: Disconnect network → "Cannot connect to AeroDB" error shown

---

### O4: No Auto-Refresh by Default

**Invariant**: Data does **not** auto-refresh unless user opts in.

**Rationale**: Reduces load on database, makes UI behavior predictable.

**Enforcement**:
- React Query `refetchInterval` is `false` by default
- Real-time subscriptions are opt-in (user clicks "Enable Live Updates")
- Manual refresh button always available

**Test**: Open dashboard → data does not change without user action

---

## Security Invariants

### S1: Tokens Not Logged

**Invariant**: Access/refresh tokens **never** appear in console logs, error messages, or analytics.

**Rationale**: Prevents token leakage.

**Enforcement**:
- Axios interceptor strips `Authorization` header from error logs
- Sentry/logging SDK configured to redact tokens
- Dev tools console checks disabled in production

**Test**: Trigger API error → inspect logs → no token visible

---

### S2: HTTPS Only

**Invariant**: Dashboard **refuses to connect** to non-HTTPS AeroDB endpoints (except localhost).

**Rationale**: Prevents token interception.

**Enforcement**:
- API client checks URL scheme
- Throws error if `http://` and not `localhost`
- CSP headers enforce `upgrade-insecure-requests`

**Test**: Set `VITE_AERODB_URL=http://example.com` → dashboard shows error

---

### S3: XSS Protection

**Invariant**: User-provided content is **sanitized** before rendering.

**Rationale**: Prevents injection attacks.

**Enforcement**:
- Use React's JSX (auto-escapes text)
- If using `dangerouslySetInnerHTML`, sanitize with DOMPurify
- CSP headers disallow inline scripts

**Test**: Insert `<script>alert('xss')</script>` in table cell → rendered as text, not executed

---

## Testing Invariants

### T1: E2E Coverage Required

**Invariant**: Every user-facing feature has **at least one E2E test**.

**Rationale**: UI bugs are caught before deployment.

**Enforcement**:
- Playwright tests for each route
- CI fails if coverage < 80%
- Critical paths (login, data fetch) have multiple tests

**Test**: Run `npm run test:e2e` → all tests pass

---

### T2: API Mocking in Dev

**Invariant**: Dashboard can run **without a real AeroDB instance** (using mocks).

**Rationale**: Speeds up frontend development.

**Enforcement**:
- MSW (Mock Service Worker) provides API mocks
- `npm run dev:mock` starts dashboard with mock API
- Mock data matches real API schema

**Test**: Run `npm run dev:mock` → dashboard functional, data shown

---

## Violation Handling

If an invariant is violated:

1. **Alert**: User sees error message (not crash)
2. **Log**: Error sent to observability backend (Sentry)
3. **Degrade**: Dashboard shows reduced functionality (read-only mode)
4. **Recover**: User can retry or refresh
