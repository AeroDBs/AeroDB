# Phase 12: Readiness Checklist

**Document Type:** Readiness Checklist  
**Phase:** 12 - Serverless Functions  
**Status:** In Progress

---

## Documentation Checklist

- [x] FUNCTIONS_VISION.md - Goals and philosophy
- [x] FUNCTIONS_ARCHITECTURE.md - WASM runtime and components
- [x] FUNCTIONS_FUNCTION_MODEL.md - Function structure and metadata
- [x] FUNCTIONS_EXECUTION_MODEL.md - Trigger types and execution flow
- [x] FUNCTIONS_INVARIANTS.md - Security and isolation invariants
- [x] FUNCTIONS_FAILURE_MODEL.md - Error handling and recovery
- [x] FUNCTIONS_TESTING_STRATEGY.md - Test coverage plan
- [x] FUNCTIONS_READINESS.md - Freeze criteria (this document)

**Status:** ✅ All 8 documents complete

---

## Implementation Checklist

### Core Modules
- [ ] `src/functions/mod.rs` - Module entry point
- [ ] `src/functions/errors.rs` - Function error types
- [ ] `src/functions/function.rs` - Function model and config
- [ ] `src/functions/trigger.rs` - Trigger types (HTTP, DB, Schedule)
- [ ] `src/functions/registry.rs` - Function registry with indexing
- [ ] `src/functions/runtime.rs` - WASM runtime integration (wasmer/wasmtime)
- [ ] `src/functions/invoker.rs` - Function execution with limits
- [ ] `src/functions/scheduler.rs` - Cron job scheduler
- [ ] `src/functions/host_functions.rs` - Host function implementations

**Priority:** runtime.rs requires WASM integration (external dependency)

### API Endpoints
- [ ] POST `/functions/v1/deploy` - Deploy new function
- [ ] GET `/functions/v1/{name}` - Get function metadata
- [ ] PATCH `/functions/v1/{name}` - Update function config
- [ ] DELETE `/functions/v1/{name}` - Undeploy function
- [ ] POST `/functions/v1/invoke/{name}` - Invoke HTTP trigger
- [ ] GET `/functions/v1/{name}/logs` - Get invocation logs

### WASM Runtime
- [ ] Integrate wasmer or wasmtime
- [ ] Module compilation and caching
- [ ] Instance creation with host functions
- [ ] Timeout enforcement (Tokio)
- [ ] Memory limit enforcement (store limiter)
- [ ] Stack limit enforcement

### Host Functions
- [ ] `db_query` - Query database with RLS
- [ ] `log` - Write logs
- [ ] `env` - Get environment variables
- [ ] `http_fetch` - External HTTP requests (future)

### Triggers
- [ ] HTTP trigger - Synchronous invocation
- [ ] Database trigger - Fire on INSERT/UPDATE/DELETE
- [ ] Schedule trigger - Cron-based execution
- [ ] Webhook trigger - External POST with signature (future)

### Metadata Storage
- [ ] `functions` collection schema
- [ ] `function_invocations` collection schema
- [ ] Executor integration for metadata queries

---

## Test Checklist

### Unit Tests
- [ ] Registry operations (> 10 tests)
- [ ] Runtime module loading (> 8 tests)
- [ ] Invoker execution (> 15 tests)
- [ ] Scheduler cron parsing (> 10 tests)
- [ ] Host functions (> 10 tests)
- [ ] Error handling (> 10 tests)

**Target:** > 60 unit tests, > 90% coverage

### Integration Tests
- [ ] HTTP function E2E
- [ ] Database trigger E2E
- [ ] Schedule trigger E2E
- [ ] RLS enforcement with functions
- [ ] Concurrent invocations

**Target:** > 10 integration tests

### Stress Tests
- [ ] 1000 concurrent invocations
- [ ] Memory leak detection
- [ ] Timeout enforcement under load

### Security Tests
- [ ] RLS bypass attempts blocked
- [ ] Secrets redacted in logs
- [ ] Sandbox escape attempts blocked

---

## Invariant Validation

Each invariant must have tests proving it holds:

| Invariant | Test Coverage | Status |
|-----------|---------------|--------|
| FUNC-I1: Sandbox Isolation | Security tests | ⬜ |
| FUNC-I2: Resource Limits | Stress tests | ⬜ |
| FUNC-I3: RLS Enforced | Integration tests | ⬜ |
| FUNC-I4: Error Isolation | Failure injection | ⬜ |
| FUNC-I5: Hash Integrity | Deploy tests | ⬜ |
| FUNC-E1: Logging | Integration tests | ⬜ |
| FUNC-E2: Concurrency | Stress tests | ⬜ |
| FUNC-T1: DB Trigger Timing | Integration tests | ⬜ |

---

## Performance Benchmarks

| Metric | Target | Actual | Status |
|--------|--------|--------|--------|
| Cold start (p95) | < 100ms | - | ⬜ |
| Execution overhead (p95) | < 5ms | - | ⬜ |
| Throughput | > 50/s | - | ⬜ |
| Concurrent invocations | > 100 | - | ⬜ |

---

## Integration Points

### AeroDB Core
- [ ] Executor integration for db_query host function
- [ ] WAL integration for database triggers
- [ ] RLS context from auth module

### REST API (Phase 9)
- [ ] Function endpoints registered
- [ ] Multipart upload for WASM binaries
- [ ] Error mapping to HTTP status codes

### Authentication (Phase 8)
- [ ] RLS context passed to all invocations
- [ ] Service role bypass tested
- [ ] JWT validation for HTTP triggers

### Real-Time (Phase 10)
- [ ] Function writes propagate to event log
- [ ] Events visible to subscribers

---

## External Dependencies

### WASM Runtime

**Option 1: wasmer** (Recommended)
```toml
[dependencies]
wasmer = "4.0"
wasmer-wasi = "4.0"
```

**Option 2: wasmtime**
```toml
[dependencies]
wasmtime = "15.0"
```

**Decision:** Choose wasmer for better resource limit API

### Cron Parser
```toml
[dependencies]
cron = "0.12"
```

---

## Deployment Checklist

### Configuration
- [ ] `functions.timeout_default` = 10s
- [ ] `functions.memory_default` = 128MB
- [ ] `functions.wasm_max_size` = 10MB
- [ ] `functions.concurrent_limit` = 1000

### Migration
- [ ] Create `functions` schema
- [ ] Create `function_invocations` schema
- [ ] Index on function name
- [ ] Index on trigger type

### Monitoring
- [ ] Metrics: invocations, duration, errors
- [ ] Metrics: cold start latency
- [ ] Logs: function invocation with context
- [ ] Alerts: error rate > 5%, timeout rate > 10%

---

## Freeze Criteria

Phase 12 can be frozen when:

1. ✅ All 8 documentation files complete
2. ⬜ All 9 core modules implemented
3. ⬜ WASM runtime integrated (wasmer)
4. ⬜ All 6 API endpoints working
5. ⬜ All 4 host functions implemented
6. ⬜ All 3 trigger types working (HTTP, DB, Schedule)
7. ⬜ Metadata stored in AeroDB via executor
8. ⬜ All 8 invariants have test coverage
9. ⬜ > 60 unit tests passing
10. ⬜ > 10 integration tests passing
11. ⬜ Performance benchmarks meet targets
12. ⬜ Timeout and memory limits enforced
13. ⬜ RLS enforcement verified

**Current Status:** 🔴 NOT READY (docs complete, implementation needed)

---

## Known Limitations (Deferred)

- WASI support for filesystem/network access
- Multi-language SDKs (JavaScript, Python compile to WASM)
- Function-to-function calls
- Warm instance pooling (cold start optimization)
- Retry policies for failed invocations
- Dead letter queue for errors
- Distributed tracing
- Custom npm/pip packages in functions
- Webhook signature verification
- Rate limiting per function

These are explicitly deferred to keep Phase 12 focused on core WASM execution.

---

## Risk Assessment

| Risk | Mitigation | Owner |
|------|------------|-------|
| WASM runtime complexity | Use battle-tested wasmer | Core team |
| Resource limit bypass | Sandbox + limits enforced | Security team |
| Function errors crash DB | Explicit isolation, non-transactional | Core team |
| Cold start latency | Module caching, pre-compilation | Performance team |
| Concurrent invocation DoS | Connection pooling, rate limits | Infrastructure team |

---

## Sign-Off Required

- [ ] Lead Engineer: Core implementation review
- [ ] Security: Sandbox and RLS review
- [ ] QA: Test coverage and edge cases
- [ ] Docs: All 8 documents complete (✅ DONE)
- [ ] Performance: Benchmarks meet targets
- [ ] DevOps: WASM runtime dependencies ready
