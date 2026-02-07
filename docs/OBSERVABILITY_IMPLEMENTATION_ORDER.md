# PHASE 5 — IMPLEMENTATION ORDER (REPLICATION)

## Status

- Phase: **5**
- Authority: **Normative**
- Scope: **Replication implementation sequencing**
- Depends on:
  - `OBSERVABILITY_VISION.md`
  - `OBSERVABILITY_INVARIANTS.md`
  - `REPL_*` specifications
  - `MVCC_*` specifications
  - `CORE_*` specifications

This document defines the **only allowed implementation order**
for Phase 5 replication.

If implementation proceeds out of order,
correctness proofs no longer hold.

---

## 1. Purpose

Replication is a **cross-cutting concern**:
- WAL
- Storage
- MVCC
- Recovery
- Concurrency
- Failure handling

Incorrect sequencing causes:
- Hidden invariant violations
- “Temporary” unsafe states
- Non-reproducible bugs

This document exists to **prevent partial correctness**.

---

## 2. Absolute Rule

> You MUST complete each stage fully  
> before starting the next.

“Partially implemented” stages are forbidden.

---

## 3. Phase 5 Implementation Stages (Authoritative Order)

There are **7 stages**.  
They MUST be implemented **in order**.

---

## STAGE 1 — Replication Configuration & Role Declaration

### Goal
Establish *static* replication identity.

### Must Implement
- Node role: `Primary` or `Replica`
- Immutable role at startup
- Replica identity (ID, UUID, or stable token)
- Replication disabled path (default-safe)

### Must NOT Implement Yet
- Networking
- WAL shipping
- Replica threads

### Required Invariants
- P5-I1 (No semantic redefinition)
- P5-I16 (Replication removable)

### Completion Criteria
- Primary runs identically with replication disabled
- Replica mode refuses all writes
- Role is observable via DX

---

## STAGE 2 — Replica WAL Receiver (Ingress Only)

### Goal
Allow a replica to **receive WAL bytes**, but not apply them.

### Must Implement
- WAL segment receiver
- Sequential append buffer
- Strict ordering enforcement
- No validation beyond framing

### Must NOT Implement Yet
- WAL checksum validation
- WAL application
- CommitId advancement

### Required Invariants
- P5-I4 (WAL prefix rule)
- P5-I10 (Explicit failure over progress)

### Completion Criteria
- WAL bytes can be received and stored
- WAL gaps cause hard failure
- Replica state remains unchanged

---

## STAGE 3 — WAL Validation Layer

### Goal
Prove WAL correctness **before** application.

### Must Implement
- Checksum validation
- Continuity validation
- Segment boundary handling
- Explicit invalid-WAL failure paths

### Must NOT Implement Yet
- WAL application
- Storage mutation
- MVCC updates

### Required Invariants
- P5-I5 (Validate before apply)
- P5-I11 (Crash safety)

### Completion Criteria
- Invalid WAL is detected deterministically
- No WAL-derived state becomes visible
- Validation is restart-safe

---

## STAGE 4 — WAL Application (Replica Storage)

### Goal
Apply validated WAL to replica storage.

### Must Implement
- WAL replay into append-only storage
- Deterministic replay ordering
- Crash-safe replay checkpoints

### Must NOT Implement Yet
- Replica read serving
- Snapshot bootstrap
- MVCC snapshot exposure

### Required Invariants
- P5-I4 (Prefix rule)
- P5-I6 (CommitId authority isolation)

### Completion Criteria
- Replica storage matches primary-derived state
- Crash + replay produces identical state
- CommitId advancement is observable but read-blocked

---

## STAGE 5 — Snapshot Bootstrap (Cold Start)

### Goal
Allow a replica to start from a snapshot instead of empty state.

### Must Implement
- Snapshot transfer protocol
- Snapshot integrity validation
- Exact MVCC cut handling
- WAL resume offset calculation

### Must NOT Implement Yet
- Replica reads
- Optimizations

### Required Invariants
- P5-I8 (Snapshot is a cut)
- P5-I9 (Snapshot + WAL continuity)

### Completion Criteria
- Replica can bootstrap from snapshot
- WAL resumes exactly after snapshot cut
- No reads are served during bootstrap

---

## STAGE 6 — Replica Recovery

### Goal
Make replication **restart-safe**.

### Must Implement
- Crash recovery for replica
- WAL replay resumption
- Partial state cleanup
- Deterministic recovery path

### Must NOT Implement Yet
- Read safety shortcuts

### Required Invariants
- P5-I11 (Crash safety)
- P5-I4 (Prefix rule restored)

### Completion Criteria
- Replica recovers identically after crash
- No partial WAL application leaks
- Recovery explanation is available

---

## STAGE 7 — Replica Read Safety Gate

### Goal
Allow reads **only when provably safe**.

### Must Implement
- Read-safety predicate
- MVCC snapshot checks
- Explicit refusal paths
- Explanation of read safety

### Must NOT Implement
- Read-your-own-writes
- Stale reads without proof
- Heuristic lag allowances

### Required Invariants
- P5-I12 (Read safety proven)
- MVCC invariants

### Completion Criteria
- Unsafe reads are refused
- Safe reads behave identically to primary reads
- `/v1/explain/replication` proves safety

---

## 4. Forbidden Shortcuts (Explicit)

The following are **illegal**:

- Implementing reads before recovery
- Applying WAL before validation
- Serving reads during bootstrap
- Inferring safety from time or lag
- “Temporary” invariant violations

If a shortcut seems necessary,
the design is wrong.

---

## 5. Observability Requirements Per Stage

Each stage MUST expose:
- Current replication state
- Blocking reasons
- Failure causes

Hidden progress is forbidden.

---

## 6. Testing Order (Mirrors Implementation)

Tests MUST be added in the same order:

1. Role enforcement tests
2. WAL ingress tests
3. WAL validation failure tests
4. WAL replay crash tests
5. Snapshot bootstrap tests
6. Replica recovery tests
7. Read safety tests

Skipping test order is not allowed.

---

## 7. Final Rule

> Replication correctness comes from sequencing,  
> not from cleverness.

If the order is respected,
correctness follows.

If the order is violated,
bugs become invisible.

---

END OF DOCUMENT
