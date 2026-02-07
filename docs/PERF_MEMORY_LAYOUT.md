# MEMORY LAYOUT OPTIMIZATION — PHASE 3

## Status

- Phase: **3**
- Authority: **Normative**
- Scope: **In-memory layout and allocation optimizations**
- Dependencies:
  - PERF_VISION.md
  - PERF_INVARIANTS.md
  - PERF_PROOF_RULES.md
  - PERFORMANCE_BASELINE.md
  - CRITICAL_PATHS.md
  - SEMANTIC_EQUIVALENCE.md
  - FAILURE_MODEL_PHASE3.md
  - ROLLBACK_AND_DISABLEMENT.md
  - PERFORMANCE_OBSERVABILITY.md

This document specifies **Memory Layout Optimization** as a correctness-preserving,
purely in-memory optimization.  
If any rule in this document cannot be satisfied, the optimization MUST NOT be implemented.

---

## 1. Purpose

Baseline AeroDB prioritizes clarity and correctness over memory efficiency:

- Straightforward data structures
- Conservative allocation patterns
- No reliance on cache behavior

This ensures correctness but may incur:
- Poor cache locality
- Excessive pointer chasing
- Allocation overhead

**Memory Layout Optimization** improves performance by:
- Improving cache locality
- Reducing allocation count
- Improving data density

While strictly preserving:
- Semantics
- Determinism
- Recovery behavior
- Replication compatibility

Memory layout is **never authoritative**.

---

## 2. Baseline Reference (Normative)

Baseline memory behavior is defined implicitly in:

- `PERFORMANCE_BASELINE.md` §11
- `CRITICAL_PATHS.md` (all sections)

Baseline properties:

- In-memory state is discardable
- Persistent correctness does not depend on memory layout
- Restart reconstructs all in-memory state
- No memory content is assumed to persist

These properties MUST remain true.

---

## 3. Definition of Memory Layout Optimization

### 3.1 Conceptual Definition

A memory layout optimization is any change that:

> Improves performance by altering how data is laid out in memory,
> without changing data meaning, lifetime, or authority.

Optimizations apply ONLY to:
- In-memory representations
- Transient execution state
- Derived structures

They NEVER apply to:
- WAL formats
- Snapshot formats
- Checkpoint formats
- Network formats

---

### 3.2 Explicit Non-Definition (What This Is NOT)

Memory Layout Optimization does NOT allow:

- Persisted memory assumptions
- Platform-specific correctness dependencies
- Undefined behavior reliance
- Pointer reinterpretation tricks
- Alignment-based semantic inference
- Architecture-specific branching

If correctness depends on CPU cache behavior, it is invalid.

---

## 4. Allowed Memory Layout Optimizations

Only the following classes are permitted.

---

### 4.1 Structure Packing and Field Reordering

#### Description

Reorder struct fields to:
- Reduce padding
- Improve cache-line utilization

#### Rules

- Field semantics unchanged
- No reliance on layout outside the struct
- Explicit `#[repr(C)]` or `#[repr(Rust)]` semantics documented

#### Proof Obligation

- Logical equality preserved
- Serialization unaffected
- No external ABI dependence introduced

---

### 4.2 Cache-Line Alignment (Advisory Only)

#### Description

Align frequently accessed structures to cache-line boundaries.

#### Rules

- Alignment is advisory, not required
- No correctness dependence on alignment
- Code must behave identically without alignment

#### Proof Obligation

- Misalignment does not change behavior
- Alignment absence does not degrade correctness

---

### 4.3 Allocation Strategy Refinement

#### Description

Reduce allocation overhead by:
- Arena allocation
- Object pooling
- Bump allocators

#### Rules

- Allocation lifetime MUST be explicit
- No reuse of live objects
- No hidden aliasing

#### Proof Obligation

- Object lifetimes remain correct
- Drop semantics remain correct
- Memory safety preserved

---

### 4.4 Data Locality Improvements

#### Description

Group frequently accessed fields together to:
- Reduce cache misses
- Improve traversal speed

#### Rules

- Access patterns must be deterministic
- No data duplication with semantic meaning
- No coupling between unrelated structures

#### Proof Obligation

- Data access yields identical values
- Traversal order unchanged

---

### 4.5 Copy Elision (Safe Only)

#### Description

Avoid copying immutable data when:
- Ownership is clear
- Lifetime is bounded
- Aliasing is controlled

#### Rules

- No mutable aliasing
- Explicit lifetime management
- Compiler-enforced safety

#### Proof Obligation

- Data immutability guaranteed
- Use-after-free impossible

---

## 5. Forbidden Memory Optimizations

The following are **explicitly forbidden**:

- Unsafe code without proof
- Pointer arithmetic affecting semantics
- Type punning
- Bit-level reinterpretation
- Architecture-specific correctness
- Lock-free tricks relying on UB
- Relaxed memory ordering affecting logic

If correctness depends on CPU behavior, it is invalid.

---

## 6. Invariant Preservation Matrix

(Referenced from `PERF_INVARIANTS.md`)

### Durability
- D-1, D-2, D-3: **Not Applicable (In-Memory Only)**

### Determinism
- DET-1 (Crash Determinism): **Preserved**
- DET-2 (Replay Equivalence): **Preserved**
- DET-3 (Bounded Execution): **Preserved**

### MVCC
- MVCC-1 (Snapshot Isolation): **Preserved**
- MVCC-2 (CommitId Authority): **Preserved**
- MVCC-3 (Version Chain Integrity): **Preserved**

### Replication
- REP-1, REP-2, REP-3: **Preserved**

### Failure & Recovery
- FR-1, FR-2, FR-3: **Preserved**

### Observability
- OBS-1, OBS-2: **Preserved**

### Disablement
- DIS-1, DIS-2, DIS-3: **Preserved**

---

## 7. Semantic Equivalence Argument

Memory Layout Optimization is semantically equivalent to baseline because:

- Only in-memory representation changes
- Logical state remains identical
- Persistent state is unaffected
- Recovery reconstructs state identically
- Replication is unaffected

Any difference is invisible to all observers.

---

## 8. Failure Matrix

### 8.1 Crash During Execution

- In-memory state lost
- Recovery reconstructs state

Equivalent.

---

### 8.2 Allocation Failure

- Baseline: operation fails
- Optimized: operation fails

Equivalent.

---

### 8.3 Memory Fragmentation

- Baseline: degraded performance
- Optimized: degraded performance

Correctness unaffected.

---

## 9. Recovery Proof

- Recovery does not consult in-memory layout
- WAL replay unaffected
- Snapshot loading unaffected
- Index rebuild unaffected

Recovery behavior is identical.

---

## 10. Disablement & Rollback

### 10.1 Disablement Mechanism

Memory layout optimizations MUST be disableable via:

- Compile-time flag **or**
- Startup configuration

Disablement restores:
- Baseline allocation paths
- Baseline structure layouts

---

### 10.2 Compatibility Proof

- WAL unchanged
- Snapshot unchanged
- Checkpoint unchanged

Data compatibility is guaranteed.

---

### 10.3 No Ghost State

- No persistent memory assumptions
- No hidden metadata
- No runtime layout flags persisted

All layout choices are ephemeral.

---

## 11. Observability

Permitted metrics (passive only):

- memory.allocations.count
- memory.arena.usage
- memory.copy_elision.count
- memory.cache_line.utilization (approximate)

Metrics MUST NOT:
- Influence allocation strategy
- Influence layout selection
- Influence execution flow

---

## 12. Testing Requirements

Memory Layout Optimization MUST introduce:

- Layout equivalence tests
- Allocation lifetime tests
- Stress tests under memory pressure
- Disablement equivalence tests
- Crash recovery tests

All existing tests MUST pass unmodified.

---

## 13. Explicit Non-Goals

Memory Layout Optimization does NOT aim to:

- Change data models
- Introduce unsafe performance hacks
- Depend on hardware features
- Improve persistence efficiency

It improves **in-memory execution cost only**.

---

## 14. Final Rule

> Memory layout may change how fast AeroDB runs,  
> but it must never change what AeroDB *is*.

If a memory optimization can affect correctness,
it does not belong in Phase 3.

---

END OF DOCUMENT
