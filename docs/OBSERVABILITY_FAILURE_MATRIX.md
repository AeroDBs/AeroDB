# PHASE 5 — FAILURE MATRIX (REPLICATION IMPLEMENTATION)

## Status

- Phase: **5**
- Authority: **Normative**
- Scope: **All replication failure scenarios**
- Depends on:
  - `OBSERVABILITY_VISION.md`
  - `OBSERVABILITY_INVARIANTS.md`
  - `OBSERVABILITY_IMPLEMENTATION_ORDER.md`
  - `REPLICATION_RUNTIME_ARCHITECTURE.md`
  - `REPL_*` specifications
  - `MVCC_*` specifications
  - `CORE_INVARIANTS.md`
  - `FAILURE_MODEL_PHASE3.md`

This document defines **exact expected outcomes** for failures during
replication implementation and operation.

If runtime behavior under failure does not match this matrix,
the implementation is incorrect.

---

## 1. Purpose

Replication failures are **expected**, not exceptional.

This matrix:
- Enumerates all relevant failure points
- Specifies required outcomes
- Forbids silent progress
- Preserves determinism and explainability

This document is **exhaustive for Phase 5**.

---

## 2. Failure Handling Principles

### F-1: Fail Closed

If replication correctness cannot be proven:
- Replication MUST stop
- Reads MUST be refused (if unsafe)
- Failure MUST be surfaced

---

### F-2: No Partial Visibility

At no point may:
- Partially applied WAL be visible
- Unvalidated WAL affect state
- Mixed snapshot/WAL state leak

---

### F-3: Deterministic Recovery

Given identical persisted state:
- Recovery behavior MUST be identical
- Outcomes MUST be reproducible

---

## 3. Failure Axes

Failures are classified along four axes:

1. **Location** — which component failed
2. **Time** — when failure occurred
3. **Persistence** — what was durably written
4. **Visibility** — what may be exposed

All axes must be considered.

---

## 4. Primary-Side Failures

### F-P1: Primary Crash Before WAL fsync

**Condition**
- Primary crashes after assigning CommitId
- WAL record NOT durably fsynced

**Required Outcome**
- Write is lost (never acknowledged)
- WAL record does not appear
- Replica never receives the record

**Explainability**
- Recovery explanation references missing WAL durability

---

### F-P2: Primary Crash After WAL fsync

**Condition**
- WAL record durably written
- Crash before replication shipment

**Required Outcome**
- WAL is replayed on primary recovery
- Replicas may receive WAL later
- CommitId continuity preserved

**Explainability**
- Recovery explanation shows WAL replay range

---

## 5. Replica WAL Receiver Failures

### F-R1: Receiver Crash Before WAL Persist

**Condition**
- WAL segment partially received
- No durable write

**Required Outcome**
- WAL segment discarded
- No validation attempted
- No state mutation

**Post-Recovery**
- Receiver resumes from last durable offset

---

### F-R2: Receiver Crash After WAL Persist

**Condition**
- WAL bytes durably stored
- Validation not completed

**Required Outcome**
- WAL segment available for validation
- No application performed

**Explainability**
- DX shows received-but-unvalidated state

---

## 6. WAL Validation Failures

### F-V1: Checksum Validation Failure

**Condition**
- WAL checksum mismatch

**Required Outcome**
- WAL segment rejected
- Replication blocked
- Replica refuses reads

**Explainability**
- Explanation references checksum rule violation

---

### F-V2: Continuity Failure (Gap or Overlap)

**Condition**
- WAL segment does not continue prefix

**Required Outcome**
- WAL rejected
- Replica state unchanged
- Explicit failure surfaced

---

## 7. WAL Application Failures

### F-A1: Crash During WAL Application

**Condition**
- WAL partially applied to storage
- Crash occurs mid-application

**Required Outcome**
- On recovery:
  - Partial application rolled back or ignored
  - WAL replay resumes deterministically
- No partial visibility

**Explainability**
- Recovery explanation shows replay restart point

---

### F-A2: Crash After WAL Apply Before Metadata Persist

**Condition**
- Storage mutated
- Replica metadata not yet updated

**Required Outcome**
- Recovery reconciles storage and metadata
- WAL re-applied idempotently or safely ignored

---

## 8. Snapshot Bootstrap Failures

### F-S1: Crash During Snapshot Transfer

**Condition**
- Snapshot partially transferred

**Required Outcome**
- Snapshot discarded
- Replica remains uninitialized
- No WAL applied

---

### F-S2: Crash After Snapshot Transfer Before Validation

**Condition**
- Snapshot data present
- Validation incomplete

**Required Outcome**
- Snapshot invalidated
- Bootstrap restarts from scratch

---

### F-S3: Snapshot/WAL Boundary Mismatch

**Condition**
- WAL resume offset does not match snapshot cut

**Required Outcome**
- Hard failure
- Replica refuses to proceed

**Explainability**
- Explanation references snapshot/WAL continuity violation

---

## 9. Replica Recovery Failures

### F-RC1: Crash During Replica Recovery

**Condition**
- Replica crashes during its own recovery

**Required Outcome**
- Recovery restarts from last durable state
- Outcome identical to single crash

---

### F-RC2: Corrupt Replica Metadata

**Condition**
- Replica metadata checksum failure

**Required Outcome**
- Replica refuses to start
- Requires operator intervention

**Explainability**
- Explicit corruption explanation

---

## 10. Read-Safety Failures

### F-RS1: Read Requested While Unsafe

**Condition**
- Replica not read-safe
- Read requested

**Required Outcome**
- Read refused
- Explicit reason returned

---

### F-RS2: State Change During Read Evaluation

**Condition**
- Replica state changes mid-read

**Required Outcome**
- Read re-evaluated or refused
- No mixed-state read allowed

---

## 11. Observability Failures

### F-O1: DX Endpoint During Failure

**Condition**
- DX endpoint queried during replication failure

**Required Outcome**
- Partial state allowed
- Must be labeled incomplete
- No inference or guessing

---

## 12. Forbidden Failure Responses

Explicitly forbidden:

- Silent retry loops
- Background “eventual catch-up”
- Serving stale reads without proof
- Auto-resetting failure state
- Masking checksum or continuity errors

If any appear, the implementation is invalid.

---

## 13. Testing Requirements (Mapping)

Each failure scenario MUST have:
- At least one deterministic test
- Crash-injection coverage where applicable
- Explanation validation

No failure is “too rare to test”.

---

## 14. Final Rule

> Replication failures must be louder than replication success.

If failure behavior is unclear,
correctness is already lost.

---

END OF DOCUMENT
