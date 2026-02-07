# AeroDB: Overcoming Limitations — A Design Manifesto

> **Governing Principles**
> - Determinism over magic
> - Explicitness over convenience
> - Correctness over flexibility

This document addresses every limitation identified in the "AeroDB vs Supabase" comparison, with explicit decisions and AeroDB-native solutions where applicable.

---

## Part 1: Limitation → Decision Matrix

### Legend
- **❌ Reject**: Fundamentally incompatible with AeroDB's philosophy
- **⚠️ Reframe**: A trade-off that must be made explicit (not a bug, a feature)
- **✅ Solve**: A solvable limitation with an AeroDB-native design

---

### 1.1 Data Model Limitations

| Limitation | Decision | Rationale |
|------------|----------|-----------|
| No SQL query language | ❌ Reject | SQL introduces non-deterministic query planning. AeroDB's operation pipeline is explicit by design. |
| No foreign keys | ✅ Solve | Implement **Declared References** — explicit, schema-declared relationships with deterministic resolution. |
| No joins | ✅ Solve | Implement **Explicit Multi-Document Resolution** — client specifies exactly what to fetch, no hidden queries. |
| No views | ✅ Solve | Implement **Named Projections** — schema-declared, immutable view definitions stored as first-class objects. |
| No stored procedures | ⚠️ Reframe | WASM functions ARE stored procedures. They run server-side with explicit invocation. Rename: "Database Functions". |
| No full-text search | ✅ Solve | Implement **Text Index Type** — explicit index declaration, deterministic ranking formula. |
| No geospatial (PostGIS) | ⚠️ Reframe | Not core to AeroDB's mission. Document as "extension point" for future WASM-based geo module. |
| No vector search (pgvector) | ⚠️ Reframe | Complex feature. Add to long-term roadmap as **Vector Index Type** with explicit similarity metric. |
| No partitioning | ⚠️ Reframe | AeroDB uses collection-level sharding model. Different approach, equally valid. Document equivalence. |
| Closed extensibility | ✅ Solve | Implement **WASM Extension Registry** — sandboxed extensions with explicit capability declarations. |
| Different index types | ⚠️ Reframe | AeroDB's B-tree is sufficient for 90% of use cases. Add **Configurable Index Types** to mid-term roadmap. |

---

### 1.2 Query & API Limitations

| Limitation | Decision | Rationale |
|------------|----------|-----------|
| No SQL surface | ❌ Reject | SQL's declarative nature conflicts with explicit execution. AeroDB uses Operation Pipeline. |
| No GraphQL API | ❌ Reject | GraphQL introduces resolver magic and N+1 ambiguity. Conflicts with determinism. |
| No nested resource fetching | ✅ Solve | Implement **Expand Directive** — `?expand=author` with explicit depth limits and deterministic fetch order. |
| No upsert primitive | ✅ Solve | Implement **Upsert Operation** — explicit `PUT` with `on_conflict` behavior declared in request. |
| No OpenAPI docs auto-generation | ✅ Solve | Generate OpenAPI from schema definitions — deterministic, versioned documentation. |
| Limited REST expressiveness | ✅ Solve | Expand filter operators, add bulk operations, improve error messages. Keep explicit semantics. |
| No API versioning strategy | ✅ Solve | Implement **Schema-Coupled API Versions** — API version locked to schema version for auditability. |

---

### 1.3 Authentication & Authorization Limitations

| Limitation | Decision | Rationale |
|------------|----------|-----------|
| No phone auth (SMS) | ✅ Solve | Implement SMS provider integration (Twilio/Vonage) with explicit rate limits and audit trails. |
| No SSO (SAML) | ✅ Solve | Implement SAML 2.0 with explicit session binding and deterministic assertion validation. |
| Basic role-based access | ✅ Solve | Implement **Permission Matrix** — explicit role → operation → collection mappings, no inheritance magic. |
| No anonymous sign-in | ✅ Solve | Implement **Ephemeral Sessions** — time-limited anonymous identities with explicit conversion path. |
| Limited server-side auth | ✅ Solve | Implement PKCE flow, cookie-based sessions for SSR frameworks with explicit token refresh. |
| RLS untested at scale | ✅ Solve | Add comprehensive benchmarks, publish performance characteristics with explicit guarantees. |
| Limited OAuth providers | ✅ Solve | Add Apple, Twitter, Facebook, LinkedIn — maintain explicit configuration per provider. |

---

### 1.4 File Storage Limitations

| Limitation | Decision | Rationale |
|------------|----------|-----------|
| No CDN integration | ✅ Solve | Implement **CDN Origin Mode** — AeroDB acts as origin, explicit cache headers per bucket. |
| No image transformation | ⚠️ Reframe | Add **Transform-on-Request** with explicit format/size params. No hidden optimization. |
| No resumable uploads | ✅ Solve | Implement TUS protocol with explicit chunk acknowledgment and deterministic resume. |
| No webhooks on file events | ✅ Solve | Implement **Storage Triggers** — explicit function bindings per bucket event. |
| No file size documentation | ✅ Solve | Define explicit limits by tier, document in API specification. |
| No file versioning | ⚠️ Reframe | Implement **Explicit Versioning Mode** per bucket — opt-in, with deterministic version ordering. |

---

### 1.5 Realtime & Eventing Limitations

| Limitation | Decision | Rationale |
|------------|----------|-----------|
| Untested scalability | ✅ Solve | Add load testing, publish explicit connection limits and throughput guarantees. |
| Best-effort delivery | ⚠️ Reframe | **Explicit Delivery Tiers**: "fire-and-forget" (default), "at-least-once" (with ack), no "exactly-once" claims. |
| No backpressure documentation | ✅ Solve | Implement and document **Explicit Backpressure** — client-side buffer limits, server drop policy. |
| No connection pooling docs | ✅ Solve | Document connection management strategy with explicit limits per tenant. |
| No offline support | ⚠️ Reframe | Offline is client responsibility. Provide **Sync Cursor** for client-side replay on reconnect. |

---

### 1.6 Edge/Serverless Functions Limitations

| Limitation | Decision | Rationale |
|------------|----------|-----------|
| WASM-only (no npm) | ⚠️ Reframe | WASM is strategic: deterministic execution, language-agnostic, sandboxed. This is a feature. |
| Limited debugging | ✅ Solve | Implement **Function Inspector** — step-through debugging via source maps + WASM debug info. |
| No streaming responses | ✅ Solve | Implement streaming with explicit chunked transfer and deterministic flush points. |
| Cold start overhead | ⚠️ Reframe | Document cold start times explicitly. Add **Warm Pool** configuration for latency-sensitive functions. |
| Limited language ecosystem | ⚠️ Reframe | Provide official WASM SDKs for Rust, Go, AssemblyScript. Community can add more. |

---

### 1.7 Replication & High Availability Limitations

| Limitation | Decision | Rationale |
|------------|----------|-----------|
| No multi-region | ⚠️ Reframe | Multi-region adds CAP complexity. Offer **Region Affinity** mode for explicit geo-locality instead. |
| No auto-scaling | ⚠️ Reframe | Auto-scaling is magic. Provide **Capacity Metrics** for external orchestrators (K8s, Nomad). |
| No connection pooling docs | ✅ Solve | Implement and document built-in connection pooling with explicit pool size limits. |
| Manual failover only | ⚠️ Reframe | Explicit failover is intentional. Add **Failover Advisor** — recommendations without automatic action. |

---

### 1.8 Backup & Restore Limitations

| Limitation | Decision | Rationale |
|------------|----------|-----------|
| Manual backup only | ✅ Solve | Implement **Backup Scheduler** via cron + CLI integration, explicit retention policy. |
| No managed backup storage | ⚠️ Reframe | Self-hosted = user manages storage. Provide **S3 Backup Sink** with explicit bucket configuration. |
| No retention policy | ✅ Solve | Implement configurable retention with explicit age-based and count-based policies. |

---

### 1.9 Observability & Debugging Limitations

| Limitation | Decision | Rationale |
|------------|----------|-----------|
| Limited dashboard coverage | ✅ Solve | Expand dashboard to 90%+ backend coverage with explicit metric definitions. |
| No alerting | ✅ Solve | Implement **Alert Rules** — explicit threshold definitions, webhook notifications. |
| No slow query tooling | ✅ Solve | Implement **Operation Log** with duration tracking, explicit slow threshold configuration. |
| No log retention docs | ✅ Solve | Document retention policy, implement configurable log rotation. |
| No real-time inspector | ✅ Solve | Add **Realtime Debug Panel** to dashboard — message inspector with explicit filtering. |

---

### 1.10 Admin Dashboard / UI Limitations

| Limitation | Decision | Rationale |
|------------|----------|-----------|
| 42% backend coverage | ✅ Solve | Systematic dashboard expansion. Each backend feature gets a corresponding UI. |
| No API documentation | ✅ Solve | Auto-generate API docs from schema, embed in dashboard. |
| No query builder | ✅ Solve | Implement **Visual Query Builder** with explicit operation preview before execution. |
| Limited function editor | ✅ Solve | Implement **Function IDE** — code editor, deploy, logs, version history. |
| No logs viewer streaming | ✅ Solve | Implement real-time log streaming with explicit filters. |

---

### 1.11 Developer Experience Limitations

| Limitation | Decision | Rationale |
|------------|----------|-----------|
| No client SDKs | ✅ Solve | Release SDKs for JavaScript, Python, Swift, Kotlin, Go — with explicit API contracts. |
| No migration tool | ✅ Solve | Implement **Deterministic Migrations** — versioned, ordered, reversible with explicit rollback. |
| No TypeScript types | ✅ Solve | Auto-generate TypeScript interfaces from schema definitions. |
| Limited documentation | ✅ Solve | Create comprehensive docs site with tutorials, API reference, architecture guides. |
| Minimal examples/templates | ✅ Solve | Build example apps for common frameworks (Next.js, Nuxt, Flutter, etc.). |
| Custom schema format | ⚠️ Reframe | AeroDB's schema format IS the standard. Provide importers from JSON Schema, Prisma, etc. |
| No VS Code extension | ✅ Solve | Build extension: schema validation, autocomplete, inline docs. |

---

### 1.12 Operational & Deployment Limitations

| Limitation | Decision | Rationale |
|------------|----------|-----------|
| No managed cloud | ⚠️ Reframe | Self-hosting is the identity. "AeroDB Cloud" could exist but won't compromise philosophy. |
| High operational overhead | ✅ Solve | Single-binary deployment, Docker/K8s templates, runbook documentation. |
| Unknown scalability | ✅ Solve | Publish benchmarks with explicit hardware specs, load profiles, and performance guarantees. |
| Custom data export format | ✅ Solve | Add **Portable Export** — standard JSON-Lines format with schema header. |

---

### 1.13 Compliance & Security Limitations

| Limitation | Decision | Rationale |
|------------|----------|-----------|
| No SOC 2/HIPAA (managed) | ⚠️ Reframe | Self-hosted = user's compliance responsibility. Provide compliance audit checklist. |
| No encryption at rest docs | ✅ Solve | Document storage-level encryption options, provide explicit configuration guidance. |
| No security audits | ✅ Solve | Commission third-party security audit, publish findings. |

---

## Part 2: AeroDB-Native Solutions

### 2.1 Declared References (Solving "No Foreign Keys")

**Design Overview:**
```json
{
  "collection": "posts",
  "schema_version": "v1",
  "fields": {
    "_id": { "type": "string", "required": true },
    "title": { "type": "string", "required": true },
    "author_id": {
      "type": "reference",
      "target_collection": "users",
      "target_field": "_id",
      "on_delete": "restrict",  // "cascade" | "set_null" | "restrict"
      "required": true
    }
  }
}
```

**Why It Fits AeroDB:**
- References are declared explicitly in schema — no hidden relationship discovery
- `on_delete` behavior is explicit, not inferred
- Validation happens at write time with deterministic error messages
- No cascading magic — "cascade" means exactly what it says, logged in WAL

**What It Intentionally Does NOT Do:**
- No automatic join queries (use Expand Directive separately)
- No cross-collection transactions (each collection is an isolation boundary)
- No bi-directional references (explicitly declare on both sides if needed)

**User Mental Model:**
> "If I declare a reference, AeroDB ensures the target exists before accepting my write. The on_delete behavior is exactly what I configured — nothing more, nothing less."

---

### 2.2 Explicit Multi-Document Resolution (Solving "No Joins")

**Design Overview:**
```http
GET /api/collections/posts?filter={"status":"published"}&expand=author_id,comments
```

The `expand` directive explicitly fetches related documents:

```json
{
  "data": [
    {
      "_id": "post_1",
      "title": "Hello World",
      "author_id": "user_42",
      "_expanded": {
        "author_id": { "_id": "user_42", "name": "Alice" },
        "comments": [
          { "_id": "comment_1", "text": "Great post!" }
        ]
      }
    }
  ],
  "_meta": {
    "queries_executed": 3,
    "documents_fetched": { "posts": 1, "users": 1, "comments": 2 }
  }
}
```

**Why It Fits AeroDB:**
- Client explicitly requests what to expand — no N+1 surprise
- `_meta` exposes exact query counts — complete visibility
- Expansion depth is limited and configurable (default: 1 level)
- Each sub-query uses the same deterministic execution as top-level

**What It Intentionally Does NOT Do:**
- No implicit joins (you must say `expand=X`)
- No unlimited depth (prevent runaway queries)
- No aggregations across expanded data (fetch, then compute client-side or via function)

**User Mental Model:**
> "Expand is a convenience, not magic. I know exactly how many queries will run, and I see the cost in _meta."

---

### 2.3 Named Projections (Solving "No Views")

**Design Overview:**
```json
{
  "projection": "public_user_profile",
  "source_collection": "users",
  "schema_version": "v1",
  "fields": ["_id", "display_name", "avatar_url", "created_at"],
  "filter": { "is_public": true },
  "immutable": true
}
```

Accessible via:
```http
GET /api/projections/public_user_profile?filter={"created_at":{"$gt":"2026-01-01"}}
```

**Why It Fits AeroDB:**
- Projections are schema objects — versioned, immutable once deployed
- Underlying query is explicit and inspectable
- No "view refresh" magic — projections are computed on read
- RLS applies to source collection, filters through to projection

**What It Intentionally Does NOT Do:**
- No materialized views (every read is computed)
- No cross-collection projections (one source, explicit)
- No projection composition (keep it flat and debuggable)

**User Mental Model:**
> "A projection is a saved query with a name. It's just a shortcut — nothing is hidden about how it works."

---

### 2.4 Text Index Type (Solving "No Full-Text Search")

**Design Overview:**
```json
{
  "collection": "articles",
  "indexes": [
    {
      "name": "content_search",
      "type": "text",
      "fields": ["title", "body"],
      "config": {
        "language": "english",
        "weights": { "title": 2.0, "body": 1.0 },
        "ranking": "bm25"  // Explicit ranking algorithm
      }
    }
  ]
}
```

Query:
```http
GET /api/collections/articles?text_search={"query":"rust database","index":"content_search"}
```

**Why It Fits AeroDB:**
- Index type is explicit — no auto-detection
- Ranking algorithm is declared — BM25, TF-IDF, or custom
- Weights are explicit — no hidden boosting heuristics
- Results include `_score` with explanation if requested

**What It Intentionally Does NOT Do:**
- No auto-indexing of text fields (you must opt-in)
- No fuzzy matching by default (explicit config required)
- No semantic/AI search (different feature, different index type)

**User Mental Model:**
> "Full-text search is an index type I configure. The ranking formula is exactly what I specified — reproducible and auditable."

---

### 2.5 Deterministic Migrations (Solving "No Migration Tool")

**Design Overview:**

Migration files in `./migrations/`:
```
./migrations/
├── 001_create_users.aeromigration.json
├── 002_add_email_verified.aeromigration.json
└── 003_create_posts.aeromigration.json
```

Format:
```json
{
  "version": "002",
  "name": "add_email_verified",
  "checksum": "sha256:abc123...",
  "up": {
    "type": "add_field",
    "collection": "users",
    "field": {
      "name": "email_verified",
      "type": "boolean",
      "default": false
    }
  },
  "down": {
    "type": "remove_field",
    "collection": "users",
    "field": "email_verified"
  },
  "applied_at": null
}
```

CLI:
```bash
aerodb migrate up              # Apply all pending
aerodb migrate up 002          # Apply through version 002
aerodb migrate down 001        # Rollback to version 001
aerodb migrate status          # Show applied migrations
aerodb migrate verify          # Check checksums match
```

**Why It Fits AeroDB:**
- Migrations are ordered by explicit version numbers
- Checksums ensure migrations haven't been modified post-apply
- `up` and `down` are explicit operations — no SQL guessing
- Each migration is atomic and WAL-backed

**What It Intentionally Does NOT Do:**
- No auto-generated migrations (you write what you mean)
- No diff-based migration inference (explicit declarations only)
- No partial migrations (all or nothing per migration file)

**User Mental Model:**
> "Migrations are numbered, versioned, and reversible. I can always rollback because the 'down' is explicitly defined."

---

### 2.6 Operation Log & Slow Query Detection (Solving Observability Gaps)

**Design Overview:**

All operations are logged to `_system.operation_log`:
```json
{
  "_id": "op_12345",
  "timestamp": "2026-02-08T04:30:00Z",
  "collection": "posts",
  "operation": "find",
  "filter": { "status": "published" },
  "user_id": "user_42",
  "duration_ms": 142,
  "documents_scanned": 10000,
  "documents_returned": 50,
  "index_used": "status_idx",
  "explain_plan": { /* deterministic explain */ }
}
```

Slow query configuration in `aerodb.toml`:
```toml
[observability.slow_queries]
enabled = true
threshold_ms = 100
sample_rate = 1.0  # Log all slow queries
alert_webhook = "https://alerts.example.com/slow-query"
```

**Why It Fits AeroDB:**
- Every operation has an explicit explain plan
- Slow threshold is configured, not guessed
- Explain plans are deterministic — same query = same plan (always)
- Alert webhooks are explicit integrations

**What It Intentionally Does NOT Do:**
- No automatic query optimization (you see the plan, you optimize)
- No hidden sampling (sample_rate is explicit)
- No magic indexing suggestions (but explain shows what index would help)

**User Mental Model:**
> "Every query is logged with its execution details. If it's slow, I know exactly why — and the answer is deterministic."

---

### 2.7 Client SDKs Strategy (Solving "No SDKs")

**SDK Philosophy:**
```
"SDKs are thin wrappers around the REST API. 
 They add type safety and convenience, not magic."
```

**Tier 1 SDKs (Official, Maintained):**
| Language | Package Name | Priority |
|----------|--------------|----------|
| TypeScript/JavaScript | `@aerodb/client` | Highest |
| Python | `aerodb-python` | High |
| Go | `aerodb-go` | High |
| Rust | `aerodb-rs` | Medium |
| Swift | `AeroDBSwift` | Medium |
| Kotlin | `aerodb-kotlin` | Medium |

**SDK Design Principles:**
1. **No hidden state** — every method call maps to one HTTP request
2. **Explicit errors** — typed error classes, no swallowed exceptions
3. **No caching** — unless explicitly enabled by caller
4. **No retries** — unless explicitly configured
5. **Full type generation** — from schema, not inferred

**Example (TypeScript):**
```typescript
import { AeroDBClient } from '@aerodb/client';

const db = new AeroDBClient({
  url: 'https://my-aerodb.example.com',
  apiKey: 'aero_xxx',
  // Explicit options:
  retries: 0,          // Default: no retries
  timeout: 10000,      // Explicit timeout
  logging: 'verbose',  // See all requests
});

// Typed from schema
const posts = await db.collection<Post>('posts').find({
  filter: { status: 'published' },
  expand: ['author_id'],
  limit: 20,
});

// Explicit error handling
if (posts.error) {
  console.error(posts.error.code, posts.error.message);
} else {
  console.log(posts.data);
}
```

**What SDKs Intentionally Do NOT Do:**
- No offline caching (client responsibility)
- No automatic retry with backoff (explicit config if wanted)
- No subscription management magic (explicit WebSocket handling)
- No ORM-style lazy loading (all fetches are explicit)

**User Mental Model:**
> "The SDK is a typed HTTP client. It doesn't hide what's happening — every call is visible, every error is typed."

---

### 2.8 Explicit Delivery Tiers for Realtime (Solving Guarantee Ambiguity)

**Design Overview:**

AeroDB offers two explicit delivery modes for realtime subscriptions:

| Mode | Guarantee | Use Case |
|------|-----------|----------|
| `fire-and-forget` | None — message may be dropped if client is slow | Live dashboards, presence, non-critical updates |
| `at-least-once` | Message delivered once ACKed, may be delivered multiple times | Order updates, notifications, critical events |

**Subscription Declaration:**
```typescript
const subscription = db.realtime.subscribe({
  channel: 'posts',
  events: ['insert', 'update'],
  delivery: 'at-least-once',  // EXPLICIT
  onMessage: (event) => {
    console.log(event);
    event.ack();  // Client must ACK
  },
  onDrop: (reason) => {
    console.warn('Messages may have been dropped:', reason);
  }
});
```

**Backpressure Policy:**
```toml
[realtime]
max_pending_messages = 1000      # Per subscription
drop_policy = "oldest_first"     # "oldest_first" | "newest_first" | "reject"
client_ack_timeout_ms = 5000     # For at-least-once mode
```

**What AeroDB Will NEVER Promise:**
- ❌ Exactly-once delivery (requires distributed consensus, adds magic)
- ❌ Ordered delivery across multiple subscriptions (use sequence numbers)
- ❌ Unlimited buffering (explicit limits, explicit drop policy)

**User Mental Model:**
> "I choose my delivery guarantee explicitly. I know the trade-offs: at-least-once means I might get duplicates, fire-and-forget means I might miss messages."

---

### 2.9 WASM Extension Registry (Solving Extensibility)

**Design Overview:**

AeroDB extensions are WASM modules with explicit capability declarations:

```json
{
  "extension": "geo-distance",
  "version": "1.0.0",
  "wasm_binary": "geo_distance.wasm",
  "capabilities": {
    "cpu_limit_ms": 100,
    "memory_limit_mb": 16,
    "network": false,
    "filesystem": false,
    "database_read": true,
    "database_write": false
  },
  "exports": [
    {
      "name": "haversine_distance",
      "input": { "lat1": "f64", "lon1": "f64", "lat2": "f64", "lon2": "f64" },
      "output": { "distance_km": "f64" }
    }
  ]
}
```

**Installation:**
```bash
aerodb extension install ./geo_distance.wasm --capabilities ./geo_distance.capabilities.json
aerodb extension list
aerodb extension remove geo-distance
```

**Usage in Queries:**
```http
GET /api/collections/stores?filter={"$ext:haversine_distance":{"lat":37.7749,"lon":-122.4194,"max_km":10}}
```

**Why WASM:**
- **Deterministic** — same input = same output, always
- **Sandboxed** — no filesystem, no network unless explicitly granted
- **Language-agnostic** — Rust, Go, C, AssemblyScript, etc.
- **Auditable** — capability manifest is inspectable

**What It Intentionally Does NOT Do:**
- No auto-updating extensions (explicit version pinning)
- No capability escalation (declared at install time, immutable)
- No npm/pip/cargo dependencies inside WASM (bundle everything)

**User Mental Model:**
> "Extensions are sandboxed WASM modules. I see exactly what they can do in the capability manifest. They can't surprise me."

---

## Part 3: Explicit Non-Goals

AeroDB will **NEVER** implement the following features:

| Feature | Reason |
|---------|--------|
| **SQL query language** | SQL's declarative nature requires a query optimizer that makes non-deterministic planning decisions. AeroDB's value is deterministic execution paths. |
| **GraphQL API** | GraphQL resolvers hide execution complexity and introduce N+1 ambiguity. Conflicts with explicit query costs. |
| **Automatic schema inference** | Schemas must be declared explicitly. No "just insert and we'll figure it out." |
| **Automatic indexing** | Index creation is a deliberate operation with performance implications. No hidden index management. |
| **Exactly-once delivery (realtime)** | Requires distributed consensus and hides failure modes. AeroDB prefers explicit delivery tiers. |
| **Magic retries/fallbacks** | All retry logic is client-side and explicit. Server never silently retries operations. |
| **Cross-collection transactions** | Each collection is an isolation boundary. Multi-collection atomicity requires explicit saga patterns. |
| **ORM-style lazy loading** | All data fetching is explicit. No `post.author` that secretly fires a query. |
| **Auto-scaling** | Scaling decisions should be explicit. AeroDB provides metrics for external orchestrators. |
| **Managed cloud with hidden infrastructure** | If AeroDB Cloud exists, it will expose full infrastructure visibility. No abstracted magic. |

---

## Part 4: Product Identity Statement

> **AeroDB** is a document-oriented backend platform designed for systems where **correctness is not optional**.
>
> It rejects the "just works" philosophy in favor of **explicit, auditable, deterministic behavior** — every query has a predictable execution path, every operation is logged, and every error is immediately visible.
>
> AeroDB is for teams who would rather understand their database than trust it.
>
> It is not a replacement for PostgreSQL or MongoDB. It is an alternative philosophy: **fail loudly, execute predictably, leave no surprises.**

---

## Part 5: Philosophy-Aligned Roadmap

### Phase 1: Make Limitations Explicit (0-3 months)

**Goal:** Every limitation becomes a documented trade-off, not a missing feature.

| Item | Status | Outcome |
|------|--------|---------|
| Document delivery guarantees for realtime | ✅ This doc | Explicit tier documentation |
| Publish performance benchmarks | TODO | Known limits with hardware specs |
| Document extension capability model | TODO | Clear isolation guarantees |
| Expand CLI with migration commands | TODO | `aerodb migrate` workflow |
| Publish slow query threshold config | TODO | Configurable, explicit alerting |
| Document backup/retention strategy | TODO | User-managed, explicit policies |

---

### Phase 2: Solve the Right Limitations (3-9 months)

**Goal:** AeroDB-native solutions for legitimate gaps, without philosophy compromise.

| Item | Priority | Description |
|------|----------|-------------|
| **Client SDKs (JS, Python)** | Highest | Thin wrappers, full type safety, no magic |
| **Dashboard expansion to 90%** | Highest | Every backend feature has UI coverage |
| **Deterministic Migration Tool** | High | Versioned, checksummed, reversible |
| **Declared References** | High | Explicit relationships with on_delete |
| **Expand Directive** | High | Explicit multi-document resolution |
| **Text Index Type** | High | Full-text search with explicit ranking |
| **Named Projections** | Medium | Schema-declared, immutable views |
| **OpenAPI Generation** | Medium | Auto-docs from schema |
| **Visual Query Builder** | Medium | Explicit preview before execution |
| **Alert Rules** | Medium | Explicit threshold webhooks |

---

### Phase 3: Long-Term Evolution (9-24 months)

**Goal:** Extend capabilities without compromising determinism.

| Item | Consideration | Status |
|------|---------------|--------|
| **Vector Index Type** | Must use explicit similarity metric, no adaptive ranking | Planned, not started |
| **WASM Extension Registry** | Official registry for community extensions | Planned, not started |
| **Region Affinity Mode** | Explicit geo-locality without multi-region CAP complexity | Under consideration |
| **Additional SDK Languages** | Swift, Kotlin, Rust, C# — following same principles | Planned, not started |
| **AeroDB Cloud (maybe)** | Only if full infrastructure visibility is preserved | Long-term consideration |

---

## Conclusion: The AeroDB Manifesto

AeroDB exists because **some systems cannot afford surprises**.

While the industry trends toward "magic" — auto-optimization, implicit behavior, hidden complexity — AeroDB moves in the opposite direction: **toward transparency, predictability, and explicit control**.

This is not a limitation. It is the product.

Every feature we add will be evaluated against one question:

> **"Does this make AeroDB behavior less predictable?"**

If yes, we do not add it.

If no, we design it to be explicit, auditable, and deterministic.

This is the AeroDB way.

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-08  
**Status:** Design Manifesto — Pending Review
