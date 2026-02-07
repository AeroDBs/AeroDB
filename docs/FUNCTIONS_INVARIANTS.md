# Phase 12: Serverless Functions Invariants

**Document Type:** Invariants Specification  
**Phase:** 12 - Serverless Functions  
**Status:** Active

---

## Core Invariants

### FUNC-I1: Sandbox Isolation
> **Functions cannot escape WASM sandbox**

**Formal Statement:**
```
∀ function f, invocation i:
  execute(f, i) → memory_isolated(f) ∧ no_filesystem_access(f) ∧ no_raw_sockets(f)
```

**Enforcement:**
- WASM sandbox by design (no syscalls)
- Host functions are only escape mechanism
- Memory mapped private per instance

**Violation Consequences:**
- Security breach
- Cross-function data leakage
- System compromise

---

### FUNC-I2: Resource Limits Enforced
> **All invocations respect timeout and memory limits**

**Formal Statement:**
```
∀ invocation i with config c:
  execution_time(i) > c.timeout → terminate(i) ∧ error("Timeout")
  memory_usage(i) > c.max_memory → panic(i) ∧ error("OutOfMemory")
```

**Enforcement:**
```rust
tokio::timeout(config.timeout, execute);
store.limiter(|_| ResourceLimiter { memory_size: config.max_memory });
```

**Violation Consequences:**
- Runaway functions consume resources
- System instability
- Denial of service

---

### FUNC-I3: RLS Enforced on Host Functions
> **Host functions respect RLS context**

**Formal Statement:**
```
∀ host_call h in function f with context c:
  h(db_query, sql) → enforce_rls(sql, c)
```

**Enforcement:**
```rust
fn db_query_host(sql: &str) -> Result<Vec<Row>> {
    let query = parse_sql(sql)?;
    executor.execute(query, &rls_context)?  // ⬅️ RLS context from invocation
}
```

**Violation Consequences:**
- Unauthorized data access
- RLS bypass
- Security breach

**Service Role Exception:** Explicit bypass via `can_bypass_rls` flag.

---

### FUNC-I4: Function Errors Isolated
> **Function failures do NOT crash database**

**Formal Statement:**
```
∀ function f, error e:
  execute(f) = Err(e) → log(e) ∧ database_unaffected
```

**Enforcement:**
```rust
match invoke_function(func, payload) {
    Ok(result) => Ok(result),
    Err(e) => {
        log::error!("Function '{}' failed: {}", func.name, e);
        Err(e)  // Database continues normally
    }
}
```

**Implication:** Functions are non-transactional with database operations.

---

### FUNC-I5: Hash Integrity
> **wasm_hash matches wasm_bytes**

**Formal Statement:**
```
∀ function f:
  f.wasm_hash = SHA256(f.wasm_bytes)
```

**Enforcement:**
```rust
pub fn verify_hash(func: &Function) -> Result<()> {
    let mut hasher = Sha256::new();
    hasher.update(&func.wasm_bytes);
    let actual_hash = format!("{:x}", hasher.finalize());
    
    if actual_hash != func.wasm_hash {
        return Err(FunctionError::HashMismatch);
    }
    Ok(())
}
```

**Purpose:** Detect corruption, ensure deployed WASM matches expected.

---

## Execution Invariants

### FUNC-E1: Invocation Logging
> **Every invocation is logged**

**Guarantee:**
```
invoke(f, payload) → INSERT INTO function_invocations (...)
```

**Fields Logged:**
- function_id, trigger, status, duration_ms, memory_peak, error_message, timestamp

**Purpose:** Observability, debugging, auditing

---

### FUNC-E2: Concurrent Invocations Isolated
> **Concurrent invocations have separate WASM instances**

**Guarantee:**
```
∀ invocations i1, i2 of function f:
  i1.instance ≠ i2.instance ∧ no_shared_memory(i1, i2)
```

**Enforcement:** New WASM instance per invocation

**Implication:** No state persists between invocations.

---

### FUNC-E3: Determinism Boundary
> **Functions are explicitly non-deterministic**

**Guarantee:**
```
replay(database_operations) → deterministic
replay(function_invocations) → UNDEFINED (non-deterministic)
```

**Rationale:**
- User code behavior undefined
- External API calls non-deterministic
- Time-based logic non-deterministic

**Implication:** Functions cannot be part of deterministic replay.

---

## Security Invariants

### FUNC-S1: Secrets Not Logged
> **Environment variables with sensitive data are redacted**

**Guarantee:**
```
log_invocation(i) → redact(i.config.environment, sensitive_keys)
```

**Sensitive Keys:** API_KEY, SECRET, PASSWORD, TOKEN

**Example:**
```json
{
  "environment": {
    "API_KEY": "***REDACTED***",
    "ENV": "production"
  }
}
```

---

### FUNC-S2: No Filesystem Access
> **Functions cannot read/write filesystem**

**Guarantee:**
```
∀ wasm_call c:
  c ∉ {open, read, write, unlink, ...}  # No syscalls
```

**Enforcement:** WASM has no syscall interface (unless WASI enabled)

**Future:** WASI with explicit file handle grants

---

### FUNC-S3: Network Access Restricted
> **Functions cannot open raw sockets**

**Guarantee:**
```
∀ wasm_call c:
  c ∉ {socket, connect, bind, listen}
```

**Network Access:** Only via `http_fetch` host function (controlled)

---

## Failure Invariants

### FUNC-F1: Timeout Cleanup
> **Timed-out functions are terminated and cleaned up**

**Guarantee:**
```
execution_time(f) > timeout → terminate(f) ∧ free_memory(f)
```

**Enforcement:** Tokio timeout drops future, WASM instance dropped

---

### FUNC-F2: Partial Work Not Rolled Back
> **Database writes committed even if function fails**

**Scenario:**
```javascript
export async function handler() {
  await db.query("INSERT INTO logs ...");  // Committed
  throw new Error("Oops");                 // Function fails
}
```

**Guarantee:**
```
db.query succeeds → commit (even if function later fails)
```

**Rationale:** Functions are non-transactional.

---

### FUNC-F3: Idempotent Cleanup
> **Invocation cleanup is idempotent**

**Guarantee:**
```
cleanup(invocation) → cleanup(invocation) = no-op
```

**Enforcement:**
- WASM instance dropped once
- Logs inserted once
- Memory freed once

---

## Trigger Invariants

### FUNC-T1: Database Triggers Fire After Commit
> **Database triggers invoked after WAL commit**

**Guarantee:**
```
BEGIN;
  INSERT INTO users ...;
  # Function NOT invoked yet
COMMIT;
# Now function invoked
```

**Implication:** Trigger sees committed data, cannot rollback triggering transaction.

---

### FUNC-T2: Schedule Triggers Fire Once Per Cron Match
> **Scheduled functions invoked once per cron expression match**

**Guarantee:**
```
cron_expression matches time T → invoke_once(f, T)
```

**No Duplicate Invocations:** Scheduler tracks last_run, updates next_run.

---

### FUNC-T3: HTTP Triggers Synchronous
> **HTTP trigger waits for function result**

**Guarantee:**
```
POST /functions/v1/invoke/f → invoke(f) → wait_for_result → return response
```

**Contrast with Database/Schedule:** Async (fire-and-forget).

---

## Invariant Testing Matrix

| Invariant | Unit Test | Integration Test | Stress Test |
|-----------|-----------|------------------|-------------|
| FUNC-I1 | ✅ Sandbox checks | ✅ Syscall denial | - |
| FUNC-I2 | ✅ Timeout/memory | ✅ Runaway function | ✅ 100 concurrent |
| FUNC-I3 | ✅ RLS enforcement | ✅ Multi-user | - |
| FUNC-I4 | ✅ Error isolation | ✅ Panic recovery | - |
| FUNC-I5 | ✅ Hash verify | ✅ Deploy+invoke | - |
| FUNC-E1 | ✅ Log insertion | ✅ E2E invocation | - |
| FUNC-E2 | ✅ Instance isolation | ✅ Concurrent invoke | ✅ 1000 parallel |
| FUNC-S1 | ✅ Secret redaction | ✅ Log inspection | - |
| FUNC-T1 | ✅ Trigger timing | ✅ DB trigger | - |
| FUNC-T2 | ✅ Cron scheduling | ✅ No duplicates | - |
| FUNC-T3 | ✅ HTTP sync | ✅ Request/response | - |

---

## Invariant Monitoring

### Runtime Checks
- Hash verification on function load
- Resource limit enforcement on every invocation
- RLS context validation before db_query

### Alerts
- Function timeout rate > 10% → WARNING
- Function error rate > 5% → CRITICAL
- Memory limit hit rate > 1% → WARNING
- Concurrent invocations > 1000 → INFO
