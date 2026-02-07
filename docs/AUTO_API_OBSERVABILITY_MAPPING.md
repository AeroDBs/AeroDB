# Phase 9: Observability Mapping

**Document Type:** Technical Specification  
**Phase:** 9 - Auto-Generated REST API  
**Status:** Active

---

## Metrics

### Counters

```
aerodb_rest_requests_total{method, collection, status}
aerodb_rest_errors_total{error_code}
aerodb_rls_filter_applied_total{collection}
```

### Histograms

```
aerodb_rest_request_duration_seconds{method, collection}
aerodb_rest_response_size_bytes{method, collection}
```

---

## Logging

### Request Log

```json
{
  "timestamp": "2026-02-06T00:00:00Z",
  "level": "INFO",
  "event": "rest.request",
  "method": "GET",
  "path": "/rest/v1/posts",
  "collection": "posts",
  "user_id": "uuid",
  "query_params": {"limit": "10"},
  "duration_ms": 45,
  "status": 200,
  "response_count": 10
}
```

### Error Log

```json
{
  "timestamp": "2026-02-06T00:00:00Z",
  "level": "WARN",
  "event": "rest.error",
  "error_code": "invalid_query_param",
  "path": "/rest/v1/posts",
  "status": 400
}
```

---

## Tracing

### Span: rest.request

```
rest.request (parent)
├── rest.parse_query
├── rest.auth_extract
├── rest.rls_filter
├── rest.execute
└── rest.format_response
```

---

## Dashboards

1. **Request Rate** (per minute)
2. **Error Rate** (4xx/5xx)
3. **Latency P50/P95/P99**
4. **Top Collections by Request**
5. **RLS Denials**
