# ROADMAP.md — AeroDB Phase 1 (Reliability & Operability)

Phase 0 delivered a complete deterministic single-node database engine.

Phase 1 transforms AeroDB from a kernel into an operational system.

This phase prioritizes:

- reliability
- recoverability
- observability
- usability

NOT performance.
NOT feature breadth.

---

## Phase 1 Mission

Make AeroDB safe to run in production on a single node.

Success criteria:

- Startup time bounded
- WAL growth bounded
- Backups possible
- Recovery verifiable
- Operators informed
- Failures diagnosable

Only after these are achieved do higher-level features begin.

---

## Guiding Principles

1. Correctness before speed
2. Explicit behavior over magic
3. Operator visibility over automation
4. Determinism preserved
5. No hidden background activity

---

# Milestone 1 — Checkpointing (Highest Priority)

### Goal

Bound recovery time and WAL growth.

---

### Deliverables

#### 1. Storage Snapshot

Introduce:

```

storage.snapshot()

```

Creates:

```

<data_dir>/snapshots/<timestamp>/

```

Containing:

- document storage copy
- schema metadata
- snapshot manifest

---

#### 2. WAL Truncation

After successful snapshot:

- truncate WAL to zero
- start new WAL segment

Rules:

- snapshot must fsync
- manifest written last
- truncation atomic

---

#### 3. Recovery Upgrade

On startup:

1. Detect snapshot
2. Load snapshot storage
3. Replay WAL from snapshot offset

Never partial.

---

### Documents to Add

- CHECKPOINT.md
- SNAPSHOT.md

---

# Milestone 2 — Backup & Restore

### Goal

Enable offline backups.

---

### Commands

```

aerodb backup --output backup.tar
aerodb restore --input backup.tar

```

---

### Backup Contents

- storage snapshot
- schemas
- WAL tail
- manifest

Restore validates:

- checksums
- schemas
- snapshot consistency

---

### Guarantees

Restored system must boot identically.

---

### Documents to Add

- BACKUP.md
- RESTORE.md

---

# Milestone 3 — Metrics & Observability

### Goal

Expose internal health.

---

### Metrics

Expose via CLI:

```

aerodb stats

```

Include:

- document count
- index size
- WAL size
- last checkpoint time
- recovery duration
- schema count

---

### Logging

Add structured startup logs:

- recovery start/end
- WAL replay count
- index rebuild time
- corruption detection

---

### Documents

- OBSERVABILITY.md

---

# Milestone 4 — Crash Injection Testing

### Goal

Prove correctness under failure.

---

### Add fault tests:

- kill during WAL write
- kill during storage write
- kill during index rebuild
- kill during checkpoint

Validate:

- recovery correctness
- no partial writes
- deterministic restart

---

# Milestone 5 — Schema Evolution (Limited)

### Goal

Allow additive schema changes.

---

Rules:

- new fields allowed
- old fields preserved
- versioned reads still enforced

No migrations yet.

---

# Milestone 6 — HTTP API (Optional)

After stability achieved:

- expose REST API
- keep CLI internally
- same API_SPEC

No streaming.

---

# Explicit Non-Goals (Phase 1)

These are forbidden:

- replication
- clustering
- sharding
- transactions
- query optimization
- async execution
- background compaction

These belong to Phase 2+.

---

# Exit Criteria

Phase 1 is complete when:

- WAL size bounded
- recovery time bounded
- backups usable
- metrics visible
- crash tests pass
- operators informed

Only then may distributed features begin.

---

# Phase 2 Preview (Not Implemented)

- replication
- leader election
- read replicas
- logical clocks
- consensus

---

## Summary

Phase 0 built a database.

Phase 1 builds a system.

Reliability before scale.

Observability before optimization.

Determinism always.