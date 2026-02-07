# PHASE 5 — OBSERVABILITY MAPPING (REPLICATION)

## Status

- Phase: **5**
- Authority: **Normative**
- Scope: **Replication observability & explanation integration**
- Depends on:
  - `DX_OBSERVABILITY_API.md`
  - `DX_EXPLANATION_MODEL.md`
  - `REPLICATION_RUNTIME_ARCHITECTURE.md`
  - `OBSERVABILITY_INVARIANTS.md`
  - `OBSERVABILITY_FAILURE_MATRIX.md`

This document defines **how replication runtime state is exposed**
via existing **Phase 4 observability and explanation mechanisms**.

Phase 5 MUST NOT introduce new observability concepts.
It must reuse and extend Phase 4 surfaces.

---

## 1. Purpose

Replication is complex and failure-prone.
Without observability, replication correctness becomes unverifiable.

This document ensures that:
- Replication state is never hidden
- Blocking conditions are visible
- Read safety decisions are explainable
- Failures can be diagnosed without logs

Observability is not optional in Phase 5.

---

## 2. Core Principle

### O5-1: Replication Is a First-Class Observable System

Every meaningful replication state MUST be:
- Observable
- Snapshot-bound
- Deterministic
- Explainable

If replication cannot explain itself,
it must refuse to proceed.

---

## 3. Mapping to DX Observability API

Phase 5 MUST use **existing DX endpoints**.

No new endpoints are introduced unless strictly required.

---

### 3.1 `/v1/replication` — Primary Endpoint

This endpoint is the **authoritative replication view**.

It MUST expose:

| Field | Meaning |
|------|--------|
| `role` | `primary` or `replica` |
| `replica_state` | Global replica state machine state |
| `replica_commit_id` | Last applied CommitId |
| `primary_commit_id` | Last known primary CommitId |
| `wal_received_offset` | Highest received WAL offset |
| `wal_validated_offset` | Highest validated WAL offset |
| `wal_applied_offset` | Highest applied WAL offset |
| `snapshot_bootstrap_state` | NotStarted / InProgress / Complete |
| `read_safety` | Safe / Unsafe |
| `blocking_reason` | Explicit reason if blocked |

No field may be inferred heuristically.

---

### 3.2 `/v1/wal` — Replica Context

When queried on a replica, `/v1/wal` MUST:

- Distinguish between:
  - Received WAL
  - Validated WAL
  - Applied WAL
- Show offsets for each stage
- Clearly mark gaps or validation failures

---

### 3.3 `/v1/mvcc` — Replica Context

On replicas, `/v1/mvcc` MUST:

- Show replica CommitId
- Show active snapshots (if any)
- Explicitly state if reads are blocked

Replica MVCC is not special.
It must look identical to primary MVCC when read-safe.

---

## 4. Mapping to Explanation Endpoints

Replication explanations MUST use **Phase 4 explanation model**.

---

### 4.1 `/v1/explain/replication`

**Explanation Type:** `replication.safety`

This endpoint MUST explain:

- Why the replica is safe or unsafe for reads
- Which invariants are satisfied
- Which invariant (if any) is violated

Evidence MUST include:
- WAL prefix comparison
- CommitId comparison
- Replica global state

---

### 4.2 `/v1/explain/recovery` (Replica)

When invoked on a replica, this endpoint MUST:

- Explain replica recovery steps
- Include WAL replay range
- Include snapshot usage (if any)
- Include failure reasons if recovery halted

---

### 4.3 `/v1/explain/read` (Replica)

For replica reads, this endpoint MUST:

- Use identical MVCC rules
- Explicitly state why read is allowed or refused
- Reference replica state and CommitId

No replica-specific shortcuts are allowed.

---

## 5. Required Observability per Runtime Component

| Component | Observable State |
|---------|------------------|
| Replica Supervisor | Global state, blocking reason |
| WAL Receiver | Last received offset |
| WAL Validator | Last validated offset |
| WAL Applier | Last applied offset |
| Snapshot Bootstrap | State + cut CommitId |
| Read Safety Gate | Safety decision + proof |

Hidden state is forbidden.

---

## 6. Failure Visibility Requirements

For every failure in `OBSERVABILITY_FAILURE_MATRIX.md`:

- The failure MUST be visible via DX
- The failure MUST have an explanation
- The system MUST NOT auto-clear failure state

Operators and developers must see **why replication stopped**.

---

## 7. Determinism Requirements

Given identical persisted state:

- Observability output MUST be identical
- Explanation output MUST be identical
- Ordering of fields MUST be stable

No timestamps or timing-derived fields allowed.

---

## 8. Forbidden Observability Patterns

Explicitly forbidden:

- “Lag OK” indicators
- Progress bars without evidence
- Health summaries
- Auto-healing messages
- Hiding blocked states

If replication is unhealthy, it must look unhealthy.

---

## 9. Testing Requirements

Replication observability MUST be tested for:

- Correct state exposure
- Correct blocking reasons
- Correct explanation evidence
- Deterministic output across restarts

Observability tests are correctness tests.

---

## 10. Final Rule

> If replication state cannot be observed,
> replication correctness cannot be proven.

Observability is part of the replication contract.

---

END OF DOCUMENT
