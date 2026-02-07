# Phase 12: Serverless Functions Architecture

**Document Type:** Technical Architecture  
**Phase:** 12 - Serverless Functions  
**Status:** Active

---

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Client Applications                      │
│                   (REST API, Cron Scheduler)                    │
└────────────────────────────┬────────────────────────────────────┘
                             │
                  ┌──────────▼──────────┐
                  │  Functions Module   │
                  │ (Invoker, Scheduler)│
                  └──────────┬──────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
┌───────▼────────┐  ┌────────▼────────┐  ┌───────▼────────┐
│WASM Runtime    │  │ Function        │  │    Database    │
│ (Sandbox)      │  │   Registry      │  │   (Executor)   │
└────────────────┘  └─────────────────┘  └────────────────┘
        │
┌───────▼────────┐
│  Host          │
│  Functions     │
│(db, log, http) │
└────────────────┘
```

---

## Component Architecture

### 1. Function Registry

**Purpose:** Store and index deployed functions

```rust
pub struct FunctionRegistry {
    functions: HashMap<Uuid, Function>,
    name_index: HashMap<String, Uuid>,
    trigger_index: HashMap<TriggerType, HashSet<Uuid>>,
}
```

**Operations:**
- `register(function)` - Add function to registry
- `get_by_name(name)` - Lookup by name
- `get_by_trigger(trigger)` - Find all functions for trigger type
- `unregister(id)` - Remove function

---

### 2. WASM Runtime

**Integration:** wasmer or wasmtime

```rust
pub struct WasmRuntime {
    engine: Engine,
    store: Store,
}

impl WasmRuntime {
    pub fn load_module(&self, wasm_bytes: &[u8]) -> Result<Module> {
        Module::new(&self.engine, wasm_bytes)
    }
    
    pub fn instantiate(&self, module: &Module) -> Result<Instance> {
        let imports = create_host_functions();
        Instance::new(&mut self.store, module, &imports)
    }
}
```

**Resource Limits:**
```rust
pub struct ResourceLimits {
    timeout: Duration,      // Default: 10s
    max_memory: usize,      // Default: 128MB
    max_stack: usize,       // Default: 1MB
}
```

---

### 3. Invoker

**Purpose:** Execute functions with resource limits

```rust
pub struct Invoker {
    runtime: WasmRuntime,
    registry: Arc<FunctionRegistry>,
    executor: Arc<Executor>,  // For database access
}

impl Invoker {
    pub async fn invoke(
        &self,
        function_name: &str,
        payload: Value,
        context: &RlsContext,
    ) -> Result<InvocationResult> {
        // 1. Lookup function
        let func = self.registry.get_by_name(function_name)?;
        
        // 2. Load WASM module
        let module = self.runtime.load_module(&func.wasm_bytes)?;
        
        // 3. Create instance with host functions
        let instance = self.runtime.instantiate(&module)?;
        
        // 4. Call handler with timeout
        let result = timeout(
            func.config.timeout,
            self.call_handler(instance, payload, context)
        ).await?;
        
        Ok(result)
    }
}
```

---

### 4. Scheduler

**Purpose:** Run cron-based jobs

```rust
pub struct FunctionScheduler {
    jobs: HashMap<Uuid, ScheduledJob>,
    invoker: Arc<Invoker>,
}

pub struct ScheduledJob {
    id: Uuid,
    function_id: Uuid,
    cron_expression: String,
    next_run: DateTime<Utc>,
    enabled: bool,
}

impl FunctionScheduler {
    pub async fn run(&self) {
        loop {
            let now = Utc::now();
            let due_jobs = self.get_due_jobs(now);
            
            for job in due_jobs {
                tokio::spawn(async move {
                    self.invoker.invoke(...).await;
                });
                
                self.update_next_run(&job);
            }
            
            sleep(Duration::from_secs(1)).await;
        }
    }
}
```

---

## Module Structure

```
src/functions/
├── mod.rs              # Module entry, exports
├── errors.rs           # Function-specific errors
├── function.rs         # Function model, metadata
├── trigger.rs          # Trigger types (HTTP, DB, Schedule)
├── registry.rs         # Function registry (in-memory index)
├── runtime.rs          # WASM runtime (wasmer integration)
├── invoker.rs          # Function execution with limits
├── scheduler.rs        # Cron job scheduler
└── host_functions.rs   # Host functions (db_query, log, etc.)
```

---

## Data Flow

### HTTP Trigger Flow

```
1. Client → REST API
   POST /functions/v1/my-function
   Headers: Authorization, Content-Type
   Body: {"key": "value"}

2. REST API → Auth Module
   Extract JWT → RlsContext

3. REST API → Invoker
   invoke("my-function", payload, context)

4. Invoker:
   a. Lookup function in registry
   b. Load WASM module
   c. Create instance with host functions
   d. Call exported `handler` function
   e. Enforce timeout (10s default)
   f. Enforce memory limit (128MB default)

5. Function (WASM):
   a. Parse payload
   b. Call host function: db_query(...)
   c. Call host function: log(...)
   d. Return result

6. Invoker → REST API
   Return result or error

7. REST API → Client
   200 OK {result} or 500 Error
```

---

### Database Trigger Flow

```
1. Database → WAL
   INSERT INTO users VALUES (...)

2. WAL → Event Log (Phase 10)
   Emit DatabaseEvent::Insert

3. Event Log → Function Trigger Dispatcher
   Check: Are there DB triggers for "users" table?

4. Trigger Dispatcher → Invoker
   For each matching function:
     invoke(function, {table: "users", op: "INSERT", row: ...})

5. Invoker executes function (same as HTTP flow)

6. Function result logged, errors don't crash database
```

**Non-Determinism Note:** Database triggers run **after** WAL commit, not during transaction. They are best-effort, not transactional.

---

### Schedule Trigger Flow

```
1. Scheduler (background thread):
   Loop every 1 second:
     a. Get current time
     b. Find jobs where next_run <= now
     c. Spawn task for each due job

2. Spawned Task → Invoker
   invoke(function, {timestamp: now})

3. Invoker executes function

4. Scheduler updates next_run based on cron expression
```

---

## Host Functions

Functions call out to AeroDB via imported host functions:

### db_query

```rust
#[host_function]
fn db_query(sql: String, params: Vec<Value>) -> Result<Vec<Row>> {
    // Parse SQL
    let query = parse_sql(&sql)?;
    
    // Execute with RLS context (from invocation)
    let result = executor.execute(query, &rls_context)?;
    
    Ok(result)
}
```

**WASM Side:**
```javascript
// Import from host
extern "C" {
    fn db_query(sql_ptr: *const u8, sql_len: usize) -> i32;
}

// Wrapper
function query(sql, params) {
    return JSON.parse(callHost(db_query, JSON.stringify({sql, params})));
}
```

---

### log

```rust
#[host_function]
fn log(level: String, message: String) {
    let level = match level.as_str() {
        "info" => log::Level::Info,
        "warn" => log::Level::Warn,
        "error" => log::Level::Error,
        _ => log::Level::Debug,
    };
    
    log::log!(level, "[Function] {}", message);
}
```

---

###  http_fetch (Future)

```rust
#[host_function]
async fn http_fetch(url: String) -> Result<String> {
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    Ok(response.text().await?)
}
```

**Security:** Whitelist allowed domains in function config

---

### env

```rust
#[host_function]
fn env(key: String) -> Option<String> {
    // Only return whitelisted env vars
    let allowed = ["DATABASE_URL", "API_KEY"];
    
    if allowed.contains(&key.as_str()) {
        std::env::var(&key).ok()
    } else {
        None
    }
}
```

---

## Resource Enforcement

### Timeout

```rust
pub async fn invoke_with_timeout(
    instance: Instance,
    payload: Value,
) -> Result<Value> {
    let timeout = Duration::from_secs(10);
    
    match tokio::time::timeout(timeout, call_handler(instance, payload)).await {
        Ok(result) => result,
        Err(_) => Err(FunctionError::Timeout),
    }
}
```

**Behavior:**
- Function exceeds 10s → Killed, error returned
- Partial work is NOT rolled back (non-transactional)

---

### Memory Limit

```rust
let mut store = Store::new(&engine);
store.limiter(|_| ResourceLimiter {
    memory_size: 128 * 1024 * 1024,  // 128MB
});

let instance = Instance::new(&mut store, &module, &imports)?;
```

**Behavior:**
- Function allocates > 128MB → Panic, error returned
- Memory is freed when function completes

---

### Stack Limit

```rust
let mut config = Config::new();
config.max_wasm_stack(1024 * 1024);  // 1MB
let engine = Engine::new(&config)?;
```

**Behavior:**
- Deep recursion → Stack overflow, error returned

---

## Error Handling

### Function Errors

```rust
pub enum FunctionError {
    NotFound,              // Function doesn't exist
    InvalidWasm,           // WASM compilation failure
    Timeout,               // Exceeded timeout
    OutOfMemory,           // Exceeded memory limit
    HostFunctionError,     // db_query failed, etc.
    RuntimePanic,          // WASM panic
}
```

**HTTP Mapping:**
```rust
impl From<FunctionError> for HttpStatus {
    fn from(err: FunctionError) -> HttpStatus {
        match err {
            FunctionError::NotFound => 404,
            FunctionError::InvalidWasm => 400,
            FunctionError::Timeout => 504,  // Gateway Timeout
            FunctionError::OutOfMemory => 507,
            FunctionError::HostFunctionError => 500,
            FunctionError::RuntimePanic => 500,
        }
    }
}
```

---

### Error Isolation

**Key Principle:** Function errors do NOT affect database or other functions.

```rust
pub async fn invoke_safe(&self, func: &Function, payload: Value) -> InvocationResult {
    match self.invoke_internal(func, payload).await {
        Ok(result) => InvocationResult::Success(result),
        Err(e) => {
            log::error!("Function '{}' failed: {}", func.name, e);
            InvocationResult::Error {
                code: e.code(),
                message: e.to_string(),
            }
        }
    }
    // Database is unaffected
}
```

---

## Integration Points

### With Authentication (Phase 8)

```rust
// HTTP trigger: RLS context from JWT
let context = extract_rls_context(&request)?;
invoker.invoke("my-function", payload, &context)?;

// Database trigger: Inherit context from triggering transaction
let context = transaction.rls_context();
invoker.invoke("on-user-created", user_data, &context)?;

// Schedule trigger: Use service role context
let context = RlsContext::service_role();
invoker.invoke("cleanup-job", empty_payload, &context)?;
```

---

### With REST API (Phase 9)

**Endpoints:**
```
POST   /functions/v1/deploy              # Deploy new function
GET    /functions/v1/{name}              # Get function metadata
DELETE /functions/v1/{name}              # Undeploy function
POST   /functions/v1/invoke/{name}       # Invoke function (HTTP trigger)
```

---

### With Real-Time (Phase 10)

Functions can write to database, events propagate automatically:

```javascript
// Function writes to DB
export async function handler() {
  await db.query("INSERT INTO notifications (user_id, message) VALUES (...) ");
  // Event emitted via Phase 10
}

// Client subscribes to notifications
supabase.channel('notifications')
  .on('INSERT', handleNotification)
  .subscribe();
```

---

## ObservabilityMetrics
```
functions_invocations_total{name, trigger, status}
functions_duration_seconds{name}
functions_memory_bytes{name}
functions_timeout_total{name}
```

### Logs
```json
{
  "level": "INFO",
  "service": "functions",
  "operation": "invoke",
  "function": "send-email",
  "trigger": "http",
  "duration_ms": 234,
  "memory_mb": 12,
  "status": "success",
  "user_id": "uuid"
}
```

### Alerts
- Timeout rate > 10%
- Error rate > 5%
- Memory usage > 90% of limit
