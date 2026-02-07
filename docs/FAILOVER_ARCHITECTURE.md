# FAILOVER_ARCHITECTURE.md — Failover & Promotion

## Status
- Phase: **6**
- Authority: **Normative**
- Depends on:
  - FAILOVER_VISION.md
  - FAILOVER_SCOPE.md
  - FAILOVER_INVARIANTS.md
- Frozen Dependencies: **Phases 0–5**

---

## 1. Purpose

This document defines the **logical architecture** for Phase 6.

It specifies:
- New logical components
- Authority boundaries
- Interaction points with frozen subsystems

It does **not** define:
- Concrete data structures
- Algorithms
- APIs
- Performance optimizations

Architecture here is **descriptive**, not prescriptive.

---

## 2. Architectural Principle

Phase 6 architecture follows one rule:

> **Failover logic observes and validates existing state;  
> it does not create new authority sources.**

All authoritative state remains governed by:
- WAL
- Replication state (Phase 5)
- Recovery logic (Phase 0–5)

Phase 6 introduces **decision logic**, not new truth.

---

## 3. New Logical Components

Phase 6 introduces the following **logical-only** components.

### 3.1 Promotion Controller

**Responsibility**
- Coordinates promotion attempts
- Orchestrates validation and decision flow
- Emits observability and explanation data

**Non-Responsibilities**
- Does not write WAL
- Does not modify replication protocol
- Does not infer liveness
- Does not retry automatically

The Promotion Controller is **purely coordinating**.

---

### 3.2 Promotion Validator

**Responsibility**
- Evaluates whether promotion is allowed
- Validates all Phase 6 safety invariants
- Produces an explicit allow/deny decision

**Inputs**
- Replica replication state
- WAL position and metadata
- Known primary authority state

**Outputs**
- Deterministic validation result
- Explicit failure reasons if denied

Validator logic MUST be:
- Deterministic
- Side-effect free
- Fully explainable

---

### 3.3 Authority Transition Manager

**Responsibility**
- Applies an approved authority transition
- Ensures atomicity of authority change
- Integrates with existing replication state

**Constraints**
- Must not overlap authority
- Must be crash-safe
- Must leave recovery unambiguous

This component performs **state transition only after validation succeeds**.

---

## 4. Interaction with Existing Subsystems

### 4.1 Replication Subsystem (Phase 5)

Phase 6:
- Reads replication role and state
- Validates replica readiness
- Updates replication role *only through allowed transitions*

Phase 6 MUST NOT:
- Modify replication protocol
- Introduce new replication states without explicit definition
- Alter Phase 5 authority checks

---

### 4.2 WAL & Recovery (Phases 0–3)

Phase 6:
- Treats WAL as the sole durability authority
- Uses WAL metadata to validate safety
- Relies on existing recovery semantics

Phase 6 MUST NOT modify WAL:
- Add WAL records
- Change WAL formats

Phase 6 MAY use non-WAL durability for authority transition:
- A single fsynced marker file (`metadata/authority_transition.marker`)
- This marker is atomic: present = new authority, absent = old authority
- The marker file is the SOLE authority state durability mechanism
- This does NOT constitute a new WAL record or format change

Phase 6 MUST NOT:
- Add recovery-time heuristics

---

### 4.3 MVCC & Visibility (Phase 2)

Phase 6:
- Preserves all MVCC visibility rules
- Ensures promotion does not alter snapshot semantics

Phase 6 MUST NOT:
- Reinterpret commit identities
- Adjust visibility boundaries
- Introduce new MVCC states

---

### 4.4 Observability & Explanation (Phase 4)

Phase 6:
- Emits observability events for promotion attempts
- Produces explanation artifacts for decisions

Phase 6 MUST NOT:
- Use observability data to guide decisions
- Introduce feedback loops

Observability remains passive.

---

## 5. Authority Boundaries

### 5.1 Authority Sources (Unchanged)

The only authoritative sources remain:
- WAL
- Replication state
- Recovery state

Phase 6 does not add authority; it **rebinds** authority explicitly.

---

### 5.2 Decision vs Authority

Important separation:

- **Decision**: Phase 6 logic (can approve or deny)
- **Authority**: Existing core systems (can commit or reject)

Phase 6 decisions are advisory until applied atomically.

---

## 6. Crash & Recovery Integration

Phase 6 architecture MUST ensure:

- No partial authority transition is durable
- Recovery always reconstructs a single authority state
- Promotion outcomes are unambiguous after restart

All crash safety relies on **existing recovery guarantees**.

---

## 7. Explicitly Forbidden Architectural Patterns

Phase 6 MUST NOT introduce:

- Background promotion threads
- Heartbeat-driven logic
- Time-based transitions
- Distributed consensus subsystems
- Hidden shared state between nodes

Any such pattern violates Phase 6 scope.

---

## 8. Architectural Completeness Criteria

This architecture is complete when:

- Every promotion path maps to components above
- Every failure path has a single handling point
- No component has mixed authority roles
- All interactions with frozen phases are read-only or explicit

---

## 9. Architectural Stability Rule

Once Phase 6 is frozen:

> No new architectural component may be added  
> without a new phase.

---

END OF DOCUMENT
