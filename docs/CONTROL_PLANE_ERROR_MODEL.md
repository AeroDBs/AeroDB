# PHASE 7 ERROR MODEL — EXPLICIT FAILURE, MEANING, AND SURFACING

**Project:** AeroDB
**Phase:** Phase 7 — Control Plane, Admin UI, and Operator Tooling
**Status:** AUTHORITATIVE · NON-NEGOTIABLE

---

## 1. Purpose of This Document

This document defines the **error model for Phase 7**.

Its purpose is to ensure that:

* Errors are explicit, deterministic, and meaningful
* Errors never mask or reinterpret kernel decisions
* Errors do not trigger retries, fallbacks, or hidden behavior
* Operators can understand *why* an action failed and *what did not happen*

Errors are a **safety mechanism**, not an inconvenience.

---

## 2. Foundational Principle

> **An unclear error is worse than a failed action.**

Phase 7 errors MUST:

* Preserve correctness
* Preserve determinism
* Preserve operator responsibility

If an error cannot be explained precisely, execution MUST NOT proceed.

---

## 3. Error Domains

Phase 7 errors originate from exactly four domains:

1. **Operator Input Errors**
2. **Phase 7 Validation Errors**
3. **Kernel Rejection Errors**
4. **Infrastructure / Transport Errors**

Errors MUST be classified into one and only one domain.

---

## 4. Operator Input Errors

### 4.1 Definition

Errors caused by invalid or incomplete operator input.

Examples:

* Missing required arguments
* Invalid node identifiers
* Malformed requests

### 4.2 Rules

* MUST be detected before kernel interaction
* MUST NOT reach the kernel
* MUST be reported with precise field-level detail

Execution MUST NOT occur.

---

## 5. Phase 7 Validation Errors

### 5.1 Definition

Errors arising from Phase 7 invariant enforcement or precondition checks.

Examples:

* Missing confirmation
* Invalid authority level
* Confirmation reuse attempt
* Scope violation

### 5.2 Rules

* MUST be deterministic
* MUST reference violated Phase 7 invariants
* MUST NOT be retried automatically

Execution MUST NOT occur.

---

## 6. Kernel Rejection Errors

### 6.1 Definition

Errors returned by the correctness kernel (Phases 0–6).

Examples:

* Invariant violations
* Promotion safety rejection
* WAL prefix mismatch

### 6.2 Rules

* MUST be surfaced verbatim
* MUST NOT be wrapped or softened
* MUST preserve kernel error codes and explanations

Phase 7 MUST NOT reinterpret kernel intent.

---

## 7. Infrastructure / Transport Errors

### 7.1 Definition

Errors arising from:

* Network failure
* Control-plane crash
* Timeout before kernel acknowledgement

### 7.2 Rules

* MUST be treated as **unknown execution outcome**
* MUST NOT assume success
* MUST require explicit operator re-issuance

If kernel state cannot prove execution, the action is considered not executed.

---

## 8. Error Semantics

### 8.1 Determinism

Given identical inputs and system state:

* The same error MUST be produced
* With the same classification and explanation

---

### 8.2 No Error Recovery

Phase 7 MUST NOT:

* Retry failed actions
* Attempt fallback execution
* Mask errors with retries

All recovery is explicit and human-driven.

---

### 8.3 No Partial Success

Errors MUST imply:

* No kernel mutation occurred
* Or kernel state must prove otherwise

Ambiguous outcomes are forbidden.

---

## 9. Error Representation

Errors MUST include:

* Error domain
* Stable error code
* Human-readable message
* Referenced invariant(s), if applicable
* Execution outcome (executed / not executed)

Free-form or ad-hoc errors are forbidden.

---

## 10. Error and Observability Interaction

Errors MUST:

* Be observable
* Be auditable
* Appear in timelines

Errors MUST NOT:

* Influence future behavior
* Trigger recommendations

---

## 11. Error and Confirmation Interaction

Errors MUST NOT:

* Consume confirmations
* Allow confirmation reuse

A new attempt always requires new confirmation.

---

## 12. Error Testing Requirements

The error model MUST be tested to ensure:

* Correct domain classification
* Correct invariant references
* No retries occur implicitly
* Deterministic output

Tests MUST include:

* Operator input errors
* Phase 7 invariant violations
* Kernel rejections
* Transport failures

---

## 13. Final Statement

Errors exist to **prevent silent harm**.

They must be:

* Explicit
* Honest
* Deterministic
* Action-stopping

> **If an error can be ignored, Phase 7 is wrong.**

This error model is authoritative.

---

END OF PHASE 7 ERROR MODEL
