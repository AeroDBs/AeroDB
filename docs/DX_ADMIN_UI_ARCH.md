# ADMIN UI ARCHITECTURE — PHASE 4

## Status

- Phase: **4**
- Authority: **Normative**
- Scope: **Read-only developer UI for observability and explanation**
- Depends on:
  - PHASE4_VISION.md
  - PHASE4_INVARIANTS.md
  - OBSERVABILITY_API.md
  - PERFORMANCE_OBSERVABILITY.md
  - SEMANTIC_EQUIVALENCE.md

This document defines the **architecture, boundaries, and obligations**
of the AeroDB Admin UI.

If the UI violates any rule in this document, it MUST NOT ship.

---

## 1. Purpose

The Admin UI exists to make AeroDB:

- Inspectable
- Explainable
- Trustworthy

It exists to **reveal internal truth**, not to simplify or abstract it away.

The UI is not:
- A database client
- An admin console
- A management plane
- A production dashboard

It is a **local inspection instrument**.

---

## 2. Core UI Principle

### The Glass Box UI Principle

> The Admin UI must show AeroDB as it actually is,  
> not as users might wish it to be.

This means:
- Internal identifiers are shown, not hidden
- Raw state is preferred over summaries
- Complexity is not collapsed into “green lights”

If something is complicated, the UI must show that complication honestly.

---

## 3. Architectural Boundaries

### UI-1: Strict Read-Only Boundary

The UI MUST:

- Consume only `OBSERVABILITY_API.md`
- Perform no writes
- Perform no control actions
- Trigger no background work

The UI is **incapable** of changing AeroDB state.

---

### UI-2: API Fidelity

For every UI element:

- There must be a direct API source
- UI labels must match API semantics
- No inferred or derived state is allowed unless explicitly labeled

If the API cannot explain it, the UI must not invent it.

---

### UI-3: Removability

The UI MUST be:

- Optional
- Fully removable
- Isolated from core runtime

Removing the UI must leave AeroDB behavior unchanged.

---

## 4. Deployment Model

### 4.1 Local-Only Assumption

The Admin UI assumes:

- Local developer environment
- Trusted user
- Debug / inspection usage

The UI MUST NOT:
- Advertise production readiness
- Imply security hardening
- Support multi-user access

---

### 4.2 UI Runtime Separation

The UI MUST:

- Run as a separate process or static frontend
- Communicate via HTTP to Observability API
- Never link directly against storage or MVCC code

This preserves isolation and passivity.

---

## 5. Information Architecture (Mandatory Views)

The UI MUST implement the following views.

---

### 5.1 Overview / Status View

**Purpose:** High-level orientation

Displays:
- Lifecycle state (booting / running / recovering)
- Current CommitId high-water mark
- WAL durability boundary
- Phase enablement flags

Rules:
- No health scores
- No “OK / WARN / ERROR” simplifications
- Raw state first, explanation second

---

### 5.2 WAL Inspector

**Purpose:** Durability and ordering transparency

Displays:
- WAL files and segments
- Append offsets
- Durable offsets
- Truncation points
- Checksum status

Rules:
- No hiding of offsets
- Explicit durability boundaries
- Visual separation of “written” vs “durable”

---

### 5.3 MVCC Inspector

**Purpose:** Visibility and isolation clarity

Displays:
- CommitId ranges
- Active snapshots
- Snapshot CommitIds
- GC watermark (if applicable)

Rules:
- Show numeric CommitIds
- No abstract “time” metaphors
- Snapshot identity must be explicit

---

### 5.4 Snapshot & Checkpoint View

**Purpose:** Persistence safety inspection

Displays:
- Active snapshots
- Last completed checkpoint
- WAL coverage per checkpoint
- Pending / aborted checkpoints

Rules:
- Clear distinction between “prepared” and “durable”
- No implied safety before fsync

---

### 5.5 Index Inspector

**Purpose:** Query acceleration transparency

Displays:
- Index identifiers
- Index type
- Build status
- Entry counts
- Rebuild state

Rules:
- Indexes clearly labeled as advisory
- No claims of completeness or authority

---

### 5.6 Replication View (Conditional)

**Purpose:** Replica safety inspection

Displays:
- Node role (primary / replica)
- WAL prefix position
- Replica lag (CommitId-based)
- Snapshot bootstrap status

Rules:
- No heuristic “lag OK” indicators
- No peer reachability assumptions

---

### 5.7 Query Explanation View

**Purpose:** Deterministic reasoning

Displays:
- Query text
- Execution plan
- Index usage
- Snapshot used
- Bounds applied

Rules:
- Explanation must match `/v1/explain/query`
- No speculative reasoning
- No performance promises

---

### 5.8 Read Visibility Explanation View

**Purpose:** MVCC trust

Displays:
- Document identifier
- Version chain
- CommitId comparisons
- Visibility decision

Rules:
- Step-by-step visibility logic
- Exact CommitId comparisons
- No narrative shortcuts

---

### 5.9 Recovery Explanation View

**Purpose:** Failure trust

Displays:
- Last crash point
- Recovery start point
- Snapshot selected
- WAL replay range
- Validation steps

Rules:
- Must reflect real recovery
- No “successful recovery” banners
- Raw steps first, summary last

---

## 6. UI Interaction Rules

### UI-4: No Control Interactions

The UI MUST NOT include:

- Buttons that trigger actions
- Forms that submit mutations
- Toggles that affect runtime behavior

If an interaction changes state, the UI is invalid.

---

### UI-5: Explicit Cost Visibility

If an API call is expensive:

- The UI MUST show that cost
- The UI MUST not auto-refresh aggressively
- Polling intervals must be explicit

Hidden cost is forbidden.

---

## 7. Determinism & Stability

### UI-6: Stable Rendering

Given identical API responses:

- UI output MUST be identical
- Ordering MUST be stable
- No random IDs or animations

The UI must be diff-friendly.

---

## 8. Error Handling

### UI-7: Error Transparency

Errors MUST be shown:

- With raw error codes
- With invariant references
- Without user-friendly dilution

The UI explains errors; it does not soften them.

---

## 9. Forbidden UI Patterns

The following are explicitly forbidden:

- “Traffic light” health indicators
- Auto-healing messaging
- Hidden state aggregation
- Collapsing failures into “unknown”
- Pretty timelines that imply causality
- Marketing language

If it looks like a dashboard, it is wrong.

---

## 10. Testing Requirements

The Admin UI MUST be tested for:

- Read-only enforcement
- API fidelity
- Deterministic rendering
- Disablement safety
- Error surface correctness

UI tests must assume:
- Partial data
- Failure states
- Mid-recovery visibility

---

## 11. Phase 4 Completion Criteria (UI)

Phase 4 UI is complete when:

- A developer can explain any read result
- A crash can be understood without logs
- WAL durability boundaries are visible
- MVCC behavior is inspectable
- Replication safety can be reasoned about

Without reading source code.

---

## 12. Final Rule

> The Admin UI must never reassure.  
> It must only reveal.

If the UI makes AeroDB feel “safe” without evidence,
it has already failed.

---

END OF DOCUMENT
