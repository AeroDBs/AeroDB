# MVCC_GC.md

## AeroDB — MVCC Garbage Collection

### Status

* This document is **authoritative**
* It defines **when and why versions may be reclaimed**
* Garbage collection (GC) is correctness-driven, not performance-driven
* No implementation details appear here

---

## 1. Purpose of MVCC Garbage Collection

MVCC garbage collection exists to:

* Reclaim storage occupied by **provably unreachable versions**
* Preserve long-term system health
* Do so **without ever weakening correctness**

GC is optional for progress, but **mandatory for correctness** in the sense that:

> A version must never be removed unless it is proven invisible to all possible reads.

---

## 2. Version Lifecycle States

Every version exists in exactly one of the following logical states.

### 2.1 Live

A version is **Live** if:

* It is visible to at least one possible read view
* Or it may become visible in the future

Live versions are untouchable by GC.

---

### 2.2 Obsolete

A version is **Obsolete** if:

* It has been superseded by a newer committed version
* But may still be visible to some read views

Obsolete does **not** mean reclaimable.

---

### 2.3 Reclaimable

A version is **Reclaimable** if and only if:

* It is not visible to **any** possible read view
* It is not required by:

  * Any snapshot
  * Any checkpoint
  * Recovery semantics

Reclaimable is a proof-based state, not a heuristic.

---

### 2.4 Collected

A version is **Collected** once:

* Its removal has been fully recorded
* Recovery will not resurrect it
* No reference to it remains

Collected versions no longer exist.

---

## 3. Visibility Lower Bound

### 3.1 Global Visibility Floor

At any time, the system has a **visibility lower bound** defined by:

* The oldest active read view
* The oldest retained snapshot boundary

No version with:

```
commit_id ≥ visibility_lower_bound
```

may be reclaimed.

---

### 3.2 Determinism Requirement

The visibility lower bound must be:

* Explicit
* Deterministic
* Recoverable after crash

It must not be inferred from timing or runtime observation.

---

## 4. GC Eligibility Rule (Formal)

A version `V` with commit identity `C` is reclaimable **if and only if**:

1. `C < visibility_lower_bound`
2. A newer version exists in the same version chain
3. No snapshot requires `V`
4. Recovery correctness is preserved without `V`

All four conditions are mandatory.

---

## 5. Interaction with WAL

### 5.1 GC as a Durable Event

* Version removal must be WAL-recorded
* GC decisions are replayable
* Recovery must never “re-collect” versions implicitly

GC without WAL representation is forbidden.

---

### 5.2 Crash Safety

If a crash occurs:

* Before GC record durability:

  * Version remains
* After GC record durability:

  * Version is considered collected

There is no intermediate GC state.

---

## 6. Snapshot Retention Rules

Snapshots impose **hard retention barriers**:

* Any version visible at a snapshot boundary:

  * Must be retained
* GC must consider **all snapshots**, not only the latest

Deleting a snapshot may lower the visibility floor, but never raises it.

---

## 7. Checkpoint Interaction

* Checkpoints may enable GC by:

  * Eliminating WAL dependencies
  * Freezing a visibility boundary
* Checkpoints do not force GC
* GC is optional, never mandatory

Checkpointing does not change eligibility rules.

---

## 8. Deterministic GC Ordering

GC must be:

* Deterministic
* Order-independent for correctness
* Safe under replay

The exact order of reclaiming eligible versions must not affect correctness.

---

## 9. Explicitly Forbidden GC Behaviors

GC must **never**:

* Guess whether a version is unused
* Use wall-clock age as a criterion
* Remove the newest version of a key
* Violate snapshot retention
* Operate outside WAL authority

Space pressure is never a justification to violate invariants.

---

## 10. GC Summary

MVCC garbage collection in AeroDB is:

* Proof-based
* Snapshot-aware
* WAL-governed
* Crash-safe
* Deterministic

GC is allowed **only** when correctness is preserved beyond doubt.
