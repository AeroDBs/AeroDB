# PHASE 7 STATE MODEL — CONTROL PLANE STATE, DERIVATION, AND AUTHORITY

**Project:** AeroDB
**Phase:** Phase 7 — Control Plane, Admin UI, and Operator Tooling
**Status:** AUTHORITATIVE · NON-NEGOTIABLE

---

## 1. Purpose of This Document

This document defines the **state model for Phase 7**.

The Phase 7 state model exists to answer one question unambiguously:

> **What state may the control plane hold without becoming a source of truth?**

Incorrect state modeling is one of the most common ways control planes accidentally become correctness-critical. This document prevents that.

---

## 2. Fundamental State Principle

> **Phase 7 MUST NOT own authoritative system state.**

All authoritative state belongs to the correctness kernel (Phases 0–6).

Phase 7 state is:

* Ephemeral
* Derived
* Replaceable
* Non-authoritative

If all Phase 7 state is lost, the system must remain fully correct and recoverable.

---

## 3. State Categories

Phase 7 state is divided into three explicit categories:

1. **Authoritative Kernel State (Referenced Only)**
2. **Derived Presentation State**
3. **Ephemeral Interaction State**

No other categories are permitted.

---

## 4. Authoritative Kernel State (Referenced Only)

### 4.1 Definition

Authoritative kernel state includes:

* Node roles and replication state
* WAL positions
* MVCC visibility state
* Promotion and demotion state
* Snapshot and checkpoint metadata

This state:

* Is owned exclusively by Phases 0–6
* Is persisted durably
* Is crash-recoverable

---

### 4.2 Rules

Phase 7 MUST:

* Read kernel state via explicit APIs
* Treat kernel state as immutable input
* Never cache kernel state as authoritative

Phase 7 MUST NOT:

* Modify kernel state implicitly
* Infer kernel state transitions
* Persist kernel state copies for recovery

---

## 5. Derived Presentation State

### 5.1 Definition

Derived presentation state is computed by Phase 7 for human consumption.

Examples:

* Aggregated health summaries
* Timelines composed from event streams
* Human-readable role summaries
* Sorted or filtered views

Derived state:

* Is recomputable
* Has no independent authority
* Exists only to improve understanding

---

### 5.2 Rules

Derived presentation state:

* MUST be derivable solely from kernel state and immutable logs
* MUST NOT introduce new semantics
* MUST NOT be persisted as authoritative

If derived state disagrees with kernel state, the kernel wins.

---

## 6. Ephemeral Interaction State

### 6.1 Definition

Ephemeral interaction state supports active operator workflows.

Examples:

* Pending confirmation dialogs
* In-progress command forms
* UI selection context
* Temporary client-side request IDs

This state:

* Exists only for the duration of interaction
* Is safe to discard at any time

---

### 6.2 Rules

Ephemeral interaction state:

* MUST NOT survive control-plane restarts
* MUST NOT be persisted durably
* MUST NOT trigger kernel actions by itself

If ephemeral state is lost, the operator must restart the interaction.

---

## 7. Forbidden State

Phase 7 MUST NOT hold:

* In-flight mutation state
* Partially executed command state
* Retry queues
* Background task state
* Decision memory

Any such state would make Phase 7 correctness-critical, which is forbidden.

---

## 8. State Transitions

### 8.1 Control-Plane State Transitions

Phase 7 state transitions:

* Are local to the control plane
* Do not affect kernel state
* Must be deterministic

Example:

* Viewing → Confirming → Viewing

---

### 8.2 Kernel State Transitions

Kernel state transitions:

* Occur only within Phases 0–6
* Are requested, not executed, by Phase 7
* Are validated exclusively by the kernel

Phase 7 observes these transitions but does not own them.

---

## 9. State and Failure Interaction

Under any Phase 7 failure:

* All Phase 7 state may be discarded
* Kernel state must remain unchanged

Recovery requires:

* Re-fetching kernel state
* Rebuilding derived views

No state reconciliation is permitted.

---

## 10. State Consistency Rules

Phase 7 MUST present state consistently:

* All views must reflect a single kernel snapshot
* Mixed-version presentation is forbidden

If consistent state cannot be obtained, Phase 7 must refuse to render.

---

## 11. State and Testing Requirements

Phase 7 state handling MUST be tested to ensure:

* Loss of control-plane state does not affect kernel state
* Derived state is recomputable
* No hidden persistence exists

Tests MUST simulate:

* Control-plane crashes
* UI reloads
* Client restarts

---

## 12. Final Statement

Phase 7 state exists to support humans, not to define reality.

The moment Phase 7 begins to remember things the kernel does not, it becomes dangerous.

> **If Phase 7 state ever becomes required for correctness, Phase 7 is wrong.**

This state model is absolute.

---

END OF PHASE 7 STATE MODEL
