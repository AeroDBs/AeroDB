# Phase 12: Execution Model

**Document Type:** Execution Specification  
**Phase:** 12 - Serverless Functions  
**Status:** Active

---

## Execution Flow

```
┌──────────┐
│ Trigger  │ (HTTP, Database, Schedule, Webhook)
└────┬─────┘
     │
     ▼
┌────────────────┐
│   Invoker      │
│ 1. Lookup func │
│ 2. Load WASM   │
│ 3. Call handler│
└────┬───────────┘
     │
     ▼
┌──────────────────────────┐
│    WASM Runtime          │
│ • Sandbox environment    │
│ • Resource limits        │
│ • Host function imports  │
└────┬─────────────────────┘
     │
     ▼
┌──────────────────────────┐
│   Function Code          │
│ exports.handler(event)   │
└────┬─────────────────────┘
     │
     ▼
┌──────────────────────────┐
│   Host Functions         │
│ • db_query(...)          │
│ • log(...)               │
│ • http_fetch(...)        │
└──────────────────────────┘
```

---

## Trigger Types

### 1. HTTP Trigger

**Invocation:**
```http
POST /functions/v1/invoke/{function_name}
Authorization: Bearer <JWT>
Content-Type: application/json

{"key": "value"}
```

**Event Payload:**
```json
{
  "trigger": "http",
  "method": "POST",
  "path": "/functions/v1/invoke/my-function",
  "headers": {"content-type": "application/json"},
  "body": {"key": "value"},
  "user_id": "uuid",  // From JWT
  "request_id": "uuid"
}
```

**Function Handler:**
```javascript
export async function handler(event) {
  const { body } = event;
  // Process request
  return {
    statusCode: 200,
    body: JSON.stringify({ result: "success" })
  };
}
```

**Response:**
Function return value becomes HTTP response body.

---

### 2. Database Trigger

**Registration:**
```rust
Function {
    trigger: TriggerType::Database {
        table: "users",
        operation: DatabaseOperation::Insert,
    }
}
```

**When Fired:**
```sql
INSERT INTO users (id, email) VALUES ('uuid', 'user@example.com');
-- After commit, function invoked
```

**Event Payload:**
```json
{
  "trigger": "database",
  "table": "users",
  "operation": "INSERT",
  "old": null,
  "new": {
    "id": "uuid",
    "email": "user@example.com",
    "created_at": "2026-02-06T09:00:00Z"
  },
  "timestamp": "2026-02-06T09:00:00Z"
}
```

**Function Handler:**
```javascript
export async function handler(event) {
  const user = event.new;
  
  // Create user profile
  await db.query(
    "INSERT INTO profiles (user_id, display_name) VALUES ($1, $2)",
    [user.id, user.email.split('@')[0]]
  );
  
  return { profileCreated: true };
}
```

**UPDATE Operation:**
```json
{
  "trigger": "database",
  "operation": "UPDATE",
  "old": {"id": "uuid", "email": "old@example.com"},
  "new": {"id": "uuid", "email": "new@example.com"},
  ...
}
```

**DELETE Operation:**
```json
{
  "trigger": "database",
  "operation": "DELETE",
  "old": {"id": "uuid", "email": "user@example.com"},
  "new": null,
  ...
}
```

---

### 3. Schedule Trigger

**Registration:**
```rust
Function {
    trigger: TriggerType::Schedule {
        cron: "0 3 * * *",  // Daily at 3 AM
    }
}
```

**Cron Format:**
```
 ┌─────minute (0 - 59)
 │ ┌───hour (0 - 23)
 │ │ ┌─day of month (1 - 31)
 │ │ │ ┌─month (1 - 12)
 │ │ │ │ ┌─day of week (0 - 6, Sunday = 0)
 │ │ │ │ │
 * * * * *
```

**Examples:**
```
"0 */6 * * *"  # Every 6 hours
"*/15 * * * *" # Every 15 minutes
"0 0 1 * *"    # First day of month at midnight
"0 9 * * 1-5"  # Weekdays at 9 AM
```

**Event Payload:**
```json
{
  "trigger": "schedule",
  "cron": "0 3 * * *",
  "scheduled_time": "2026-02-06T03:00:00Z",
  "actual_time": "2026-02-06T03:00:01Z"
}
```

**Function Handler:**
```javascript
export async function handler(event) {
  console.log("Running cleanup job");
  
  const result = await db.query(
    "DELETE FROM logs WHERE created_at < NOW() - INTERVAL '30 days'"
  );
  
  return { deletedRows: result.rowCount };
}
```

---

### 4. Webhook Trigger (Future)

**Registration:**
```rust
Function {
    trigger: TriggerType::Webhook {
        secret: "whsec_...",  // Verify signature
    }
}
```

**Invocation:**
```http
POST /functions/v1/webhook/{function_name}
X-Webhook-Signature: sha256=...
Content-Type: application/json

{"event": "payment.succeeded", "amount": 1000}
```

**Event Payload:**
```json
{
  "trigger": "webhook",
  "headers": {"x-webhook-signature": "sha256=..."},
  "body": {"event": "payment.succeeded", "amount": 1000},
  "timestamp": "2026-02-06T09:00:00Z"
}
```

---

## Resource Limits

### Timeout

**Default:** 10 seconds  
**Range:** 1s - 300s (5 minutes)  
**Configurable:** Per-function

**Enforcement:**
```rust
let result = tokio::time::timeout(
    func.config.timeout,
    call_handler(instance, payload)
).await;

match result {
    Ok(value) => Ok(value),
    Err(_) => Err(FunctionError::Timeout),
}
```

**Behavior:**
- Function exceeds timeout → Killed immediately
- Partial work NOT rolled back (non-transactional)
- Error logged, caller receives 504

---

### Memory Limit

**Default:** 128 MB  
**Range:** 16MB - 512MB  
**Configurable:** Per-function

**Enforcement:**
```rust
let mut store = Store::new(&engine);
store.limiter(|_| ResourceLimiter {
    memory_size: func.config.max_memory,
});
```

**Behavior:**
- Allocation exceeds limit → WASM panic
- Error logged, caller receives 507

---

### CPU (Future)

**Proposal:** CPU instruction count limit
- Track WASM instructions executed
- Terminate after threshold (e.g., 1 billion instructions)

**Not Implemented:** Current timeout is wall-clock based

---

## Host Functions

Functions call AeroDB via imported host functions.

### db_query

**Signature:**
```rust
fn db_query(sql: String, params: Vec<Value>) -> Result<Vec<Row>>
```

**WASM Import:**
```javascript
// Provided by AeroDB runtime
extern "aerodb" {
  function db_query(sql: string, params: any[]): Row[];
}
```

**Example:**
```javascript
const users = await db.query(
  "SELECT * FROM users WHERE created_at > $1",
  [new Date('2026-01-01')]
);
```

**RLS Enforcement:**
Function inherits RLS context from invocation. Cannot bypass without service role.

---

### log

**Signature:**
```rust
fn log(level: String, message: String)
```

**Levels:** debug, info, warn, error

**Example:**
```javascript
log.info("Processing user registration");
log.error("SMTP connection failed");
```

**Output:**
```json
{
  "level": "INFO",
  "source": "function:send_welcome_email",
  "message": "Processing user registration",
  "timestamp": "2026-02-06T09:00:00Z"
}
```

---

### http_fetch (Future)

**Signature:**
```rust
async fn http_fetch(url: String, options: HttpOptions) -> Result<HttpResponse>
```

**Example:**
```javascript
const response = await http.fetch("https://api.stripe.com/v1/charges", {
  method: "POST",
  headers: {"Authorization": `Bearer ${env("STRIPE_KEY")}`},
  body: JSON.stringify({amount: 1000})
});
```

**Security:**
- Whitelist allowed domains in function config
- Rate limiting per function
- Timeout applies to HTTP request

---

### env

**Signature:**
```rust
fn env(key: String) -> Option<String>
```

**Example:**
```javascript
const apiKey = env("STRIPE_API_KEY");
const dbUrl = env("DATABASE_URL");
```

**Security:**
- Only whitelisted env vars returned
- Secrets redacted in logs
- Per-function environment isolation

---

## Error Handling

### Function Errors

**Types:**
- **Timeout:** Function exceeded time limit
- **OutOfMemory:** Function exceeded memory limit
- **Panic:** WASM runtime panic (assertion failure, null pointer, etc.)
- **HostFunctionError:** db_query failed, http_fetch failed, etc.

**Error Propagation:**
```javascript
export async function handler(event) {
  try {
    const result = await db.query("SELECT * FROM users");
    return { users: result };
  } catch (error) {
    // Error logged, propagated to caller
    throw new Error(`Database query failed: ${error.message}`);
  }
}
```

**Caller Receives:**
```json
{
  "error": "500 Internal Server Error",
  "message": "Database query failed: Connection refused",
  "code": "FUNCTION_ERROR"
}
```

---

### Database Isolation

**Key Principle:** Function errors do NOT affect database.

**Scenario:**
```javascript
// Database trigger on INSERT users
export async function handler(event) {
  // This query succeeds
  await db.query("INSERT INTO profiles ...");
  
  // This panics
  throw new Error("Oops");
}
```

**Result:**
- User INSERT: ✅ Committed
- Profile INSERT: ✅ Committed (separate transaction)
- Function: ❌ Error logged
- Database: ✅ Unaffected

**No Atomicity:** Functions are NOT transactional with triggering operation.

---

## Concurrency

### Parallel Invocations

Multiple invocations of same function run concurrently:

```
Time     Invocation 1    Invocation 2    Invocation 3
T0       Start
T1       db_query        Start
T2                       db_query        Start
T3       Return          Return          db_query
T4                                       Return
```

**No Shared State:** Each invocation has isolated WASM instance.

---

### Database Trigger Concurrency

```sql
-- Transaction 1
BEGIN;
INSERT INTO users VALUES (...);  -- Triggers function
COMMIT;

-- Transaction 2 (concurrent)
BEGIN;
INSERT INTO users VALUES (...);  -- Triggers function
COMMIT;
```

**Behavior:**
- Both functions invoked concurrently
- No ordering guarantee
- Each sees its own inserted row

---

## Cold Start

**Definition:** Time to load WASM module and create instance

**Typical:** 50-100ms

**Components:**
1. WASM module compilation: 30ms
2. Instance creation: 10ms
3. Host function setup: 10ms

**Optimization (Future):**
- Module caching (reuse compiled modules)
- Warm instance pool
- Pre-instantiation on deploy

---

## Invocation Lifecycle

```
1. Trigger fires
   ├─ HTTP: Client request
   ├─ Database: WAL commit
   ├─ Schedule: Cron tick
   └─ Webhook: External POST

2. Invoker: Lookup function
   └─ Registry.get_by_name(name) or get_by_trigger(trigger)

3. Invoker: Load WASM module
   └─ Runtime.load_module(func.wasm_bytes)

4. Invoker: Create instance
   └─ Runtime.instantiate(module, host_functions)

5. Invoker: Call handler with timeout
   └─ tokio::timeout(func.config.timeout, call_handler(instance, payload))

6. Function: Execute
   ├─ Parse event payload
   ├─ Call host functions (db_query, log, etc.)
   └─ Return result

7. Invoker: Cleanup
   └─ Drop instance, free memory

8. Invoker: Log invocation
   └─ INSERT INTO function_invocations (...)

9. Invoker: Return result to caller
```

---

## Performance Characteristics

| Metric | Typical | Target |
|--------|---------|--------|
| Cold start | 80ms | < 100ms |
| Execution overhead | 3ms | < 5ms |
| Memory overhead | 8MB | < 10MB |
| Throughput | 100/s | > 50/s |
| Concurrent invocations | 100 | > 100 |

---

## Examples

### HTTP Function with Database Access
```javascript
export async function handler(event) {
  const { user_id } = event.body;
  
  const user = await db.query(
    "SELECT * FROM users WHERE id = $1",
    [user_id]
  );
  
  return {
    statusCode: 200,
    body: JSON.stringify(user[0])
  };
}
```

### Database Trigger with External API
```javascript
export async function onOrderCreated(event) {
  const order = event.new;
  
 log.info(`Processing order ${order.id}`);
  
  // Call external warehouse API
  const response = await http.fetch("https://warehouse.example.com/ship", {
    method: "POST",
    body: JSON.stringify(order)
  });
  
  // Update order status
  await db.query(
    "UPDATE orders SET status = 'shipped' WHERE id = $1",
    [order.id]
  );
  
  return { shipped: true };
}
```

### Scheduled Cleanup Job
```javascript
export async function cleanupExpiredSessions() {
  const result = await db.query(
    "DELETE FROM sessions WHERE expires_at < NOW()"
  );
  
  log.info(`Cleaned up ${result.rowCount} expired sessions`);
  
  return { cleaned: result.rowCount };
}
```
