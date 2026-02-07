# PHASE 7 SCOPE — CONTROL PLANE & OPERATOR INTERACTION

**Project:** AeroDB
**Phase:** Phase 7 — Control Plane, Admin UI, and Operator Tooling
**Status:** AUTHORITATIVE · PRE-IMPLEMENTATION

---

## 1. Purpose of This Document

This document defines the **exact scope boundaries** of Phase 7.

Its purpose is to:

* Explicitly state **what Phase 7 is allowed to do**
* Explicitly state **what Phase 7 is forbidden from doing**
* Prevent scope creep, implicit behavior, and accidental automation

Phase 7 introduces power. This document limits that power.

If any behavior is not explicitly permitted in this document, it is **out of scope**.

---

## 2. Phase 7 Position in the AeroDB Architecture

Phase 7 exists **above** the correctness kernel (Phases 0–6).

Architectural layering:

1. **Phases 0–6** — Correctness kernel

   * Durability
   * Determinism
   * Replication correctness
   * Failover safety

2. **Phase 7** — Control plane

   * Human interaction
   * Visibility
   * Explicit commands

Phase 7:

* May observe kernel state
* May request kernel actions
* May explain kernel decisions

Phase 7:

* May NOT reinterpret kernel state
* May NOT infer kernel safety
* May NOT bypass kernel validation

---

## 3. In-Scope Capabilities (Allowed)

The following capabilities are **explicitly in scope** for Phase 7.

### 3.1 Read-Only Observability

Phase 7 MAY provide:

* Cluster topology views
* Node role and health summaries
* Replication lag visibility
* WAL position visibility
* Promotion state visibility
* Historical timelines of events

Rules:

* All views must be derived from authoritative kernel state
* No derived view may be authoritative
* Observability must be passive

---

### 3.2 Explicit Operator Commands

Phase 7 MAY expose **explicit, human-triggered commands**, including:

* Request promotion or demotion
* Request role inspection
* Trigger diagnostics
* Inspect snapshots, checkpoints, and WAL state

Rules:

* Commands must map 1:1 to kernel operations
* No command may bundle multiple kernel actions implicitly
* Commands must be rejected if kernel validation fails

---

### 3.3 Pre-Execution Explanation

Phase 7 MAY:

* Explain what a requested command will do
* Explain why it may be rejected
* Explain which invariants are involved

Rules:

* Explanations must be deterministic
* Explanations must not speculate
* Explanations must not guarantee success

---

### 3.4 Explicit Confirmation Flows

Phase 7 MAY require:

* Explicit confirmation for dangerous actions
* Multi-step acknowledgements for irreversible commands

Rules:

* Confirmations must be explicit
* Confirmations must never auto-expire into execution
* No implicit confirmation is allowed

---

### 3.5 Audit & Trace Surfaces

Phase 7 MAY provide:

* Operator action logs
* Command execution timelines
* Decision explanations
* Post-incident reconstruction tools

Rules:

* Audit data must be append-only
* Audit data must not affect execution

---

## 4. Explicitly Out-of-Scope Capabilities (Forbidden)

The following capabilities are **explicitly forbidden** in Phase 7.

### 4.1 Automation and Autonomy

Phase 7 MUST NOT:

* Automatically trigger promotion or demotion
* Automatically retry failed commands
* Automatically heal replicas
* Automatically reconfigure the cluster

Any behavior that changes system state **without direct human intent** is forbidden.

---

### 4.2 Heuristics and Policy Engines

Phase 7 MUST NOT:

* Evaluate thresholds
* Rank nodes
* Recommend actions
* Make safety judgments

The system must never tell the operator what it “thinks should be done”.

---

### 4.3 Background Control Loops

Phase 7 MUST NOT:

* Poll kernel state and act on changes
* Schedule actions
* Maintain feedback loops

All actions must originate from a human request.

---

### 4.4 Implicit State Mutation

Phase 7 MUST NOT:

* Mutate state as a side effect of viewing
* Mutate state during explanation
* Mutate state during validation

Read-only operations must remain read-only.

---

### 4.5 Correctness Overrides

Phase 7 MUST NOT:

* Override durability rules
* Override replication invariants
* Override fail-closed behavior
* Override kernel rejection decisions

If the kernel rejects an action, Phase 7 must surface the rejection verbatim.

---

### 4.6 Bulk or Compound Actions

Phase 7 MUST NOT:

* Chain multiple actions implicitly
* Execute batches of mutating commands without explicit operator intent

Every kernel action must be individually requested.

---

## 5. Explicit Non-Goals

Phase 7 does NOT aim to:

* Optimize performance
* Reduce operator workload via automation
* Hide complexity
* Abstract away failure modes

Phase 7 intentionally exposes complexity rather than masking it.

---

## 6. Failure Handling Scope

Phase 7 is responsible for:

* Surfacing failures clearly
* Ensuring no partial execution
* Requiring explicit operator re-issuance after failure

Phase 7 is NOT responsible for:

* Recovering from failures automatically
* Rolling forward incomplete actions
* Repairing inconsistent state

---

## 7. Interaction with Future Phases

Phase 7 MUST NOT assume:

* Authentication mechanisms
* Authorization models
* Multi-tenant environments
* Cloud-managed infrastructure

Those concerns belong to later phases.

---

## 8. Scope Enforcement Rule

If a proposed feature or behavior:

* Is not listed in Section 3
* Or violates any rule in Section 4

Then it is **out of scope** for Phase 7 and must be rejected.

There are no implicit inclusions.

---

## 9. Final Statement

Phase 7 is intentionally narrow.

It is designed to:

* Give operators power
* Without giving the system autonomy

Any attempt to expand Phase 7 beyond this scope risks violating AeroDB’s core philosophy.

> **Phase 7 must never act on behalf of the operator.**

This scope is non-negotiable.

---

END OF PHASE 7 SCOPE
