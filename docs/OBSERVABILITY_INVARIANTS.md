# PHASE 5 — INVARIANTS (REPLICATION IMPLEMENTATION)

## Status

- Phase: **5**
- Authority: **Normative**
- Scope: **All replication implementation code**
- Depends on:
  - `OBSERVABILITY_VISION.md`
  - `REPL_INVARIANTS.md`
  - `REPL_PROOFS.md`
  - `MVCC_*` documents
  - `CORE_INVARIANTS.md`
  - `FAILURE_MODEL_PHASE3.md`

This document defines the **implementation-time invariants** that MUST hold
at **all times** during Phase 5.

These invariants are **not new semantics**.
They are **enforcement rules** to ensure the implementation does not violate
already-frozen replication semantics.

---

## 1. Purpose

Replication implementations fail most often due to:

- “Temporary” invariant violations
- Incomplete ordering guarantees
- Hidden state during concurrency
- Assumptions about network or timing

This document exists to make such failures **illegal**.

If an invariant cannot be maintained continuously,
the implementation approach is invalid.

---

## 2. Phase 5 Invariant Hierarchy

Phase 5 invariants are **subordinate** to:

1. `CORE_INVARIANTS.md`
2. `MVCC_*` invariants
3. `REPL_INVARIANTS.md`

If any Phase 5 invariant conflicts with the above,
**Phase 5 loses**.

---

## 3. Global Phase 5 Invariants

### P5-I1: No Semantic Redefinition

Phase 5 code MUST NOT:

- Change replication meaning
- Add new replication states
- Infer behavior not present in specs
- Introduce “temporary” semantics

Replication behavior is defined elsewhere.
Phase 5 implements it exactly.

---

### P5-I2: Continuous Invariant Enforcement

All replication invariants MUST hold:

- During steady state
- During startup
- During shutdown
- During crash windows
- During partial failure
- During recovery

There is no “initialization exception”.

---

### P5-I3: Explicit State Machines Only

All replication logic MUST be implemented as:

- Explicit state machines
- With enumerated states
- With explicit transitions
- With documented invariants per state

Implicit states are forbidden.

---

## 4. WAL-Related Invariants

### P5-I4: WAL Prefix Rule (Continuous)

At all times:

> The replica WAL MUST be a strict prefix of the primary WAL.

This invariant MUST hold:

- During WAL reception
- During WAL validation
- During WAL application
- During replica recovery

Partial or speculative WAL application is forbidden.

---

### P5-I5: WAL Validation Before Application

A replica MUST NOT:

- Apply WAL records
- Expose WAL-derived state
- Advance replica CommitId

Until:

- WAL checksum is validated
- WAL ordering is verified
- WAL continuity is confirmed

Validation precedes application, always.

---

## 5. CommitId & MVCC Invariants

### P5-I6: CommitId Authority Isolation

Only the Primary may:

- Assign CommitIds
- Advance global CommitId

Replicas MUST:

- Treat CommitIds as immutable facts
- Never infer missing CommitIds
- Never synthesize CommitIds

---

### P5-I7: MVCC Rules Are Identical on Replicas

Replica reads MUST obey:

- Identical MVCC visibility rules
- Identical snapshot semantics

No replica-specific MVCC shortcuts are allowed.

---

## 6. Snapshot & Bootstrap Invariants

### P5-I8: Snapshot Bootstrap Is a Cut

Snapshot bootstrap MUST:

- Represent a precise MVCC cut
- Be durably complete before use
- Be validated before WAL resume

Replica MUST NOT serve reads during bootstrap.

---

### P5-I9: Snapshot + WAL Continuity

After bootstrap:

- WAL resume MUST start exactly after snapshot cut
- No WAL gaps are allowed
- No WAL replay overlap is allowed

Any mismatch is a hard failure.

---

## 7. Failure Handling Invariants

### P5-I10: Explicit Failure Over Progress

If replication cannot safely proceed:

- It MUST stop
- It MUST surface the failure
- It MUST refuse unsafe reads

Continuing “as best as possible” is forbidden.

---

### P5-I11: Crash Safety

After any crash:

- Replica state MUST be derivable
- WAL prefix invariant MUST be restored
- No partial application may remain visible

Recovery MUST be deterministic.

---

## 8. Read-Safety Invariants

### P5-I12: Read Safety Is Proven, Not Assumed

A replica may serve reads ONLY IF:

- WAL prefix condition holds
- Snapshot CommitId ≤ replica CommitId
- No in-flight application exists

If safety cannot be proven, reads are forbidden.

---

## 9. Concurrency Invariants

### P5-I13: No Concurrent Authority

Replication code MUST NOT:

- Mutate shared authoritative state concurrently
- Rely on timing to serialize actions
- Use locks as semantic boundaries without state checks

Concurrency MUST be controlled via explicit states.

---

## 10. Observability & Explanation Invariants

### P5-I14: Replication State Is Observable

All replication states MUST be:

- Observable via DX APIs
- Explainable via explanation engine

Hidden replication state is forbidden.

---

### P5-I15: Explanations Reflect Reality

Replication explanations MUST:

- Reference real WAL offsets
- Reference real CommitIds
- Reference real state machine states

No synthetic explanations allowed.

---

## 11. Disablement Invariants

### P5-I16: Replication Is Removable

Replication MUST be:

- Disableable at startup
- Removable at compile time

Disabling replication MUST:

- Not affect primary behavior
- Not affect durability
- Not affect MVCC semantics

---

## 12. Testing Invariants

### P5-I17: Every Invariant Is Testable

For each Phase 5 invariant:

- At least one test MUST exist
- Crash tests MUST cover it
- Recovery tests MUST validate it

If an invariant cannot be tested,
the implementation is incomplete.

---

## 13. Forbidden Implementation Patterns

The following are explicitly forbidden:

- Heuristic retries
- Time-based assumptions
- “Eventually consistent” behavior
- Background best-effort cleanup
- Silent error handling
- Partial progress masking failures

If code requires these, the design is wrong.

---

## 14. Final Rule

> Replication correctness is not about making progress.  
> It is about **never being wrong**.

If replication cannot proceed safely,
it must stop and explain why.

---

END OF DOCUMENT
