# REPLICATION_PROOFS.md

## AeroDB Phase 2B — Replication Correctness Proofs

### Status

* This document is **authoritative**
* It provides argument-based proofs of replication correctness
* Proofs rely only on invariants and WAL semantics
* No implementation assumptions appear here
* Phase-1 and MVCC semantics are **frozen and assumed correct**

---

## 1. Proof Methodology

All proofs in this document are constructed from:

* Phase-1 invariants
* MVCC invariants
* Replication invariants
* WAL as the sole authority
* Exhaustive failure enumeration

Each proof answers the question:

> “Why does this behavior remain correct under all crashes, replays, and replication lag?”

---

## 2. Proof of Single-Writer Safety

### Claim

At most one node can create authoritative history at any time.

### Argument

1. Only a node in `PrimaryActive` may assign `CommitId`
2. CommitIds originate solely from WAL records
3. Replicas reject local commit attempts
4. Authority is externally configured, never inferred

Therefore:

* Only one node can create new WAL history
* No two nodes can legally acknowledge writes

### Conclusion

Single-writer safety is enforced by authority rules, not timing.

---

## 3. Proof of Global History Linearity

### Claim

All nodes observe a single, totally ordered history.

### Argument

1. WAL records are totally ordered on the Primary
2. Replication preserves strict WAL order
3. Replicas apply only WAL prefixes
4. Gaps and reordering are detected and fatal

Therefore:

* Replica history is always a prefix of Primary history
* No forks or divergence can exist silently

### Conclusion

Global history linearity is preserved deterministically.

---

## 4. Proof of Deterministic State Reconstruction

### Claim

Given identical WAL prefixes (and snapshots), all nodes reconstruct identical state.

### Argument

1. WAL replay is deterministic (Phase-1 invariant)
2. MVCC reconstruction is deterministic
3. GC actions are WAL-represented
4. Snapshot boundaries are explicit

Therefore:

* State reconstruction is a pure function of WAL + snapshot
* No runtime or timing influence exists

### Conclusion

Replication preserves deterministic replay.

---

## 5. Proof of Replica Read Safety

### Claim

Replica reads never return incorrect or future-visible data.

### Argument

1. Replica reads are allowed only when:

   * Read view ≤ applied WAL prefix
2. Visibility rules are identical to Primary
3. Read views are immutable
4. WAL gaps or uncertainty force refusal

Therefore:

* Replica reads are either correct or refused
* No speculative visibility exists

### Conclusion

Replica reads are MVCC-correct or fail-stop.

---

## 6. Proof of Snapshot Bootstrap Equivalence

### Claim

Snapshot + WAL bootstrap is semantically equivalent to full WAL replay.

### Argument

1. Snapshot represents a valid MVCC cut at `C_snap`
2. Snapshot includes all state ≤ `C_snap`
3. WAL replay resumes strictly after `C_snap`
4. No WAL entries ≤ `C_snap` are replayed

Therefore:

* Final reconstructed state is identical
* No history is skipped or rewritten

### Conclusion

Snapshot transfer preserves full correctness.

---

## 7. Proof of Crash Safety Under Replication

### Claim

Replication introduces no undefined states under crash.

### Argument

1. All crash points are enumerated in the failure matrix
2. WAL durability defines commit existence
3. Partial replication state is discarded
4. Recovery is deterministic and idempotent

Therefore:

* Every crash maps to exactly one valid recovery outcome
* No ambiguous state exists

### Conclusion

Replication is crash-safe by construction.

---

## 8. Proof of No Silent Divergence

### Claim

Replication cannot silently diverge.

### Argument

1. Replicas require WAL prefix equality
2. Checksums and ordering enforce integrity
3. Any divergence triggers `ReplicationHalted`
4. No auto-healing is permitted

Therefore:

* Divergence is detectable
* Divergence is fatal, not hidden

### Conclusion

Silent divergence is impossible.

---

## 9. Proof of Phase-1 & MVCC Compatibility

### Claim

Replication does not alter Phase-1 or MVCC semantics.

### Argument

1. WAL semantics are unchanged
2. CommitId semantics are unchanged
3. Visibility rules are unchanged
4. GC rules are unchanged
5. Snapshot semantics are unchanged

Therefore:

* Replication adds no new meanings
* Existing behaviors remain authoritative

### Conclusion

Replication is a strict extension, not a modification.

---

## 10. Global Replication Correctness Theorem

### Statement

> Given:
>
> * Phase-1 invariants
> * MVCC invariants
> * Replication invariants
> * WAL as the sole authority
>
> AeroDB replication guarantees:
>
> * Single authoritative history
> * Deterministic state propagation
> * MVCC-correct reads
> * Crash-safe recovery
> * No silent divergence
>
> Under all crash, partition, and replay scenarios.

---

## 11. Proof Closure

At this point:

* Replication semantics are fully specified
* All failure modes are defined
* Correctness arguments are complete
* No implementation assumptions remain

Replication is now **provably safe to implement**.
