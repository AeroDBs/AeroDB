# Phase 12: Function Model

**Document Type:** Data Model Specification  
**Phase:** 12 - Serverless Functions  
**Status:** Active

---

## Function Structure

```rust
pub struct Function {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub trigger: TriggerType,
    pub wasm_hash: String,          // SHA-256 of WASM bytes
    pub wasm_bytes: Vec<u8>,        // Raw WASM binary
    pub config: FunctionConfig,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct FunctionConfig {
    pub timeout: Duration,          // Default: 10s
    pub max_memory: usize,          // Default: 128MB
    pub environment: HashMap<String, String>,
}
```

---

## Fields

### id
- **Type:** UUID v4
- **Purpose:** Primary key
- **Immutable:** Yes

### name
- **Type:** String
- **Constraints:**
  - Unique across all functions
  - 1-64 characters
  - Lowercase alphanumeric + hyphens + underscores
  - Must start with letter
- **Regex:** `^[a-z][a-z0-9_-]*$`

**Examples:**
```
✅ send_welcome_email
✅ cleanup-old-data
✅ process_payment
❌ SendEmail (uppercase)
❌ _cleanup (starts with underscore)
❌ send email (space)
```

### trigger
- **Type:** Enum (TriggerType)
- **Purpose:** How function is invoked
- **See:** FUNCTIONS_EXECUTION_MODEL.md for details

### wasm_hash
- **Type:** String (hex SHA-256)
- **Purpose:** Verify WASM integrity, detect changes
- **Calculate:** `SHA256(wasm_bytes)`

### wasm_bytes
- **Type:** Binary blob
- **Purpose:** Compiled WASM module
- **Size:** Typically 10KB-1MB (limit: 10MB)
- **Storage:** In-memory for fast loading, disk backup

### config
Runtime configuration for this function

#### timeout
- **Default:** 10 seconds
- **Range:** 1s - 300s (5 minutes)
- **Enforcement:** Tokio timeout, kills function if exceeded

#### max_memory
- **Default:** 128 MB
- **Range:** 16MB - 512MB
- **Enforcement:** WASM store limiter

#### environment
- **Type:** HashMap<String, String>
- **Purpose:** Environment variables accessible to function
- **Security:** Redact secrets in logs
- **Example:**
  ```json
  {
    "DATABASE_URL": "postgres://...",
    "API_KEY": "sk_***",
    "ENV": "production"
  }
  ```

### enabled
- **Type:** Boolean
- **Purpose:** Disable without deleting
- **Default:** true
- **Behavior:** Disabled functions return 404 on invocation

---

## Trigger Types

```rust
pub enum TriggerType {
    Http,
    Database {
        table: String,
        operation: DatabaseOperation,  // INSERT | UPDATE | DELETE
    },
    Schedule {
        cron: String,  // "0 */6 * * *"
    },
    Webhook {
        secret: String,
    },
}

pub enum DatabaseOperation {
    Insert,
    Update,
    Delete,
}
```

See FUNCTIONS_EXECUTION_MODEL.md for trigger details.

---

## Function Lifecycle

### Deploy
```
1. Client uploads WASM binary
2. Server validates WASM (can compile?)
3. Server calculates hash
4. Server stores function in registry
5. Server returns function ID
```

### Invoke
```
1. Trigger fires (HTTP, DB, Schedule)
2. Lookup function by name or trigger
3. Load WASM module
4. Create instance with host functions
5. Call handler, enforce limits
6. Return result or error
```

### Update
```
1. Client uploads new WASM binary
2. Server recomputes hash
3. Server updates wasm_bytes, wasm_hash
4. Old invocations use old WASM (no mid-flight updates)
```

### Delete
```
1. Client deletes function
2. Server removes from registry
3. In-flight invocations complete
4. New invocations return 404
```

---

## Function Metadata Collection

```sql
CREATE COLLECTION functions (
    id           TEXT PRIMARY KEY,
    name         TEXT UNIQUE NOT NULL,
    description  TEXT,
    trigger_type TEXT NOT NULL,  -- JSON-serialized TriggerType
    wasm_hash    TEXT NOT NULL,
    wasm_data    BYTEA NOT NULL,  -- WASM binary
    config       JSONB NOT NULL,
    enabled      BOOLEAN DEFAULT TRUE,
    created_at   TIMESTAMP NOT NULL,
    updated_at   TIMESTAMP NOT NULL
);

CREATE INDEX idx_functions_name ON functions(name);
CREATE INDEX idx_functions_trigger ON functions(trigger_type);
```

**Example Row:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "send_welcome_email",
  "description": "Send welcome email to new users",
  "trigger_type": "{\"Database\":{\"table\":\"users\",\"operation\":\"Insert\"}}",
  "wasm_hash": "a7ffc6f8bf1ed76651c14756a061d662...",
  "wasm_data": "<binary>",
  "config": {
    "timeout": 10,
    "max_memory": 134217728,
    "environment": {
      "SMTP_HOST": "smtp.example.com"
    }
  },
  "enabled": true,
  "created_at": "2026-02-06T09:00:00Z",
  "updated_at": "2026-02-06T09:00:00Z"
}
```

---

## Invocation Log Collection

Track every function invocation for observability.

```sql
CREATE COLLECTION function_invocations (
    id            TEXT PRIMARY KEY,
    function_id   TEXT NOT NULL REFERENCES functions(id),
    trigger       TEXT NOT NULL,  -- "http" | "database" | "schedule"
    status        TEXT NOT NULL,  -- "success" | "error" | "timeout"
    duration_ms   INTEGER,
    memory_peak   INTEGER,        -- Bytes
    error_message TEXT,
    user_id       TEXT,           -- If invoked via HTTP with auth
    timestamp     TIMESTAMP NOT NULL
);

CREATE INDEX idx_invocations_function ON function_invocations(function_id);
CREATE INDEX idx_invocations_timestamp ON function_invocations(timestamp);
```

**Example Row:**
```json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "function_id": "550e8400-e29b-41d4-a716-446655440000",
  "trigger": "database",
  "status": "success",
  "duration_ms": 234,
  "memory_peak": 12582912,  // 12MB
  "error_message": null,
  "user_id": null,
  "timestamp": "2026-02-06T09:15:00Z"
}
```

---

## Function Operations

### Deploy Function

**Request:**
```http
POST /functions/v1/deploy
Authorization: Bearer <JWT>
Content-Type: multipart/form-data

name: send_welcome_email
description: Send email to new users
trigger: {"Database": {"table": "users", "operation": "Insert"}}
wasm: <binary file>
config: {"timeout": 15, "max_memory": 134217728}
```

**Response:**
```http
HTTP/1.1 201 Created
Content-Type: application/json

{
  "id": "550e8400-...",
  "name": "send_welcome_email",
  "wasm_hash": "a7ffc6f8...",
  "created_at": "2026-02-06T09:00:00Z"
}
```

**Errors:**
- `400 Bad Request` - Invalid WASM or configuration
- `409 Conflict` - Function name already exists
- `413 Payload Too Large` - WASM exceeds 10MB

---

### Get Function

**Request:**
```http
GET /functions/v1/send_welcome_email
Authorization: Bearer <JWT>
```

**Response:**
```http
HTTP/1.1 200 OK

{
  "id": "550e8400-...",
  "name": "send_welcome_email",
  "description": "Send email to new users",
  "trigger": {"Database": {"table": "users", "operation": "Insert"}},
  "config": {"timeout": 15, "max_memory": 134217728},
  "enabled": true,
  "invocation_count": 1234,
  "last_invocation": "2026-02-06T09:15:00Z"
}
```

**Note:** WASM binary NOT returned (too large), only hash

---

### Update Function

**Request:**
```http
PATCH /functions/v1/send_welcome_email
Authorization: Bearer <JWT>
Content-Type: application/json

{
  "config": {
    "timeout": 20
  }
}
```

**Response:**
```http
HTTP/1.1 200 OK

{
  "id": "550e8400-...",
  "config": {"timeout": 20, "max_memory": 134217728},
  "updated_at": "2026-02-06T10:00:00Z"
}
```

---

### Delete Function

**Request:**
```http
DELETE /functions/v1/send_welcome_email
Authorization: Bearer <JWT>
```

**Response:**
```http
HTTP/1.1 204 No Content
```

---

### Invoke Function (HTTP Trigger)

**Request:**
```http
POST /functions/v1/invoke/send_welcome_email
Authorization: Bearer <JWT>
Content-Type: application/json

{
  "email": "user@example.com",
  "name": "Alice"
}
```

**Response (Success):**
```http
HTTP/1.1 200 OK
Content-Type: application/json

{
  "status": "sent",
  "message_id": "abc123"
}
```

**Response (Error):**
```http
HTTP/1.1 500 Internal Server Error

{
  "error": "Function error",
  "message": "SMTP connection failed",
  "code": "HOST_FUNCTION_ERROR"
}
```

---

## Validation Rules

### WASM Validation
```rust
pub fn validate_wasm(bytes: &[u8]) -> Result<()> {
    // Must compile
    let engine = Engine::default();
    Module::new(&engine, bytes)?;
    
    // Size limit
    if bytes.len() > 10 * 1024 * 1024 {
        return Err(FunctionError::WasmTooLarge);
    }
    
    Ok(())
}
```

### Name Validation
```rust
pub fn validate_function_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 64 {
        return Err(FunctionError::InvalidName("Length must be 1-64"));
    }
    
    let regex = Regex::new(r"^[a-z][a-z0-9_-]*$").unwrap();
    if !regex.is_match(name) {
        return Err(FunctionError::InvalidName("Invalid format"));
    }
    
    Ok(())
}
```

---

## Invariants

### FUNC-I1: Name Uniqueness
> **No two functions have the same name**

**Enforcement:** UNIQUE constraint on `name` field

### FUNC-I2: Hash Integrity
> **wasm_hash = SHA256(wasm_bytes)**

**Verification:** Recalculate hash on load, compare

### FUNC-I3: Resource Limits Enforced
> **All invocations respect timeout and memory limits**

**Enforcement:** Tokio timeout + WASM store limiter

---

## Examples

### HTTP Function
```rust
Function {
    name: "get_user_stats",
    trigger: TriggerType::Http,
    config: FunctionConfig {
        timeout: Duration::from_secs(5),
        max_memory: 64 * 1024 * 1024,  // 64MB
        ...
    },
}
```

### Database Trigger
```rust
Function {
    name: "on_order_created",
    trigger: TriggerType::Database {
        table: "orders",
        operation: DatabaseOperation::Insert,
    },
    config: FunctionConfig::default(),
}
```

### Scheduled Job
```rust
Function {
    name: "cleanup_old_logs",
    trigger: TriggerType::Schedule {
        cron: "0 3 * * *",  // Daily at 3 AM
    },
    config: FunctionConfig {
        timeout: Duration::from_secs(300),  // 5 minutes
        ...
    },
}
```
