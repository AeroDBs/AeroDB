## Purpose

This document defines the reliability guarantees of aerodb.

Reliability is not a feature.
It is the foundation upon which all other features depend.

aerodb is expected to meet the reliability expectations commonly associated
with mature infrastructure platforms such as Supabase, while maintaining
its own stricter guarantees around determinism and explicit behavior.

These guarantees are mandatory.
Any design or implementation that weakens them is rejected.

---

## Reliability Philosophy

aerodb assumes:
- Hardware will fail
- Processes will crash
- Power will be lost
- Humans will make mistakes

The system must remain correct and predictable in the presence of these failures.

Reliability in aerodb is defined as:
- Data safety over availability
- Explicit failure over silent degradation
- Deterministic recovery over best-effort repair
- Operational clarity over hidden resilience

---

## Core Reliability Guarantees

### R0. No Acknowledged Data Loss

Once aerodb acknowledges a write operation as successful:
- That data must survive process crashes
- That data must survive power loss
- That data must survive system restarts

Any violation of this guarantee is considered catastrophic.

---

### R1. Write-Ahead Logging Is Mandatory

- All write operations must be recorded in a Write-Ahead Log (WAL)
  before being acknowledged.
- WAL persistence must be enforced with explicit fsync boundaries.
- There are no modes that bypass WAL for acknowledged writes.

Disabling or weakening WAL durability is forbidden.

---

### R2. Crash Recovery Is Deterministic

- Given the same on-disk data and WAL, recovery must always:
  - Produce the same resulting state
  - Replay the same operations in the same order
- Recovery behavior must not depend on:
  - Timing
  - Thread scheduling
  - Environment-specific factors

Non-deterministic recovery is unacceptable.

---

### R3. Recovery Completeness Is Explicit

After recovery, aerodb must be able to explicitly report:
- Whether recovery completed successfully
- Whether any WAL entries were skipped
- Whether any data loss occurred

Ambiguous recovery states are forbidden.

---

## Consistency Guarantees

### C1. Single-Document Atomicity

- Writes to a single document are atomic.
- Partial updates to a document must never be observable.
- Reads must never return partially written or invalid data.

---

### C2. Read-After-Write Consistency (Single Node)

- After a successful write acknowledgment:
  - Subsequent reads on the same node must observe the write.
- Stale reads after acknowledged writes are forbidden.

---

### C3. Schema-Consistent Reads

- Reads must always return data that conforms to the declared schema.
- Schema-invalid data must never be readable.
- If schema consistency cannot be guaranteed, the read must fail.

---

## Failure Handling Guarantees

### F1. Fail Fast, Fail Loud

- Failures must be detected as early as possible.
- Failures must be surfaced explicitly to the caller.
- Silent failure, silent retries, or silent degradation are forbidden.

---

### F2. No Partial Success Masking

- Operations must be:
  - Fully successful, or
  - Fully failed
- aerodb must not report success for operations that only partially completed.

---

### F3. Deterministic Error Semantics

- The same failure condition must always result in:
  - The same error code
  - The same failure classification
- Random, timing-dependent, or environment-dependent error behavior is forbidden.

---

## Corruption Handling Guarantees

### K1. Corruption Detection Is Mandatory

- On-disk data structures must include integrity checks (e.g. checksums).
- Corruption must be detected deterministically.

---

### K2. Corruption Is Never Silently Repaired

- Detected corruption must:
  - Halt affected operations
  - Be reported explicitly
- Silent repair, skipping, or masking of corruption is forbidden.

Downtime is preferable to silent corruption.

---

## Operational Reliability Guarantees

### O1. Predictable Startup and Shutdown

- Startup behavior must be deterministic and observable.
- Shutdown must:
  - Complete pending durable operations, or
  - Explicitly fail and report incomplete work

---

### O2. Minimal Configuration, Safe Defaults

- Default configuration must be safe and durable.
- Configuration options must not allow users to:
  - Disable durability guarantees
  - Weaken consistency guarantees
  - Violate invariants

Unsafe configurations must be rejected.

---

### O3. Observable System State

At all times, aerodb must be able to explain:
- What it is doing
- What guarantees are currently upheld
- Why an operation failed

Opacity is treated as a reliability failure.

---

## Upgrade and Evolution Guarantees

### U1. Data Safety Across Upgrades

- Upgrading aerodb must never:
  - Corrupt existing data
  - Silently rewrite data
  - Change semantics without explicit opt-in

---

### U2. Explicit Compatibility Contracts

- Backward compatibility guarantees must be documented.
- Breaking changes must:
  - Be intentional
  - Be visible
  - Be justified

Accidental breaking changes are unacceptable.

---

## Institutional Responsibility

aerodb is backed by Tonmoy Infrastructure & Vision.

This implies:
- Long-term maintenance expectations
- Conservative evolution
- Reliability over experimentation
- Accountability for failures

Reliability claims must be defensible through:
- Code
- Tests
- Documentation
- Operational behavior

---

## Enforcement

- Reliability guarantees override performance optimizations.
- Any optimization that weakens reliability is rejected.
- Reliability-related tests take precedence over feature tests.

If a choice must be made between:
- Being fast, or
- Being correct and predictable

aerodb always chooses correctness and predictability.

---

## Final Statement

Reliability is not something aerodb hopes to achieve.
It is something aerodb enforces by design.

If these guarantees cannot be upheld,
the system must fail explicitly rather than pretend otherwise.

Trust is earned only through discipline.
