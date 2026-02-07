## Purpose

This document defines the absolute, non-negotiable invariants of aerodb.

An invariant is a property that must hold true at all times:
- During normal operation
- Under load
- During crashes
- During recovery
- Across upgrades
- Across refactors

If any invariant is violated, it is a critical bug.
There are no exceptions, workarounds, or “temporary” violations.

aerodb is infrastructure software.
Infrastructure without invariants is unreliable by definition.

---

## Invariant Categories

The invariants are grouped into the following categories:

1. Data Safety Invariants  
2. Durability & Recovery Invariants  
3. Schema Invariants  
4. Query & Execution Invariants  
5. Determinism Invariants  
6. Failure Invariants  
7. Operational Invariants  
8. Evolution & Upgrade Invariants  

Each invariant is mandatory.

---

## 1. Data Safety Invariants

### D1. No Acknowledged Write Is Ever Lost
- Once a write operation is acknowledged as successful, it must survive:
  - Process crashes
  - Power loss
  - System restarts
- Violating this invariant is a catastrophic failure.

---

### D2. Data Corruption Is Never Ignored
- Any detected corruption must:
  - Halt affected operations
  - Be surfaced explicitly
  - Never be silently repaired or skipped
- Silent corruption is worse than downtime.

---

### D3. Reads Never Observe Invalid State
- A read must never return:
  - Partially written data
  - Corrupted records
  - Schema-invalid data
- If consistency cannot be guaranteed, the read must fail.

---

## 2. Durability & Recovery Invariants

### R1. WAL Precedes Acknowledgement
- All acknowledged writes must be recorded in the Write-Ahead Log
  before the client receives success.
- There are no exceptions.

---

### R2. Recovery Is Deterministic
- Given the same on-disk state and WAL,
  recovery must always produce the same result.
- Recovery must not depend on timing, heuristics, or environment.

---

### R3. Recovery Completeness Is Verifiable
- After recovery, aerodb must be able to assert:
  - Whether recovery completed successfully
  - Whether any data loss occurred
- Ambiguous recovery states are forbidden.

---

## 3. Schema Invariants

### S1. Schema Presence Is Mandatory
- Every document belongs to a schema.
- Writes without a schema are forbidden.
- Partial schema validation is forbidden.

---

### S2. Schema Validity Is Enforced on Write
- No write may bypass schema validation.
- Invalid data must never enter persistent storage.

---

### S3. Schema Versions Are Explicit
- Schema changes create new schema versions.
- Schema versions are immutable once published.
- Documents must always reference an explicit schema version.

---

### S4. Schema Violations Are Fatal to the Write
- If data violates schema rules:
  - The write fails
  - No partial data is persisted
- There is no “best effort” mode.

---

## 4. Query & Execution Invariants

### Q1. Queries Must Be Bounded
- Every query must have a provable upper bound on:
  - Data scanned
  - Time complexity
  - Memory usage
- Queries without a bounded cost are rejected before execution.

---

### Q2. No Implicit Full Scans
- Full collection scans are forbidden unless explicitly declared
  and bounded.
- Hidden or accidental scans are a critical violation.

---

### Q3. Execution Never Guesses
- The engine must not guess user intent.
- If intent is ambiguous, the query is rejected.

---

## 5. Determinism Invariants

### T1. Deterministic Planning
- Given the same:
  - Query
  - Schema
  - Indexes
  - Data statistics
- The query planner must produce the same plan.

---

### T2. Deterministic Execution
- Query execution must not depend on:
  - Runtime timing
  - Thread scheduling
  - Non-deterministic iteration order
- Results must be stable and repeatable.

---

### T3. Planner Changes Are Explicit
- Any change in planner behavior must:
  - Be versioned
  - Be opt-in
  - Be observable
- Silent planner behavior changes are forbidden.

---

## 6. Failure Invariants

### F1. Fail Loudly, Not Silently
- Any failure must:
  - Be explicit
  - Be observable
  - Be explainable
- Silent degradation is forbidden.

---

### F2. Partial Success Is Not Allowed
- Operations must be:
  - Fully successful, or
  - Fully failed
- Mixed or partial outcomes are forbidden.

---

### F3. Errors Are Deterministic
- The same failure condition must produce the same error code
  and classification.
- Random or context-dependent errors are forbidden.

---

## 7. Operational Invariants

### O1. Behavior Is Environment-Independent
- aerodb must behave identically across:
  - Local development
  - Self-hosted production
  - CI environments
- Environment-specific behavior is forbidden.

---

### O2. Configuration Cannot Violate Safety
- Configuration options must not allow:
  - Disabling durability
  - Weakening validation
  - Bypassing invariants
- Unsafe configuration states must be rejected.

---

### O3. Observability Is Mandatory
- The system must always be able to explain:
  - What it is doing
  - Why it failed
  - What guarantees are currently upheld
- Opaque operation is forbidden.

---

## 8. Evolution & Upgrade Invariants

### E1. Backward Compatibility Is Explicit
- Backward compatibility guarantees must be documented.
- Breaking changes must:
  - Be deliberate
  - Be visible
  - Be justified
- Accidental breaking changes are unacceptable.

---

### E2. Upgrades Must Not Corrupt Data
- Upgrading aerodb must never:
  - Corrupt data
  - Silently rewrite data
  - Change semantics without opt-in

---

### E3. Downgrade Safety Must Be Defined
- If downgrades are unsupported, this must be explicit.
- Undefined downgrade behavior is forbidden.

---

## Enforcement Rules

- All invariants must be enforceable by code.
- Invariants must be testable.
- Tests that validate invariants are higher priority than features.
- Performance optimizations must not weaken invariants.

If an optimization conflicts with an invariant,
the optimization is rejected.

---

## Final Statement

These invariants are not aspirational.
They are mandatory.

aerodb is trusted only insofar as these invariants hold.
If they are violated, trust is lost.

There are no exceptions.
