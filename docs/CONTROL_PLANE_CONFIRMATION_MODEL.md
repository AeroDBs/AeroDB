# PHASE 7 CONFIRMATION MODEL — EXPLICIT CONSENT, RISK ACKNOWLEDGEMENT, AND IRREVERSIBILITY

**Project:** AeroDB
**Phase:** Phase 7 — Control Plane, Admin UI, and Operator Tooling
**Status:** AUTHORITATIVE · NON-NEGOTIABLE

---

## 1. Purpose of This Document

This document defines the **confirmation model** for Phase 7.

Its purpose is to ensure that:

* Dangerous or irreversible actions are never executed accidentally
* Operator intent is explicit, informed, and auditable
* No UI, CLI, or transport behavior can substitute for human consent

Confirmation is a **safety boundary**, not a usability feature.

---

## 2. Foundational Principle

> **No irreversible or high-risk action may execute without explicit, contemporaneous human confirmation.**

Confirmation is required to:

* Prove intent
* Transfer responsibility
* Prevent accidental execution

Absence of confirmation is always interpreted as **rejection**.

---

## 3. Actions Requiring Confirmation

The following classes of commands REQUIRE confirmation:

1. All **Control Commands** (mutating)
2. All **Override / Force Commands**
3. Any command explicitly marked as dangerous or disruptive

Read-only inspection commands NEVER require confirmation.

---

## 4. Confirmation Properties

Every valid confirmation MUST satisfy **all** of the following properties:

### 4.1 Explicitness

* Confirmation must be an explicit operator action
* Defaults, timeouts, or focus changes are forbidden

---

### 4.2 Contemporaneity

* Confirmation must occur immediately after explanation
* Stored, delayed, or pre-approved confirmations are forbidden

---

### 4.3 Specificity

* Confirmation must apply to exactly one command
* Confirmation cannot be reused

---

### 4.4 Visibility of Consequences

Before confirmation, the system MUST surface:

* The exact action to be taken
* The affected components
* The irreversible consequences (if any)
* The invariants involved

---

## 5. Confirmation Workflow

The confirmation workflow MUST follow this exact sequence:

1. Operator issues command
2. System generates pre-execution explanation
3. System requests confirmation
4. Operator explicitly confirms
5. Command is dispatched to the kernel

If any step is interrupted, the workflow is aborted.

---

## 6. Confirmation and Failure

### 6.1 Failure Before Confirmation

If failure occurs before confirmation:

* The command MUST NOT execute

---

### 6.2 Failure After Confirmation, Before Execution

If failure occurs after confirmation but before kernel dispatch:

* The command MUST NOT execute

Confirmation is not durable intent.

---

### 6.3 Failure After Execution

If failure occurs after kernel execution:

* Kernel state is authoritative
* Confirmation remains valid only for audit

---

## 7. Confirmation and Retries

Confirmations MUST NOT be reused for retries.

Rules:

* Retrying a command requires a new confirmation
* Network retries must never imply consent

---

## 8. Confirmation and Overrides

Override commands (e.g. force promotion) require **enhanced confirmation**:

* Explicit acknowledgement of overridden invariants
* Explicit acceptance of risk
* Clear statement of responsibility transfer

If enhanced confirmation is incomplete, the override MUST be rejected.

---

## 9. Confirmation Record

Every confirmation MUST produce an audit record including:

* Operator identity (if available)
* Timestamp
* Command
* Confirmation type (standard / override)
* Acknowledged risks

Lack of a confirmation record invalidates execution.

---

## 10. Forbidden Confirmation Patterns

Phase 7 MUST NOT:

* Auto-confirm actions
* Confirm via implicit gestures
* Use time-based confirmation
* Allow bulk confirmation

Any such pattern is a correctness violation.

---

## 11. Testing Requirements

The confirmation model MUST be tested to ensure:

* No execution without confirmation
* Confirmation loss aborts execution
* Confirmation cannot be replayed
* Override confirmation enforces enhanced rules

---

## 12. Final Statement

Confirmation is the final barrier between human intent and system mutation.

It must be:

* Explicit
* Informed
* Auditable
* Non-reusable

> **If an action executes without a human explicitly saying “yes” at that moment, Phase 7 is wrong.**

This confirmation model is absolute.

---

END OF PHASE 7 CONFIRMATION MODEL
