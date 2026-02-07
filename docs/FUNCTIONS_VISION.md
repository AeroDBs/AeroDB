# Phase 12: Serverless Functions Vision

**Document Type:** Vision Statement  
**Phase:** 12 - Serverless Functions  
**Status:** Active

---

## Goal

Enable **custom server-side logic** via **WebAssembly (WASM) functions** that can be triggered by HTTP requests, database events, or schedules. Functions run in **sandboxed environments** with **resource limits**, providing a secure extension point for application logic without modifying AeroDB core.

---

## Philosophy

### 1. **Non-Deterministic by Design**
Functions are **explicitly non-deterministic**, living outside AeroDB's deterministic core:

| Layer | Determinism | Rationale |
|-------|-------------|-----------|
| Database | Fully Deterministic | Ensures correctness, replayability |
| WAL/MVCC | Fully Deterministic | State machine behavior |
| Functions | **Non-Deterministic** | User code, external APIs, undefined behavior |

**Key Principle:** Functions **read from** and **write to** the database, but are NOT part of the database's deterministic execution path.

---

### 2. **Sandboxed Execution**
Functions run in **WebAssembly** for isolation:
- Memory-safe (no buffer overflows)
- CPU-limited (timeouts enforced)
- Memory-limited (heap size caps)
- No filesystem access (host functions only)
- No network access (unless granted via host functions)

**Security Model:** Zero-trust. Functions cannot:
- Access other functions' memory
- Bypass RLS
- Crash the database
- Consume unbounded resources

---

### 3. **Fail-Open**
Function failures **do NOT crash the database**:
- Timeout → Error logged, function terminates
- Panic → Error logged, isolation maintained
- Infinite loop → Timeout, function killed

**Contrast with Database:**
- Database query error → Transaction rolled back
- Function error → Logged, caller notified, database unaffected

---

### 4. **Explicit Triggers**
Functions must be **explicitly invoked**:
- HTTP trigger: `/functions/v1/{name}` endpoint
- Database trigger: `INSERT`, `UPDATE`, `DELETE` on specific table
- Schedule trigger: Cron expression
- Webhook trigger: External HTTP POST (future)

**No Implicit Behavior:** Functions don't auto-run on arbitrary events.

---

### 5. **Host Functions**
Functions interact with AeroDB via **host functions** (imported from WASM):

```rust
// Available to WASM functions
fn db_query(sql: &str) -> Result<Vec<Row>>;
fn http_fetch(url: &str) -> Result<String>;
fn log(message: &str);
fn env(key: &str) -> Option<String>;
```

**Access Control:** Host functions respect RLS context passed to function invocation.

---

## Use Cases

### 1. **HTTP API Endpoints**
Custom business logic beyond CRUD:

```javascript
// POST /functions/v1/send-welcome-email
export async function handler(event) {
  const user = await db.query("SELECT * FROM users WHERE id = $1", [event.user_id]);
  await email.send(user.email, "Welcome to AeroDB!");
  return { status: "sent" };
}
```

---

### 2. **Database Triggers**
React to data changes:

```javascript
// Trigger: INSERT on orders table
export async function onOrderCreated(order) {
  // Update inventory
  await db.query("UPDATE products SET stock = stock - $1 WHERE id = $2", 
                 [order.quantity, order.product_id]);
  
  // Notify warehouse
  await http.post("https://warehouse.example.com/ship", order);
}
```

---

### 3. **Scheduled Jobs**
Background tasks:

```javascript
// Schedule: "0 2 * * *" (daily at 2 AM)
export async function cleanupExpiredSessions() {
  await db.query("DELETE FROM sessions WHERE expires_at < NOW()");
  console.log("Cleaned up expired sessions");
}
```

---

### 4. **Webhook Handlers**
Process external events:

```javascript
// Webhook: Stripe payment success
export async function handleStripeWebhook(event) {
  if (event.type === "payment_intent.succeeded") {
    await db.query("UPDATE subscriptions SET status = 'active' WHERE user_id = $1",
                   [event.metadata.user_id]);
  }
}
```

---

## Design Principles

### Simplicity Over Features
**Include:**
- WASM execution (wasmer/wasmtime)
- Resource limits (timeout, memory)
- Basic host functions (db_query, log)
- HTTP, database, schedule triggers

**Exclude (defer):**
- Multi-language support (JS, Python, etc.) - WASM only
- Stateful functions (cold start every invocation)
- Function-to-function calls
- Distributed tracing
- Custom dependencies/packages

**Rationale:** Focus on core execution. Language SDKs can compile to WASM.

---

### Security by Default
**Functions get minimal permissions:**
- RLS enforced on db_query
- No file system access
- No raw network sockets
- Environment variables are whitelisted
- Secrets via secure env vars only

**Privilege Escalation:** Service role context can be passed explicitly (admin functions).

---

### Observable
**All function invocations are logged:**
```json
{
  "function": "send-welcome-email",
  "trigger": "http",
  "duration_ms": 234,
  "memory_peak_mb": 12,
  "status": "success",
  "user_id": "uuid",
  "timestamp": "2026-02-06T09:00:00Z"
}
```

**Metrics:**
- Invocation count per function
- Success/failure rate
- Execution duration (p50, p95, p99)
- Memory usage

---

## Success Criteria

Phase 12 is successful when:

### Functional
1. ✅ Deploy WASM function via REST API
2. ✅ Invoke function via HTTP (`POST /functions/v1/{name}`)
3. ✅ Database trigger fires on INSERT/UPDATE/DELETE
4. ✅ Schedule trigger runs on cron expression
5. ✅ Function can query database (RLS enforced)
6. ✅ Function can log messages
7. ✅ Timeout enforced (default: 10s)
8. ✅ Memory limit enforced (default: 128MB)

### Non-Functional
1. ✅ Function cold start < 100ms (p95)
2. ✅ Execution overhead < 5ms (p95)
3. ✅ Handle 100 concurrent function invocations
4. ✅ Timeout kills runaway functions
5. ✅ Memory limit prevents OOM

### Security
1. ✅ Functions cannot bypass RLS
2. ✅ Functions cannot access filesystem
3. ✅ Functions cannot crash database
4. ✅ Secrets not leaked in logs

---

## Integration with AeroDB

### Authentication (Phase 8)
```rust
// Function inherits RLS context from caller
let context = extract_rls_context(&request)?;
functions.invoke("my-function", payload, &context)?;
```

### Database (Core)
```rust
// Host function: db_query
fn db_query(sql: &str, params: Vec<Value>) -> Result<Vec<Row>> {
    let query = parse_sql(sql)?;
    executor.execute(query, &rls_context)?  // ⬅️ RLS enforced
}
```

### Real-Time (Phase 10)
```javascript
// Function can emit events
export async function handler(event) {
  await db.query("INSERT INTO events ...");
  // Event automatically propagated to subscribers via Phase 10
}
```

---

## Examples

### Example 1: HTTP Function
```javascript
// functions/hello.js
export async function handler(request) {
  return {
    statusCode: 200,
    body: JSON.stringify({ message: `Hello, ${request.name}!` })
  };
}
```

**Deploy:**
```bash
curl -X POST /functions/v1/deploy \
  -H "Authorization: Bearer $JWT" \
  -F "name=hello" \
  -F "wasm=@hello.wasm"
```

**Invoke:**
```bash
curl -X POST /functions/v1/hello \
  -H "Content-Type: application/json" \
  -d '{"name": "World"}'

→ {"message": "Hello, World!"}
```

---

### Example 2: Database Trigger
```javascript
// Trigger: INSERT on users table
export async function onUserCreated(user) {
  // Automatically create user profile
  await db.query(
    "INSERT INTO profiles (user_id, display_name) VALUES ($1, $2)",
    [user.id, user.email.split('@')[0]]
  );
}
```

---

### Example 3: Scheduled Cleanup
```javascript
// Schedule: "0 3 * * *" (daily at 3 AM)
export async function cleanupOldData() {
  const result = await db.query(
    "DELETE FROM logs WHERE created_at < NOW() - INTERVAL '30 days'"
  );
  console.log(`Deleted ${result.rowCount} old log entries`);
}
```

---

## Non-Goals

**What Phase 12 does NOT do:**

### No State Between Invocations
Each invocation starts fresh (cold start).
**Future:** Warm instances, connection pooling

### No Direct Function-to-Function Calls
Functions cannot call each other directly.
**Workaround:** HTTP trigger or database as message queue

### No Custom npm/pip Packages
WASM only, no dependency management (yet).
**Future:** WASI support for packages

### No WebSocket from Functions
Functions cannot hold WebSocket connections.
**Workaround:** Write to database, Phase 10 broadcasts

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Runaway function (CPU) | Timeout enforcement (default: 10s) |
| Memory leak | Heap size limit (default: 128MB) |
| Database overload | Rate limiting, connection pooling |
| Secrets in logs | Redact sensitive env vars |
| Malicious WASM | Sandbox isolation, resource limits |
| Function versioning | Store WASM hash, rollback support (future) |

---

## Future Phases

### Phase 12.1: Language SDKs
- JavaScript/TypeScript SDK (compile to WASM)
- Python SDK (compile to WASM via RustPython)
- Rust SDK (native WASM)

### Phase 12.2: Stateful Functions
- Warm instance pooling
- Connection reuse
- In-memory caching

### Phase 12.3: Function Orchestration
- Function-to-function calls
- Workflow engine (step functions)
- Retry policies

---

## Stakeholders

- **Application developers:** Extend AeroDB with custom logic
- **DevOps:** Observable, resource-limited execution
- **Security team:** Sandboxed, RLS-enforced functions
- **AeroDB core:** Clean separation from deterministic core
