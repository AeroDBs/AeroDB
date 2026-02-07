# FAILOVER_SCOPE.md — Failover & Promotion

## Status
- Phase: **6**
- Authority: **Normative**
- Depends on: **FAILOVER_VISION.md**
- Frozen Dependencies: **Phases 0–5**

---

## 1. Purpose of This Document

This document defines the **exact scope boundaries** of Phase 6.

Its role is to make Phase 6:
- Explicit
- Finite
- Non-expanding
- Immune to scope creep

Anything not explicitly allowed here is **forbidden**.

---

## 2. In-Scope Capabilities (Allowed)

Phase 6 is limited to the following capabilities.

### 2.1 Explicit Replica Promotion

Phase 6 defines:
- When a replica **may be promoted**
- What conditions **must be satisfied**
- How promotion is **validated**
- How authority is **transferred**

Promotion is:
- Explicitly triggered
- Deterministically evaluated
- Either accepted or rejected atomically

---

### 2.2 Promotion Safety Validation

Before promotion, the system MUST be able to prove:

- Replica WAL position is sufficient
- No acknowledged writes would be lost
- No dual-primary condition can arise
- MVCC visibility rules remain intact
- Replication invariants from Phase 5 are preserved

If proof fails, promotion MUST be rejected.

---

### 2.3 Failover State Modeling

Phase 6 introduces:
- A formal failover-related state model
- Explicit state transitions
- Explicit forbidden transitions

No implicit or inferred transitions are allowed.

---

### 2.4 Authority Rebinding

Phase 6 defines:
- How write authority is transferred
- When the old primary is considered invalid
- How the new primary assumes authority

Authority rebinding is:
- Single-writer
- Non-overlapping
- Explicitly observable

---

### 2.5 Observability & Explanation (Additive Only)

Phase 6 may add:
- New observability events
- New explanation surfaces

But:
- Must reuse existing observability infrastructure
- Must remain passive
- Must not influence control flow

---

## 3. Explicit Non-Goals (Forbidden)

Phase 6 MUST NOT introduce any of the following.

### 3.1 Automatic Failover

Forbidden:
- Automatic leader election
- Background promotion
- Health-check-driven failover
- Time-based decisions

All promotion is explicit.

---

### 3.2 Consensus Protocols

Phase 6 MUST NOT:
- Introduce Raft, Paxos, Zab, etc.
- Add quorum voting
- Add majority-based authority

Single-writer authority remains absolute.

---

### 3.3 Multi-Primary or Split-Brain Handling

Phase 6 does NOT:
- Allow dual primaries
- Tolerate split brain
- Attempt conflict resolution

If safety cannot be proven, the system must halt or reject.

---

### 3.4 Replication Redesign

Phase 6 MUST NOT:
- Change WAL semantics
- Change replication protocol
- Change snapshot behavior
- Change recovery logic

Replication from Phase 5 is frozen.

---

### 3.5 Admin UI or Operator Tooling

Phase 6 does NOT include:
- Admin dashboards
- Web UI
- CLI convenience tooling
- Operator workflows

These belong to a later phase.

---

## 4. Scope Boundaries with Frozen Phases

### 4.1 Phase 5 Boundary

Phase 6 may:
- Read Phase 5 replication state
- Validate Phase 5 invariants
- React to Phase 5 states

Phase 6 may NOT:
- Modify Phase 5 state machines
- Add hidden transitions
- Alter replication correctness rules

---

### 4.2 Phase 0–4 Boundary

Phase 6 MUST NOT:
- Affect WAL durability semantics
- Affect MVCC visibility
- Affect recovery determinism
- Affect observability guarantees

---

## 5. Failure Handling Scope

Phase 6 is responsible for:
- Defining promotion failure cases
- Making failures explicit
- Explaining why promotion failed

Phase 6 is NOT responsible for:
- Repairing failures
- Masking failures
- Retrying promotion automatically

---

## 6. Completeness Criteria

Phase 6 scope is considered complete when:

- All allowed behaviors are specified
- All forbidden behaviors are explicitly excluded
- No ambiguity exists about system behavior
- No frozen phase semantics are touched

---

## 7. Scope Lock Rule

Once **FAILOVER_SCOPE.md** is approved:

> Any request not covered by this document  
> MUST be deferred to a later phase.

---

END OF DOCUMENT
