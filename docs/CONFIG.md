This document defines the **authoritative configuration surface** for AeroDB Phase 0.

It governs:

- Config loader implementation
- CLI startup
- main.rs behavior
- Recovery Manager initialization

If implementation behavior conflicts with this document, the implementation is wrong.

Configuration exists to provide paths and resource bounds — not behavior changes.

Unsafe configurations are rejected.

---

## 1. Principles

AeroDB configuration follows strict rules:

- Minimal surface area
- Immutable after first startup
- Safe-only defaults
- No performance tuning knobs
- No correctness overrides

Configuration may not weaken any invariant.

---

## 2. Configuration File

Format: JSON

Default location:

```

./aerodb.json

```

Alternate path allowed via CLI:

```

aerodb start --config /path/to/aerodb.json

```

---

## 3. Configuration Schema

### Required Fields

```

{
"data_dir": "/absolute/or/relative/path"
}

```

`data_dir` must:

- exist or be creatable
- be writable
- remain immutable after first startup

Changing `data_dir` after initialization is forbidden.

---

### Optional Fields (Phase 0)

```

{
"data_dir": "./data",
"max_wal_size_bytes": 1073741824,
"max_memory_bytes": 536870912,
"wal_sync_mode": "fsync"
}

```

---

## 4. Field Definitions

### data_dir (string, REQUIRED)

Root directory for all AeroDB data.

Subdirectories:

```

<data_dir>/
├── wal/
├── data/
├── metadata/
│   └── schemas/
└── clean_shutdown

```

Rules:

- Created if missing
- Must be writable
- Immutable after first successful startup

Violation → FATAL.

---

### max_wal_size_bytes (integer, OPTIONAL)

Default: `1073741824` (1GB)

Rules:

- Must be > 0
- Immutable after first startup

Phase 0 behavior:

- WAL is NOT truncated
- Value is informational only

---

### max_memory_bytes (integer, OPTIONAL)

Default: `536870912` (512MB)

Rules:

- Must be > 0
- Immutable after first startup

Phase 0 behavior:

- No enforcement
- Reserved for future use

---

### wal_sync_mode (string, OPTIONAL)

Allowed values:

```

"fsync"

```

Any other value → startup failure.

Phase 0 enforces:

- WAL must fsync every write

This field exists only for forward compatibility.

---

## 5. Forbidden Configuration

The following are explicitly rejected:

- Disabling WAL fsync
- Disabling schema validation
- Disabling checksums
- Allowing unbounded queries
- Partial success modes
- Any undocumented fields

Unknown fields → FATAL.

---

## 6. Startup Validation

Startup sequence:

1. Parse config JSON
2. Validate schema
3. Reject unknown keys
4. Validate paths
5. Validate values
6. Persist immutable fields (first startup only)

Any failure → immediate exit.

No files opened before config validation completes.

---

## 7. Immutability Rules

After first successful startup:

These fields are immutable:

- data_dir
- max_wal_size_bytes
- max_memory_bytes
- wal_sync_mode

Changing any → FATAL on next startup.

This prevents silent behavioral drift.

---

## 8. Error Handling

Config errors use:

- AERO_CONFIG_INVALID
- AERO_CONFIG_IMMUTABLE
- AERO_CONFIG_IO_FAILED

All are FATAL.

AeroDB does not attempt recovery from config errors.

---

## 9. Determinism

Given identical config file and filesystem:

- AeroDB must initialize identically
- Paths resolved deterministically
- Defaults applied deterministically

No environment-dependent behavior allowed.

---

## 10. Phase-0 Limitations

Configuration does NOT support:

- logging levels
- thread counts
- buffer sizes
- cache tuning
- feature flags

These belong to Phase 1+.

---

## 11. Authority

This document governs:

- Config loader
- CLI startup
- main.rs initialization
- Recovery Manager initialization

Violations of this contract are correctness bugs.
