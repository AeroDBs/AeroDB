# BACKUP.md — AeroDB Backup Specification (Phase 1)

This document defines the authoritative backup behavior for AeroDB Phase 1.

Backups provide an offline, portable representation of database state suitable for:

- disaster recovery
- migration
- archival storage

Backups are created from snapshots and WAL.

If implementation behavior conflicts with this document, the implementation is wrong.

---

## 1. Principles

Backups must satisfy:

- Atomic consistency
- Full durability
- Explicit integrity verification
- Deterministic restore
- Zero partial success

Backups are not incremental.

Each backup is complete.

---

## 2. Backup Command

Backups are triggered manually:

```

aerodb backup --config aerodb.json --output backup.tar

```

No automatic backups.

No background jobs.

---

## 3. Backup Contents

A backup archive contains:

```

backup.tar
├── snapshot/
│   ├── storage.dat
│   ├── schemas/
│   └── manifest.json
├── wal/
│   └── wal.log
└── backup_manifest.json

````

---

### 3.1 snapshot/

Exact copy of latest valid snapshot directory.

Includes:

- storage.dat
- schemas
- snapshot manifest

Indexes excluded.

---

### 3.2 wal/

Contains WAL tail after snapshot.

Rules:

- byte-for-byte copy
- fsync before packaging

---

### 3.3 backup_manifest.json

Top-level descriptor.

Example:

```json
{
  "backup_id": "20260204T120000Z",
  "created_at": "2026-02-04T12:00:00Z",
  "snapshot_id": "20260204T113000Z",
  "wal_present": true,
  "format_version": 1
}
````

---

## 4. Backup Creation Algorithm

Backup creation MUST follow:

1. Acquire global execution lock
2. fsync WAL
3. Identify latest valid snapshot
4. Copy snapshot → temp directory
5. Copy WAL tail → temp directory
6. Generate backup_manifest.json
7. fsync temp directory
8. Package temp directory into tar
9. fsync backup.tar
10. Release global execution lock

Any failure aborts backup.

Temporary directories removed.

---

## 5. Atomicity

Backup becomes valid only after:

* tar creation complete
* tar fsync

Partial backups must be deleted.

---

## 6. Integrity Verification

Backup must contain:

* snapshot manifest
* schema files
* WAL

Restore verifies:

* snapshot checksums
* schema checksums
* WAL checksums

Any mismatch → FATAL.

---

## 7. Determinism

Given identical:

* snapshot
* WAL tail

Backup archives must be identical.

No timestamps inside tar except manifest fields.

---

## 8. Corruption Policy

Backup corruption detected during restore:

* restore aborts
* database not modified

No partial restore.

---

## 9. Phase-1 Limitations

Backups do NOT support:

* encryption
* compression
* streaming
* incremental mode

These belong to later phases.

---

## 10. Authority

This document governs:

* aerodb backup
* backup format
* restore inputs
* disaster recovery

Violations are correctness bugs.
