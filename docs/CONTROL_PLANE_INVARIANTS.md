# PHASE 7 INVARIANTS — CONTROL PLANE SAFETY & DISCIPLINE

**Project:** AeroDB
**Phase:** Phase 7 — Control Plane, Admin UI, and Operator Tooling
**Status:** AUTHORITATIVE · NON-NEGOTIABLE

---

## 1. Purpose of This Document

This document defines the **absolute invariants** governing Phase 7.

An invariant is a rule that MUST hold:

* During normal operation
* Under operator error
* Under partial failure
* Under crashes of the control plane
* Under retries and reconnects

If any invariant is violated, Phase 7 is **incorrect**, regardless of intent or convenience.

Phase 7 invariants exist to ensure that **power does not become autonomy**.

---

## 2. Relationship to Earlier Phases

Phase 7 invariants are **strictly subordinate** to Phases 0–6.

Rules:

* Phase 7 MUST NOT weaken, reinterpret, or bypass any invariant from Phases 0–6
* Phase 7 invariants apply only to the control plane
* If Phase 7 conflicts with the kernel, the kernel always wins

Phase 7 is replaceable. The correctness kernel is not.

---

## 3. Invariant Categories

Phase 7 invariants are grouped into the following categories:

1. Authority & Intent Invariants
2. Execution Invariants
3. Failure Invariants
4. Determinism Invariants
5. Observability & Audit Invariants
6. Scope & Non-Interference Invariants

Every invariant is mandatory.

---

## 4. Authority & Intent Invariants

### P7-A1 — No Implicit Authority

Phase 7 MUST NOT create authority implicitly.

Rules:

* Every mutating action MUST originate from an explicit operator request
* Viewing state MUST NOT grant authority
* UI focus, refresh, or reconnect MUST NOT trigger actions

Violation of this invariant is a critical bug.

---

### P7-A2 — Explicit Human Intent

Every mutating action MUST be traceable to a conscious human decision.

Rules:

* Commands MUST be explicitly issued
* Dangerous actions MUST require confirmation
* No default acceptance is allowed

If intent cannot be proven, the action MUST NOT execute.

---

### P7-A3 — No Delegated Decision-Making

Phase 7 MUST NOT decide on behalf of the operator.

Rules:

* No automatic recommendations that execute
* No ranking or scoring of nodes
* No policy engines

Phase 7 may explain facts, never choices.

---

## 5. Execution Invariants

### P7-E1 — Single Explicit Action

Each operator command MUST correspond to exactly one kernel action.

Rules:

* No command chaining
* No implicit follow-up actions
* No batch mutation without explicit repetition

If multiple kernel actions are desired, they must be requested separately.

---

### P7-E2 — No Partial Execution

Phase 7 MUST NOT produce partial success.

Rules:

* Actions are either executed fully or not at all
* If execution outcome is unknown, it MUST be treated as failed

Ambiguous outcomes are forbidden.

---

### P7-E3 — Kernel Validation Is Final

All safety decisions are made by the kernel.

Rules:

* Phase 7 MUST forward requests verbatim
* Kernel rejections MUST NOT be overridden
* Phase 7 MUST surface kernel errors exactly

Control plane logic may not second-guess correctness logic.

---

## 6. Failure Invariants

### P7-F1 — Fail Closed

Under any failure, Phase 7 MUST fail closed.

Rules:

* No retries that mutate state
* No fallback execution paths
* No speculative execution

Operator re-issuance is the only recovery path.

---

### P7-F2 — Control Plane Crash Safety

A crash of the control plane MUST NOT:

* Mutate kernel state
* Complete partially executed actions
* Leave the system in an ambiguous state

If the control plane crashes mid-request, the request is considered not executed.

---

### P7-F3 — No Hidden Retries

Phase 7 MUST NOT retry mutating actions automatically.

Rules:

* Network retries MUST be surfaced
* Duplicate requests MUST be detected or rejected

Silent retry is forbidden.

---

## 7. Determinism Invariants

### P7-D1 — Deterministic Presentation

Given identical kernel state and inputs, Phase 7 MUST:

* Display identical information
* Produce identical explanations

UI rendering MUST NOT affect semantics.

---

### P7-D2 — Deterministic Decision Results

Given identical requests and kernel state, Phase 7 MUST:

* Accept or reject requests identically
* Surface identical denial reasons

No timing-based divergence is allowed.

---

## 8. Observability & Audit Invariants

### P7-O1 — Complete Observability

All Phase 7 actions MUST be observable.

Rules:

* Requests, decisions, and outcomes must be logged
* Observability must not block execution

---

### P7-O2 — Explainability

Every decision MUST be explainable.

Rules:

* Rejections MUST reference violated invariants
* Explanations MUST be deterministic

---

### P7-O3 — Auditability

Phase 7 MUST support post-incident reconstruction.

Audit records MUST include:

* Operator identity (if known)
* Timestamp
* Requested action
* Kernel state summary
* Outcome

If an action cannot be reconstructed, this invariant is violated.

---

## 9. Scope & Non-Interference Invariants

### P7-S1 — No Background Activity

Phase 7 MUST NOT run background processes that mutate state.

All mutation must be synchronous with an operator request.

---

### P7-S2 — No Correctness Interference

Phase 7 MUST NOT:

* Alter kernel timing
* Alter execution order
* Alter durability semantics

Kernel behavior must be identical with or without Phase 7.

---

### P7-S3 — No Expansion Beyond Scope

Any feature not explicitly allowed in CONTROL_PLANE_SCOPE.md is forbidden.

This invariant is enforced socially and technically.

---

## 10. Enforcement & Testing Requirements

Each invariant MUST:

* Be enforceable by code
* Be testable
* Have at least one negative test

If an invariant cannot be tested, Phase 7 is incomplete.

---

## 11. Final Statement

Phase 7 invariants exist to prevent **operator tooling from becoming an autonomous system**.

They are intentionally conservative.

> **If Phase 7 ever acts without a human asking it to, it is wrong.**

These invariants are absolute.

---

END OF PHASE 7 INVARIANTS
