## AeroDB Phase 2 — MVCC Invariants

### Status

* This document is **authoritative**
* All invariants herein are **non-negotiable**
* Phase-1 invariants remain in force unless explicitly restated as unchanged
* No implementation details appear here

---

## 1. Phase-1 Invariants That Remain Unchanged

The following Phase-1 invariants are **fully preserved** under MVCC.
MVCC **must not** weaken, reinterpret, or bypass them.

### 1.1 Durability Invariants

* Any write acknowledged to a client is **durable**
* Durability is defined by:

  * WAL append
  * Checksum verification
  * fsync completion
* MVCC metadata is subject to the **same durability rules** as document data

### 1.2 Recovery Invariants

* Recovery is **deterministic**
* WAL replay produces:

  * Identical document state
  * Identical MVCC version state
* Recovery never guesses, skips, or heals inconsistencies

### 1.3 Corruption Detection Invariants

* All persisted MVCC data is checksummed
* Corruption is:

  * Always detected
  * Never repaired silently
* Detection halts recovery with explicit error

### 1.4 Query Determinism Invariants

* Queries are:

  * Bounded
  * Deterministic
  * Independent of timing or thread scheduling
* MVCC visibility must not introduce nondeterministic results

### 1.5 Snapshot & Checkpoint Invariants

* Snapshots are:

  * Read-only
  * fsync-safe
  * Manifest-driven
* MVCC state included in snapshots must be **complete and self-consistent**
* Checkpoints remain crash-safe and WAL-truncation-safe

---

## 2. Core MVCC Invariants (New)

These invariants define what **must always be true** once MVCC exists.

---

### 2.1 Version Immutability Invariant

* Once created, a document version is **immutable**
* No in-place modification of historical data is allowed
* Updates create **new versions only**
* Deletes are represented as **explicit tombstone versions**

> History is append-only in meaning, even if storage optimizations exist.

---

### 2.2 Visibility Determinism Invariant

* Given:

  * A database state
  * A read view identifier
* The set of visible versions is **fully deterministic**
* Visibility:

  * Does not depend on wall-clock time
  * Does not depend on thread interleaving
  * Does not depend on runtime heuristics

The same inputs **always** produce the same visible results.

---

### 2.3 Read View Stability Invariant

* A read operation observes a **stable view**
* Once established, a read view:

  * Never changes
  * Never sees partial writes
  * Never sees future versions

Readers are never blocked by writers **for correctness**.

---

### 2.4 Write Atomicity Invariant

* A write either:

  * Produces a fully visible new version, or
  * Produces nothing
* No reader may observe:

  * Half-written versions
  * Partially linked metadata
* Atomicity holds across crashes and recovery

---

### 2.5 Commit Ordering Invariant

* All committed versions have a **total, deterministic order**
* That order:

  * Is preserved across restarts
  * Is reproducible during WAL replay
* There are no “ties” or ambiguous commit positions

This ordering is the backbone of visibility, recovery, and replication.

---

### 2.6 Snapshot Compatibility Invariant

* MVCC must integrate with existing snapshot semantics
* A snapshot:

  * Represents a valid MVCC cut
  * Contains all versions required to satisfy that cut
* No snapshot may reference:

  * Missing versions
  * External state
  * Implicit history

---

### 2.7 Crash Safety Invariant (MVCC-Specific)

At any crash point:

* Recovery must land in one of two states:

  * Before a version exists
  * After the version exists fully
* No intermediate MVCC state is valid
* WAL is the **sole authority** for resolving ambiguity

---

### 2.8 Garbage Collection Safety Invariant

* A version may be reclaimed **only if**:

  * It is provably invisible to all possible read views
* GC decisions must be:

  * Deterministic
  * Recoverable
  * Replayable
* GC must never race visibility correctness

> Space may be sacrificed for correctness, never the reverse.

---

### 2.9 Replication Readiness Invariant

Even before replication is implemented:

* MVCC semantics must be:

  * Serializable into WAL
  * Reconstructible on another node
* No local-only shortcuts
* No hidden state outside formal persistence

---

## 3. Explicitly Forbidden Behaviors

MVCC must **never**:

* Infer visibility from system time
* Allow “best-effort” reads
* Auto-resolve write-write conflicts silently
* Skip MVCC metadata during recovery
* Leak uncommitted state to readers
* Introduce implicit isolation levels

If behavior cannot be described as an invariant, it does not exist.

---

## 4. Invariant Enforcement Philosophy

* Invariants are enforced by:

  * Design
  * WAL structure
  * Recovery logic
* Tests prove enforcement
* Observability only **reports**, never enforces