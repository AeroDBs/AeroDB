# SNAPSHOT.md — AeroDB Storage Snapshot Specification (Phase 1)

This document defines the authoritative snapshot format and semantics for AeroDB Phase 1.

Snapshots provide a point-in-time, durable copy of database state used for:

- checkpointing
- backups
- restore
- accelerated recovery

If implementation behavior conflicts with this document, the implementation is wrong.

Snapshots are designed for correctness, not speed.

---

## 1. Principles

Snapshots must obey:

- Deterministic creation
- Atomic visibility
- Full durability
- Explicit integrity verification
- Zero partial success

Snapshots capture:

- document storage
- schema metadata
- manifest

Indexes are NOT included.

Indexes are always rebuilt.

---

## 2. Snapshot Directory Layout

Snapshots live under:

```

<data_dir>/snapshots/<snapshot_id>/

```

Where:

```

snapshot_id = UTC timestamp in RFC3339 basic format

```

Example:

```

20260204T113000Z

```

---

### Required Files

```

snapshots/<snapshot_id>/
├── storage.dat
├── schemas/
│   └── *.json
└── manifest.json

```

---

## 3. Snapshot Contents

### 3.1 storage.dat

Binary copy of document storage file.

Rules:

- byte-for-byte copy
- fsync before manifest creation
- immutable after creation

---

### 3.2 schemas/

Exact copy of:

```

<data_dir>/metadata/schemas/

````

Rules:

- copied recursively
- filenames preserved
- fsync directory

---

### 3.3 manifest.json

Authoritative snapshot descriptor.

Example:

```json
{
  "snapshot_id": "20260204T113000Z",
  "created_at": "2026-02-04T11:30:00Z",
  "storage_checksum": "crc32:deadbeef",
  "schema_checksums": {
    "user_v1.json": "crc32:abcd1234"
  },
  "format_version": 1
}
````

---

## 4. Snapshot Creation Algorithm

Snapshot creation must follow this exact sequence:

1. Pause writes (acquire global execution lock)
2. fsync WAL
3. Copy storage.dat → snapshot/storage.dat
4. fsync snapshot/storage.dat
5. Copy schemas → snapshot/schemas/
6. fsync snapshot/schemas directory
7. Generate manifest.json
8. fsync manifest.json
9. fsync snapshot directory
10. Release global lock

Any failure aborts snapshot.

Partial snapshots are deleted.

---

## 5. Atomic Visibility

Snapshots become visible only after:

* manifest.json written
* snapshot directory fsynced

Incomplete directories must be ignored on startup.

---

## 6. Integrity Verification

Every snapshot includes:

* CRC32 of storage.dat
* CRC32 of every schema file

During restore or recovery:

* all checksums verified
* any mismatch → FATAL

No auto-repair.

---

## 7. Snapshot Immutability

Once created:

* snapshot files are read-only
* never modified
* never appended

New snapshots always create new directories.

---

## 8. Snapshot Discovery

On startup:

```
<data_dir>/snapshots/
```

is scanned.

Valid snapshots:

* must contain manifest.json
* must pass checksum verification

Newest valid snapshot is selected.

Others remain untouched.

---

## 9. Corruption Policy

If snapshot corruption detected:

* snapshot ignored
* previous snapshot attempted
* if none valid → fallback to WAL-only recovery

All corruption events logged.

---

## 10. Phase-1 Limitations

Snapshots do NOT include:

* indexes
* WAL tail
* runtime state

Those are reconstructed.

---

## 11. Determinism Guarantees

Given identical:

* snapshot directory
* WAL tail
* schemas

AeroDB must recover to identical state.

---

## 12. Authority

This document governs:

* snapshot creation
* snapshot format
* checkpointing input
* backup input
* restore input

Violations are correctness bugs.
