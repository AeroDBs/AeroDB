# ROLLBACK AND DISABLEMENT — PHASE 3

## Status

- Phase: **3**
- Authority: **Normative**
- Scope: **All Phase 3 optimizations**
- Dependencies:
  - PERF_VISION.md
  - PERF_INVARIANTS.md
  - PERF_PROOF_RULES.md
  - SEMANTIC_EQUIVALENCE.md
  - FAILURE_MODEL_PHASE3.md

This document defines the **mandatory rollback, disablement, and compatibility rules**
for all Phase 3 performance optimizations.

If an optimization cannot be disabled safely, it MUST NOT exist.

---

## 1. Purpose

Performance optimizations must never trap AeroDB in a state that:

- Requires data migration
- Requires cleanup tooling
- Requires replay special-casing
- Requires operator intervention

This document ensures that:
- Optimizations are *optional*
- Data formats remain *stable*
- Recovery remains *unconditional*

---

## 2. Definition of Disablement

Disablement means:

> The optimization is turned off, and AeroDB behaves exactly like the Phase 2 baseline.

Disablement MUST be possible:
- Without rewriting WAL
- Without rewriting snapshots
- Without rewriting checkpoints
- Without rewriting indexes

Disablement is not “best effort”.
It is a **hard requirement**.

---

## 3. Forms of Disablement

Phase 3 allows **two and only two** forms of disablement.

### 3.1 Compile-Time Disablement

- Optimization is conditionally compiled
- Code is entirely removed from the binary
- Baseline behavior is compiled instead

Requirements:
- Persistent formats MUST remain compatible
- WAL replay MUST not branch on compile flags
- No conditional decoding paths

Compile-time disablement is preferred where feasible.

---

### 3.2 Startup-Time Disablement

- Optimization is toggled via configuration
- Decision is made before database opens
- Decision is immutable for the process lifetime

Requirements:
- No runtime toggling
- No mid-flight switching
- No partial activation

Runtime enable/disable is forbidden.

---

## 4. Forbidden Disablement Models

The following are **explicitly forbidden**:

- Disabling after startup
- Disabling based on load or timing
- Automatic fallback
- Progressive degradation
- Partial disablement
- Per-request switching

If an optimization needs “fallback logic”, it is invalid.

---

## 5. Data Compatibility Invariants

### 5.1 WAL Compatibility

- WAL format MUST NOT change
- WAL records written with optimization enabled:
  - MUST be replayable without optimization
  - MUST preserve logical meaning

Optimizations MAY:
- Change how WAL is *produced*
Optimizations MUST NOT:
- Change what WAL *means*

---

### 5.2 Snapshot Compatibility

- Snapshot file formats MUST remain unchanged
- Snapshot manifests MUST remain valid
- Snapshot readers MUST be identical

Optimizations MUST NOT:
- Add hidden snapshot metadata
- Encode optimization state into snapshots

---

### 5.3 Checkpoint Compatibility

- Checkpoint markers MUST be unchanged
- WAL truncation logic MUST be identical
- Recovery selection logic MUST be unchanged

---

## 6. No Persistent Optimization State

### 6.1 Ghost State Prohibition

Optimizations MUST NOT introduce:

- Hidden persistent files
- Undocumented metadata
- Implicit flags in WAL
- Implicit flags in snapshots
- Optimization-only markers

All persistent state MUST be:
- Explicit
- Documented
- Semantically neutral

---

### 6.2 In-Memory State Rules

Optimizations MAY introduce in-memory state ONLY if:

- It is reconstructible
- It is non-authoritative
- Its loss does not affect correctness

In-memory state MUST be discardable at any time.

---

## 7. Recovery Behavior Under Disablement

Recovery MUST behave as follows:

- Recovery logic MUST NOT branch on optimization state
- WAL replay MUST be identical
- Snapshot loading MUST be identical
- Index rebuild MUST be identical

If recovery behavior differs, the optimization is invalid.

---

## 8. Rolling Back After Failure

If a crash occurs:

- During optimized execution
- With optimization enabled

Then after restart:

- Optimization may be disabled
- Recovery MUST succeed
- Final state MUST be correct

Optimizations MUST NOT:
- Require post-crash cleanup
- Leave ambiguous persistent state

---

## 9. Disablement Proof Obligations

Every Phase 3 optimization specification MUST include:

1. Disablement mechanism (compile-time or startup-time)
2. Proof of WAL compatibility
3. Proof of snapshot compatibility
4. Proof of recovery equivalence
5. Proof of no ghost state

Missing any proof invalidates the optimization.

---

## 10. Operational Guarantees

From an operator’s perspective:

- Disabling an optimization is safe
- No data loss occurs
- No special procedures are required
- No downgrade path is needed

If an operator needs instructions, the optimization is invalid.

---

## 11. Interaction with Replication

Disablement MUST preserve:

- WAL prefix rule
- CommitId ordering
- Replica derivability

Primary and replica MUST NOT:
- Require matching optimization settings
- Coordinate optimization state

Replication MUST remain agnostic.

---

## 12. Testing Requirements

Disablement MUST be tested via:

- Enable → write → crash → disable → recover
- Enable → write → disable → restart
- Mixed WAL segments across enable/disable boundaries

All tests MUST pass unmodified baseline recovery logic.

---

## 13. What Rollback Is NOT

Rollback does NOT mean:

- Undoing committed data
- Reverting WAL
- Removing snapshots
- Replaying differently

Rollback means **ceasing to use the optimization**, not reversing history.

---

## 14. Final Rule

> If an optimization cannot be safely disabled,
> it is not an optimization — it is a liability.

Phase 3 only allows optimizations that can be abandoned
without consequence.

---

END OF DOCUMENT
