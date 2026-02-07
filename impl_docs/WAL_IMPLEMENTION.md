# WAL Implementation Walkthrough

## Summary

Implemented the Write-Ahead Log (WAL) subsystem for aerodb Phase 0, fully compliant with governing documents.

## Module Structure

```
src/wal/
├── mod.rs       # Public exports
├── errors.rs    # Error types per ERRORS.md
├── checksum.rs  # CRC32 checksums
├── record.rs    # WAL record format per WAL.md
├── writer.rs    # Append with fsync enforcement
└── reader.rs    # Sequential read with corruption detection
```

---

## Record Format (per WAL.md §87-116)

| Field           | Size     | Description                    |
|-----------------|----------|--------------------------------|
| Record Length   | u32 LE   | Total record size              |
| Record Type     | u8       | 0=INSERT, 1=UPDATE, 2=DELETE   |
| Sequence Number | u64 LE   | Monotonic, starts at 1         |
| Payload         | variable | Full document state            |
| Checksum        | u32 LE   | CRC32 over all except checksum |

---

## Invariants Enforced

| Invariant | Enforcement |
|-----------|-------------|
| D1 | fsync before acknowledgment |
| R1 | WAL precedes storage writes |
| R2 | Sequential deterministic replay |
| K1 | CRC32 checksums on every record |
| K2 | Halt-on-corruption policy (FATAL) |
| C1 | Full-document atomicity |

---

## Error Codes (per ERRORS.md)

| Code | Severity | Invariant |
|------|----------|-----------|
| AERO_WAL_APPEND_FAILED | ERROR | D1 |
| AERO_WAL_FSYNC_FAILED | FATAL | D1 |
| AERO_WAL_CORRUPTION | FATAL | K2 |

---

## Test Results

```
running 39 tests
test wal::checksum::tests::test_checksum_deterministic ... ok
test wal::checksum::tests::test_checksum_detects_single_bit_flip ... ok
test wal::errors::tests::test_error_codes_match_spec ... ok
test wal::errors::tests::test_severity_levels_match_spec ... ok
test wal::record::tests::test_record_roundtrip ... ok
test wal::record::tests::test_checksum_detects_corruption ... ok
test wal::reader::tests::test_corruption_detected_on_checksum_failure ... ok
test wal::reader::tests::test_truncation_detected ... ok
test wal::reader::tests::test_replay_idempotency ... ok
test wal::writer::tests::test_sequence_numbers_increment ... ok
test wal::writer::tests::test_records_are_durable_after_append ... ok
... (39 total, all passed)
```

---

## Key Design Decisions

1. **fsync every write** — No batching, no group commit (per WAL.md §191-198)
2. **Full document state** — No deltas (per WAL.md §130-137)
3. **Zero tolerance corruption** — Any checksum failure is FATAL (per WAL.md §216-229)
4. **Sequence from 1** — Monotonic, never reused (per WAL.md §77-83)
5. **CRC32 checksums** — Using crc32fast for performance
