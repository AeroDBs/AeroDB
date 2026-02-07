# AeroDB — Specification Index (Authoritative)

## Status

- Scope: **Entire AeroDB specification**
- Authority: **Index-only, non-semantic**
- Purpose: **Canonical reading order and authority map**

This document defines:
- Which documents exist
- What they govern
- In which order they MUST be read

This document does NOT define behavior.
It defines **how to understand behavior correctly**.

---

## 1. How to Use This Index

### Mandatory Rule

> You MUST read documents in the order specified here.

Skipping ahead, cherry-picking, or reading by filename order
WILL produce an incorrect understanding of AeroDB.

---

### Authority Rule

If two documents appear to conflict:
- The document that appears **earlier in this index** wins
- Later documents MUST defer

---

## 2. Tier 0 — Project Constitution (Absolute Authority)

These documents define AeroDB’s identity and non-negotiable rules.

Read **first**. Never reinterpret.

1. `CORE_VISION.md`  
   **Governs:** What AeroDB is and why it exists

2. `CORE_SCOPE.md`  
   **Governs:** What AeroDB explicitly does and does not do

3. `CORE_INVARIANTS.md`  
   **Governs:** Global correctness, durability, determinism rules

4. `CORE_RELIABILITY.md`  
   **Governs:** Failure philosophy and correctness under fault

---

## 3. Tier 1 — Core System (Phase 1, Frozen)

These documents define the **foundational database mechanics**.
They are fully implemented and frozen.

5. `CORE_BOOT.md`  
   **Governs:** Startup and initialization sequence

6. `CORE_LIFECYCLE.md`  
   **Governs:** Runtime lifecycle and state transitions

7. `CORE_WAL.md`  
   **Governs:** Write-ahead log semantics and durability

8. `CORE_STORAGE.md`  
   **Governs:** Append-only storage model

9. `CORE_SNAPSHOT.md`  
   **Governs:** Snapshot creation and immutability

10. `CORE_CHECKPOINT.md`  
    **Governs:** Checkpoint creation and WAL truncation

11. `CORE_BACKUP.md`  
    **Governs:** Backup semantics

12. `CORE_RESTORE.md`  
    **Governs:** Restore semantics

13. `CORE_SCHEMA.md`  
    **Governs:** Schema definition and validation

14. `CORE_QUERY.md`  
    **Governs:** Query parsing, planning, execution, bounds

15. `CORE_API_SPEC.md`  
    **Governs:** External client-facing API (non-admin)

16. `CORE_ERRORS.md`  
    **Governs:** Error taxonomy and guarantees

---

## 4. Tier 2 — MVCC (Phase 2A, Frozen)

These documents define **visibility, isolation, and versioning**.
Semantics are frozen and authoritative.

17. `MVCC_MODEL.md`  
    **Governs:** MVCC conceptual model

18. `MVCC_VISIBILITY_RULES.md`  
    **Governs:** Snapshot isolation and version visibility

19. `MVCC_WAL_INTEGRATION.md`  
    **Governs:** WAL ↔ MVCC interaction

20. `MVCC_SNAPSHOT_MODEL.md`  
    **Governs:** MVCC-aware snapshots

21. `MVCC_GC_MODEL.md`  
    **Governs:** Garbage collection rules

22. `MVCC_FAILURE_MATRIX.md`  
    **Governs:** Crash behavior under MVCC

23. `MVCC_COMPATIBILITY.md`  
    **Governs:** Compatibility with other phases

24. `MVCC_TESTING.md`  
    **Governs:** MVCC testing strategy

---

## 5. Tier 3 — Replication (Phase 2B, Semantics Frozen)

These documents define **multi-node correctness**.
Implementation may follow, semantics must not change.

25. `REPL_VISION.md`  
    **Governs:** Replication goals and model

26. `REPL_INVARIANTS.md`  
    **Governs:** Replication correctness rules

27. `REPL_PROOFS.md`  
    **Governs:** Correctness proofs

28. `REPL_MODEL.md`  
    **Governs:** Primary/replica architecture

29. `REPL_WAL_FLOW.md`  
    **Governs:** WAL shipping and prefix rules

30. `REPL_READ_RULES.md`  
    **Governs:** Replica read safety

31. `REPL_RECOVERY_MODEL.md`  
    **Governs:** Replica recovery semantics

32. `REPL_SNAPSHOT_TRANSFER.md`  
    **Governs:** Snapshot bootstrap

33. `REPL_FAILURE_MATRIX.md`  
    **Governs:** Replication failure handling

34. `REPL_COMPATIBILITY.md`  
    **Governs:** Interaction with MVCC and Phase 1

---

## 6. Tier 4 — Performance (Phase 3, Frozen Semantics)

These documents define **correctness-preserving optimizations**.

35. `PERF_VISION.md`  
    **Governs:** Phase 3 intent and limits

36. `PERF_INVARIANTS.md`  
    **Governs:** Performance safety rules

37. `PERF_PROOF_RULES.md`  
    **Governs:** How optimizations are proven

38. `PERF_BASELINE.md`  
    **Governs:** Baseline performance reference

39. `CRITICAL_PATHS.md`  
    **Governs:** Optimization-eligible execution paths

40. `PERF_SEMANTIC_EQUIVALENCE.md`  
    **Governs:** What “equivalent behavior” means

41. `PERF_FAILURE_MODEL.md`  
    **Governs:** Failure assumptions for optimizations

42. `PERF_DISABLEMENT.md`  
    **Governs:** Rollback and disablement rules

43. `PERF_OBSERVABILITY.md`  
    **Governs:** Performance metrics without behavior impact

---

### Phase 3 Optimization Specs (Read After Above)

44. `PERF_GROUP_COMMIT.md`  
45. `PERF_WAL_BATCHING.md`  
46. `PERF_READ_PATH.md`  
47. `PERF_INDEX_ACCELERATION.md`  
48. `PERF_CHECKPOINT_PIPELINING.md`  
49. `PERF_REPLICA_READ_FAST_PATH.md`  
50. `PERF_MEMORY_LAYOUT.md`  

---

## 7. Tier 5 — Developer Experience (Phase 4)

These documents make AeroDB **visible and explainable**.
They add no semantics.

51. `DX_VISION.md`  
    **Governs:** Phase 4 goals

52. `DX_INVARIANTS.md`  
    **Governs:** UI and observability safety

53. `DX_OBSERVABILITY_PRINCIPLES.md`  
    **Governs:** Passive observability philosophy

54. `DX_OBSERVABILITY_API.md`  
    **Governs:** Read-only admin APIs

55. `DX_EXPLANATION_MODEL.md`  
    **Governs:** Explanation structure and rules

56. `DX_ADMIN_UI_ARCH.md`  
    **Governs:** Admin UI architecture

---

## 8. Tier 6 — Meta, Tooling, and Process

These documents guide usage and contribution.
They have **no semantic authority**.

57. `PROJECT_PLAN.md`  
58. `INIT_READINESS.md`  
59. `REPLICATION_VISION.md`  
60. `REPLICATION_ROADMAP.md`  
61. `REPLICATION_READINESS.md`  
62. `INSTALL.md`  
63. `CONFIG.md`  
64. `CRASH_TESTING.md`  
65. `CLAUDE.md`  
66. `tree.md`

---

## 9. Final Authority Rule (Repeat)

> If you are unsure which document governs a behavior,
> the one that appears **earlier in this index** is authoritative.

This index is the **map**.
The documents are the **law**.

---

END OF DOCUMENT
