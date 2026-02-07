# PHASE 7 COMMAND MODEL — EXPLICIT OPERATOR ACTIONS AND GUARANTEES

**Project:** AeroDB
**Phase:** Phase 7 — Control Plane, Admin UI, and Operator Tooling
**Status:** AUTHORITATIVE · NON-NEGOTIABLE

---

## 1. Purpose of This Document

This document defines the **complete and closed set of operator commands** exposed by Phase 7.

Its goals are to:

* Enumerate every allowed mutating and non-mutating command
* Define command semantics precisely
* Prevent the introduction of ad-hoc or implicit actions
* Ensure every command is auditable, explainable, and deterministic

If a command is not defined in this document, it **must not exist**.

---

## 2. Fundamental Command Principles

All Phase 7 commands MUST obey the following principles:

1. **Explicitness** — every command is intentionally issued by a human
2. **Singularity** — one command maps to one kernel action
3. **Determinism** — same input yields same result
4. **Explainability** — every command can be explained before execution
5. **Auditability** — every command leaves a permanent audit trail

No command may violate Phase 7 invariants.

---

## 3. Command Classification

Phase 7 commands are divided into three classes:

1. **Inspection Commands** (read-only)
2. **Diagnostic Commands** (read-only but potentially expensive)
3. **Control Commands** (mutating, high-risk)

Each class has different confirmation and failure requirements.

---

## 4. Inspection Commands (Read-Only)

Inspection commands MUST NOT mutate any kernel state.

### 4.1 `inspect_cluster_state`

**Purpose:**

* Retrieve current cluster topology and roles

**Kernel Interaction:**

* Read-only

**Confirmation Required:** No

**Failure Semantics:**

* Fail closed
* Partial data forbidden

---

### 4.2 `inspect_node`

**Purpose:**

* Inspect a specific node’s role, WAL position, and health

**Kernel Interaction:**

* Read-only

**Confirmation Required:** No

---

### 4.3 `inspect_replication_status`

**Purpose:**

* View replication lag and replica health

**Kernel Interaction:**

* Read-only

---

### 4.4 `inspect_promotion_state`

**Purpose:**

* View current promotion / demotion state machine status

**Kernel Interaction:**

* Read-only

---

## 5. Diagnostic Commands

Diagnostic commands are read-only but may be disruptive or expensive.

### 5.1 `run_diagnostics`

**Purpose:**

* Collect kernel diagnostic information

**Kernel Interaction:**

* Read-only

**Confirmation Required:** Yes

**Rules:**

* Must declare cost before execution
* Must not mutate state

---

### 5.2 `inspect_wal`

**Purpose:**

* Inspect WAL metadata and boundaries

**Kernel Interaction:**

* Read-only

---

### 5.3 `inspect_snapshots`

**Purpose:**

* Inspect available snapshots and checkpoints

**Kernel Interaction:**

* Read-only

---

## 6. Control Commands (Mutating)

Control commands mutate kernel state and are strictly regulated.

### 6.1 `request_promotion`

**Purpose:**

* Request promotion of a replica to primary

**Kernel Interaction:**

* Promotion state machine

**Confirmation Required:** Yes (mandatory)

**Preconditions:**

* Kernel validation must succeed
* WAL prefix rules must hold

**Failure Semantics:**

* Reject on ambiguity

---

### 6.2 `request_demotion`

**Purpose:**

* Request demotion of a primary

**Kernel Interaction:**

* Demotion state machine

**Confirmation Required:** Yes

---

### 6.3 `force_promotion`

**Purpose:**

* Explicit operator override for promotion

**Kernel Interaction:**

* Promotion with override flag

**Confirmation Required:** Yes (explicit override acknowledgement)

**Rules:**

* Must reference overridden invariants
* Must surface full risk

---

## 7. Command Preconditions and Validation

For every command:

* Preconditions MUST be checked before execution
* Failed preconditions MUST be reported explicitly
* Phase 7 MUST NOT attempt corrective action

Validation failure always results in rejection.

---

## 8. Confirmation Model Integration

Control commands MUST integrate with the confirmation model:

* Clear description of action
* Clear description of consequences
* Explicit acknowledgement

No confirmation → no execution.

---

## 9. Command Idempotency and Duplication

Commands are **not implicitly idempotent**.

Rules:

* Duplicate requests MUST be detected or rejected
* Silent re-execution is forbidden

---

## 10. Error Semantics

Command errors MUST:

* Reference kernel or Phase 7 invariants
* Be deterministic
* Be auditable

No generic errors are allowed.

---

## 11. Extensibility Rules

New commands MAY be added only by:

* Updating this document
* Updating testing strategy
* Passing an explicit audit

Ad-hoc commands are forbidden.

---

## 12. Testing Requirements

Every command MUST have tests covering:

* Successful execution (if allowed)
* Validation failure
* Confirmation refusal
* Crash safety

Missing tests block Phase 7 freeze.

---

## 13. Final Statement

The Phase 7 command set is intentionally small.

Every command represents power.

> **If an operator action is not defined here, it must not exist.**

This command model is closed and authoritative.

---

END OF PHASE 7 COMMAND MODEL
