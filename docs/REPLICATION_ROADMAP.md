# REPLICATION_ROADMAP.md

## AeroDB Phase 2B — Replication Roadmap

### Status

* This document governs **replication design sequencing**
* It defines **what must be designed, in what order, and why**
* No implementation is permitted until all design stages are approved
* Phase-1 and MVCC semantics are **frozen and authoritative**

---

## 1. Replication Entry Conditions (Already Satisfied)

Replication work proceeds under the following **verified conditions**:

* Phase-1 is complete, correct, and frozen
* MVCC (Phase 2A) is complete, correct, and frozen
* WAL, CommitId, snapshot, and GC semantics are authoritative
* Crash testing infrastructure exists and is deterministic

Replication is an **extension**, not a correction.

---

## 2. Design-First Rule (Strict)

Replication follows the same rule as MVCC:

> **No code before specs**
> **No optimization before correctness**
> **No liveness before safety**

Every document below must be approved before moving forward.

---

## 3. Replication Design Stages

### Stage 1 — Replication Model Definition (Conceptual)

This stage defines **what replication is**, not how it is implemented.

#### 3.1 `REPLICATION_MODEL.md`

**Purpose**

* Define the Primary / Replica roles formally
* Define authority boundaries
* Define allowed and forbidden state transitions

**Must Answer**

* What makes a node a Primary?
* What makes a node a Replica?
* When must writes be rejected?
* What states are legal vs illegal?

**Exit Criteria**

* Single-writer invariant is unambiguous
* No dual-authority state exists
* No time-based assumptions appear

---

### Stage 2 — State Transfer Semantics (Design Only)

This stage defines **how state moves**, without code.

#### 3.2 `REPLICATION_LOG_FLOW.md`

**Purpose**

* Define WAL shipping semantics
* Define ordering guarantees
* Define gap detection and handling

**Must Answer**

* Is replication log-based, snapshot-based, or hybrid?
* How are WAL gaps detected?
* How is ordering preserved?
* What happens if records are missing?

**Constraints**

* WAL is the unit of truth
* CommitId ordering is preserved exactly

---

#### 3.3 `REPLICATION_SNAPSHOT_TRANSFER.md`

**Purpose**

* Define snapshot-based replication bootstrap
* Define snapshot + WAL catch-up semantics

**Must Answer**

* When is snapshot transfer required?
* How is snapshot completeness verified?
* How does a Replica resume WAL replay?

**Exit Criteria**

* Snapshot transfer does not alter semantics
* WAL replay resumes deterministically

---

### Stage 3 — Read Semantics & Safety

This stage defines **what Replicas are allowed to expose**.

#### 3.4 `REPLICATION_READ_SEMANTICS.md`

**Purpose**

* Define whether Replicas may serve reads
* Define MVCC interaction on Replicas
* Define read refusal conditions

**Must Answer**

* Can Replicas serve reads?
* Under what commit boundary?
* How is staleness defined explicitly?
* When must reads be refused?

**Constraints**

* No speculative reads
* No future visibility
* No weakening of MVCC rules

---

### Stage 4 — Failure & Crash Semantics

This stage defines **what happens when things go wrong**.

#### 3.5 `REPLICATION_FAILURE_MATRIX.md`

**Purpose**

* Enumerate every meaningful replication failure
* Define explicit outcomes

**Must Cover**

* Primary crash
* Replica crash
* Network partition
* Partial WAL transfer
* Snapshot transfer interruption
* WAL corruption during replication

**Exit Criteria**

* Every failure maps to exactly one outcome
* No “undefined” or “best-effort” states

---

### Stage 5 — Recovery & Restart Semantics

This stage defines **how nodes come back safely**.

#### 3.6 `REPLICATION_RECOVERY.md`

**Purpose**

* Define restart behavior for Primary and Replicas
* Define safety checks before resuming replication

**Must Answer**

* How does a Replica validate its history?
* How does a Primary reassert authority?
* When must nodes refuse to start?

---

### Stage 6 — Compatibility & Proof

This stage proves replication does not break AeroDB.

#### 3.7 `REPLICATION_COMPATIBILITY.md`

**Purpose**

* Prove Phase-1 + MVCC compatibility
* Prove snapshot and restore correctness

---

#### 3.8 `REPLICATION_PROOFS.md`

**Purpose**

* Argument-based proofs for:

  * No divergence
  * Deterministic replay
  * Safe lag
  * Crash correctness

Replication correctness must be provable without code.

---

### Stage 7 — Implementation Gating

Only after all design documents are approved:

#### 3.9 `REPLICATION_BUILD_PROMPTS.md`

* Step-by-step implementation prompts
* Crash-gated, spec-linked
* No heuristic shortcuts

---

#### 3.10 `REPLICATION_READINESS.md`

* Readiness checklist
* Semantic freeze statement
* Explicit non-features list

After this, replication semantics are frozen.

---

## 4. Explicit Deferrals

The following are **explicitly deferred** beyond this roadmap:

* Automatic leader election
* Dynamic membership
* Quorum writes
* Read-your-own-writes on Replicas
* Performance optimizations
* Geo-replication

If needed, they require a **new phase**.

---

## 5. Roadmap Summary

Replication design proceeds in this exact order:

1. Authority model
2. WAL & snapshot flow
3. Read semantics
4. Failure matrix
5. Recovery rules
6. Compatibility & proofs
7. Build prompts
8. Semantic freeze

Skipping any step is a correctness violation.
