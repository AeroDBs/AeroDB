# FAILOVER_VISION.md — Failover & Promotion

## Status
- Phase: **6**
- Authority: **Normative**
- Depends on: **Phases 0–5 (Frozen)**
- Scope: **Failover & Promotion semantics only**

---

## 1. Purpose

Phase 6 introduces **explicit, correctness-preserving failover and promotion**
for AeroDB’s replication system.

After Phase 5, AeroDB supports:
- Deterministic primary–replica replication
- Single-writer authority
- Replica read safety
- Crash-safe recovery
- Full observability and explanation

What is missing is a **defined, safe transition of write authority**
when a primary becomes unavailable.

Phase 6 exists to answer one question, and one question only:

> *When and how may a replica become the primary, without violating correctness?*

---

## 2. Design Philosophy

Phase 6 follows AeroDB’s core philosophy:

1. **Correctness over availability**
2. **Determinism over automation**
3. **Explicit authority over heuristics**
4. **Explainability over convenience**

Failover in Phase 6 is:
- **Explicit**, not heuristic
- **Safe**, not fast
- **Auditable**, not magical
- **Deterministic**, not adaptive

---

## 3. What Phase 6 Introduces

Phase 6 introduces:

- A **formal promotion model** for replicas
- Explicit **authority transfer rules**
- A well-defined **failover state machine**
- Clear **safety checks** before promotion
- Observable and explainable failover decisions

Phase 6 does **not** introduce:
- Automatic leader election
- Consensus protocols (Raft, Paxos, etc.)
- Split-brain tolerance
- Background retries
- Hidden recovery behavior

---

## 4. Explicit Non-Goals

Phase 6 explicitly does **not** aim to:

- Maximize availability at all costs
- Mask failures from operators
- Automatically “heal” the system
- Optimize for cloud orchestration platforms
- Introduce multi-writer semantics
- Redefine replication from Phase 5

If a system cannot prove safety, it must refuse promotion.

---

## 5. Relationship to Frozen Phases

Phase 6:

- **Does not modify** Phase 0–5 invariants
- **Does not reinterpret** replication semantics
- **Does not change** WAL, MVCC, or recovery rules
- **Does not weaken** failure guarantees

All Phase 0–5 behavior remains authoritative and frozen.

Phase 6 is strictly **additive**.

---

## 6. Operational Model (High-Level)

At a conceptual level, Phase 6 enables:

- A replica to be **considered** for promotion
- A promotion to be **validated** against safety rules
- A promotion to either:
  - Succeed explicitly, or
  - Fail explicitly with explanation

There is no partial success.

---

## 7. Observability & Explanation

Every failover-related decision must be:

- Observable via existing observability infrastructure
- Explainable via the explanation engine
- Traceable to explicit invariants and rules

AeroDB must always be able to answer:

> *Why was promotion allowed or denied?*

---

## 8. Success Criteria

Phase 6 is complete when:

- Promotion rules are fully specified
- Failure cases are exhaustively defined
- No ambiguity exists in authority transitions
- All behavior is deterministic and testable
- All new logic is observable and explainable
- All Phase 0–5 tests remain unchanged and passing

---

## 9. Exit Condition

Once Phase 6 is audited and frozen:

> **AeroDB is launch-ready as a correctness-first replicated database.**

Admin UI, operator tooling, and security enhancements are explicitly deferred
to later phases.

---

END OF DOCUMENT
