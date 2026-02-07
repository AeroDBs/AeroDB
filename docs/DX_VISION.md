# PHASE 4 — DEVELOPER EXPERIENCE & VISIBILITY

## Status

- Phase: **4**
- State: **Design-first**
- Authority: **Normative**
- Depends on:
  - Phase 1 — Core Storage (Frozen)
  - Phase 2 — MVCC & Replication Semantics (Frozen)
  - Phase 3 — Performance (Frozen)

Phase 4 introduces **no new database semantics**.
It exists to make AeroDB *observable, explorable, and trustworthy*.

---

## 1. Purpose of Phase 4

Phase 4 exists to solve a single problem:

> AeroDB is correct — but invisible.

This phase makes AeroDB:
- Understandable
- Inspectable
- Auditable
- Demonstrable

Without making it:
- Less correct
- Less deterministic
- Less explicit

Phase 4 is about **developer trust**, not features.

---

## 2. Core Principle

### The Glass Box Principle

> AeroDB must behave like a *glass box*, not a black box.

This means:
- Every decision can be observed
- Every invariant can be inspected
- Every recovery can be explained
- Every failure has a visible reason

Phase 4 does **not** simplify the system.
It makes the system **legible**.

---

## 3. Explicit Non-Goals

Phase 4 does **NOT** include:

- New query capabilities
- New storage features
- New consistency levels
- New performance optimizations
- New replication semantics
- Production authentication or authorization
- Multi-tenant UI
- Hosted or cloud features

If it changes data behavior, it does not belong in Phase 4.

---

## 4. What Phase 4 Introduces

Phase 4 introduces **read-only introspection surfaces**.

### 4.1 Observability APIs (Read-Only)

AeroDB exposes internal state via **explicit, deterministic APIs**.

These APIs allow inspection of:
- WAL state
- MVCC state
- Snapshot and checkpoint state
- Index state
- Replication state (if enabled)
- Lifecycle and recovery events

These APIs:
- Never mutate state
- Never influence execution
- Never alter scheduling
- Never bypass invariants

Observability remains passive.

---

### 4.2 Developer UI (Local, Read-Only)

A minimal UI that:
- Runs locally
- Connects to observability APIs
- Visualizes internal state

The UI exists to:
- Teach how AeroDB works
- Prove correctness properties
- Make failures understandable

The UI is **not a database client**.
It is an **inspection tool**.

---

### 4.3 Deterministic Explanation Surfaces

Phase 4 introduces explanation artifacts, such as:
- Query execution plans
- MVCC visibility explanations
- Recovery step traces
- Checkpoint and WAL progression

These explanations:
- Are deterministic
- Are derived from real execution
- Are not heuristic summaries

---

## 5. Phase 4 Trust Model

Phase 4 assumes:
- Single-node usage
- Local developer environment
- Trusted user
- Debug / inspection context

Security hardening is **out of scope** for this phase.

---

## 6. Phase 4 Architecture Boundaries

Phase 4 components MUST obey:

- No background tasks that affect correctness
- No polling loops that alter timing
- No UI-driven behavior changes
- No conditional logic based on observability

Removing Phase 4 must leave AeroDB behavior unchanged.

---

## 7. Phase 4 Deliverables

### 7.1 Required Documents

Phase 4 will introduce:

- `PHASE4_VISION.md` (this document)
- `PHASE4_INVARIANTS.md`
- `OBSERVABILITY_API.md`
- `ADMIN_UI_ARCHITECTURE.md`
- `EXPLANATION_MODEL.md`

No implementation begins without these.

---

### 7.2 Required Capabilities

By the end of Phase 4, a developer must be able to:

- See current WAL position and durability boundary
- Inspect MVCC commit ranges and snapshots
- Understand why a read sees a given version
- See recovery steps after a crash
- Verify checkpoint safety
- Observe replica safety (if enabled)

Without reading the code.

---

## 8. Phase 4 Invariants

Phase 4 MUST preserve:

- All Phase 1 invariants
- All MVCC invariants
- All replication invariants
- All Phase 3 performance invariants

Additionally:

- Observability MUST be passive
- UI MUST be read-only
- Explanations MUST reflect reality
- No synthetic or inferred state is allowed

If something cannot be explained truthfully, it must not be shown.

---

## 9. Phase 4 Success Criteria

Phase 4 is successful if:

- A new developer can understand AeroDB by exploring it
- A crash can be explained visually and textually
- MVCC behavior can be reasoned about without reading code
- Trust is earned through transparency, not claims

Performance impact is irrelevant.
Correctness visibility is everything.

---

## 10. Phase 4 Failure Modes

Phase 4 fails if:

- UI hides complexity instead of exposing it
- Observability changes execution behavior
- Explanations are approximate or heuristic
- The system becomes harder to reason about

If visibility introduces ambiguity, Phase 4 must be rolled back.

---

## 11. Guiding Statement

> Phase 4 does not make AeroDB easier by hiding reality.  
> It makes AeroDB usable by revealing reality.

AeroDB’s advantage is not speed.
It is **explainability under failure**.

Phase 4 exists to prove that.

---

END OF DOCUMENT
