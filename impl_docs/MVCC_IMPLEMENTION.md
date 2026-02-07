# AeroDB MVCC Implementation Status

## Phase 2: MVCC Implementation Complete ✓

> **SEMANTIC FREEZE**: MVCC semantics are now frozen.
> No semantic changes allowed without a new phase.

---

## MVCC-01: Domain Foundations ✓

| Type | Description |
|------|-------------|
| `CommitId` | Opaque, totally ordered commit identity |
| `Version` | Immutable document version |
| `VersionPayload` | Explicit Document/Tombstone enum |
| `VersionChain` | Document history container |
| `ReadView` | Stable snapshot boundary |

---

## MVCC-02: Commit Identity WAL Authority ✓

| Component | Description |
|-----------|-------------|
| `RecordType::MvccCommit` | WAL record type (value 3) |
| `MvccCommitPayload` | Commit identity payload |
| `CommitAuthority` | WAL-based commit identity assignment |

---

## MVCC-03: Version Persistence ✓

| Component | Description |
|-----------|-------------|
| `RecordType::MvccVersion` | WAL record type (value 4) |
| `MvccVersionPayload` | Version with commit binding |
| `VersionValidator` | Cross-validation for recovery |

---

## MVCC-04: Read Views & Visibility ✓

| Component | Description |
|-----------|-------------|
| `Visibility` | Stateless visibility resolver |
| `VisibilityResult` | Visible/Invisible result enum |
| `visible_version()` | Apply exact MVCC_VISIBILITY.md rules |

---

## MVCC-05: MVCC-Aware Snapshots ✓

| Component | Description |
|-----------|-------------|
| `SnapshotManifest.commit_boundary` | MVCC cut point in manifest |
| `format_version: 2` | MVCC snapshot format |
| `create_mvcc_snapshot()` | Snapshot with MVCC boundary |

---

## MVCC-06: Garbage Collection ✓

| Component | Description |
|-----------|-------------|
| `RecordType::MvccGc` | WAL record type (value 5) |
| `VersionLifecycleState` | Live → Obsolete → Reclaimable → Collected |
| `VisibilityFloor` | Tracks oldest read view + snapshot boundary |
| `GcEligibility` | All 4 mandatory rules per MVCC_GC.md |
| `GcRecordPayload` | WAL serialization for GC events |

### GC Eligibility Rules (ALL mandatory)

1. `commit_id < visibility_lower_bound`
2. A newer version exists in the chain
3. No snapshot requires the version
4. Recovery correctness preserved

---

## MVCC-07: Failure Matrix ✓

All crash points per MVCC_FAILURE_MATRIX.md:

| Crash Point | Outcome |
|-------------|---------|
| Before commit intent | Write discarded |
| After commit identity, before version | Recovery fails explicitly |
| After full persistence | Visible |
| Before GC WAL record | Version remains |
| After GC WAL record | Version collected |

---

## MVCC-08: Phase-1 Regression ✓

- All 511 tests pass
- All Phase-1 behaviors unchanged
- MVCC is a strict superset of Phase-1

---

## MVCC-09: Readiness Lock ✓

### Guaranteed Behaviors

- ✅ Deterministic visibility
- ✅ Atomic writes
- ✅ Snapshot isolation
- ✅ Crash-safe recovery
- ✅ Safe garbage collection
- ✅ Phase-1 behavioral equivalence

### Explicit Non-Features

- ❌ Write-write conflict detection (future phase)
- ❌ Serializable isolation (future phase)
- ❌ Distributed MVCC (future phase)

### Semantic Freeze

After this point:
- MVCC semantics are **immutable**
- Only replication and performance may build on top
- Any MVCC change requires a new phase

---

## Test Coverage

| Category | Tests |
|----------|-------|
| MVCC Total | 50+ |
| GC Eligibility | 14 |
| GC Crash Semantics | 6 |
| Phase-1 Regression | All pass |
| **Total** | **511** |
