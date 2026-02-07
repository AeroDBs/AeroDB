# Phase 12: Serverless Functions Failure Model

**Document Type:** Failure Model  
**Phase:** 12 - Serverless Functions  
**Status:** Active

---

## Error Classification

### Client Errors (4xx)

#### 404 Not Found
**Causes:**
- Function doesn't exist
- Function disabled

**Example:**
```http
POST /functions/v1/invoke/nonexistent
→ 404: Function not found
```

**Recovery:** Verify function name, check if deployed and enabled

---

#### 400 Bad Request
**Causes:**
- Invalid WASM binary
- Invalid function name format
- Invalid trigger configuration  
- Malformed invocation payload

**Example:**
```http
POST /functions/v1/deploy
Body: {name: "Send Email", wasm: <invalid>}
→ 400: Invalid function name (contains space)
```

**Recovery:** Fix request format

---

### Server Errors (5xx)

#### 500 Internal Server Error
**Causes:**
- WASM runtime panic
- Host function error (db_query failed, etc.)
- Unexpected error in function code

**Example:**
```javascript
export async function handler() {
  throw new Error("Unexpected error");
}
→ 500: Function error
```

**Recovery:** Check function logs, fix function code

---

#### 504 Gateway Timeout
**Causes:**
- Function execution exceeds timeout

**Example:**
```javascript
export async function handler() {
  while (true) { /* infinite loop */ }
}
→ 504: Function timeout after 10s
```

**Recovery:** Optimize function, increase timeout limit

---

#### 507 Insufficient Storage
**Causes:**
- Function exceeds memory limit

**Example:**
```javascript
export async function handler() {
  const huge = new Array(1000000000);  // OOM
}
→ 507: Function exceeded memory limit (128MB)
```

**Recovery:** Optimize memory usage, increase limit

---

## Failure Scenarios

### 1. Function Timeout

#### Scenario: Infinite loop
```javascript
export async function handler() {
  let i = 0;
  while (true) {
    i++;  // Never returns
  }
}
```

**Behavior:**
- Execution starts
- After 10s (default timeout), Tokio kills task
- WASM instance dropped
- Error logged
- Caller receives 504

**Database Impact:** None (previous db_query calls already committed)

**Cleanup:** Automatic (instance memory freed)

---

### 2. Memory Exhaustion

#### Scenario: Allocate too much memory
```javascript
export async function handler() {
  const huge = new Array(200 * 1024 * 1024);  // 200MB
  // Exceeds 128MB limit
}
```

**Behavior:**
- WASM allocation triggers store limiter check
- Limit exceeded → WASM panic ("out of memory")
- Instance dropped
- Error logged
- Caller receives 507

**Partial Allocations:** Freed when instance dropped

---

### 3. Database Trigger Function Failure

#### Scenario: Function fails after DB commit
```sql
INSERT INTO users (id, email) VALUES ('uuid', 'user@example.com');
COMMIT;  -- Database committed
```

```javascript
// Trigger function
export async function onUserCreated(user) {
  // This succeeds
  await db.query("INSERT INTO profiles ...");
  
  // This fails
  throw new Error("Email service down");
}
```

**State:**
- User inserted: ✓ (committed)
- Profile inserted: ✓ (committed in separate transaction)
- Function: ✗ (error)

**Recovery:**
- Error logged
- Application continues
- Function may be retried manually (future: retry policies)

**Key Point:** Database trigger functions are **non-transactional** with triggering operation.

---

### 4. Host Function Error

#### Scenario: db_query fails
```javascript
export async function handler() {
  await db.query("SELECT * FROM nonexistent_table");
}
```

**Behavior:**
- db_query host function executes
- Query parser fails (table not found)
- Error propagated to WASM
- Function returns error
- Caller receives 500

**Database Impact:** None (query never executed)

---

### 5. WASM Compilation Failure

#### Scenario: Invalid WASM binary
```http
POST /functions/v1/deploy
Body: {name: "my-function", wasm: <corrupted binary>}
```

**Behavior:**
- Server attempts to compile WASM
- Module::new() fails
- Deployment rejected
- Caller receives 400

**Prevention:** Client-side WASM validation before upload

---

### 6. Concurrent Invocation Limit

#### Scenario: Too many concurrent invocations
```
1000 concurrent POST /functions/v1/invoke/slow-function
```

**Behavior (Current):** All invocations spawned concurrently (may overload)

**Behavior (Future with Limit):**
- First 100 invocations accepted
- Remaining rejected with 429 (Too Many Requests)
- Queued for later execution

**Mitigation:** Connection pooling, rate limiting

---

## Cascading Failures

### Database Unavailable

**Impact:**
- db_query host function fails
- Functions cannot query database
- HTTP trigger invocations fail with 500

**Mitigation:**
- Circuit breaker on db connection
- Graceful degradation (return cached data)
- Retry with exponential backoff

**Non-Impact:** Functions already running continue (WASM isolated).

---

### WASM Runtime Crash

**Impact:**
- All function invocations fail
- No new functions can be deployed

**Mitigation:**
- Restart runtime
- Fallback to secondary instance
- Alert ops team

**Database Impact:** None (runtime is separate process)

---

### Scheduler Failure

**Impact:**
- Scheduled functions don't run
- Cron jobs missed

**Mitigation:**
- Scheduler health check
- Restart scheduler on failure
- Persistent job queue (future)

**Recovery:** Re-schedule missed jobs based on last_run

---

## Recovery Strategies

### 1. Automatic Retry (Future)

**Proposal:** Retry failed invocations automatically

```rust
pub struct RetryPolicy {
    max_retries: u32,
    backoff: Duration,
}

async fn invoke_with_retry(func: &Function, payload: Value) -> Result<Value> {
    for attempt in 0..policy.max_retries {
        match invoke(func, payload).await {
            Ok(result) => return Ok(result),
            Err(e) if e.is_retriable() => {
                sleep(policy.backoff * attempt).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    Err(FunctionError::MaxRetriesExceeded)
}
```

**Retriable Errors:** Timeout, database connection failure, host function transient errors

**Non-Retriable:** Invalid WASM, memory limit, user code errors

---

### 2. Dead Letter Queue (Future)

**Proposal:** Failed invocations go to DLQ for manual inspection

```rust
async fn handle_failure(func: &Function, payload: Value, error: FunctionError) {
    dead_letter_queue.push(DeadLetter {
        function_id: func.id,
        payload,
        error: error.to_string(),
        timestamp: Utc::now(),
    });
}
```

**Manual Recovery:** Admin inspects DLQ, fixes function, retries

---

### 3. Circuit Breaker

**Proposal:** Prevent cascading failures from external services

```rust
if error_rate(last_5_minutes) > 50% {
    return Err(FunctionError::CircuitOpen);
}
```

**States:**
- Closed: Normal operation
- Open: All invocations fail fast
- Half-Open: Test with single invocation

---

## Error Logging

### Structured Logs
```json
{
  "level": "ERROR",
  "service": "functions",
  "operation": "invoke",
  "function": "send_email",
  "error": "504 Timeout",
  "duration_ms": 10000,
  "user_id": "uuid",
  "request_id": "uuid",
  "timestamp": "2026-02-06T09:00:00Z"
}
```

### Metrics
```
functions_errors_total{name, error_type}
functions_timeout_total{name}
functions_memory_exceeded_total{name}
```

### Alerts
- **CRITICAL:** Error rate > 10% for 5 minutes
- **WARNING:** Timeout rate > 5%
- **INFO:** Memory limit hit

---

## Client Retry Strategy

### HTTP Trigger (Synchronous)

```javascript
async function invokeFunctionWithRetry(name, payload, maxRetries = 3) {
  for (let attempt = 0; attempt < maxRetries; attempt++) {
    try {
      return await fetch(`/functions/v1/invoke/${name}`, {
        method: "POST",
        body: JSON.stringify(payload)
      });
    } catch (error) {
      if (error.status >= 400 && error.status < 500) {
        throw error;  // Client error - don't retry
      }
      if (attempt === maxRetries - 1) {
        throw error;
      }
      await sleep(1000 * Math.pow(2, attempt));  // Exponential backoff
    }
  }
}
```

### Database Trigger (Asynchronous)

**No Client Retry:** Function invoked automatically, retry handled server-side (future).

---

## Testing Failures

### Chaos Testing

**Inject Failures:**
```
rust
#[cfg(test)]
mod chaos_tests {
    #[test]
    fn test_function_timeout() {
        // Function that sleeps 20s (exceeds 10s timeout)
        assert_eq!(invoke(sleeping_func), Err(FunctionError::Timeout));
    }
    
    #[test]
    fn test_memory_limit() {
        // Function that allocates 256MB (exceeds 128MB limit)
        assert_eq!(invoke(memory_hog_func), Err(FunctionError::OutOfMemory));
    }
    
    #[test]
    fn test_database_down() {
        // db_query fails with connection error
        assert_eq!(invoke(db_func), Err(FunctionError::HostFunctionError));
    }
}
```

---

## Failure Documentation

**Per-Error Code:**

| Error | User-Facing Message | Admin Action |
|-------|---------------------|--------------|
| 404 | Function not found | Deploy function |
| 400 | Invalid request | Fix request format |
| 500 | Function error | Check logs, fix code |
| 504 | Function timeout | Optimize or increase timeout |
| 507 | Memory exceeded | Optimize or increase limit |
