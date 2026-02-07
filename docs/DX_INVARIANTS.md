# PHASE 4 — INVARIANTS (DEVELOPER EXPERIENCE & VISIBILITY)

## Status

- Phase: **4**
- Authority: **Normative**
- Scope: **All Phase 4 observability, APIs, and UI**
- Depends on:
  - INVARIANTS.md (global)
  - PERF_INVARIANTS.md
  - PERFORMANCE_OBSERVABILITY.md

This document defines the **non-negotiable invariants** governing Phase 4.
Any Phase 4 component that violates these invariants is invalid and must be removed.

---

## 1. Purpose

Phase 4 exists to make AeroDB **observable and explainable**.

This document ensures that:
- Visibility never alters behavior
- Explanation never diverges from reality
- UI never becomes a control surface
- Trust is increased, not compromised

Phase 4 invariants are **additive constraints** on top of all earlier-phase invariants.

---

## 2. Phase 4 Scope Invariant

### P4-1: Zero Semantic Authority

Phase 4 components:
- Have **no semantic authority**
- Cannot create, modify, or influence database state
- Cannot affect execution order, timing, or decisions

All authoritative behavior remains in Phases 1–3.

---

## 3. Read-Only Invariants

### P4-2: Strict Read-Only Surfaces

All Phase 4 interfaces (APIs, UI, tools):

- MUST be read-only
- MUST reject any mutation attempt
- MUST NOT expose write, admin, or control endpoints

There is no “safe write” exception.

---

### P4-3: No Side-Effect Reads

Observability reads MUST NOT:
- Allocate unbounded memory
- Trigger background work
- Trigger lazy computation that persists state
- Trigger compaction, cleanup, or refresh logic

If a read causes work, it must be **explicitly visible and discardable**.

---

## 4. Passivity Invariants

### P4-4: Observability Passivity

Phase 4 components MUST NOT:
- Influence scheduling
- Influence batching
- Influence WAL behavior
- Influence MVCC visibility
- Influence replication flow

Removing all Phase 4 code must produce **bit-for-bit identical behavior**.

---

### P4-5: No Feedback Loops

Phase 4 MUST NOT:
- Use metrics to tune behavior
- Use UI state to gate execution
- Use access patterns to optimize internals

There are no adaptive or reactive systems in Phase 4.

---

## 5. Determinism Invariants

### P4-6: Deterministic Observation

Given identical database state:

- Observability outputs MUST be identical
- Explanation outputs MUST be identical
- UI representations MUST be stable

Non-deterministic output is a correctness bug.

---

### P4-7: Snapshot-Bound Observation

All Phase 4 reads MUST:
- Be explicitly snapshot-bound
- Declare which CommitId or snapshot they observe
- Never mix state across snapshots implicitly

If state is mixed, it must be explicit.

---

## 6. Explanation Integrity Invariants

### P4-8: No Heuristic Explanations

Explanations MUST:
- Reflect real execution
- Be derived from actual data
- Avoid summaries that infer intent

Forbidden:
- “Likely because…”
- “Probably due to…”
- “Optimized path chosen…”

If an explanation cannot be exact, it must say:
> “This cannot be explained precisely.”

---

### P4-9: Explanation = Evidence

Every explanation MUST:
- Reference concrete state (CommitId, WAL offset, snapshot id)
- Be traceable to internal structures
- Be verifiable by re-running the system

Explanations are **evidence**, not documentation.

---

## 7. Failure Visibility Invariants

### P4-10: Failure Transparency

When failures occur:

- Phase 4 MUST expose:
  - Where the failure occurred
  - Which invariant was violated
  - What state was preserved
- Phase 4 MUST NOT:
  - Mask failures
  - Collapse error categories
  - Replace errors with generic messages

Failure visibility is mandatory.

---

### P4-11: Recovery Explainability

After recovery:

- Phase 4 MUST show:
  - Recovery start point
  - WAL replay range
  - Snapshot or checkpoint used
  - Final consistency result

If recovery cannot be explained, Phase 4 is incomplete.

---

## 8. UI-Specific Invariants

### P4-12: UI Is Not a Control Plane

The UI MUST NOT:
- Trigger operations
- Start checkpoints
- Trigger backups
- Modify configuration
- Influence runtime behavior

The UI is a **viewer**, not an operator.

---

### P4-13: UI Fidelity Over Friendliness

The UI MUST:
- Prefer accuracy over simplicity
- Show raw identifiers (CommitId, WAL seq)
- Avoid hiding complexity

If something is complex, it should look complex.

---

## 9. Security & Trust Invariants

### P4-14: No False Sense of Safety

Phase 4 MUST NOT:
- Imply production readiness
- Imply security guarantees
- Hide dangerous states

If something is unsafe, Phase 4 must show it plainly.

---

## 10. Performance Invariants

### P4-15: Observability Overhead Is Explicit

Any performance overhead introduced by Phase 4 MUST:
- Be measurable
- Be attributable
- Be bounded

Hidden overhead is forbidden.

---

## 11. Disablement Invariants

### P4-16: Complete Removability

Phase 4 MUST be fully disableable:

- At compile time, or
- At startup

Disabling Phase 4 MUST:
- Require no migration
- Require no data changes
- Require no behavior changes

---

## 12. Invariant Enforcement

Every Phase 4 component MUST:

- Explicitly list which Phase 4 invariants it touches
- Prove compliance
- Include tests that verify passivity

Violation of any Phase 4 invariant is a **blocking defect**.

---

## 13. Invariant Precedence

If Phase 4 invariants conflict with:

- Phase 1–3 invariants → Phase 4 loses
- Performance goals → Phase 4 loses
- UI convenience → Phase 4 loses

Visibility never outranks correctness.

---

## 14. Final Rule

> Phase 4 may explain everything —  
> but it may change nothing.

If Phase 4 alters behavior,
it has already failed.

---

END OF DOCUMENT
