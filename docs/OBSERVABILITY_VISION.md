# PHASE 5 — REPLICATION IMPLEMENTATION

## Status

- Phase: **5**
- State: **Implementation**
- Authority: **Normative**
- Depends on:
  - Phase 0–4 (Frozen and Audited)
  - Replication semantics defined in Phase 2B
  - Observability & Explanation infrastructure from Phase 4

Phase 5 implements replication exactly as specified.
It introduces **no new replication semantics**.

---

## 1. Purpose of Phase 5

Phase 5 exists to answer one question:

> “Can AeroDB’s replication semantics, already proven on paper,  
> be implemented faithfully, observably, and deterministically?”

This phase turns **frozen replication specifications** into **running code**.

---

## 2. What Phase 5 Is (and Is Not)

### Phase 5 IS

- A faithful implementation of Phase 2B replication semantics
- Single-writer, primary/replica replication
- WAL-based log shipping
- Snapshot-based bootstrap
- Deterministic replica recovery
- Observable and explainable replication state

### Phase 5 IS NOT

- A redesign of replication
- Leader election
- Automatic failover
- Multi-primary writes
- Consensus protocols (Raft/Paxos)
- Availability-first behavior
- Performance tuning beyond correctness

If it changes replication meaning, it does not belong in Phase 5.

---

## 3. Replication Model (Frozen Reference)

Phase 5 implements the following **unchanged model**:

- One **Primary**
- One or more **Replicas**
- **CommitId authority** resides only on the Primary
- Replicas apply WAL as a **strict prefix**
- Replicas never acknowledge writes
- Replica reads are allowed **only when MVCC-safe**

All of this is already defined.
Phase 5 makes it real.

---

## 4. Phase 5 Core Principles

### P5-1: Semantics Are Frozen

Replication behavior is governed by:
- `REPL_VISION.md`
- `REPL_INVARIANTS.md`
- `REPL_PROOFS.md`

Phase 5 MUST NOT:
- Add rules
- Remove rules
- Infer rules

Implementation follows spec verbatim.

---

### P5-2: Determinism Over Availability

Replication prioritizes:
1. Correctness
2. Determinism
3. Explicit failure
4. Availability (last)

If replication cannot proceed safely:
- It must stop
- It must explain why

---

### P5-3: Explicit Failure Over Silent Degradation

Replication failures MUST be:
- Detected
- Surfaced
- Observable
- Explainable

There is no “best effort” replication.

---

## 5. Phase 5 Scope

### 5.1 In Scope

Phase 5 implements:

- WAL segment shipping from Primary to Replicas
- Replica WAL validation (checksums, ordering)
- Replica WAL application
- Snapshot bootstrap (MVCC cut + WAL resume)
- Replica restart and recovery
- Replica read-safety enforcement
- Replication observability
- Replication explanations (why a replica is / is not safe)

---

### 5.2 Explicitly Out of Scope

Phase 5 does NOT include:

- Automatic role changes
- Promotion of replicas
- Failover orchestration
- Split-brain handling beyond detection
- Network encryption
- Authentication / authorization
- Dynamic cluster membership

These belong to future phases.

---

## 6. Interaction with Earlier Phases

### Phase 1 (Core)
- WAL format is unchanged
- Storage format is unchanged
- Recovery logic is reused

### Phase 2A (MVCC)
- Snapshot isolation rules remain authoritative
- Replica reads use identical MVCC rules

### Phase 3 (Performance)
- Replication may benefit from existing optimizations
- No new optimizations are introduced

### Phase 4 (DX)
- Replication state is fully observable
- Replication behavior is explainable
- No replication control via DX

---

## 7. Phase 5 Invariants (High-Level)

Phase 5 MUST preserve:

- WAL prefix rule
- CommitId monotonicity
- Deterministic replay
- Snapshot consistency
- Replica derivability
- No acknowledged write loss
- No silent divergence

Violation of any invariant is a Phase 5 failure.

---

## 8. Phase 5 Success Criteria

Phase 5 is successful if:

- A replica can be bootstrapped from scratch
- A replica can crash and recover deterministically
- Replica reads are allowed only when safe
- Replication lag is observable
- Replication failures are explicit and explainable
- Primary behavior is unchanged

All of this must be provable via tests and explanations.

---

## 9. Phase 5 Failure Philosophy

Replication failure is **not exceptional**.
It is expected and must be handled explicitly.

If replication cannot guarantee correctness:
- The replica must refuse to serve reads
- The system must explain why

Availability is optional.
Correctness is not.

---

## 10. Deliverables of Phase 5

Phase 5 produces:

- Replication runtime implementation
- Replica bootstrap mechanism
- Replica recovery logic
- Replication observability endpoints
- Replication explanation artifacts
- Exhaustive replication tests

No UI or operational tooling is required in this phase.

---

## 11. Phase 5 Completion Rules

Phase 5 is complete when:

- All replication specs are implemented
- All replication invariants are enforced
- All failure modes are tested
- Replication explanations are accurate
- Phase 0–4 behavior remains unchanged

Only then may replication be considered “implemented”.

---

## 12. Guiding Statement

> Phase 5 does not make AeroDB highly available.  
> It makes AeroDB **correctly replicated**.

Correct replication is more valuable than fast replication.
Explainable replication is more valuable than opaque replication.

---

END OF DOCUMENT
