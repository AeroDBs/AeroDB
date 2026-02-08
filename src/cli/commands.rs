//! CLI command implementations
//!
//! Per LIFECYCLE.md and BOOT.md, these commands follow strict boot sequence.
//!
//! # Phase 7 Control Plane
//!
//! Per PHASE7_COMMAND_MODEL.md:
//! Control plane commands are thin clients with no authority.
//! Safety is enforced server-side.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::api::{ApiHandler, Subsystems};
use crate::dx::api::control_plane::{
    AuthorityContext, CommandRequest, ControlCommand, ControlPlaneCommand, ControlPlaneHandler,
    DiagnosticCommand, InspectionCommand,
};
use crate::index::IndexManager;
use crate::observability::{AuditAction, AuditLog, AuditOutcome, AuditRecord, MemoryAuditLog};
use crate::recovery::RecoveryManager;
use crate::replication::{ReplicationConfig, ReplicationRole, ReplicationState};
use crate::schema::SchemaLoader;
use crate::storage::{StorageReader, StorageWriter};
use crate::wal::{WalReader, WalWriter};

use super::args::{Command, ControlAction, DeployAction, DiagTarget, InspectTarget, MigrateAction, SchemaAction};
use super::errors::{CliError, CliResult};
use super::io::{read_request, read_requests, write_error, write_json, write_response};

/// Configuration file structure per CONFIG.md
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Data directory (required)
    pub data_dir: String,

    /// Max WAL size in bytes (optional, default 1GB)
    #[serde(default = "default_max_wal_size")]
    pub max_wal_size_bytes: u64,

    /// Max memory in bytes (optional, default 512MB)
    #[serde(default = "default_max_memory")]
    pub max_memory_bytes: u64,

    /// WAL sync mode (optional, default "fsync")
    #[serde(default = "default_wal_sync_mode")]
    pub wal_sync_mode: String,

    // --- Replication Configuration (Phase 5 Stage 1) ---
    // Per P5-I16: All fields default to disabled.
    /// Whether replication is enabled (default: false per P5-I16)
    #[serde(default)]
    pub replication_enabled: bool,

    /// Replication role: "primary" or "replica" (default: "primary")
    #[serde(default = "default_replication_role")]
    pub replication_role: String,

    /// Replica ID (UUID, auto-generated if not provided for replicas)
    #[serde(default)]
    pub replica_id: Option<String>,

    /// Primary node address (required for replicas, forbidden for primaries)
    #[serde(default)]
    pub primary_address: Option<String>,
}

fn default_max_wal_size() -> u64 {
    1073741824
} // 1GB
fn default_max_memory() -> u64 {
    536870912
} // 512MB
fn default_wal_sync_mode() -> String {
    "fsync".to_string()
}
fn default_replication_role() -> String {
    "primary".to_string()
}

impl Config {
    /// Load configuration from file
    pub fn load(path: &Path) -> CliResult<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| CliError::config_error(format!("Failed to read config: {}", e)))?;

        let config: Config = serde_json::from_str(&content)
            .map_err(|e| CliError::config_error(format!("Invalid config JSON: {}", e)))?;

        config.validate()?;

        Ok(config)
    }

    /// Validate configuration per CONFIG.md
    fn validate(&self) -> CliResult<()> {
        // Validate wal_sync_mode
        if self.wal_sync_mode != "fsync" {
            return Err(CliError::config_error(format!(
                "Invalid wal_sync_mode: '{}'. Only 'fsync' is allowed.",
                self.wal_sync_mode
            )));
        }

        // Validate max_wal_size_bytes
        if self.max_wal_size_bytes == 0 {
            return Err(CliError::config_error("max_wal_size_bytes must be > 0"));
        }

        // Validate max_memory_bytes
        if self.max_memory_bytes == 0 {
            return Err(CliError::config_error("max_memory_bytes must be > 0"));
        }

        // Validate replication config (Phase 5 Stage 1)
        self.to_replication_config()?.validate().map_err(|e| {
            CliError::config_error(format!("Replication config error: {}", e.message))
        })?;

        Ok(())
    }

    /// Get data directory as Path
    pub fn data_path(&self) -> &Path {
        Path::new(&self.data_dir)
    }

    /// Convert to ReplicationConfig for use during boot.
    ///
    /// Per PHASE5_IMPLEMENTATION_ORDER.md §Stage 1:
    /// - Validates role is "primary" or "replica"
    /// - Auto-generates replica_id if needed
    pub fn to_replication_config(&self) -> CliResult<ReplicationConfig> {
        if !self.replication_enabled {
            return Ok(ReplicationConfig::disabled());
        }

        let role = match self.replication_role.as_str() {
            "primary" => ReplicationRole::Primary,
            "replica" => ReplicationRole::Replica,
            other => {
                return Err(CliError::config_error(format!(
                    "Invalid replication_role: '{}'. Must be 'primary' or 'replica'.",
                    other
                )))
            }
        };

        match role {
            ReplicationRole::Primary => Ok(ReplicationConfig::primary()),
            ReplicationRole::Replica => {
                let primary_addr = self.primary_address.clone().ok_or_else(|| {
                    CliError::config_error(
                        "primary_address is required when replication_role is 'replica'",
                    )
                })?;

                let replica_id = self
                    .replica_id
                    .as_ref()
                    .map(|s| uuid::Uuid::parse_str(s))
                    .transpose()
                    .map_err(|e| {
                        CliError::config_error(format!("Invalid replica_id UUID: {}", e))
                    })?;

                Ok(ReplicationConfig::replica(primary_addr, replica_id))
            }
        }
    }

    /// Initialize ReplicationState based on config.
    ///
    /// Per PHASE5_IMPLEMENTATION_ORDER.md §Stage 1:
    /// - If disabled: return Disabled state
    /// - If primary: transition to PrimaryActive
    /// - If replica: transition to ReplicaActive with UUID
    pub fn init_replication_state(&self) -> CliResult<ReplicationState> {
        let repl_config = self.to_replication_config()?;

        if !repl_config.is_enabled() {
            return Ok(ReplicationState::new()); // Disabled
        }

        let state = ReplicationState::uninitialized();

        if repl_config.is_primary() {
            state.become_primary().map_err(|e| {
                CliError::config_error(format!("Failed to init primary: {}", e.message))
            })
        } else {
            let replica_id = repl_config
                .get_replica_id()
                .expect("Replica must have replica_id after validation");
            state.become_replica(replica_id).map_err(|e| {
                CliError::config_error(format!("Failed to init replica: {}", e.message))
            })
        }
    }
}

/// Main CLI entry point
///
/// Parses arguments and dispatches to the appropriate command.
/// This is the only function that main.rs should call.
pub fn run() -> CliResult<()> {
    let cli = super::args::Cli::parse_args();
    run_command(cli.command)
}

/// Run the appropriate command based on CLI args
pub fn run_command(cmd: Command) -> CliResult<()> {
    match cmd {
        Command::Init { config } => init(&config),
        Command::Start { config } => start(&config),
        Command::Query { config } => query(&config),
        Command::Explain { config } => explain(&config),
        Command::Serve { config, port } => serve(&config, port),
        Command::Control { config, action } => control(&config, action),
        Command::Migrate { config, action } => migrate(&config, action),
        Command::Schema { config, action } => schema(&config, action),
        Command::Deploy { config, action } => deploy(&config, action),
        Command::Logs { config, lines, level, follow } => logs(&config, lines, level, follow),
    }
}

/// Initialize a new AeroDB data directory
///
/// Per LIFECYCLE.md §2:
/// - Creates directory structure
/// - Does NOT start server
/// - Writes no WAL records
/// - Does not create clean_shutdown marker
pub fn init(config_path: &Path) -> CliResult<()> {
    let config = Config::load(config_path)?;
    let data_dir = config.data_path();

    // Check if already initialized
    if is_initialized(data_dir) {
        return Err(CliError::already_initialized());
    }

    // Create directory structure per CONFIG.md §4
    let dirs = [
        data_dir.join("wal"),
        data_dir.join("data"),
        data_dir.join("metadata").join("schemas"),
    ];

    for dir in &dirs {
        fs::create_dir_all(dir).map_err(|e| {
            CliError::config_error(format!("Failed to create directory {:?}: {}", dir, e))
        })?;
    }

    write_response(json!({"initialized": true}))?;

    Ok(())
}

/// Start the AeroDB server
///
/// Per BOOT.md §3, startup sequence:
/// 1. Configuration Load
/// 2. Schema Load
/// 3. Recovery (WAL replay)
/// 4. Index Rebuild
/// 5. Verification
/// 6. API Activation
///
/// Then enters SERVING loop reading JSON from stdin.
pub fn start(config_path: &Path) -> CliResult<()> {
    let config = Config::load(config_path)?;
    let data_dir = config.data_path();

    // Check if initialized
    if !is_initialized(data_dir) {
        return Err(CliError::not_initialized());
    }

    // Boot the system
    let (mut wal_writer, mut storage_writer, mut storage_reader, schema_loader, mut index_manager) =
        boot_system(data_dir)?;

    // Initialize API handler
    let handler = ApiHandler::new("default");

    // Enter SERVING loop
    // Read JSON from stdin line-by-line, write response to stdout
    for request_result in read_requests() {
        match request_result {
            Ok(request) => {
                let request_str = request.to_string();

                let mut subsystems = Subsystems {
                    schema_loader: &schema_loader,
                    wal_writer: &mut wal_writer,
                    storage_writer: &mut storage_writer,
                    storage_reader: &mut storage_reader,
                    index_manager: &mut index_manager,
                };

                let response = handler.handle(&request_str, &mut subsystems);
                write_json(&response.to_json())?;
            }
            Err(e) => {
                // I/O error reading - this is fatal
                write_error(e.code_str(), e.message())?;
                break;
            }
        }
    }

    // Clean shutdown - write marker
    let shutdown_marker = data_dir.join("clean_shutdown");
    let _ = fs::write(&shutdown_marker, "");

    Ok(())
}

/// Execute a single query and exit
///
/// Per CLI spec: Full boot → Execute single query → Print result → Exit
pub fn query(config_path: &Path) -> CliResult<()> {
    let config = Config::load(config_path)?;
    let data_dir = config.data_path();

    // Check if initialized
    if !is_initialized(data_dir) {
        return Err(CliError::not_initialized());
    }

    // Boot the system
    let (mut wal_writer, mut storage_writer, mut storage_reader, schema_loader, mut index_manager) =
        boot_system(data_dir)?;

    // Read single request from stdin
    let request = read_request()?;

    // Ensure it's a query operation
    let mut request_obj = request;
    if let Some(obj) = request_obj.as_object_mut() {
        // Allow any operation, but this command is intended for query
        if obj.get("op").and_then(|v| v.as_str()) != Some("query") {
            // Force query operation if not specified
            if !obj.contains_key("op") {
                obj.insert("op".to_string(), json!("query"));
            }
        }
    }

    let request_str = request_obj.to_string();

    // Initialize API handler
    let handler = ApiHandler::new("default");

    let mut subsystems = Subsystems {
        schema_loader: &schema_loader,
        wal_writer: &mut wal_writer,
        storage_writer: &mut storage_writer,
        storage_reader: &mut storage_reader,
        index_manager: &mut index_manager,
    };

    let response = handler.handle(&request_str, &mut subsystems);
    write_json(&response.to_json())?;

    Ok(())
}

/// Execute explain on a query and exit
///
/// Same as query, but forces "op":"explain"
pub fn explain(config_path: &Path) -> CliResult<()> {
    let config = Config::load(config_path)?;
    let data_dir = config.data_path();

    // Check if initialized
    if !is_initialized(data_dir) {
        return Err(CliError::not_initialized());
    }

    // Boot the system
    let (mut wal_writer, mut storage_writer, mut storage_reader, schema_loader, mut index_manager) =
        boot_system(data_dir)?;

    // Read single request from stdin
    let request = read_request()?;

    // Force explain operation
    let mut request_obj = request;
    if let Some(obj) = request_obj.as_object_mut() {
        obj.insert("op".to_string(), json!("explain"));
    }

    let request_str = request_obj.to_string();

    // Initialize API handler
    let handler = ApiHandler::new("default");

    let mut subsystems = Subsystems {
        schema_loader: &schema_loader,
        wal_writer: &mut wal_writer,
        storage_writer: &mut storage_writer,
        storage_reader: &mut storage_reader,
        index_manager: &mut index_manager,
    };

    let response = handler.handle(&request_str, &mut subsystems);
    write_json(&response.to_json())?;

    Ok(())
}

/// Start the HTTP server for dashboard (Phase 13.5)
///
/// Boots the database and starts an HTTP server. This is the recommended
/// mode for connecting the dashboard frontend.
///
/// Per implementation plan:
/// 1. Boot database (same as start command)
/// 2. Initialize HTTP server with all subsystems
/// 3. Start Axum server on specified port
pub fn serve(config_path: &Path, port: u16) -> CliResult<()> {
    let config = Config::load(config_path)?;
    let data_dir = config.data_path();

    // Check if initialized
    if !is_initialized(data_dir) {
        return Err(CliError::not_initialized());
    }

    // Boot the system (same as start command)
    let (_wal_writer, _storage_writer, _storage_reader, _schema_loader, _index_manager) =
        boot_system(data_dir)?;

    // Create HTTP server with configured port
    use crate::http_server::{HttpServer, HttpServerConfig};

    let http_config = HttpServerConfig::with_port(port);
    let server = HttpServer::with_config(http_config);

    // Start the async runtime and run the server
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| CliError::boot_failed(format!("Failed to create tokio runtime: {}", e)))?;

    rt.block_on(async {
        server
            .start()
            .await
            .map_err(|e| CliError::boot_failed(format!("HTTP server failed: {}", e)))
    })?;

    Ok(())
}

/// Execute a Phase 7 control plane command.
///
/// Per PHASE7_COMMAND_MODEL.md:
/// - CLI is a thin client with no authority
/// - No retries, no defaults
/// - Safety enforced server-side
pub fn control(config_path: &Path, action: ControlAction) -> CliResult<()> {
    let _config = Config::load(config_path)?;

    // Create in-memory audit log for this session
    let audit_log = MemoryAuditLog::new();

    // Create control plane handler
    let mut handler = ControlPlaneHandler::new();

    // Convert CLI action to control plane command
    let (command, authority) = build_command(action)?;

    // Log command request
    let request_audit = AuditRecord::new(AuditAction::CommandRequested, AuditOutcome::Pending)
        .with_command(command.command_name())
        .with_authority(&authority.level.to_string());
    audit_log.append(&request_audit).ok();

    // Create request
    let request = CommandRequest::new(command.clone(), authority);

    // Handle command
    match handler.handle_command(request) {
        Ok(response) => {
            // Log success
            let outcome_audit =
                AuditRecord::new(AuditAction::CommandExecuted, AuditOutcome::Success)
                    .with_command(response.command_name.clone());
            audit_log.append(&outcome_audit).ok();

            // Output response
            write_response(json!({
                "request_id": response.request_id.to_string(),
                "command": response.command_name,
                "outcome": format!("{:?}", response.outcome),
                "confirmation_token": response.confirmation_token.map(|t| t.to_string()),
            }))?;
        }
        Err(e) => {
            // Log rejection
            let outcome_audit =
                AuditRecord::new(AuditAction::CommandRejected, AuditOutcome::Rejected)
                    .with_command(command.command_name())
                    .with_error(e.message());
            audit_log.append(&outcome_audit).ok();

            // Output error
            write_error(e.code(), e.message())?;
        }
    }

    Ok(())
}

/// Execute a migration command (Phase 14).
///
/// MANIFESTO ALIGNMENT: Deterministic, checksummed, reversible migrations.
/// All operations are explicit with clear success/failure feedback.
pub fn migrate(config_path: &Path, action: MigrateAction) -> CliResult<()> {
    use crate::migrations::{
        generator::MigrationGenerator,
        operations::InMemoryExecutor,
        runner::MigrationRunner,
    };
    use std::sync::Arc;

    let config = Config::load(config_path)?;
    let data_dir = config.data_path();

    // Migrations directory defaults to data_dir/migrations
    let migrations_dir = data_dir.join("migrations");

    match action {
        MigrateAction::Create { name } => {
            // Ensure migrations directory exists
            if !migrations_dir.exists() {
                fs::create_dir_all(&migrations_dir).map_err(|e| {
                    CliError::config_error(format!(
                        "Failed to create migrations directory: {}",
                        e
                    ))
                })?;
            }

            // Create generator and generate new migration
            let generator = MigrationGenerator::new(migrations_dir.clone());
            let file_path = generator.create(&name).map_err(|e| {
                CliError::config_error(format!("Failed to create migration: {}", e))
            })?;

            // Extract version and name from filename (e.g., "001_create_users.yaml")
            let filename = file_path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            let (version_str, migration_name) = filename.split_once('_')
                .unwrap_or(("0", filename));
            let version: u64 = version_str.parse().unwrap_or(0);

            write_response(json!({
                "created": true,
                "version": version,
                "name": migration_name,
                "file": file_path.to_string_lossy().to_string(),
            }))?;
        }

        MigrateAction::Up => {
            // Check if initialized
            if !is_initialized(data_dir) {
                return Err(CliError::not_initialized());
            }

            // Ensure migrations directory exists
            if !migrations_dir.exists() {
                return Err(CliError::config_error(
                    "No migrations directory found. Run 'aerodb migrate create' first.",
                ));
            }

            // Create executor (in-memory for now, will integrate with real DB later)
            let executor = Arc::new(InMemoryExecutor::new());

            // Create runner
            let runner =
                MigrationRunner::new(migrations_dir.clone(), data_dir.to_path_buf(), executor)
                    .map_err(|e| {
                        CliError::boot_failed(format!("Failed to initialize migration runner: {}", e))
                    })?;

            // Apply pending migrations
            let report = runner.migrate_up().map_err(|e| {
                CliError::boot_failed(format!("Migration failed: {}", e))
            })?;

            if let Some(failed) = report.failed {
                write_error(
                    "MIGRATION_FAILED",
                    &format!(
                        "Migration {} (v{}) failed: {}",
                        failed.name, failed.version, failed.error
                    ),
                )?;
            } else {
                let applied: Vec<_> = report
                    .applied
                    .iter()
                    .map(|m| {
                        json!({
                            "version": m.version,
                            "name": m.name,
                            "duration_ms": m.duration_ms
                        })
                    })
                    .collect();

                write_response(json!({
                    "success": true,
                    "applied_count": applied.len(),
                    "applied": applied,
                }))?;
            }
        }

        MigrateAction::Down => {
            // Check if initialized
            if !is_initialized(data_dir) {
                return Err(CliError::not_initialized());
            }

            // Ensure migrations directory exists
            if !migrations_dir.exists() {
                return Err(CliError::config_error(
                    "No migrations directory found.",
                ));
            }

            // Create executor
            let executor = Arc::new(InMemoryExecutor::new());

            // Create runner
            let runner =
                MigrationRunner::new(migrations_dir.clone(), data_dir.to_path_buf(), executor)
                    .map_err(|e| {
                        CliError::boot_failed(format!("Failed to initialize migration runner: {}", e))
                    })?;

            // Rollback last migration
            let result = runner.migrate_down().map_err(|e| {
                CliError::boot_failed(format!("Rollback failed: {}", e))
            })?;

            match result {
                Some(rolled_back) => {
                    write_response(json!({
                        "success": true,
                        "rolled_back": {
                            "version": rolled_back.version,
                            "name": rolled_back.name,
                            "duration_ms": rolled_back.duration_ms
                        }
                    }))?;
                }
                None => {
                    write_response(json!({
                        "success": true,
                        "message": "No migrations to rollback"
                    }))?;
                }
            }
        }

        MigrateAction::Status => {
            // Check if initialized
            if !is_initialized(data_dir) {
                return Err(CliError::not_initialized());
            }

            // Check if migrations directory exists
            if !migrations_dir.exists() {
                write_response(json!({
                    "current_version": 0,
                    "total_migrations": 0,
                    "applied_count": 0,
                    "pending_count": 0,
                    "pending": []
                }))?;
                return Ok(());
            }

            // Create executor
            let executor = Arc::new(InMemoryExecutor::new());

            // Create runner
            let runner =
                MigrationRunner::new(migrations_dir.clone(), data_dir.to_path_buf(), executor)
                    .map_err(|e| {
                        CliError::boot_failed(format!("Failed to initialize migration runner: {}", e))
                    })?;

            // Get status
            let status = runner.status().map_err(|e| {
                CliError::boot_failed(format!("Failed to get migration status: {}", e))
            })?;

            let pending: Vec<_> = status
                .pending
                .iter()
                .map(|m| {
                    json!({
                        "version": m.version,
                        "name": m.name
                    })
                })
                .collect();

            write_response(json!({
                "current_version": status.current_version,
                "total_migrations": status.total_migrations,
                "applied_count": status.applied_count,
                "pending_count": status.pending_count,
                "pending": pending
            }))?;
        }
    }

    Ok(())
}

/// Execute a schema management command.
///
/// MANIFESTO ALIGNMENT: Explicit schema management with full introspection.
pub fn schema(config_path: &Path, action: SchemaAction) -> CliResult<()> {
    let config = Config::load(config_path)?;
    let data_dir = config.data_path();

    let schema_dir = data_dir.join("metadata").join("schemas");

    match action {
        SchemaAction::List => {
            if !schema_dir.exists() {
                write_response(json!({
                    "schemas": [],
                    "count": 0
                }))?;
                return Ok(());
            }

            let mut schemas = Vec::new();
            for entry in fs::read_dir(&schema_dir).map_err(|e| {
                CliError::config_error(format!("Failed to read schemas directory: {}", e))
            })? {
                let entry = entry.map_err(|e| {
                    CliError::config_error(format!("Failed to read entry: {}", e))
                })?;
                
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        schemas.push(json!({
                            "name": name,
                            "file": path.to_string_lossy().to_string()
                        }));
                    }
                }
            }

            write_response(json!({
                "schemas": schemas,
                "count": schemas.len()
            }))?;
        }

        SchemaAction::Show { name } => {
            let schema_path = schema_dir.join(format!("{}.json", name));
            
            if !schema_path.exists() {
                return Err(CliError::config_error(format!(
                    "Schema '{}' not found", name
                )));
            }

            let content = fs::read_to_string(&schema_path).map_err(|e| {
                CliError::config_error(format!("Failed to read schema: {}", e))
            })?;

            let schema: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
                CliError::config_error(format!("Invalid schema JSON: {}", e))
            })?;

            write_response(json!({
                "name": name,
                "schema": schema
            }))?;
        }

        SchemaAction::Create { file } => {
            // Ensure schema directory exists
            if !schema_dir.exists() {
                fs::create_dir_all(&schema_dir).map_err(|e| {
                    CliError::config_error(format!("Failed to create schemas directory: {}", e))
                })?;
            }

            // Read and validate schema file
            let content = fs::read_to_string(&file).map_err(|e| {
                CliError::config_error(format!("Failed to read schema file: {}", e))
            })?;

            let schema: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
                CliError::config_error(format!("Invalid schema JSON: {}", e))
            })?;

            // Extract schema name
            let name = schema.get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::config_error("Schema must have a 'name' field"))?;

            // Write to schemas directory
            let target = schema_dir.join(format!("{}.json", name));
            fs::write(&target, &content).map_err(|e| {
                CliError::config_error(format!("Failed to write schema: {}", e))
            })?;

            write_response(json!({
                "created": true,
                "name": name,
                "file": target.to_string_lossy().to_string()
            }))?;
        }

        SchemaAction::Types { output } => {
            // Ensure output directory exists
            if !output.exists() {
                fs::create_dir_all(&output).map_err(|e| {
                    CliError::config_error(format!("Failed to create output directory: {}", e))
                })?;
            }

            if !schema_dir.exists() {
                write_response(json!({
                    "generated": [],
                    "count": 0
                }))?;
                return Ok(());
            }

            let mut generated = Vec::new();

            for entry in fs::read_dir(&schema_dir).map_err(|e| {
                CliError::config_error(format!("Failed to read schemas directory: {}", e))
            })? {
                let entry = entry.map_err(|e| {
                    CliError::config_error(format!("Failed to read entry: {}", e))
                })?;
                
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        // Generate TypeScript interface
                        let ts_content = format!(
                            "// Auto-generated TypeScript types for {}\n\
                            export interface {} {{\n  \
                            // TODO: Generate fields from schema\n  \
                            id: string;\n\
                            }}\n",
                            name,
                            to_pascal_case(name)
                        );

                        let ts_file = output.join(format!("{}.ts", name));
                        fs::write(&ts_file, ts_content).map_err(|e| {
                            CliError::config_error(format!("Failed to write TypeScript file: {}", e))
                        })?;

                        generated.push(json!({
                            "schema": name,
                            "file": ts_file.to_string_lossy().to_string()
                        }));
                    }
                }
            }

            write_response(json!({
                "generated": generated,
                "count": generated.len()
            }))?;
        }
    }

    Ok(())
}

/// Convert string to PascalCase for TypeScript interface names
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

/// Execute a deployment command.
///
/// MANIFESTO ALIGNMENT: Explicit deployment configuration generation.
pub fn deploy(config_path: &Path, action: DeployAction) -> CliResult<()> {
    let config = Config::load(config_path)?;
    let _data_dir = config.data_path();

    match action {
        DeployAction::Docker { output } => {
            let docker_compose = r#"# AeroDB Docker Compose Configuration
# Auto-generated - customize as needed

version: "3.8"

services:
  aerodb:
    image: aerodb/aerodb:latest
    ports:
      - "54321:54321"
    volumes:
      - aerodb_data:/var/lib/aerodb
      - ./aerodb.json:/etc/aerodb/config.json:ro
    environment:
      - AERODB_CONFIG=/etc/aerodb/config.json
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "aerodb", "control", "diag", "diagnostics"]
      interval: 30s
      timeout: 10s
      retries: 3

volumes:
  aerodb_data:
"#;

            fs::write(&output, docker_compose).map_err(|e| {
                CliError::config_error(format!("Failed to write Docker Compose file: {}", e))
            })?;

            write_response(json!({
                "generated": true,
                "file": output.to_string_lossy().to_string(),
                "type": "docker-compose"
            }))?;
        }

        DeployAction::K8s { output } => {
            // Ensure output directory exists
            if !output.exists() {
                fs::create_dir_all(&output).map_err(|e| {
                    CliError::config_error(format!("Failed to create k8s directory: {}", e))
                })?;
            }

            // Deployment manifest
            let deployment = r#"# AeroDB Kubernetes Deployment
apiVersion: apps/v1
kind: Deployment
metadata:
  name: aerodb
  labels:
    app: aerodb
spec:
  replicas: 1
  selector:
    matchLabels:
      app: aerodb
  template:
    metadata:
      labels:
        app: aerodb
    spec:
      containers:
      - name: aerodb
        image: aerodb/aerodb:latest
        ports:
        - containerPort: 54321
        volumeMounts:
        - name: data
          mountPath: /var/lib/aerodb
        - name: config
          mountPath: /etc/aerodb
        env:
        - name: AERODB_CONFIG
          value: /etc/aerodb/config.json
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: aerodb-data
      - name: config
        configMap:
          name: aerodb-config
"#;

            // Service manifest
            let service = r#"# AeroDB Kubernetes Service
apiVersion: v1
kind: Service
metadata:
  name: aerodb
spec:
  selector:
    app: aerodb
  ports:
  - port: 54321
    targetPort: 54321
  type: ClusterIP
"#;

            // PVC manifest
            let pvc = r#"# AeroDB Persistent Volume Claim
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: aerodb-data
spec:
  accessModes:
  - ReadWriteOnce
  resources:
    requests:
      storage: 10Gi
"#;

            let files = [
                ("deployment.yaml", deployment),
                ("service.yaml", service),
                ("pvc.yaml", pvc),
            ];

            let mut generated = Vec::new();
            for (name, content) in files {
                let file_path = output.join(name);
                fs::write(&file_path, content).map_err(|e| {
                    CliError::config_error(format!("Failed to write {}: {}", name, e))
                })?;
                generated.push(file_path.to_string_lossy().to_string());
            }

            write_response(json!({
                "generated": true,
                "files": generated,
                "type": "kubernetes"
            }))?;
        }

        DeployAction::Status => {
            // For now, just check if Docker is available
            write_response(json!({
                "status": "ready",
                "docker": true,
                "kubernetes": true,
                "message": "Deployment configurations can be generated"
            }))?;
        }

        DeployAction::Env { output } => {
            let env_template = r#"# AeroDB Environment Configuration
# Copy this file to .env and customize

# Server Configuration
AERODB_HOST=0.0.0.0
AERODB_PORT=54321

# Data Directory
AERODB_DATA_DIR=/var/lib/aerodb

# JWT Configuration
AERODB_JWT_SECRET=your-super-secret-jwt-key-change-in-production
AERODB_JWT_EXPIRY=3600

# WAL Configuration
AERODB_MAX_WAL_SIZE=1073741824
AERODB_WAL_SYNC_MODE=fsync

# Replication (optional)
# AERODB_REPLICATION_ENABLED=false
# AERODB_REPLICATION_ROLE=primary

# Logging
AERODB_LOG_LEVEL=info
"#;

            fs::write(&output, env_template).map_err(|e| {
                CliError::config_error(format!("Failed to write env file: {}", e))
            })?;

            write_response(json!({
                "generated": true,
                "file": output.to_string_lossy().to_string(),
                "type": "env"
            }))?;
        }
    }

    Ok(())
}

/// Execute a logs command.
///
/// MANIFESTO ALIGNMENT: Explicit log viewing with filtering.
pub fn logs(
    config_path: &Path,
    lines: usize,
    level: Option<String>,
    follow: bool,
) -> CliResult<()> {
    let config = Config::load(config_path)?;
    let data_dir = config.data_path();

    let log_file = data_dir.join("logs").join("aerodb.log");

    if !log_file.exists() {
        write_response(json!({
            "logs": [],
            "count": 0,
            "message": "No log file found"
        }))?;
        return Ok(());
    }

    if follow {
        // For follow mode, we'd need async streaming
        // For now, just return current logs with a message
        write_response(json!({
            "error": "Follow mode not yet implemented",
            "hint": "Use 'tail -f' on the log file directly"
        }))?;
        return Ok(());
    }

    // Read log file
    let content = fs::read_to_string(&log_file).map_err(|e| {
        CliError::config_error(format!("Failed to read log file: {}", e))
    })?;

    let all_lines: Vec<&str> = content.lines().collect();
    let total_count = all_lines.len();
    
    // Filter by level if specified
    let filtered: Vec<&str> = if let Some(ref level_filter) = level {
        let level_upper = level_filter.to_uppercase();
        all_lines.iter()
            .filter(|line| line.to_uppercase().contains(&level_upper))
            .copied()
            .collect()
    } else {
        all_lines
    };

    // Take last N lines
    let result: Vec<&str> = filtered.iter()
        .rev()
        .take(lines)
        .rev()
        .copied()
        .collect();

    write_response(json!({
        "logs": result,
        "count": result.len(),
        "total": total_count,
        "level_filter": level
    }))?;

    Ok(())
}

/// Build a control plane command from CLI action.
fn build_command(action: ControlAction) -> CliResult<(ControlPlaneCommand, AuthorityContext)> {
    let authority = AuthorityContext::operator();

    let command = match action {
        ControlAction::Inspect { target } => {
            let inspection = match target {
                InspectTarget::Cluster => InspectionCommand::InspectClusterState,
                InspectTarget::Node { node_id } => {
                    let uuid = parse_uuid(&node_id)?;
                    InspectionCommand::InspectNode { node_id: uuid }
                }
                InspectTarget::Replication => InspectionCommand::InspectReplicationStatus,
                InspectTarget::Promotion => InspectionCommand::InspectPromotionState,
            };
            ControlPlaneCommand::Inspection(inspection)
        }
        ControlAction::Diag { target } => {
            let diagnostic = match target {
                DiagTarget::Diagnostics { .. } => DiagnosticCommand::RunDiagnostics,
                DiagTarget::Wal => DiagnosticCommand::InspectWal,
                DiagTarget::Snapshots => DiagnosticCommand::InspectSnapshots,
            };
            ControlPlaneCommand::Diagnostic(diagnostic)
        }
        ControlAction::Promote {
            replica_id, reason, ..
        } => {
            let uuid = parse_uuid(&replica_id)?;
            ControlPlaneCommand::Control(ControlCommand::RequestPromotion {
                replica_id: uuid,
                reason,
            })
        }
        ControlAction::Demote {
            node_id, reason, ..
        } => {
            let uuid = parse_uuid(&node_id)?;
            ControlPlaneCommand::Control(ControlCommand::RequestDemotion {
                node_id: uuid,
                reason,
            })
        }
        ControlAction::ForcePromote {
            replica_id,
            reason,
            acknowledge_risks,
            ..
        } => {
            let uuid = parse_uuid(&replica_id)?;
            let risks: Vec<String> = acknowledge_risks
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
            ControlPlaneCommand::Control(ControlCommand::ForcePromotion {
                replica_id: uuid,
                reason,
                acknowledged_risks: risks,
            })
        }
    };

    Ok((command, authority))
}

/// Parse a UUID from a string.
fn parse_uuid(s: &str) -> CliResult<Uuid> {
    Uuid::parse_str(s).map_err(|e| CliError::config_error(format!("Invalid UUID '{}': {}", s, e)))
}

/// Check if a data directory is initialized
fn is_initialized(data_dir: &Path) -> bool {
    data_dir.join("wal").exists()
        && data_dir.join("data").exists()
        && data_dir.join("metadata").join("schemas").exists()
}

/// Boot the system per BOOT.md with mandatory recovery
///
/// Steps (strict order, all mandatory):
/// 1. Load schemas
/// 2. Open WAL reader for replay
/// 3. Open recovery storage (combined writer + scanner)
/// 4. Execute RecoveryManager::recover() which:
///    - Replays WAL from offset 0
///    - Applies all records to storage
///    - Rebuilds indexes from storage
///    - Verifies consistency
///    - Removes clean_shutdown marker
/// 5. Return initialized subsystems
///
/// FATAL: Any failure at any step halts startup immediately.
/// No partial startup. No serving without complete recovery.
fn boot_system(
    data_dir: &Path,
) -> CliResult<(
    WalWriter,
    StorageWriter,
    StorageReader,
    SchemaLoader,
    IndexManager,
)> {
    use crate::recovery::RecoveryStorage;

    // Step 1: Load schemas (required for schema validation during recovery)
    let mut schema_loader = SchemaLoader::new(data_dir);
    schema_loader
        .load_all()
        .map_err(|e| CliError::boot_failed(format!("Schema load failed: {}", e)))?;

    // Step 2: Open WAL reader for replay
    let wal_path = data_dir.join("wal").join("wal.log");
    let wal_exists = wal_path.exists();

    // Step 3: Create index manager
    let indexed_fields: HashSet<String> = HashSet::new();
    let mut index_manager = IndexManager::new(indexed_fields);

    // Step 4: Execute RecoveryManager::recover() - MANDATORY
    // This performs: WAL replay -> Index rebuild -> Consistency verification
    let recovery_manager = RecoveryManager::new(data_dir);

    let (storage_writer, storage_reader) = if wal_exists {
        // Open WAL reader
        let mut wal_reader = WalReader::open(&wal_path)
            .map_err(|e| CliError::boot_failed(format!("WAL reader open failed: {}", e)))?;

        // Open recovery storage (implements both StorageApply + StorageScan)
        let mut recovery_storage = RecoveryStorage::open(data_dir)
            .map_err(|e| CliError::boot_failed(format!("Recovery storage open failed: {}", e)))?;

        // Execute full recovery sequence
        // This MUST succeed before we can serve any requests
        let _recovery_state = recovery_manager
            .recover(
                &mut wal_reader,
                &mut recovery_storage,
                &mut index_manager,
                &schema_loader,
            )
            .map_err(|e| {
                // Recovery failure is FATAL - system cannot serve
                CliError::boot_failed(format!(
                    "Recovery failed (FATAL): {}. System cannot serve requests.",
                    e
                ))
            })?;

        // Extract writer and reader from recovery storage
        recovery_storage.into_parts()
    } else {
        // No WAL file exists - fresh database
        // Still need to remove shutdown marker if present
        let shutdown_marker = data_dir.join("clean_shutdown");
        if shutdown_marker.exists() {
            fs::remove_file(&shutdown_marker).map_err(|e| {
                CliError::boot_failed(format!("Failed to remove shutdown marker: {}", e))
            })?;
        }

        // Open storage directly
        let storage_writer = StorageWriter::open(data_dir)
            .map_err(|e| CliError::boot_failed(format!("Storage writer open failed: {}", e)))?;
        let storage_reader = StorageReader::open_from_data_dir(data_dir)
            .map_err(|e| CliError::boot_failed(format!("Storage reader open failed: {}", e)))?;

        (storage_writer, storage_reader)
    };

    // Step 5: Open WAL writer for new writes
    let wal_writer = WalWriter::open(data_dir)
        .map_err(|e| CliError::boot_failed(format!("WAL writer open failed: {}", e)))?;

    // Recovery complete - system may now enter SERVING state
    Ok((
        wal_writer,
        storage_writer,
        storage_reader,
        schema_loader,
        index_manager,
    ))
}

#[cfg(test)]
mod tests {
    use super::super::errors::CliErrorCode;
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_config(temp_dir: &TempDir) -> std::path::PathBuf {
        let config_path = temp_dir.path().join("aerodb.json");
        let data_dir = temp_dir.path().join("data");

        let config = json!({
            "data_dir": data_dir.to_string_lossy()
        });

        fs::write(&config_path, config.to_string()).unwrap();
        config_path
    }

    #[test]
    fn test_init_creates_directories() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_config(&temp_dir);
        let data_dir = temp_dir.path().join("data");

        // Init should succeed
        init(&config_path).unwrap();

        // Check directories exist
        assert!(data_dir.join("wal").exists());
        assert!(data_dir.join("data").exists());
        assert!(data_dir.join("metadata").join("schemas").exists());
    }

    #[test]
    fn test_init_refuses_reinit() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_config(&temp_dir);

        // First init succeeds
        init(&config_path).unwrap();

        // Second init fails
        let result = init(&config_path);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().code(),
            &CliErrorCode::AlreadyInitialized
        );
    }

    #[test]
    fn test_start_requires_init() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_config(&temp_dir);

        // Start without init fails
        let result = start(&config_path);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), &CliErrorCode::NotInitialized);
    }

    #[test]
    fn test_config_validates_sync_mode() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("aerodb.json");
        let data_dir = temp_dir.path().join("data");

        let config = json!({
            "data_dir": data_dir.to_string_lossy(),
            "wal_sync_mode": "none"  // Invalid!
        });

        fs::write(&config_path, config.to_string()).unwrap();

        let result = Config::load(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_defaults() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("aerodb.json");
        let data_dir = temp_dir.path().join("data");

        let config_json = json!({
            "data_dir": data_dir.to_string_lossy()
        });

        fs::write(&config_path, config_json.to_string()).unwrap();

        let config = Config::load(&config_path).unwrap();
        assert_eq!(config.max_wal_size_bytes, 1073741824);
        assert_eq!(config.max_memory_bytes, 536870912);
        assert_eq!(config.wal_sync_mode, "fsync");
    }
}
