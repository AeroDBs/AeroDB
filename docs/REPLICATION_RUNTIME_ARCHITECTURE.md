
# REPLICATION RUNTIME ARCHITECTURE

## Status

- Phase: **5**
- Authority: **Normative**
- Scope: **Runtime structure of replication implementation**
- Depends on:
  - `OBSERVABILITY_VISION.md`
  - `OBSERVABILITY_INVARIANTS.md`
  - `OBSERVABILITY_IMPLEMENTATION_ORDER.md`
  - `REPL_*` specifications
  - `MVCC_*` specifications
  - `CORE_*` specifications

This document defines the **only permitted runtime architecture**
for AeroDB replication in Phase 5.

If implementation deviates from this architecture,
correctness proofs no longer apply.

---

## 1. Purpose

Replication correctness depends as much on **runtime structure** as on logic.

This document:
- Prevents hidden authority
- Prevents accidental concurrency
- Makes failure boundaries explicit
- Ensures explainability of replication behavior

This is an **architecture document**, not a behavior redesign.

---

## 2. Architectural Principles

### A-1: Single Authority Per Responsibility

Each replication responsibility MUST have:
- Exactly one owner
- Exactly one mutation path
- Explicit read-only consumers

Shared mutable ownership is forbidden.

---

### A-2: Explicit State Machines

All replication components MUST:
- Be driven by explicit state machines
- Enumerate all states
- Validate transitions

Implicit or boolean-driven states are forbidden.

---

### A-3: Deterministic Concurrency

Concurrency is permitted ONLY when:
- Responsibilities are disjoint
- Ordering is explicit
- State transitions are serialized

Timing-based coordination is forbidden.

---

## 3. High-Level Runtime Components

Replication runtime consists of **five components**.
They communicate via **explicit channels** and **immutable messages**.

```

+------------------+
| Primary Runtime  |
+------------------+
|
| WAL Segments (immutable)
v
+------------------+       +------------------+
| WAL Receiver     | ----> | WAL Validator    |
+------------------+       +------------------+
|
v
+------------------+
| WAL Applier      |
+------------------+
|
v
+------------------+
| Replica State    |
+------------------+
|
v
+------------------+
| Read Safety Gate |
+------------------+

````

---

## 4. Primary-Side Architecture (Minimal)

### 4.1 Primary Responsibilities

The Primary:
- Assigns CommitIds
- Appends WAL
- Exposes WAL segments for shipping

Primary replication logic MUST be:
- Read-only with respect to replication
- Passive (no push, only serve)

There are **no replication threads on the Primary**.

---

## 5. Replica-Side Architecture (Authoritative)

All replication runtime complexity lives on the **Replica**.

---

### 5.1 Replica Supervisor

**Role:** Top-level state owner

Responsibilities:
- Replica lifecycle
- Crash recovery orchestration
- State transitions between stages

Properties:
- Single-threaded
- Owns replica global state enum
- Exposes state to DX

Forbidden:
- Performing WAL IO directly
- Applying WAL
- Serving reads

---

### 5.2 WAL Receiver

**Role:** WAL ingress only

Responsibilities:
- Receive WAL segments
- Enforce ordering
- Persist raw WAL bytes

Properties:
- Single-threaded or async task
- No validation
- No state mutation beyond WAL storage

Input:
- Primary WAL stream

Output:
- Immutable WAL segments

Forbidden:
- Applying WAL
- Advancing CommitId
- Skipping segments

---

### 5.3 WAL Validator

**Role:** WAL correctness proof

Responsibilities:
- Check checksums
- Verify continuity
- Validate segment boundaries

Properties:
- Deterministic
- Stateless across runs (except checkpoints)
- Restart-safe

Input:
- WAL segments from Receiver

Output:
- Validated WAL segments OR explicit failure

Forbidden:
- Mutating storage
- Advancing CommitId

---

### 5.4 WAL Applier

**Role:** State mutation engine

Responsibilities:
- Apply validated WAL to storage
- Advance replica CommitId
- Maintain replay checkpoints

Properties:
- Single-threaded
- Crash-safe
- Deterministic

Input:
- Validated WAL segments

Output:
- Updated replica storage

Forbidden:
- Serving reads
- Skipping validation
- Applying speculative WAL

---

### 5.5 Replica State Store

**Role:** Canonical replica metadata

Responsibilities:
- Store:
  - Replica CommitId
  - Applied WAL offset
  - Snapshot bootstrap metadata
- Persist restart-safe state

Properties:
- Mutated only by:
  - Replica Supervisor
  - WAL Applier (scoped)

Forbidden:
- Concurrent mutation
- Hidden derived state

---

### 5.6 Read Safety Gate

**Role:** Read authorization

Responsibilities:
- Evaluate read safety predicates
- Enforce MVCC visibility rules
- Refuse unsafe reads

Properties:
- Read-only
- Pure function of observable state
- Explainable via DX

Forbidden:
- Heuristics
- Timing assumptions
- Lag-based shortcuts

---

## 6. State Machines (Mandatory)

### 6.1 Replica Global State

```text
Disabled
  ↓
Initializing
  ↓
ReceivingWAL
  ↓
ValidatingWAL
  ↓
ApplyingWAL
  ↓
Recovering
  ↓
Ready (read-safe OR read-blocked)
````

Transitions MUST:

* Be explicit
* Be logged
* Be observable

---

### 6.2 WAL Segment State

```text
Received → Validated → Applied
```

No skipping allowed.
No rollback allowed except via crash recovery.

---

## 7. Concurrency Model

### Allowed Concurrency

* WAL Receiver may run concurrently with Validator
* Validator may run concurrently with Applier ONLY via queueing
* Read Safety Gate runs concurrently but is read-only

### Forbidden Concurrency

* Multiple Appliers
* Receiver applying WAL
* Validator mutating storage
* Read path mutating replica state

---

## 8. Failure Boundaries

Each component MUST fail independently and explicitly.

| Component  | Failure Effect      |
| ---------- | ------------------- |
| Receiver   | Stops replication   |
| Validator  | Blocks application  |
| Applier    | Triggers recovery   |
| Supervisor | Replica unavailable |
| Read Gate  | Refuses reads       |

No cascading silent failure is allowed.

---

## 9. Observability Requirements

Each component MUST expose:

* Current state
* Blocking reason
* Last successful transition

DX APIs MUST surface:

* Replica global state
* WAL offsets
* CommitIds
* Read safety status

---

## 10. Explainability Requirements

For any replica state, the system MUST be able to explain:

* Why replication is blocked
* Why a read is allowed or denied
* Why a WAL segment is rejected

Explanations MUST reference:

* State machine states
* WAL offsets
* CommitIds

---

## 11. Forbidden Architecture Patterns

Explicitly forbidden:

* Background “best effort” threads
* Shared mutable state across components
* Lock-based correctness without state checks
* Implicit progress via retries
* Timing-based coordination

If any of these appear, the implementation is invalid.

---

## 12. Final Rule

> Replication correctness lives in structure,
> not in clever code.

If runtime ownership is clear,
correctness follows.

---

END OF DOCUMENT

