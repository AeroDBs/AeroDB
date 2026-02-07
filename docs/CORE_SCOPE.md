## Purpose

This document strictly defines what aerodb will and will not build.

Scope control is mandatory.
Any feature, optimization, or design that is not explicitly allowed here
is considered out of scope and must not be implemented without revising
this document.

aerodb is infrastructure software.
Uncontrolled scope is a reliability risk.

---

## Scope Philosophy

aerodb intentionally starts small, strict, and predictable.

We do not attempt to:
- Match feature parity with existing databases
- Solve distributed systems problems prematurely
- Optimize for novelty or marketing checklists

We only build what is required to deliver correctness,
determinism, and reliability at a single-node level.

Everything else is deferred by design.

---


## Phase 0: Minimum Viable Infrastructure (MVI)

> **Note on Terminology:** "Phase 0" and "Phase 1" refer to the same foundational phase in AeroDB's development. Current documentation (e.g., PERF_VISION.md) uses "Phase 1" terminology. This document preserves the original "Phase 0" naming for historical continuity.

Phase 0 defines the first **production-credible** version of aerodb.
This is not a demo phase.
This is the smallest system that can be trusted.

### Phase 0 Goals
- Deterministic behavior
- Strong correctness guarantees
- Crash-safe persistence
- Clear operational semantics

---

## Explicitly In-Scope (Phase 0)

### 1. Deployment Model
- Single-node database engine
- Local disk persistence
- Self-hosted operation only

No clustering assumptions are allowed.

---

### 2. Data Model
- Typed document model
- Mandatory schema per collection
- Schema versioning support
- Strict validation on write

Schemaless writes are forbidden.

---

### 3. Storage Engine
- Append-only Write-Ahead Log (WAL)
- On-disk storage format with checksums
- Deterministic crash recovery
- Explicit fsync boundaries

All acknowledged writes must be durable.

---

### 4. Indexing
- One index type only (B-tree or equivalent)
- Explicit index creation and removal
- Deterministic index selection

Automatic or adaptive indexing is out of scope.

---

### 5. Query Capabilities
Supported operations:
- find
- filter (equality + bounded range only)
- sort (indexed fields only)
- limit

No implicit full scans.
No hidden query rewrites.

---

### 6. Query Planning
- Cost estimation required before execution
- Deterministic query planner
- Query rejection if cost cannot be bounded
- Stable plan generation

Adaptive or heuristic-based planning is forbidden.

---

### 7. Write Operations
- Insert
- Update (by primary key or indexed field)
- Delete (explicit and bounded)

Bulk writes are allowed only if fully bounded.

---

### 8. Failure Handling
- Explicit error codes
- Loud failures
- No silent retries
- No partial success masking

Failure behavior must be observable and explainable.

---

### 9. Observability
- Structured logs
- Deterministic error messages
- Human-readable explain plans
- Explicit startup and recovery logs

Metrics are secondary to explainability.

---

### 10. Configuration
- Minimal configuration surface
- Safe defaults only
- Explicit limits for memory and disk usage

Hidden tuning knobs are forbidden.

---

## Explicitly Out of Scope (Phase 0)

The following are **not allowed**, regardless of perceived usefulness:

### Architecture & Scaling
- Sharding
- Replication
- Clustering
- Leader election
- Multi-node coordination
- Multi-region deployments

---

### Performance Features
- Adaptive query optimization
- Automatic index creation
- Background query rewriting
- Speculative execution

---

### Data & Query Features
- Joins
- Aggregation pipelines
- Map-reduce
- Full-text search
- Geospatial queries
- Vector search
- Time-series optimizations

---

### Transactions
- Multi-document transactions
- Cross-collection transactions
- Distributed transactions

Single-document atomicity only.

---

### Operational Features
- Auto-scaling
- Serverless execution
- Managed service assumptions
- Built-in UI dashboards

---

### Developer Convenience
- Schemaless modes
- Implicit coercion
- “Best effort” writes
- Magic defaults
- Silent fallbacks

---

## Completed Phases

### Phase 1: Core Storage & Correctness
- Single-node WAL-backed persistence
- Deterministic query planning
- Crash-safe recovery
- Bounded query execution
- **Status:** Frozen

### Phase 2A: MVCC (Multi-Version Concurrency Control)
- Snapshot isolation
- CommitId authority model
- Version chain management
- Deterministic visibility rules
- **Status:** Frozen

### Phase 2B: Replication
- Single-writer replication model
- WAL shipping semantics
- Snapshot transfer
- Replica read rules
- Fail-stop semantics
- **Status:** Frozen

### Phase 3: Performance Optimizations (Current)
- Group Commit (PERF-01)
- WAL Batching (PERF-02)
- Read Path Optimization (PERF-03)
- Index Acceleration (PERF-04)
- Checkpoint Pipelining (PERF-05)
- Replica Read Fast Path (PERF-06)
- Memory Layout Optimization (PERF-07)
- **Status:** Implementation complete, correctness-preserving only

---

## Future Considerations (Post-Phase 3)

The following may be considered after Phase 3 stabilizes:

- Multi-document transactions
- Backup and restore tooling enhancements
- Schema migration automation
- Additional secondary index types
- CLI and admin tooling improvements
- Cross-shard operations (if sharding becomes in-scope)

These are explicitly deferred.

---

## Scope Enforcement Rules

- Any feature not listed as “In-Scope” is forbidden.
- Any proposal to expand scope must:
  1. Clearly state the problem
  2. Demonstrate no violation of VISION.md
  3. Show no impact on determinism or reliability
  4. Explicitly update this document

If these conditions are not met, the proposal is rejected.

---

## Final Statement

aerodb does not win by doing more.
It wins by doing less, correctly, predictably, and transparently.

Scope discipline is a core feature.
