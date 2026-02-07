# OBSERVABILITY API — PHASE 4

## Status

- Phase: **4**
- Authority: **Normative**
- Scope: **Read-only observability and explanation APIs**
- Depends on:
  - PHASE4_VISION.md
  - PHASE4_INVARIANTS.md
  - INVARIANTS.md
  - PERFORMANCE_OBSERVABILITY.md
  - SEMANTIC_EQUIVALENCE.md

This document defines the **only permitted external observability surface**
for AeroDB in Phase 4.

If a state is not exposed here, it MUST NOT be exposed elsewhere.

---

## 1. Purpose

The Observability API exists to:

- Expose internal state truthfully
- Allow developers to inspect correctness
- Enable explanation of behavior and failures
- Support a read-only admin UI

The API is **not**:
- A control plane
- A management interface
- A mutation surface
- A production admin API

---

## 2. Core Principles

### OAPI-1: Read-Only Absoluteness

- All endpoints are **strictly read-only**
- No endpoint may cause mutation
- No endpoint may trigger background work
- No endpoint may influence scheduling

If an endpoint cannot be implemented without side effects, it is invalid.

---

### OAPI-2: Snapshot-Explicit Semantics

Every response MUST clearly state:
- Which snapshot it observes
- The CommitId boundary
- Whether the snapshot is stable or live

No implicit snapshot context is allowed.

---

### OAPI-3: Deterministic Responses

Given identical database state and identical request parameters:

- Responses MUST be identical
- Ordering MUST be stable
- Output MUST be reproducible

Non-determinism is a correctness violation.

---

## 3. API Shape & Transport

### 3.1 Transport

- HTTP (local only)
- JSON responses
- UTF-8 encoding

Transport choice is **non-authoritative**.
Semantics are authoritative.

---

### 3.2 Versioning

- API is versioned explicitly: `/v1/...`
- Version changes require explicit spec update
- No silent backward-incompatible changes

---

## 4. Global Response Envelope

All responses MUST follow this structure:

```json
{
  "api_version": "v1",
  "observed_at": {
    "snapshot": "explicit | live",
    "commit_id": 12345
  },
  "data": { ... },
  "notes": [ "optional explanatory strings" ]
}
````

Rules:

* `observed_at` is mandatory
* `notes` MUST NOT include heuristics
* All numeric identifiers are raw, not prettified

---

## 5. Core Endpoints (Mandatory)

### 5.1 `/v1/status`

**Purpose:** Overall database state

Returns:

* Lifecycle state (booting, running, recovering)
* Current CommitId high-water mark
* WAL durability boundary
* Phase enablement flags (Phase 4 enabled/disabled)

Must NOT:

* Trigger refresh
* Probe liveness indirectly

---

### 5.2 `/v1/wal`

**Purpose:** Inspect WAL state

Returns:

* WAL file identifiers
* Current append offset
* Last durable offset
* Checksum status
* Truncation point (if any)

All offsets are raw integers.

---

### 5.3 `/v1/mvcc`

**Purpose:** Inspect MVCC state

Returns:

* Oldest retained CommitId
* Latest committed CommitId
* Active snapshots (count + IDs)
* GC watermark (if applicable)

Must NOT:

* Trigger GC
* Trigger snapshot creation

---

### 5.4 `/v1/snapshots`

**Purpose:** Snapshot visibility

Returns:

* List of active snapshots
* CommitId each snapshot observes
* Snapshot source (user, internal, recovery)

Snapshots are informational only.

---

### 5.5 `/v1/checkpoints`

**Purpose:** Checkpoint state

Returns:

* Last completed checkpoint CommitId
* Checkpoint durability status
* WAL range covered
* Pending or aborted checkpoint info (if any)

---

### 5.6 `/v1/indexes`

**Purpose:** Index health and coverage

Returns:

* Index identifiers
* Index type
* Build status
* Entry counts
* Rebuild progress (if rebuilding)

Indexes remain advisory.

---

### 5.7 `/v1/replication` (Conditional)

**Purpose:** Replication state (if enabled)

Returns:

* Role (primary / replica)
* WAL prefix position
* Replica lag (CommitId-based)
* Snapshot bootstrap state

Must NOT:

* Attempt to contact peers
* Infer health heuristically

---

## 6. Explanation Endpoints (Derived, Mandatory)

### 6.1 `/v1/explain/query`

**Purpose:** Explain query execution

Input:

* Query (read-only)
* Optional snapshot CommitId

Returns:

* Deterministic execution plan
* Index usage (if any)
* Snapshot used
* Bounds applied

No execution is performed unless explicitly allowed.

---

### 6.2 `/v1/explain/read`

**Purpose:** Explain why a document version is visible

Input:

* Document identifier
* Snapshot CommitId

Returns:

* Version chain
* CommitId comparisons
* Visibility decision

This endpoint is critical for MVCC trust.

---

### 6.3 `/v1/explain/recovery`

**Purpose:** Explain last recovery

Returns:

* Crash detection point
* Snapshot chosen
* WAL replay range
* Validation steps
* Final state summary

If recovery has not occurred, return empty explanation.

---

## 7. Forbidden Endpoints

The following are explicitly forbidden:

* `/write`
* `/admin`
* `/config/set`
* `/gc/run`
* `/checkpoint/run`
* `/backup/start`
* Any endpoint that mutates state

If a UI needs it, Phase 4 design is wrong.

---

## 8. Error Model

Errors MUST:

* Be explicit
* Reference internal error codes
* Include invariant identifiers where applicable

Example:

```json
{
  "error": {
    "code": "MVCC_VISIBILITY_VIOLATION",
    "invariant": "MVCC-1",
    "message": "Requested snapshot observes uncommitted version"
  }
}
```

No generic errors allowed.

---

## 9. Performance & Safety Guarantees

* All endpoints are bounded
* Large responses must be explicitly paginated
* No endpoint may scan unbounded state implicitly

If inspection is expensive, that cost must be visible.

---

## 10. Disablement Rules

* Entire Observability API MUST be disableable
* Disablement MUST:

  * Remove HTTP listeners
  * Remove handlers
  * Leave core system untouched

API disablement MUST NOT affect behavior.

---

## 11. Testing Requirements

The Observability API MUST be tested for:

* Read-only enforcement
* Snapshot correctness
* Deterministic output
* Disablement equivalence
* Crash and recovery transparency

Any test requiring state mutation invalidates the endpoint.

---

## 12. Final Rule

> If the Observability API can change the system,
> it is not observability — it is control.

Phase 4 succeeds only if every endpoint
can be removed without changing AeroDB’s behavior.

---

END OF DOCUMENT
