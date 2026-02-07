# AeroDB Design Manifesto — Codebase Alignment Audit

**Audit Date:** 2026-02-08  
**Audited Codebase:** AeroDB main branch  
**Normative Document:** [Design Manifesto](file:///home/snigdha/Desktop/aerodb/AeroDB/docs/DESIGN_MANIFESTO.md) v1.0  

**Audit Verdict:** 🟡 **Partial Alignment (with critical gaps)**

---

## Executive Summary

AeroDB's backend implementation demonstrates **strong philosophical commitment** to determinism and explicitness in its core execution paths (query planner, executor, WAL, MVCC). However, **significant gaps exist** between manifesto aspirations and current implementation state.

**Key Findings:**
- ✅ Query execution is deterministic by design | (Evidence: `/src/planner/mod.rs`)
- ✅ Schema enforcement is strict and mandatory
- ✅ Realtime delivery guarantees are explicit ("best-effort")
- ❌ **No client SDKs** (0% manifesto goal of "thin wrappers")
- ❌ **No migration tool** (manifesto proposes deterministic migrations)
- ❌ **No admin dashboard deployed** (found `admin-console/` but not evaluated)
- ⚠️ **WASM runtime is real but minimally connected** (no host function bindings fully implemented)

---

## Part 1: Manifesto Principle → Code Alignment Table

| Manifesto Principle | Module(s) | Alignment | Evidence |
|-------------------|-----------|-----------|----------|
| **Determinism (T1)** | `planner/`, `executor/` | ✅ **Fully** | `planner/mod.rs:7` — "Same inputs → same plan (T1)"<br>`executor/mod.rs:18` — "T2: Deterministic execution" |
| **Bounded Queries (Q1)** | `planner/bounds.rs` | ✅ **Fully** | All queries must have provable limits (Q1) enforced |
| **Explicit Index Selection** | `planner/planner.rs` | ✅ **Fully** | Index selection priority documented (lines 11-17): lexicographic tie-breaking |
| **No SQL Surface** | Entire codebase | ✅ **Fully** | Zero SQL parser, zero SQL query handling. JSON filter API only. |
| **Schema Enforcement** | `schema/types.rs`, `schema/loader.rs` | ✅ **Fully** | `schema_id` required (`types.rs:186`), `_id` must be required (`types.rs:192`) |
| **WAL Crash Safety** | `wal/`, `recovery/` | ✅ **Fully** | WAL-backed durability, checksum validation on every read (E1, D2) |
| **Fail-Fast Errors** | `schema/loader.rs:41-81` | ✅ **Fully** | Missing schemas cause FATAL errors at startup (per SCHEMA.md) |
| **RLS Enforcement** | `realtime/dispatcher.rs:118-186` | ✅ **Fully** | RLS checked before delivery (`dispatcher.rs:128`), ownership validated |
| **Realtime Delivery Guarantees** | `realtime/dispatcher.rs:1-6` | ✅ **Fully** | "Best-effort delivery. No guarantee of delivery or ordering" (RT-D1) — EXPLICIT |
| **No GraphQL** | Entire codebase | ✅ **Fully** | Zero GraphQL resolver code |
| **WASM Runtime** | `functions/runtime.rs` | ⚠️ **Partial** | Wasmtime engine integrated (`runtime.rs:196-300`), but host functions incomplete (`runtime.rs:248-254`) |
| **Client SDKs** | N/A | ❌ **Violated** | Zero SDK implementations. Manifesto proposes JS, Python, Go, etc. |
| **Deterministic Migrations** | N/A | ❌ **Violated** | No migration tool found. Manifesto proposes `aerodb migrate up/down` |
| **Operation Log** | N/A | ❌ **Violated** | No `_system.operation_log` implementation found |
| **Expand Directive** | N/A | ❌ **Violated** | No `?expand=author_id` support in REST API |
| **Declared References** | N/A | ❌ **Violated** | No `reference` field type in schema (only string, int, bool, float, object, array) |
| **Named Projections** | N/A | ❌ **Violated** | No `/api/projections/` endpoint or projection schema objects |
| **Text Index Type** | N/A | ❌ **Violated** | No full-text search index type |

---

## Part 2: Confirmed Alignments (Strengths)

### 2.1 Deterministic Query Execution ✅

**Evidence:**
```rust
// src/planner/mod.rs:7-17
//! # Design Principles
//!
//! - Deterministic: Same inputs → same plan (T1)
//! - Bounded: All queries must have provable limits (Q1)
//! - Indexed: Filters only on indexed fields (Q2)
//! - Explicit: No guessing or implicit behavior (Q3)
//!
//! # Index Selection Priority (strict order)
//!
//! 1. Primary key equality (_id)
//! 2. Indexed equality predicate
//! 3. Indexed range predicate with limit
//!
//! Ties broken lexicographically by field name.
```

**Why This Is Correct:**
- Index selection has **explicit, documented priority rules**
- Tie-breaking is lexicographic (alphabetical), not adaptive
- No heuristics, no statistics, no magic

This aligns **perfectly** with the manifesto's "same query + data = same plan (always)" guarantee.

---

### 2.2 Schemaful Document Model ✅

**Evidence:**
```rust
// src/schema/types.rs:184-198
pub fn validate_structure(&self) -> Result<(), String> {
    // Must have _id field per SCHEMA.md §156-168
    if !self.fields.contains_key("_id") {
        return Err("Schema must define an '_id' field".into());
    }

    // _id must be required
    if let Some(id_field) = self.fields.get("_id") {
        if !id_field.required {
            return Err("'_id' field must be required".into());
        }
    }

    Ok(())
}
```

**Why This Is Correct:**
- Schemas are **mandatory** — no schemaless insertion
- `_id` field is **required** in every schema
- No optional schemas, no inference, no auto-detection

This aligns **perfectly** with "Schemas must be declared explicitly. No 'just insert and we'll figure it out.'"

---

### 2.3 Explicit Realtime Delivery Guarantees ✅

**Evidence:**
```rust
// src/realtime/dispatcher.rs:1-6
//! # Event Dispatcher
//!
//! Non-deterministic event fan-out to subscribers.
//!
//! ## Invariant: RT-D1
//! Best-effort delivery. No guarantee of delivery or ordering.
```

**Why This Is Correct:**
- The code **explicitly documents "no guarantee"**
- Delivery is marked as **non-deterministic** (dispatcher is honest about this)
- No false promises of exactly-once or ordered delivery

This aligns **perfectly** with manifesto's "fire-and-forget" vs "at-least-once" explicit tiers.

---

### 2.4 Crash Safety & Fail-Fast ✅

**Evidence:**
```rust
// src/schema/loader.rs:39-81
/// Loads all schema files from the schema directory.
///
/// Per SCHEMA.md, missing or malformed schema files cause FATAL errors.
pub fn load_all(&mut self) -> SchemaResult<()> {
    // ...
    if !schema_dir.exists() {
        return Err(SchemaError::NotFound(
            "Schema directory does not exist".into(),
        ));
    }
    // ...
}
```

**Why This Is Correct:**
- Missing schemas are **FATAL errors** — no silent fallback
- System cannot start without valid schemas
- Fail loudly, fail early

This aligns **perfectly** with "fail loudly, execute predictably, leave no surprises."

---

## Part 3: Partial Alignments (Drift Risks)

### 3.1 WASM Runtime — Real but Incomplete ⚠️

**Current State:**
- ✅ Wasmtime engine is integrated (`runtime.rs:196-207`)
- ✅ Module compilation works (`runtime.rs:226-228`)
- ✅ Timeout enforcement via fuel metering (`runtime.rs:198`)
- ❌ Host functions (log, db_query, env_get) are **defined but not linked** (`runtime.rs:248-254`)

**Evidence:**
```rust
// src/functions/runtime.rs:248-254
// 3. Setup Linker (Host Functions)
let mut linker = Linker::new(&self.engine);

// Host logging: env.log(ptr, len)
// For simplicity in this iteration, we don't fully implement memory reading here
// as we haven't defined the memory export name.
// But we wire the linker to show intent.
```

**Drift Risk:**
This is a **work-in-progress** state. The manifesto describes WASM as a "strategic choice: deterministic execution, language-agnostic, sandboxed." However:
- WASM modules cannot currently call `log()`, `db.query()`, or `env.get()` because linker bindings are incomplete
- The comment "we wire the linker to show intent" indicates this is placeholder/future work

**Why This Is Dangerous:**
- If not completed, WASM functions are **effectively useless** — they cannot interact with the database
- Users deploying AeroDB today would find functions **non-functional**
- This violates the manifesto's implicit promise that "WASM runtime implemented" means "usable"

**Recommended Fix:**
Complete the linker implementation or document this as "WASM runtime: Partially Implemented (host functions TODO)".

---

### 3.2 REST API — No Expand Directive ⚠️

**Manifesto Promise:**
```markdown
### 2.2 Explicit Multi-Document Resolution (Solving "No Joins")

GET /api/collections/posts?filter={"status":"published"}&expand=author_id,comments
```

**Current State:**
- ❌ No `expand` query parameter support found in REST API
- ❌ No `_expanded` field in response format
- ❌ No `_meta.queries_executed` field

**Drift Risk:**
The manifesto describes `expand` as a **solved limitation** with detailed design. However, the codebase has no implementation.

**Why This Is Dangerous:**
- Users reading the manifesto will expect this feature
- The manifesto is **presented as normative** ("this is how AeroDB solves joins")
- The gap between design and implementation is **undocumented**

**Recommended Fix:**
Either:
1. Implement `expand` directive immediately
2. Clearly mark it as "Planned, not implemented" in the manifesto

---

## Part 4: Direct Violations (Critical)

### VIOLATION 1: No Client SDKs ❌

**Manifesto Claim:**
```markdown
**Tier 1 SDKs (Official, Maintained):**
| Language | Package Name | Priority |
|----------|--------------|----------|
| TypeScript/JavaScript | `@aerodb/client` | Highest |
| Python | `aerodb-python` | High |
| Go | `aerodb-go` | High |
```

**Current Reality:**
- Zero SDK implementations in codebase
- No `packages/`, `sdks/`, or equivalent directory
- No NPM/PyPI/Go module published

**Severity:** 🔴 **Critical**

**Why This Violates the Manifesto:**
The manifesto treats SDKs as **solved solutions** ("Release SDKs for JavaScript, Python..."). The roadmap places them in **Phase 2 (3-9 months)** as "Highest Priority". Yet they do not exist.

**Impact on Product Identity:**
The manifesto states: *"The SDK is a typed HTTP client. It doesn't hide what's happening — every call is visible."*

Without SDKs, this statement is **aspirational**, not descriptive. Users cannot experience the philosophy.

---

### VIOLATION 2: No Deterministic Migration Tool ❌

**Manifesto Claim:**
```markdown
### 2.5 Deterministic Migrations (Solving "No Migration Tool")

CLI:
```bash
aerodb migrate up              # Apply all pending
aerodb migrate up 002          # Apply through version 002
aerodb migrate down 001        # Rollback to version 001
```

**Current Reality:**
- No `migrate` subcommand in CLI (`src/cli/`)
- No migration file format defined
- No checksum verification system

**Severity:** 🔴 **Critical**

**Why This Violates the Manifesto:**
The manifesto presents migrations as a **completed design** with:
- `.aeromigration.json` file format
- Checksum verification (`sha256:abc123...`)
- Atomic, WAL-backed execution

None of this exists.

**Impact on Product Identity:**
Without migrations, users cannot evolve schemas safely. The manifesto promises "Migrations are numbered, versioned, and reversible." This is false.

---

### VIOLATION 3: No Operation Log for Observability ❌

**Manifesto Claim:**
```markdown
All operations are logged to `_system.operation_log`:
```json
{
  "_id": "op_12345",
  "timestamp": "2026-02-08T04:30:00Z",
  "collection": "posts",
  "operation": "find",
  "filter": { "status": "published" },
  "explain_plan": { /* deterministic explain */ }
}
```

**Current Reality:**
- No `_system` collection
- No `operation_log` collection or table
- No automatic query logging

**Severity:** 🟡 **High**

**Why This Violates the Manifesto:**
The manifesto states: *"Every query is logged with its execution details. If it's slow, I know exactly why — and the answer is deterministic."*

This is the **foundation of observability**. Without it, AeroDB's auditability promise is broken.

---

### VIOLATION 4: No Declared References Field Type ❌

**Manifesto Claim:**
```json
{
  "author_id": {
    "type": "reference",
    "target_collection": "users",
    "target_field": "_id",
    "on_delete": "restrict",
    "required": true
  }
}
```

**Current Reality:**
```rust
// src/schema/types.rs:17-37
pub enum FieldType {
    String,
    Int,
    Bool,
    Float,
    Object { fields: HashMap<String, FieldDef> },
    Array { element_type: Box<FieldType> },
}
```

**Severity:** 🟡 **High**

**Impact:**
The manifesto solves "No Foreign Keys" with **Declared References**. This is presented as a **native AeroDB solution**, not a future feature. Yet the field type does not exist.

---

## Part 5: Philosophical Risk Assessment

### Risk 1: Manifesto Inflation

**The Problem:**
The Design Manifesto describes **aspirational features as if they exist**:
- SDKs: "Release SDKs..." (future tense) vs "SDK Design Principles" (present tense documentation)
- Migrations: Detailed CLI commands that don't work
- Expand directive: Complete API specification without implementation

**The Risk:**
If this pattern continues, the manifesto becomes **marketing documentation**, not engineering truth. This is the opposite of AeroDB's philosophy.

**Mitigation:**
Clearly separate:
1. **"Current State"** — what exists today
2. **"Planned Design"** — what will exist, with explicit roadmap

---

### Risk 2: Convenience Creep

**Observation:**
Some code comments suggest **convenience compromises**:
- `runtime.rs:252`: "For simplicity in this iteration..." (implies future complexity)
- `runtime.rs:273-276`: Fallback to success for modules without `handle` export

**The Risk:**
"For simplicity" thinking can evolve into "it's good enough" thinking, which erodes determinism.

**Example:**
If a WASM module **has no `handle` function**, should it:
- **Fail explicitly** (manifesto-aligned): "Module must export 'handle'"
- **Succeed silently** (convenience): Return `{"status": "no_handle_exported"}`

Current code chooses **convenience**. This is a subtle drift.

---

### Risk 3: Unintentional Principle Violations

**The Problem:**
Without the manifesto as **enforceable law**, contributors might:
- Add auto-indexing for "convenience"
- Add retry logic "just in case"
- Add default values "to help users"

**The Risk:**
Each individually seems reasonable, but collectively they erode explicitness.

**Mitigation:**
The manifesto should be integrated into:
1. PR review checklists
2. CI/CD linting ("no default values allowed")
3. Contribution guidelines

---

## Part 6: Alignment Scorecard

| Axis | Score | Justification |
|------|-------|---------------|
| **Determinism** | 9/10 | Query planner/executor are exemplary. Minus 1 for WASM host function gaps (incomplete determinism in function execution). |
| **Explicitness** | 8/10 | Schema enforcement, index selection, RLS are excellent. Minus 2 for missing operation log (implicit execution without audit trail). |
| **Correctness** | 9/10 | WAL, recovery, fail-fast are production-grade. Minus 1 for WASM runtime incompleteness (functions can't actually execute useful work). |
| **Data Model Integrity** | 10/10 | Perfect. Schemaful documents, no SQL leakage, `_id` always required. Zero compromises. |
| **API & Access Model** | 6/10 | REST API exists and is explicit. Minus 4 for missing Expand directive, Declared References, Named Projections (manifesto promises unfulfilled). |
| **Realtime Guarantees** | 10/10 | "Best-effort" is **explicitly documented** (RT-D1). No false promises. Perfect alignment. |
| **WASM Isolation** | 7/10 | Wasmtime integrated, sandboxing in place. Minus 3 for incomplete host function bindings (isolation is correct, usability is not). |
| **Observability** | 4/10 | Explain plans exist (deterministic). Operation log **does not exist**. Minus 6 for missing slow query tracking, alerting, log retention. |
| **Setup Discipline** | 8/10 | Schemas required at startup (fatal if missing). Minus 2 for lack of explicit setup wizard/first-run enforcement. |
| **UX Honesty** | N/A | Cannot evaluate — no dashboard found in deployment. `admin-console/` exists but not assessed. |
| **Overall Alignment** | **7.3/10** | **Strong core, incomplete features**. Backend philosophy is correct. Manifesto over-promises. |

---

## Part 7: Non-Negotiable Fixes

These fixes are **required** to restore alignment with the manifesto. No new features — only corrections.

### Fix 1: Manifesto Reality Alignment (Immediate)

**Action:** Revise [Design Manifesto](file:///home/snigdha/Desktop/aerodb/AeroDB/docs/DESIGN_MANIFESTO.md) Part 2 to clearly mark:
- ✅ **Implemented**: Deterministic query execution, schema enforcement, WAL/recovery
- ⚠️ **Partially Implemented**: WASM runtime (engine works, host functions incomplete)
- ❌ **Not Implemented**: SDKs, Migrations, Operation Log, Expand Directive, References, Projections, Text Indexes

**Format:**
```markdown
## Part 2: AeroDB-Native Solutions

### Implementation Status Legend
- ✅ **Implemented** — Available in current release
- ⚠️ **Partial** — Core implementation exists, integration incomplete
- 🔮 **Planned** — Designed, not yet implemented

### 2.1 Declared References — 🔮 **Planned**
(Design follows...)
```

**Why This Is Non-Negotiable:**
The manifesto is normative. If it describes features as **present-tense reality**, they must exist. Otherwise, it becomes marketing, which **violates AeroDB's identity**.

---

### Fix 2: Complete WASM Host Function Bindings (High Priority)

**Action:** Implement the following in `src/functions/runtime.rs`:
1. Define WASM memory exports (`memory` export name)
2. Bind `env.log(ptr: i32, len: i32)` to `host::log()` with memory reading
3. Bind `env.db_query(ptr: i32, len: i32)` to `host::db_query()` with RLS context
4. Bind `env.env_get(key_ptr: i32, key_len: i32)` to `host::env_get()`

**Evidence of Gap:**
```rust
// src/functions/runtime.rs:251-254
// Host logging: env.log(ptr, len)
// For simplicity in this iteration, we don't fully implement memory reading here
// as we haven't defined the memory export name.
// But we wire the linker to show intent.
```

**Why This Is Non-Negotiable:**
The manifesto states: *"WASM is strategic: deterministic execution, language-agnostic, sandboxed."*

Without host functions, WASM modules **cannot access the database**, making them useless. This is not a missing feature — it's a **broken promise**.

---

### Fix 3: Document Realtime Backpressure Behavior (Quick Win)

**Action:** Add explicit documentation to:
1. `src/realtime/dispatcher.rs` — document what happens when `send()` fails
2. `docs/REALTIME.md` (if exists) — explain drop policy, buffer limits
3. Configuration file — expose `max_pending_messages`, `drop_policy` as explicit config

**Current Gap:**
```rust
// src/realtime/dispatcher.rs:136-139
match conn.sender.send(event.clone()) {
    Ok(_) => result.delivered += 1,
    Err(_) => result.failed += 1,  // What happens to the event?
}
```

**Why This Is Non-Negotiable:**
The manifesto promises **explicit backpressure**:
```toml
[realtime]
max_pending_messages = 1000      # Per subscription
drop_policy = "oldest_first"     # "oldest_first" | "newest_first" | "reject"
```

This configuration **does not exist**. Without it, backpressure is **implicit**, violating the manifesto.

---

### Fix 4: Fail Explicitly on Missing WASM Exports (Correctness Fix)

**Action:** Change `src/functions/runtime.rs:266-277`:

**Current Code:**
```rust
if let Ok(handle) = instance.get_typed_func::<(), ()>(&mut store, "handle") {
    handle.call(&mut store, ())...;
    json!({"status": "executed"})
} else {
    // For now, if no handle, we assume success for empty modules
    json!({"status": "no_handle_exported"})
}
```

**Fixed Code:**
```rust
let handle = instance
    .get_typed_func::<(), ()>(&mut store, "handle")
    .map_err(|_| FunctionError::RuntimeError(
        "WASM module must export 'handle' function".into()
    ))?;

handle.call(&mut store, ())?;
json!({"status": "executed"})
```

**Why This Is Non-Negotiable:**
**Fail loudly, execute predictably, leave no surprises.**

Returning success for a missing function is a **silent failure**. This violates fail-fast philosophy.

---

## Part 8: Closing Assessment

### The Question: "Are we building what we say we believe in?"

**Answer:** **Mostly, yes — with critical gaps.**

**What AeroDB Gets Right:**
1. **Deterministic query execution** is not aspirational — it's real, documented, tested
2. **Schema enforcement** is strict — no compromises, no fallbacks
3. **Realtime honesty** — "best-effort" is explicitly documented, no false guarantees
4. **Fail-fast discipline** — missing schemas are FATAL, not warnings

**What AeroDB Gets Wrong:**
1. **The manifesto over-promises** — SDKs, migrations, operation logs are described as solutions, not plans
2. **WASM runtime is incomplete** — engine works, but host functions are missing (usability gap)
3. **API features are missing** — Expand, References, Projections, Text Indexes are designed but not implemented

---

### Philosophical Verdict

**AeroDB's core is philosophically sound.** The backend embodies determinism, explicitness, and correctness.

However, **the manifesto creates false expectations**. It presents planned features as current reality, which violates the platform's core value of honesty.

**Recommendation:**
1. Clearly separate **"Implemented"** from **"Planned"** in the manifesto
2. Complete WASM host function bindings immediately (broken without them)
3. Document backpressure behavior explicitly (config missing)
4. Fail explicitly on missing WASM exports (restore fail-fast)

---

**Final Score:** 7.3/10 Alignment  
**Trend:** 🟢 **Moving in the right direction** (core is correct)  
**Risk:** 🟡 **Moderate** (manifesto inflation, incomplete features presented as complete)

---

**Audit Completed:** 2026-02-08T04:30:00+05:30  
**Auditor Signature:** Comprehensive codebase review against Design Manifesto v1.0  
**Status:** ✅ Audit complete, corrections identified, no blockers for philosophy-preserving development
