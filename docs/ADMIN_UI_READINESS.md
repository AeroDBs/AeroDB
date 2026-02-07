# Phase 13: Admin Dashboard - Readiness Criteria

## Freeze Checklist

Phase 13 is ready to freeze when **all items below** are complete:

---

## 1. Core Features Implemented

- [ ] **Database Management**
  - [ ] Table browser with pagination (20 items/page)
  - [ ] Column sorting (click header)
  - [ ] Filter builder (field, operator, value)
  - [ ] SQL console with Monaco Editor
  - [ ] Query execution (with timeout)
  - [ ] Explain plan viewer
  - [ ] Schema visualizer (tables + relations)
  - [ ] Schema editor (generates migration SQL)

- [ ] **Authentication & Authorization**
  - [ ] User list (name, email, created_at, last_login)
  - [ ] Add/edit/delete users
  - [ ] Active sessions table
  - [ ] Revoke session action
  - [ ] RLS policy viewer (read-only)

- [ ] **File Storage**
  - [ ] Bucket list
  - [ ] File browser with breadcrumbs
  - [ ] Upload (drag-and-drop or browse)
  - [ ] Download file
  - [ ] Delete file (with confirmation)
  - [ ] Storage metrics (used/total per bucket)

- [ ] **Real-Time Monitoring**
  - [ ] Active subscriptions table (user, channel, filter)
  - [ ] Event log (last 100 events)
  - [ ] Enable live updates toggle
  - [ ] Connection list (WebSocket connections)

- [ ] **Cluster Management**
  - [ ] Topology view (authority + replicas)
  - [ ] Replication lag chart
  - [ ] Promote replica button (with confirmation)
  - [ ] WAL viewer (read-only)

- [ ] **Observability**
  - [ ] Structured log viewer (filter by level, time, module)
  - [ ] Metrics dashboard (queries/sec, latency, errors)
  - [ ] Audit log (user actions)

---

## 2. Testing Complete

- [ ] **Unit Tests**
  - [ ] 80%+ line coverage
  - [ ] All React components tested
  - [ ] All custom hooks tested
  - [ ] All utilities tested

- [ ] **Integration Tests**
  - [ ] Login flow
  - [ ] CRUD operations on each resource
  - [ ] API error handling

- [ ] **E2E Tests (Playwright)**
  - [ ] Critical path: Login → browse table → logout
  - [ ] Table filtering and pagination
  - [ ] User management workflow
  - [ ] File upload/download
  - [ ] Real-time subscription

- [ ] **Visual Regression**
  - [ ] Screenshots for all major pages
  - [ ] Dark/light theme variants

- [ ] **Performance**
  - [ ] Lighthouse score > 90
  - [ ] Page load < 1s
  - [ ] Query results < 500ms (p95)

---

## 3. Security Verified

- [ ] **Authentication**
  - [ ] JWT validation on all endpoints
  - [ ] Token refresh flow
  - [ ] Auto-logout on token expiry
  - [ ] Session hijacking protection (rotate tokens)

- [ ] **XSS Protection**
  - [ ] All user input sanitized
  - [ ] CSP headers configured
  - [ ] No `dangerouslySetInnerHTML` without DOMPurify

- [ ] **CSRF Protection**
  - [ ] Destructive actions require confirmation
  - [ ] No state mutations via GET requests

- [ ] **HTTPS Only**
  - [ ] Dashboard rejects HTTP endpoints (except localhost)
  - [ ] Tokens never logged

---

## 4. Invariants Enforced

All invariants from [ADMIN_UI_INVARIANTS.md](ADMIN_UI_INVARIANTS.md) verified:

- [ ] I1: Dashboard failures don't break database
- [ ] I2: Database is source of truth
- [ ] I3: No hidden mutations
- [ ] I4: RLS respected
- [ ] I5: API version compatibility
- [ ] O1: Pagination always required
- [ ] O2: Stale data indicated
- [ ] O3: Error states shown
- [ ] O4: No auto-refresh by default
- [ ] S1: Tokens not logged
- [ ] S2: HTTPS only
- [ ] S3: XSS protection
- [ ] T1: E2E coverage required
- [ ] T2: API mocking in dev

---

## 5. Documentation Complete

- [x] ADMIN_UI_VISION.md
- [x] ADMIN_UI_ARCHITECTURE.md
- [x] ADMIN_UI_UI_MODEL.md
- [x] ADMIN_UI_INVARIANTS.md
- [x] ADMIN_UI_TESTING_STRATEGY.md
- [x] ADMIN_UI_READINESS.md (this file)
- [ ] ADMIN_UI_DEPLOYMENT.md
- [ ] ADMIN_UI_OBSERVABILITY.md
- [ ] README.md in dashboard/

---

## 6. Deployment Ready

- [ ] **Build**
  - [ ] `npm run build` succeeds
  - [ ] Bundle size < 500KB (gzipped)
  - [ ] No unused dependencies
  - [ ] Tree shaking enabled

- [ ] **Environment Config**
  - [ ] `.env.example` provided
  - [ ] Required vars documented: VITE_AERODB_URL, VITE_WS_URL
  - [ ] Config validation on startup

- [ ] **Static Hosting**
  - [ ] Deployable to Vercel/Netlify
  - [ ] `dist/` contains all necessary files
  - [ ] `_redirects` handles SPA routing

- [ ] **Docker**
  - [ ] Dockerfile provided (nginx serving `dist/`)
  - [ ] Multi-stage build (reduce image size)

---

## 7. Accessibility

- [ ] **WCAG 2.1 AA**
  - [ ] Keyboard navigation works
  - [ ] Screen reader compatible
  - [ ] Focus indicators visible
  - [ ] Color contrast ≥ 4.5:1 (text)
  - [ ] ARIA labels on interactive elements

- [ ] **Responsive Design**
  - [ ] Mobile (< 768px): Sidebar collapses
  - [ ] Tablet (768-1024px): Single column layouts
  - [ ] Desktop (> 1024px): Multi-column layouts

---

## 8. Observability

- [ ] **Error Tracking**
  - [ ] Sentry integrated (or equivalent)
  - [ ] Errors include user ID, route, stack trace
  - [ ] PII (passwords, tokens) redacted

- [ ] **Analytics**
  - [ ] Page views tracked
  - [ ] User actions logged (button clicks, queries executed)
  - [ ] Performance metrics (LCP, FID, CLS)

- [ ] **Logging**
  - [ ] Console logs suppressed in production
  - [ ] Structured logs sent to backend

---

## 9. User Documentation

- [ ] **Admin Guide**
  - [ ] Getting started (login, navigation)
  - [ ] Table browser usage
  - [ ] SQL console tips
  - [ ] User management
  - [ ] File upload/download

- [ ] **Developer Guide**
  - [ ] Setup instructions (`npm install`, `npm run dev`)
  - [ ] Environment variables
  - [ ] Extending the dashboard (add new page)

---

## 10. Known Limitations Documented

- [ ] **Limitations**
  - [ ] Dashboard is read-heavy (writes go through API)
  - [ ] No offline mode (requires network)
  - [ ] RLS policies not editable via UI (use SQL)
  - [ ] Large result sets require pagination

---

## Sign-Off

Phase 13 is **frozen** when:

1. All checklist items above are complete
2. All tests pass (unit, integration, E2E)
3. Code review approved by at least one other developer
4. Security audit complete (no critical vulnerabilities)

**Frozen on**: [DATE]

**Approved by**: [NAME]
