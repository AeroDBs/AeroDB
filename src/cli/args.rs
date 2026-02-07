//! CLI argument definitions using clap
//!
//! Commands:
//! - aerodb init --config <path>
//! - aerodb start --config <path>
//! - aerodb query --config <path>
//! - aerodb explain --config <path>
//!
//! # Phase 7 Control Plane Commands
//!
//! Per PHASE7_COMMAND_MODEL.md:
//! - aerodb control inspect <cluster|node|replication|promotion>
//! - aerodb control diag <diagnostics|wal|snapshots>
//! - aerodb control <promote|demote|force-promote>

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// AeroDB - A strict, deterministic, self-hostable database
#[derive(Parser, Debug)]
#[command(name = "aerodb")]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Initialize a new AeroDB data directory
    Init {
        /// Path to configuration file
        #[arg(long, default_value = "./aerodb.json")]
        config: PathBuf,
    },

    /// Start the AeroDB server
    Start {
        /// Path to configuration file
        #[arg(long, default_value = "./aerodb.json")]
        config: PathBuf,
    },

    /// Execute a single query and exit
    Query {
        /// Path to configuration file
        #[arg(long, default_value = "./aerodb.json")]
        config: PathBuf,
    },

    /// Execute explain on a query and exit
    Explain {
        /// Path to configuration file
        #[arg(long, default_value = "./aerodb.json")]
        config: PathBuf,
    },

    /// Start HTTP server for dashboard (Phase 13.5)
    ///
    /// Starts an HTTP server exposing REST API for the dashboard.
    /// This replaces stdin/stdout mode with HTTP mode.
    Serve {
        /// Path to configuration file
        #[arg(long, default_value = "./aerodb.json")]
        config: PathBuf,

        /// Port to bind to (default: 54321)
        #[arg(long, default_value = "54321")]
        port: u16,
    },

    /// Control plane commands (Phase 7)
    ///
    /// Per PHASE7_COMMAND_MODEL.md: Operator control surface for AeroDB.
    /// All mutating commands require explicit confirmation.
    Control {
        /// Path to configuration file
        #[arg(long, default_value = "./aerodb.json")]
        config: PathBuf,

        #[command(subcommand)]
        action: ControlAction,
    },
}

/// Control plane actions.
///
/// Per PHASE7_COMMAND_MODEL.md:
/// - Inspection: read-only state views
/// - Diagnostic: read-only but potentially expensive
/// - Control: mutating, requires confirmation
#[derive(Subcommand, Debug)]
pub enum ControlAction {
    /// Inspect cluster, node, or replication state (read-only)
    Inspect {
        #[command(subcommand)]
        target: InspectTarget,
    },

    /// Run diagnostics (read-only, may be expensive)
    Diag {
        #[command(subcommand)]
        target: DiagTarget,
    },

    /// Request promotion of a replica to primary
    ///
    /// Requires confirmation. Maps to promotion state machine.
    Promote {
        /// Replica UUID to promote
        #[arg(long)]
        replica_id: String,

        /// Reason for promotion (for audit)
        #[arg(long)]
        reason: Option<String>,

        /// Confirmation token (from previous request)
        #[arg(long)]
        confirm: Option<String>,
    },

    /// Request demotion of the current primary
    ///
    /// Requires confirmation.
    Demote {
        /// Node UUID to demote
        #[arg(long)]
        node_id: String,

        /// Reason for demotion (for audit)
        #[arg(long)]
        reason: Option<String>,

        /// Confirmation token (from previous request)
        #[arg(long)]
        confirm: Option<String>,
    },

    /// Force promotion bypassing safety checks
    ///
    /// DANGER: Requires enhanced confirmation.
    /// May violate single-primary invariant.
    ForcePromote {
        /// Replica UUID to force promote
        #[arg(long)]
        replica_id: String,

        /// Reason for force promotion (required)
        #[arg(long)]
        reason: String,

        /// Acknowledged risks (required, comma-separated)
        #[arg(long)]
        acknowledge_risks: String,

        /// Confirmation token (from previous request)
        #[arg(long)]
        confirm: Option<String>,
    },
}

/// Inspection targets.
#[derive(Subcommand, Debug)]
pub enum InspectTarget {
    /// Inspect cluster topology and roles
    Cluster,

    /// Inspect a specific node
    Node {
        /// Node UUID to inspect
        #[arg(long)]
        node_id: String,
    },

    /// Inspect replication status
    Replication,

    /// Inspect promotion state machine
    Promotion,
}

/// Diagnostic targets.
#[derive(Subcommand, Debug)]
pub enum DiagTarget {
    /// Run full diagnostics (requires confirmation due to cost)
    Diagnostics {
        /// Confirmation token (from previous request)
        #[arg(long)]
        confirm: Option<String>,
    },

    /// Inspect WAL metadata
    Wal,

    /// Inspect snapshots and checkpoints
    Snapshots,
}

impl Cli {
    /// Parse command line arguments
    pub fn parse_args() -> Self {
        Cli::parse()
    }
}
