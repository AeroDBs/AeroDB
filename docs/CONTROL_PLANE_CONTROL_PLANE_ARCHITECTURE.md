# PHASE 7 CONTROL PLANE ARCHITECTURE — STRUCTURE, BOUNDARIES, AND DATA FLOW

**Project:** AeroDB
**Phase:** Phase 7 — Control Plane, Admin UI, and Operator Tooling
**Status:** AUTHORITATIVE · PRE-IMPLEMENTATION

---

## 1. Purpose of This Document

This document defines the **architectural structure of the Phase 7 control plane**.

Its purpose is to:

* Establish clear component boundaries
* Define allowed communication paths
* Prevent hidden coupling with the correctness kernel
* Ensure Phase 7 remains **replaceable, observable, and non-authoritative**

Architecture exists to **constrain behavior**, not to enable convenience.

---

## 2. Architectural Principle

> **The control plane observes and requests. It never decides or executes.**

All correctness-critical execution occurs exclusively within Phases 0–6.

Phase 7 architecture must ensure:

* No backchannels into the kernel
* No implicit execution paths
* No stateful coupling that affects correctness

If the control plane is removed, the kernel must continue to function correctly.

---

## 3. High-Level Architecture Overview

Phase 7 consists of four primary components:

1. **Control Plane API**
2. **Operator Interfaces (UI / CLI)**
3. **Explanation & Observability Layer**
4. **Kernel Boundary Adapter**

These components are strictly layered.

```
┌────────────────────────────┐
│   Operator Interfaces      │  (UI / CLI)
└────────────▲───────────────┘
             │
┌────────────┴───────────────┐
│      Control Plane API     │
└────────────▲───────────────┘
             │
┌────────────┴───────────────┐
│ Explanation & Observability│
└────────────▲───────────────┘
             │
┌────────────┴───────────────┐
│   Kernel Boundary Adapter  │
└────────────▲───────────────┘
             │
┌────────────┴───────────────┐
│   Correctness Kernel       │ (Phases 0–6)
└────────────────────────────┘
```

Data flows **downward** only. Decisions flow **upward** only as explanations.

---

## 4. Component Responsibilities

### 4.1 Operator Interfaces (UI / CLI)

Responsibilities:

* Present state to humans
* Collect explicit operator intent
* Require confirmations for dangerous actions

Rules:

* MUST NOT contain business logic
* MUST NOT cache authoritative state
* MUST NOT retry mutating requests automatically

Interfaces are replaceable.

---

### 4.2 Control Plane API

Responsibilities:

* Validate request structure
* Enforce Phase 7 invariants
* Orchestrate confirmation flows
* Route requests to kernel boundary adapter

Rules:

* MUST be stateless across restarts
* MUST treat kernel responses as authoritative
* MUST NOT mutate kernel state directly

---

### 4.3 Explanation & Observability Layer

Responsibilities:

* Produce explanations for requests
* Surface kernel decisions
* Aggregate observable events

Rules:

* MUST be passive
* MUST NOT influence control flow
* MUST NOT trigger execution

Observability must never become a control mechanism.

---

### 4.4 Kernel Boundary Adapter

Responsibilities:

* Translate control-plane requests into kernel calls
* Enforce strict API boundaries
* Prevent forbidden invocation patterns

Rules:

* MUST expose only explicitly allowed kernel operations
* MUST NOT batch or chain kernel calls
* MUST NOT infer or synthesize kernel behavior

This is the **only** component allowed to call kernel APIs.

---

## 5. Communication Paths

### 5.1 Allowed Paths

* UI → Control Plane API
* CLI → Control Plane API
* Control Plane API → Explanation Layer
* Control Plane API → Kernel Boundary Adapter
* Kernel Boundary Adapter → Kernel
* Kernel → Explanation Layer → Control Plane API → UI / CLI

---

### 5.2 Forbidden Paths

* UI / CLI → Kernel (direct)
* Explanation Layer → Kernel
* Kernel → UI / CLI (direct)
* Control Plane API → Kernel (bypassing adapter)

Any forbidden path is an architectural violation.

---

## 6. Request Lifecycle

1. Operator issues request via UI or CLI
2. Control Plane API validates structure and authority
3. Explanation Layer generates pre-execution explanation
4. Operator confirms (if required)
5. Control Plane API forwards request to Kernel Boundary Adapter
6. Kernel validates and executes (or rejects)
7. Result propagates back through Explanation Layer
8. Outcome is surfaced to operator

At no point does Phase 7 assume success.

---

## 7. State Placement Rules

Phase 7 state placement MUST follow `CONTROL_PLANE_STATE_MODEL.md`:

* No authoritative state stored in Phase 7
* Only ephemeral and derived state allowed
* No recovery logic required for Phase 7 state

Any persisted state in Phase 7 is a design error.

---

## 8. Failure Containment

Failures must be contained to the layer in which they occur:

* UI failure → UI restarts
* API failure → request aborted
* Explanation failure → explanation unavailable
* Adapter failure → request rejected

Kernel state must remain unchanged under all Phase 7 failures.

---

## 9. Extensibility Rules

Phase 7 architecture MAY allow:

* New UI implementations
* New CLI implementations
* New explanation renderers

Phase 7 architecture MUST NOT allow:

* New execution paths
* Background schedulers
* Automated decision engines

Extensibility must never increase system autonomy.

---

## 10. Testing Requirements

The architecture MUST be tested to ensure:

* Forbidden paths are impossible
* Removing Phase 7 does not affect kernel behavior
* No component holds authoritative state

Architecture tests are mandatory.

---

## 11. Final Statement

Phase 7 architecture exists to **constrain power, not distribute it**.

It is deliberately layered, deliberately limited, and deliberately boring.

> **If Phase 7 can accidentally execute something, the architecture is wrong.**

This architecture is authoritative.

---

END OF PHASE 7 CONTROL PLANE ARCHITECTURE
