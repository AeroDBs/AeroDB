# Phase 12: Testing Strategy

**Document Type:** Testing Strategy  
**Phase:** 12 - Serverless Functions  
**Status:** Active

---

## Test Coverage Goals

| Component | Target | Focus Areas |
|-----------|--------|-------------|
| Function registry | 95% | CRUD, indexing, concurrency |
| WASM runtime (stubbed) | 90% | Module loading, resource limits |
| Invoker | 95% | Execution, timeout, error handling |
| Scheduler | 90% | Cron parsing, job execution |
| Host functions | 95% | db_query, log, RLS enforcement |
| Triggers | 90% | HTTP, database, schedule |

---

## Unit Tests

### Registry Tests
```rust
#[test]
fn test_register_function()
fn test_duplicate_name_errors()
fn test_get_by_name()
fn test_get_by_trigger()
fn test_unregister()
fn test_concurrent_registration()
```

### Runtime Tests
```rust
#[test]
fn test_load_valid_wasm()
fn test_invalid_wasm_rejected()
fn test_timeout_enforcement()
fn test_memory_limit_enforcement()
fn test_cold_start_performance()
```

### Invoker Tests
```rust
#[test]
fn test_invoke_http_function()
fn test_invoke_with_rls_context()
fn test_disabled_function_returns_error()
fn test_timeout_kills_function()
fn test_memory_exceeded_error()
fn test_invocation_logged()
```

### Scheduler Tests
```rust
#[test]
fn test_parse_cron_expression()
fn test_calculate_next_run()
fn test_get_due_jobs()
fn test_invalid_cron_rejected()
fn test_job_invocation()
fn test_concurrent_jobs()
```

---

## Integration Tests

### End-to-End Scenarios

#### 1. HTTP Function Flow
```rust
#[tokio::test]
async fn test_http_function_e2e() {
    // 1. Deploy function
    let func = deploy_function("hello", http_trigger, hello_wasm).await?;
    
    // 2. Invoke function
    let response = invoke_http("hello", json!({"name": "World"})).await?;
    
    // 3. Verify response
    assert_eq!(response.status, 200);
    assert_eq!(response.body["message"], "Hello, World!");
    
    // 4. Verify invocation logged
    let logs = query_invocations(func.id).await?;
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].status, "success");
}
```

#### 2. Database Trigger Flow
```rust
#[tokio::test]
async fn test_database_trigger_e2e() {
    // 1. Deploy database trigger function
    let func = deploy_function(
        "on_user_created",
        database_trigger("users", "INSERT"),
        create_profile_wasm
    ).await?;
    
    // 2. Insert user (triggers function)
    db.execute("INSERT INTO users (id, email) VALUES ($1, $2)", 
               ["uuid", "user@example.com"]).await?;
    
    // 3. Wait for async trigger
    sleep(Duration::from_millis(100)).await;
    
    // 4. Verify profile created
    let profiles = db.query("SELECT * FROM profiles WHERE user_id = $1", ["uuid"]).await?;
    assert_eq!(profiles.len(), 1);
    
    // 5. Verify function invoked
    let logs = query_invocations(func.id).await?;
    assert_eq!(logs[0].trigger, "database");
}
```

#### 3. Scheduled Job Flow
```rust
#[tokio::test]
async fn test_schedule_trigger_e2e() {
    // 1. Deploy scheduled function (every minute)
    let func = deploy_function(
        "cleanup_job",
        schedule_trigger("* * * * *"),
        cleanup_wasm
    ).await?;
    
    // 2. Wait for job to run
    sleep(Duration::from_secs(61)).await;
    
    // 3. Verify function invoked
    let logs = query_invocations(func.id).await?;
    assert!(logs.len() >= 1);
    assert_eq!(logs[0].trigger, "schedule");
}
```

---

## Host Function Tests

### db_query Tests
```rust
#[test]
fn test_db_query_respects_rls() {
    let context = RlsContext { user_id: Some(user_a_id), ... };
    
    // User A can query their own data
    let result = db_query_host("SELECT * FROM users WHERE id = $1", [user_a_id], &context)?;
    assert_eq!(result.len(), 1);
    
    // User A cannot query user B's data (RLS blocks)
    let result = db_query_host("SELECT * FROM users WHERE id = $1", [user_b_id], &context)?;
    assert_eq!(result.len(), 0);
}

#[test]
fn test_db_query_service_role_bypass() {
    let context = RlsContext { can_bypass_rls: true, ... };
    
    // Service role can query all data
    let result = db_query_host("SELECT * FROM users", [], &context)?;
    assert_eq!(result.len(), 2);  // All users
}
```

### log Tests
```rust
#[test]
fn test_log_host_function() {
    log_host("info", "Test message");
    
    // Verify log written
    let logs = capture_logs();
    assert!(logs.contains("[Function] Test message"));
}
```

---

## Stress Tests

### Concurrent Invocations
```rust
#[tokio::test]
async fn test_1000_concurrent_invocations() {
    let func = deploy_function("fast", http_trigger, fast_wasm).await?;
    
    let mut tasks = vec![];
    for i in 0..1000 {
        tasks.push(tokio::spawn(invoke_http("fast", json!({"id": i}))));
    }
    
    let results = futures::future::join_all(tasks).await;
    
    // All invocations succeed
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1000);
}
```

### Memory Leak Detection
```rust
#[test]
fn test_no_memory_leak_after_1000_invocations() {
    let initial_memory = get_memory_usage();
    
    for _ in 0..1000 {
        invoke_and_wait("simple_function", json!({}));
    }
    
    let final_memory = get_memory_usage();
    
    // Memory usage should not grow significantly
    assert!(final_memory - initial_memory < 10 * 1024 * 1024);  // < 10MB growth
}
```

---

## Failure Injection

| Scenario | Expected Behavior |
|----------|-------------------|
| Function timeout | Error 504, invocation logged, instance cleaned up |
| Memory exceeded | Error 507, WASM panic, instance dropped |
| Invalid WASM | Error 400, deployment rejected |
| db_query fails | Error 500, function error logged |
| Database down | Error 500, circuit breaker (future) |
| Concurrent limit | Error 429 (future), queue invocation |

### Timeout Test
```rust
#[test]
fn test_timeout_enforcement() {
    let func = deploy_function("infinite_loop", http_trigger, loop_wasm)?;
    func.config.timeout = Duration::from_secs(2);
    
    let start = Instant::now();
    let result = invoke("infinite_loop", json!({}));
    let duration = start.elapsed();
    
    assert_eq!(result, Err(FunctionError::Timeout));
    assert!(duration >= Duration::from_secs(2));
    assert!(duration < Duration::from_secs(3));  // Killed promptly
}
```

### Memory Limit Test
```rust
#[test]
fn test_memory_limit_enforcement() {
    let func = deploy_function("memory_hog", http_trigger, allocate_wasm)?;
    func.config.max_memory = 64 * 1024 * 1024;  // 64MB
    
    let result = invoke("memory_hog", json!({"mb": 128}));  // Try 128MB
    
    assert_eq!(result, Err(FunctionError::OutOfMemory));
}
```

---

## Security Tests

### RLS Bypass Attempts
```rust
#[test]
fn test_cannot_bypass_rls_without_service_role() {
    let user_context = RlsContext { user_id: Some(user_id), can_bypass_rls: false, ... };
    
    // Try to query all users (RLS should filter)
    let result = db_query_host("SELECT * FROM private_table", [], &user_context)?;
    
    // Should only see own data
    assert_eq!(result.len(), 1);
    assert_eq!(result[0]["owner_id"], user_id);
}
```

### Secret Redaction
```rust
#[test]
fn test_secrets_redacted_in_logs() {
    let func = deploy_function("with_secrets", http_trigger, wasm)?;
    func.config.environment.insert("API_KEY", "secret123");
    
    invoke("with_secrets", json!({}));
    
    let logs = query_invocation_logs(func.id)?;
    assert!(!logs.contains("secret123"));
    assert!(logs.contains("***REDACTED***"));
}
```

---

## Performance Benchmarks

### Target Metrics
- Cold start: < 100ms (p95)
- Execution overhead: < 5ms (p95)
- Throughput: > 50 invocations/sec
- Concurrent invocations: > 100

### Benchmark Tests
```rust
#[bench]
fn bench_cold_start(b: &mut Bencher) {
    b.iter(|| {
        let func = load_function("benchmark");
        let instance = create_instance(func);
    });
}

#[bench]
fn bench_invoke_simple_function(b: &mut Bencher) {
    let func = deploy_function("simple", http_trigger, simple_wasm)?;
    
    b.iter(|| {
        invoke("simple", json!({}));
    });
}
```

---

## Invariant Validation

Each test category validates specific invariants:

- **FUNC-I1 (Sandbox)**: Security tests verify no escape
- **FUNC-I2 (Limits)**: Stress tests verify timeout/memory enforcement
- **FUNC-I3 (RLS)**: Security tests verify RLS on db_query
- **FUNC-I4 (Isolation)**: Failure injection tests verify database unaffected
- **FUNC-I5 (Hash)**: Deploy tests verify hash integrity

---

## Coverage Validation

Run with coverage:
```bash
cargo tarpaulin --lib --packages aerodb --out Lcov -- functions::
```

**Acceptance:** > 90% line coverage, > 85% branch coverage

---

## Test Data

### Sample WASM Functions
- `hello_wasm`: Returns "Hello, World!"
- `db_query_wasm`: Queries database
- `loop_wasm`: Infinite loop (for timeout testing)
- `allocate_wasm`: Allocates large memory (for limit testing)
- `panic_wasm`: Panics immediately

### Fixtures
- Test functions stored in `tests/fixtures/functions/`
- Compiled to WASM in CI
- Versioned with tests

---

## CI Integration

### Pipeline
```yaml
test_functions:
  - cargo test functions:: --lib
  - cargo bench functions:: --no-run
  - cargo tarpaulin functions::
```

### Acceptance Criteria
- All unit tests pass
- All integration tests pass
- Coverage > 90%
- No memory leaks detected
- Performance benchmarks meet targets
