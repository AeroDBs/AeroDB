# Phase 8: Observability Mapping

**Document Type:** Normative Specification  
**Phase:** 8 - Authentication & Authorization  
**Status:** Active

---

## Overview

This document maps authentication events to observability primitives (logs, metrics, traces).

---

## Event Catalog

### Authentication Events

| Event | Level | Metrics | Alert |
|-------|-------|---------|-------|
| `auth.signup.success` | INFO | counter | No |
| `auth.signup.failure` | WARN | counter | >10/min |
| `auth.login.success` | INFO | counter | No |
| `auth.login.failure` | WARN | counter | >20/min |
| `auth.logout` | INFO | counter | No |
| `auth.token.refresh` | DEBUG | counter | No |
| `auth.token.expired` | DEBUG | counter | No |
| `auth.session.revoked` | INFO | counter | No |

### Authorization Events

| Event | Level | Metrics | Alert |
|-------|-------|---------|-------|
| `rls.filter.applied` | DEBUG | counter | No |
| `rls.access.denied` | WARN | counter | >5/min |
| `rls.bypass.service` | DEBUG | counter | No |

### Security Events

| Event | Level | Metrics | Alert |
|-------|-------|---------|-------|
| `auth.brute_force.detected` | ERROR | counter | Yes |
| `auth.account.locked` | WARN | counter | Yes |
| `auth.token.reuse` | ERROR | counter | Yes |

---

## Metrics

### Counters

```
aerodb_auth_signups_total{status="success|failure"}
aerodb_auth_logins_total{status="success|failure"}
aerodb_auth_token_refreshes_total
aerodb_auth_sessions_active
aerodb_rls_decisions_total{result="allowed|denied"}
```

### Histograms

```
aerodb_auth_login_duration_seconds
aerodb_auth_token_validation_duration_seconds
aerodb_rls_filter_application_duration_seconds
```

### Gauges

```
aerodb_auth_active_sessions
aerodb_auth_locked_accounts
```

---

## Log Format

### Structured Log Entry

```json
{
  "timestamp": "2026-02-06T00:00:00.000Z",
  "level": "INFO",
  "event": "auth.login.success",
  "user_id": "uuid",
  "session_id": "uuid",
  "ip_address": "x.x.x.x",
  "user_agent": "...",
  "duration_ms": 45
}
```

### Sensitive Data Handling

- `email` → log as `email_hash` (SHA-256)
- `password` → never log
- `token` → never log
- `ip_address` → log (security requirement)

---

## Tracing

### Span Hierarchy

```
auth.login (parent)
├── auth.validate_credentials
│   ├── db.user.find
│   └── crypto.password_verify
├── auth.create_session
│   └── db.session.insert
└── auth.issue_tokens
    └── jwt.sign
```

### Span Attributes

```
span.auth.user_id = "uuid"
span.auth.method = "password|oauth|magic_link"
span.auth.result = "success|failure"
```

---

## Dashboards

### Auth Overview Dashboard

1. **Login Success Rate** (%)
2. **Signup Rate** (per hour)
3. **Active Sessions** (gauge)
4. **Failed Login Attempts** (per minute)
5. **RLS Denials** (per minute)

### Security Dashboard

1. **Brute Force Attempts**
2. **Locked Accounts**
3. **Token Reuse Attempts**
4. **Geographic Anomalies**

---

## Alerting Rules

| Alert | Condition | Severity |
|-------|-----------|----------|
| HighLoginFailures | >50 failures/5min | Warning |
| BruteForceDetected | >10 failures/min same IP | Critical |
| AccountLocked | any lockout | Warning |
| TokenReuseDetected | any reuse | Critical |
