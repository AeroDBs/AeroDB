# AeroDB Demo Functions Analysis

## Executive Summary

This document catalogs all demo, example, sample, and test utility functions found in the AeroDB codebase. These functions serve as:
- **Sample data generators** for testing
- **Test fixtures** for unit and integration tests
- **Helper functions** to create test environments
- **Utility builders** for test scenarios

## Categories of Demo Functions

### 1. Sample Data Generators

These direct "sample" functions create example data for testing and demonstrations:

| Function | File | Purpose |
|----------|------|---------|
| `sample_payload()` | `src/wal/record.rs:817` | Creates sample WAL payload with user data |
| `sample_payload()` | `src/storage/record.rs:293` | Creates sample storage payload with user data |
| `sample_schema()` | `src/schema/types.rs:205` | Creates sample schema with _id, name, age fields |
| `sample_schema()` | `src/schema/loader.rs:218` | Creates sample user schema with _id, name fields |

### 2. Test Configuration Creators (`create_test_*`)

These functions generate test configurations and test data:

#### Backup Module
- `create_test_config(enabled, interval_hours)` - `src/backup/scheduler.rs:115`
- `create_test_config(backup_dir)` - `src/backup/manager.rs:466`

#### WAL (Write-Ahead Log) Module
- `create_test_payload(doc_id)` - `src/wal/writer.rs:317`
- `create_test_payload(doc_id)` - `src/wal/reader.rs:316`

#### Storage Module
- `create_test_payload(doc_id)` - `src/storage/writer.rs:241`
- `create_test_payload(doc_id)` - `src/storage/reader.rs:257`

#### REST API Module
- `create_test_handler()` - `src/rest_api/handler.rs:334`
- `create_test_server()` - `src/rest_api/server.rs:156`

#### Replication Module
- `create_test_record()` - `src/replication/wal_receiver.rs:295`
- `create_test_metadata()` - `src/replication/snapshot_transfer.rs:321`

#### Restore Module
- `create_test_archive(archive_path)` - `src/restore/extractor.rs:115`
- `create_test_extracted_backup(dir)` - `src/restore/restorer.rs:282`
- `create_test_backup_archive(archive_path)` - `src/restore/mod.rs:185`

#### Snapshot Module
- `create_test_manifest()` - `src/snapshot/manifest.rs:222`

#### Authentication Module
- `create_test_service()` - `src/auth/magic_link.rs:391`
- `create_test_manager()` - `src/auth/jwt.rs:182`
- `create_test_user()` - `src/auth/jwt.rs:191`
- `create_test_service()` - `src/auth/oauth.rs:650`
- `create_test_service()` - `src/auth/api.rs:451`

#### File Storage Module
- `create_test_service()` - `src/file_storage/file.rs:231`
- `create_test_object(bucket_id, path)` - `src/file_storage/metadata.rs:355`

#### Realtime Module
- `create_test_rls()` - `src/realtime/subscription.rs:375`

#### Functions Module
- `create_test_function()` - `src/functions/runtime.rs:334`

#### Checkpoint Module
- `create_test_payload(doc_id)` - `src/checkpoint/mod.rs:211`
- `create_test_payload(doc_id)` - `src/checkpoint/coordinator.rs:193`

#### Core Module
- `create_test_schema_loader(path)` - `src/core/adapter.rs:325`

### 3. Test Environment Setup Functions (`setup_test_*`)

These functions create complete test environments with multiple components:

| Function | File | Purpose |
|----------|------|---------|
| `setup_test_environment()` | `src/snapshot/mod.rs:218` | Returns (TempDir, PathBuf, PathBuf, WalWriter) |
| `setup_test_environment()` | `src/snapshot/creator.rs:382` | Returns (TempDir, PathBuf, PathBuf) |
| `setup_test_environment()` | `src/checkpoint/mod.rs:184` | Returns (TempDir, PathBuf, PathBuf, WalWriter) |
| `setup_test_environment()` | `src/checkpoint/coordinator.rs:166` | Returns (TempDir, PathBuf, PathBuf, WalWriter) |
| `setup_loader()` | `src/schema/validator.rs:346` | Returns (TempDir, SchemaLoader) |
| `setup_handler()` | `src/rest_api/pipeline_handler.rs:263` | Returns (PipelineRestHandler, Runtime) |
| `setup_test_env()` | `src/api/handler.rs:455` | Sets up API test environment |

### 4. Data Builder/Maker Functions (`make_*`, `build_*`)

These functions construct test data objects:

#### Recovery Module
- `make_record(id, schema_id, version, offset)` - `src/recovery/verifier.rs:194`
- `make_tombstone(id, offset)` - `src/recovery/verifier.rs:204`
- `make_insert_record(seq, id)` - `src/recovery/startup.rs:278`
- `make_insert_record(seq, id)` - `src/recovery/replay.rs:193`
- `make_delete_record(seq, id)` - `src/recovery/replay.rs:200`

#### Promotion Module
- `make_replica_context()` - `src/promotion/validator.rs:194`
- `make_manager()` - `src/promotion/crash_tests.rs:30`

#### MVCC Module
- `make_version(key, commit)` - `src/mvcc/gc.rs:326`

#### Executor Module
- `make_doc(id, age)` - `src/executor/sorter.rs:87`
- `make_doc_with_name(id, name)` - `src/executor/sorter.rs:128`
- `make_record(id, schema_id, version, body)` - `src/executor/executor.rs:297`

#### Planner Module
- `make_indexes(fields)` - `src/planner/bounds.rs:112`

#### Schema Module
- `make_path(prefix, field)` - `src/schema/validator.rs:316`

#### Storage Module
- `build_offset_index(storage_path)` - `src/storage/writer.rs:92`
- `build_document_map()` - `src/storage/reader.rs:227`

#### REST API Module
- `build_context()` - `src/rest_api/unified_api.rs:185`
- `build_rls_context(ctx)` - `src/rest_api/unified_api.rs:212`
- `create_posts_schema()` - `src/rest_api/generator.rs:245`
- `create_facade()` - `src/rest_api/database.rs:338`

### 5. Other Test Utilities

Additional test helper functions:

| Function | File | Purpose |
|----------|------|---------|
| `create_backup()` | `src/backup/manager.rs:88` | Public API to create backups |
| `create_tenant()` | `src/control_plane/provisioning.rs:54` | Creates tenant in control plane |
| `create_snapshot()` | `src/snapshot/mod.rs:148` | Creates snapshot |
| `create_mvcc_snapshot()` | `src/snapshot/mod.rs:190` | Creates MVCC snapshot |
| `create_snapshot_impl()` | `src/snapshot/creator.rs:165` | Snapshot implementation |
| `create_snapshot_contents()` | `src/snapshot/creator.rs:209` | Creates snapshot contents |
| `create_mvcc_snapshot_impl()` | `src/snapshot/creator.rs:289` | MVCC snapshot implementation |
| `create_temp_restore_dir(data_dir)` | `src/restore/extractor.rs:18` | Creates temp restore directory |
| `create_tar_archive(source_dir, archive_path)` | `src/backup/manager.rs:347` | Creates tar archive |
| `create_valid_backup_structure(dir)` | `src/restore/validator.rs:214` | Creates valid backup structure |
| `create_existing_data_dir(data_dir)` | `src/restore/mod.rs:230` | Creates existing data directory |

## Module Distribution

The demo/test functions are distributed across modules as follows:

| Module | Function Count | Primary Purpose |
|--------|----------------|-----------------|
| **Auth** | 5 | OAuth, JWT, Magic Link, Session testing |
| **Backup/Restore** | 8 | Backup creation, restoration testing |
| **Checkpoint** | 4 | Checkpoint creation and testing |
| **File Storage** | 2 | File service and object testing |
| **Functions** | 1 | Serverless function testing |
| **MVCC** | 1 | Version control testing |
| **Planner** | 1 | Query planning testing |
| **Promotion** | 2 | Failover and promotion testing |
| **Realtime** | 1 | RLS context testing |
| **Recovery** | 5 | WAL replay, verification testing |
| **Replication** | 2 | WAL receiver, snapshot transfer testing |
| **REST API** | 6 | Handler, server, database facade testing |
| **Schema** | 4 | Schema creation and validation testing |
| **Snapshot** | 7 | Snapshot creation and management testing |
| **Storage** | 4 | Storage payload and indexing testing |
| **WAL** | 4 | WAL payload creation and testing |
| **Core** | 1 | Schema loader testing |
| **Executor** | 3 | Document sorting and execution testing |

**Total:** ~61 distinct demo/test utility functions

## Usage Patterns

### Pattern 1: Sample Data for Documentation/Tests
```rust
// Creates simple, reusable sample data
fn sample_schema() -> Schema {
    let mut fields = HashMap::new();
    fields.insert("_id".into(), FieldDef::required_string());
    fields.insert("name".into(), FieldDef::required_string());
    Schema::new("users", "v1", fields)
}
```

### Pattern 2: Test Configuration Builders
```rust
// Generates test configurations
fn create_test_config(enabled: bool, interval_hours: u32) -> BackupConfig {
    BackupConfig {
        enabled,
        interval_hours,
        retention_days: 7,
        // ...
    }
}
```

### Pattern 3: Test Environment Setup
```rust
// Sets up complete test environments
fn setup_test_environment() -> (TempDir, PathBuf, PathBuf, WalWriter) {
    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("wal");
    let storage_path = temp_dir.path().join("storage");
    let writer = WalWriter::new(&wal_path).unwrap();
    (temp_dir, wal_path, storage_path, writer)
}
```

### Pattern 4: Test Data Generators
```rust
// Generates specific test data
fn create_test_payload(doc_id: &str) -> WalPayload {
    WalPayload::new(
        "users",
        doc_id,
        "user_schema",
        "v1",
        b"{\"name\": \"Test\"}".to_vec(),
    )
}
```

## Key Observations

1. **No Dedicated Examples Directory**: AeroDB doesn't have a separate `examples/` directory. All demo functions are embedded within test modules.

2. **Test-Driven Design**: The demo functions are primarily designed for internal testing rather than external documentation or tutorials.

3. **Consistent Naming**: Functions follow consistent naming patterns:
   - `sample_*` for simple data examples
   - `create_test_*` for test fixture creation
   - `setup_test_*` for environment setup
   - `make_*` for simple data builders
   - `build_*` for complex data construction

4. **Module Coverage**: Nearly every major module has associated test utilities, demonstrating comprehensive test coverage.

5. **Integration Testing Focus**: Many functions create complete test environments (like `setup_test_environment`), indicating a focus on integration testing.

## Recommendations

1. **Create Examples Directory**: Consider creating a dedicated `examples/` directory with standalone examples showing common use cases.

2. **Documentation Examples**: Some `sample_*` functions could be promoted to documentation examples to help new users understand the API.

3. **Example Applications**: Build small demo applications (e.g., a todo app, blog backend) using AeroDB to showcase real-world usage.

4. **API Usage Tutorials**: Create tutorials that use these test utilities as starting points for explaining API usage.

## Conclusion

AeroDB contains **61+ demo/test utility functions** spread across 17 major modules. These functions primarily serve internal testing purposes but could be leveraged for external documentation and examples. The functions demonstrate:

- Comprehensive test coverage across all modules
- Consistent design patterns
- Strong focus on integration testing
- Well-structured test utilities

While these functions are valuable for development and testing, there's an opportunity to create more user-facing examples and documentation to help new users understand and adopt AeroDB.
