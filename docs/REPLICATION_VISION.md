# REPLICATION_VISION.md

## AeroDB Phase 2 — Vision

### Status

* Phase-1 is **complete, correct, tested, and frozen**
* Phase-2 builds **on top of** Phase-1 without modifying or weakening it
* This document defines **intent and boundaries only**
* **No implementation details appear here**

---

## Purpose of Phase 2

Phase 2 extends AeroDB from a *single-version, single-timeline* database into a system capable of:

* **Concurrent readers and writers**
* **Stable, repeatable read views**
* **Deterministic historical visibility**
* **Strictly defined isolation guarantees**

This is achieved through **Multi-Version Concurrency Control (MVCC)**, designed to coexist with AeroDB’s Phase-1 durability, recovery, and determinism guarantees.

MVCC in AeroDB exists **to make correctness scalable**, not to chase throughput at the expense of safety.

---

## Non-Negotiable Continuity from Phase 1

All Phase-1 guarantees remain **fully intact and unchanged**.

### The following are explicitly preserved:

* **Durability**

  * No acknowledged write is ever lost
  * fsync semantics remain authoritative
* **Deterministic Recovery**

  * WAL replay produces identical state
  * MVCC metadata must replay deterministically
* **Corruption Detection**

  * Checksums remain mandatory
  * No silent repair, no best-effort recovery
* **Bounded Queries**

  * MVCC does not introduce unbounded scans
* **Crash Safety**

  * Every MVCC state transition must be crash-recoverable
* **Observability Neutrality**

  * Metrics and logs never influence MVCC behavior

> MVCC is an *extension*, never a reinterpretation, of Phase-1 correctness.

---

## What MVCC Means in AeroDB

In AeroDB, MVCC is:

* **Explicit** — visibility rules are defined in specs, not inferred
* **Deterministic** — no timing-based or scheduler-dependent behavior
* **Versioned** — documents have well-defined historical versions
* **Auditable** — visibility decisions can be reasoned about post-hoc
* **Recoverable** — all MVCC state is WAL-governed

MVCC is **not** a performance hack or a concurrency band-aid.
It is a correctness model for time, visibility, and isolation.

---

## What MVCC Is *Not*

AeroDB MVCC will **not**:

* Introduce “eventual” visibility without formal definition
* Depend on wall-clock time for correctness
* Allow ambiguous isolation levels
* Hide conflicts or auto-resolve inconsistencies
* Bypass WAL, snapshot, or checkpoint guarantees
* Assume optimistic success paths

If a behavior cannot be precisely specified, it does not exist.

---

## High-Level MVCC Goals

Phase-2 MVCC must provide:

1. **Stable Read Views**

   * Readers observe a consistent snapshot
   * Reads are not affected by concurrent writes

2. **Well-Defined Write Semantics**

   * Writers create new versions, never mutate history
   * Conflicts are explicit and deterministic

3. **Snapshot Visibility**

   * MVCC integrates with existing snapshot and checkpoint mechanisms
   * Snapshot semantics remain predictable and fsync-safe

4. **Deterministic Garbage Collection**

   * Old versions are reclaimed only when provably unreachable
   * GC is correctness-driven, not heuristic-driven

5. **Replication-Ready Semantics**

   * MVCC state can be replicated deterministically in later Phase-2 work
   * No local-only shortcuts

---

## Explicit Deferrals

This document intentionally does **not** define:

* Isolation levels (defined later)
* Version storage layout
* Transaction APIs
* Locking or lock-free strategies
* Performance optimizations
* Garbage collection algorithms

Those belong in subsequent Phase-2 documents, governed by this vision.

---

## Phase-2 Design Ethos

> **Correctness first.
> Determinism second.
> Explicitness always.
> Performance only when proven safe.**

MVCC in AeroDB is not about being fast first —
it is about being **right forever**.
