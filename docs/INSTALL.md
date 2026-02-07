This document explains how to build, initialize, and run AeroDB Phase 0 from source.

AeroDB Phase 0 is a **single-node, deterministic document database** with:

- Mandatory schemas
- WAL-backed durability
- Full crash recovery
- Deterministic query planning and execution

This is infrastructure software, not a toy.

---

## 1. System Requirements

### Operating System

- Linux (recommended)
- macOS (untested but should work)

Windows is not supported in Phase 0.

---

### Rust Toolchain

Minimum:

```
rustc 1.75+
cargo 1.75+
```

Check:

```bash
rustc --version
cargo --version
```

If not installed:

```bash
curl https://sh.rustup.rs -sSf | sh
```

Restart your shell after installation.

---

## 2. Clone the Repository

```bash
git clone <your-repo-url> aerodb
cd aerodb
```

---

## 3. Build AeroDB

Debug build:

```bash
cargo build
```

Release build (recommended):

```bash
cargo build --release
```

Binary location:

```
target/release/aerodb
```

---

## 4. Directory Layout

AeroDB stores all data under `data_dir`.

After initialization:

```
data/
├── wal/
├── data/
├── metadata/
│   └── schemas/
└── clean_shutdown
```

You must provide a config file.

---

## 5. Create Configuration

Create `aerodb.json`:

```json
{
  "data_dir": "./data",
  "wal_sync_mode": "fsync",
  "max_wal_size_bytes": 1073741824,
  "max_memory_bytes": 536870912
}
```

Notes:

* `data_dir` is required
* Other fields are optional
* All values become immutable after first startup

---

## 6. Initialize Database

Run:

```bash
./target/release/aerodb init --config aerodb.json
```

This creates:

```
./data/
```

If this directory already exists, init will fail.

---

## 7. Define a Schema

Schemas must exist before any writes.

Create:

```
data/metadata/schemas/user_v1.json
```

Example:

```json
{
  "schema_id": "user",
  "schema_version": "v1",
  "fields": {
    "_id": { "type": "string" },
    "name": { "type": "string" },
    "age": { "type": "int" }
  },
  "indexes": ["_id", "name", "age"]
}
```

Rules:

* `_id` is mandatory
* All indexed fields must be declared
* Schema versions are immutable

---

## 8. Start AeroDB

Run:

```bash
./target/release/aerodb start --config aerodb.json
```

Startup will:

1. Load config
2. Load schemas
3. Replay WAL
4. Rebuild indexes
5. Verify consistency
6. Enter SERVING state

On success, AeroDB waits for JSON requests on stdin.

---

## 9. Insert a Document

Create `insert.json`:

```json
{
  "op": "insert",
  "schema_id": "user",
  "schema_version": "v1",
  "document": {
    "_id": "1",
    "name": "alice",
    "age": 30
  }
}
```

Send it:

```bash
echo '{"op":"insert","schema_id":"user","schema_version":"v1","document":{"_id":"1","name":"alice","age":30}}' | ./target/release/aerodb query --config aerodb.json
```

Expected response:

```json
{
  "status": "ok",
  "data": []
}
```

---

## 10. Query Documents

Create `query.json`:

```json
{
  "op": "query",
  "schema_id": "user",
  "schema_version": "v1",
  "filter": {
    "age": 30
  },
  "limit": 10
}
```

Run:

```bash
cat query.json | ./target/release/aerodb query --config aerodb.json
```

Example output:

```json
{
  "status": "ok",
  "data": [
    {
      "_id": "1",
      "name": "alice",
      "age": 30
    }
  ]
}
```

---

## 11. Explain a Query

```bash
cat query.json | ./target/release/aerodb explain --config aerodb.json
```

Returns deterministic execution plan.

---

## 12. Crash Recovery Test

You can simulate a crash:

1. Start AeroDB
2. Insert data
3. Kill process (`Ctrl+C` or SIGKILL)
4. Restart

On restart:

* WAL is replayed
* indexes rebuilt
* data is preserved

This is mandatory behavior.

---

## 13. Common Failures

### Missing Schema

```
AERO_RECOVERY_SCHEMA_MISSING
```

→ Schema files not present or mismatched.

---

### WAL / Storage Corruption

```
AERO_DATA_CORRUPTION
```

→ Database halts. Restore from backup.

AeroDB never auto-repairs.

---

### Invalid Config

```
AERO_CONFIG_INVALID
```

→ Fix config and retry.

---

## 14. Phase-0 Limitations

AeroDB Phase 0 does NOT support:

* HTTP server
* joins
* aggregations
* transactions
* replication
* checkpoints
* WAL truncation
* schema migrations

This is expected.

---

## 15. Next Steps

Recommended after installation:

* Read `BOOT.md`
* Read `API_SPEC.md`
* Read `LIFECYCLE.md`

These define system guarantees.
