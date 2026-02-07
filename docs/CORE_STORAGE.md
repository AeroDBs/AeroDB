# aerodb — Storage Engine Layout (Phase 0)

## 1. Design Goals

The storage engine exists to provide:

* **Durable persistence** (WAL-backed)
* **Deterministic recovery**
* **Schema-valid data only**
* **Corruption detection, not concealment**
* **Simplicity over cleverness**

It explicitly does **not** optimize for:

* Space efficiency
* Write amplification reduction
* Fast recovery
* Concurrent access

Those are deferred.

---

## 2. On-Disk Directory Layout

Single database instance = single directory.

```
data_dir/
├── MANIFEST
├── LOCK
├── wal/
│   └── wal.log
├── data/
│   └── documents.dat
├── indexes/
│   └── (empty at rest; rebuilt on startup)
└── metadata/
    ├── schemas/
    │   └── schema_<id>.json
    └── state.json
```

### Key Rules

* **No hidden files**
* **No background compaction**
* **Indexes are NOT persisted**
* **Only WAL + document storage are authoritative**

---

## 3. MANIFEST File

### Purpose

Establishes identity and compatibility.

### Contents

* Database UUID
* aerodb version
* Storage format version
* Creation timestamp

### Rules

* Written once at init
* Immutable
* Read at every startup
* Version mismatch = startup failure

---

## 4. LOCK File

### Purpose

Prevent multiple processes from opening the same data directory.

### Rules

* Acquired exclusively on startup
* Released only on clean shutdown
* If lock exists → startup fails

No “force unlock”. Operator intervention required.

---

## 5. Write-Ahead Log (WAL)

### Location

```
wal/wal.log
```

### Role

**Single source of truth for durability.**

Every acknowledged write MUST appear here first.

---

### 5.1 WAL Record Structure

Each WAL record is **append-only** and **self-describing**.

```
+------------------+
| Record Length    |  (u32)
+------------------+
| Record Type      |  (u8)
+------------------+
| Sequence Number  |  (u64)
+------------------+
| Payload          |  (variable)
+------------------+
| Checksum         |  (u32)
+------------------+
```

#### Record Types

* `INSERT`
* `UPDATE`
* `DELETE`

No schema changes in Phase 0.

---

### 5.2 WAL Payload

Payload contains:

* Collection identifier
* Document primary key
* Full document body (post-update)
* Schema version ID

**Rule**: WAL always logs **full document state**, not deltas.

Reason:

* Simpler recovery
* Deterministic replay
* No dependency on previous state

---

### 5.3 WAL Durability Rules

* WAL append → `fsync` → only then acknowledge
* No batching
* No group commit
* No async writes

Violating this breaks `D1` and is forbidden.

---

## 6. Document Storage (`documents.dat`)

### Role

Holds the **canonical persistent state** of all documents.

Indexes derive from this.

---

### 6.1 Storage Model

Append-only record file.

No in-place updates.

```
documents.dat:
[ Record ][ Record ][ Record ] ...
```

Each record represents the **latest version** of a document.

---

### 6.2 Document Record Format

```
+------------------+
| Record Length    | (u32)
+------------------+
| Document ID      | (fixed / variable)
+------------------+
| Schema Version   | (u32 or UUID)
+------------------+
| Tombstone Flag   | (u8)
+------------------+
| Document Payload | (binary / JSON)
+------------------+
| Checksum         | (u32)
+------------------+
```

#### Tombstone Rules

* Deletes write a tombstone record
* Deleted documents remain in storage
* Compaction is **out of scope** for Phase 0

---

### 6.3 Write Rules

* Document record is written **after WAL fsync**
* Storage write failure after WAL fsync:

  * Startup recovery will reapply WAL
  * Operation must not be acknowledged unless storage write completes

---

### 6.4 Read Rules

* Reads scan via index (not storage)
* Storage is accessed only by:

  * Primary key lookup
  * Recovery
  * Index rebuild

Every read validates checksum.

---

## 7. Index Storage Strategy

### Phase 0 Rule

**Indexes are not persisted.**

---

### 7.1 Index Lifecycle

* Indexes exist only in memory
* Built at startup by scanning `documents.dat`
* Updated during normal operation
* Rebuilt fully during recovery

---

### 7.2 Rationale

* WAL stays simple
* No dual-write atomicity problems
* Deterministic recovery
* Index corruption cannot corrupt data

Trade-off: slower startup. Accepted.

---

## 8. Schema Storage

### Location

```
metadata/schemas/schema_<id>.json
```

### Rules

* One file per schema version
* Immutable once written
* Referenced by schema version ID in documents and WAL

If a schema file is missing or corrupted → startup fails.

---

## 9. Metadata State (`state.json`)

### Purpose

Track minimal runtime metadata.

Contents:

* Last clean shutdown marker
* Last WAL sequence number written
* Engine state flags

### Rules

* Written only on clean shutdown
* Never trusted alone
* WAL + storage are authoritative

---

## 10. Crash Recovery Interaction

### Recovery Reads

* `MANIFEST`
* `state.json`
* Entire `wal.log`
* Entire `documents.dat`

### Recovery Writes

* Rewrites `documents.dat` state logically
* Rebuilds all indexes
* Writes clean shutdown marker only at successful end

---

## 11. Corruption Handling

### WAL Corruption

* Any checksum failure → startup abort
* No partial replay

### Storage Corruption

* Any checksum failure on read → operation abort
* During recovery → startup abort

**Downtime > silent corruption**

---

## 12. What Is Explicitly Not Implemented (Phase 0)

* Compaction
* Garbage collection
* WAL truncation
* Checkpointing
* Page cache
* Compression
* Encryption-at-rest
* Partial document updates

All intentionally excluded.

---

## 13. Invariant Mapping

| Invariant | Enforced By                  |
| --------- | ---------------------------- |
| D1        | WAL fsync before ack         |
| D2        | Checksums everywhere         |
| D3        | Schema + checksum validation |
| R1        | WAL-first writes             |
| R2        | Sequential replay            |
| R3        | Explicit recovery outcome    |
| C1        | Full-document writes         |
| K1        | Checksums                    |
| K2        | Halt-on-corruption           |

---

## 14. Summary

The aerodb storage engine is:

* **Append-only**
* **WAL-driven**
* **Checksum-verified**
* **Index-as-derived-state**
* **Deterministic to the bone**

It is boring by design.

And boring storage engines are the ones that survive.

