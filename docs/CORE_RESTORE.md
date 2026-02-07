# RESTORE.md — AeroDB Restore Specification (Phase 1)

This document defines the authoritative restore behavior for AeroDB Phase 1.

Restore reconstructs a complete AeroDB instance from a backup archive.

Restore is destructive.

Existing data is overwritten.

If implementation behavior conflicts with this document, the implementation is wrong.

---

## 1. Principles

Restore must satisfy:

- Atomic replacement
- Full integrity verification
- Deterministic outcome
- Zero partial success
- Explicit failure

Restore never merges data.

Restore always replaces.

---

## 2. Restore Command

Triggered manually:

```

aerodb restore --config aerodb.json --input backup.tar

```

Restore may only run when AeroDB is NOT serving.

---

## 3. Restore Preconditions

Before restore:

- AeroDB must not be running
- data_dir must exist
- backup.tar must exist and be readable

If any fail → abort.

---

## 4. Restore Contents

Restore expects:

```

backup.tar
├── snapshot/
├── wal/
└── backup_manifest.json

```

Missing any → FATAL.

---

## 5. Restore Algorithm (Strict Order)

Restore MUST follow:

1. Verify AeroDB not running
2. Extract backup.tar to temp directory
3. Validate backup_manifest.json
4. Validate snapshot manifest and checksums
5. Validate schema files
6. Validate WAL checksum
7. Move existing data_dir → data_dir.old
8. Create fresh data_dir
9. Copy snapshot storage → data_dir/data/
10. Copy schemas → data_dir/metadata/schemas/
11. Copy WAL → data_dir/wal/
12. fsync data_dir recursively
13. Delete data_dir.old

Only after all succeed is restore complete.

Any failure aborts.

---

## 6. Atomic Replacement

Restore uses directory swap:

- Original data_dir moved aside
- New data_dir constructed
- Only after success is old data deleted

Crash during restore:

- Either old or new directory remains
- Never partial mixed state

---

## 7. Startup After Restore

After restore:

```

aerodb start

```

Startup performs:

- snapshot loading
- WAL replay
- index rebuild
- verification

Restore does NOT build indexes.

Restore prepares data only.

---

## 8. Corruption Policy

If corruption detected:

- abort restore
- original data_dir preserved
- operator intervention required

No auto-repair.

---

## 9. Determinism

Given identical backup.tar:

Restore must produce identical data_dir.

No timestamps or random IDs introduced.

---

## 10. Phase-1 Limitations

Restore does NOT support:

- partial restore
- namespace restore
- selective collections
- live restore

These belong to later phases.

---

## 11. Authority

This document governs:

- aerodb restore
- disaster recovery
- backup validation

Violations are correctness bugs.
