//! Observability events for AeroDB
//!
//! Per OBSERVABILITY.md, this module defines all observable events
//! that can occur during AeroDB operation.
//!
//! Events are explicit and typed.

use std::fmt;

/// Observable events in AeroDB
///
/// Per OBSERVABILITY.md §3-4, these events cover:
/// - Boot & Lifecycle
/// - WAL operations
/// - Snapshot / Checkpoint
/// - Backup / Restore
/// - Recovery
/// - Query processing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    // Boot & Lifecycle
    /// AeroDB startup begins
    BootStart,
    /// AeroDB startup complete, ready to serve
    BootComplete,
    /// Shutdown initiated
    ShutdownStart,
    /// Shutdown complete
    ShutdownComplete,

    // Configuration
    /// Configuration loaded
    ConfigLoaded,
    /// Schemas loaded
    SchemasLoaded,

    // WAL operations
    /// WAL record appended
    WalAppend,
    /// WAL fsynced to disk
    WalFsync,
    /// WAL truncated
    WalTruncate,
    /// WAL corruption detected (FATAL)
    WalCorruption,

    // Snapshot operations
    /// Snapshot creation started
    SnapshotStart,
    /// Snapshot creation complete
    SnapshotComplete,

    // Checkpoint operations
    /// Checkpoint started
    CheckpointStart,
    /// Checkpoint complete
    CheckpointComplete,
    /// Checkpoint failed
    CheckpointFailed,

    // Backup operations
    /// Backup started
    BackupStart,
    /// Backup complete
    BackupComplete,
    /// Backup failed
    BackupFailed,

    // Restore operations
    /// Restore started
    RestoreStart,
    /// Restore complete
    RestoreComplete,
    /// Restore aborted/failed
    RestoreAborted,

    // Recovery operations
    /// Recovery started
    RecoveryStart,
    /// WAL replay begins
    RecoveryReplayBegin,
    /// WAL replay complete
    RecoveryReplayComplete,
    /// Index rebuild begins
    RecoveryIndexRebuildBegin,
    /// Index rebuild complete
    RecoveryIndexRebuildComplete,
    /// Verification begins
    RecoveryVerifyBegin,
    /// Verification complete
    RecoveryVerifyComplete,
    /// Recovery failed (FATAL)
    RecoveryFailed,

    // Query operations
    /// Query received
    QueryReceived,
    /// Query planned
    QueryPlanned,
    /// Query executed successfully
    QueryExecuted,
    /// Query rejected
    QueryRejected,

    // Write operations
    /// Write operation begins
    WriteBegin,
    /// Write committed
    WriteCommit,

    // Explain operations
    /// Explain begins
    ExplainBegin,
    /// Explain complete
    ExplainComplete,

    // Server operations
    /// Server serving (ready for requests)
    Serving,
}

impl Event {
    /// Returns the string representation of the event
    pub fn as_str(&self) -> &'static str {
        match self {
            // Boot & Lifecycle
            Event::BootStart => "AERODB_STARTUP_BEGIN",
            Event::BootComplete => "AERODB_STARTUP_COMPLETE",
            Event::ShutdownStart => "SHUTDOWN_START",
            Event::ShutdownComplete => "SHUTDOWN_COMPLETE",

            // Configuration
            Event::ConfigLoaded => "CONFIG_LOADED",
            Event::SchemasLoaded => "SCHEMAS_LOADED",

            // WAL
            Event::WalAppend => "WAL_APPEND",
            Event::WalFsync => "WAL_FSYNC",
            Event::WalTruncate => "WAL_TRUNCATED",
            Event::WalCorruption => "WAL_CORRUPTION",

            // Snapshot
            Event::SnapshotStart => "SNAPSHOT_START",
            Event::SnapshotComplete => "SNAPSHOT_CREATED",

            // Checkpoint
            Event::CheckpointStart => "CHECKPOINT_BEGIN",
            Event::CheckpointComplete => "CHECKPOINT_COMPLETE",
            Event::CheckpointFailed => "CHECKPOINT_FAILED",

            // Backup
            Event::BackupStart => "BACKUP_BEGIN",
            Event::BackupComplete => "BACKUP_COMPLETE",
            Event::BackupFailed => "BACKUP_FAILED",

            // Restore
            Event::RestoreStart => "RESTORE_BEGIN",
            Event::RestoreComplete => "RESTORE_COMPLETE",
            Event::RestoreAborted => "RESTORE_FAILED",

            // Recovery
            Event::RecoveryStart => "RECOVERY_BEGIN",
            Event::RecoveryReplayBegin => "WAL_REPLAY_BEGIN",
            Event::RecoveryReplayComplete => "WAL_REPLAY_COMPLETE",
            Event::RecoveryIndexRebuildBegin => "INDEX_REBUILD_BEGIN",
            Event::RecoveryIndexRebuildComplete => "INDEX_REBUILD_COMPLETE",
            Event::RecoveryVerifyBegin => "VERIFICATION_BEGIN",
            Event::RecoveryVerifyComplete => "VERIFICATION_COMPLETE",
            Event::RecoveryFailed => "RECOVERY_FAILED",

            // Query
            Event::QueryReceived => "QUERY_BEGIN",
            Event::QueryPlanned => "QUERY_PLANNED",
            Event::QueryExecuted => "QUERY_COMPLETE",
            Event::QueryRejected => "QUERY_REJECTED",

            // Write
            Event::WriteBegin => "WRITE_BEGIN",
            Event::WriteCommit => "WRITE_COMMIT",

            // Explain
            Event::ExplainBegin => "EXPLAIN_BEGIN",
            Event::ExplainComplete => "EXPLAIN_COMPLETE",

            // Server
            Event::Serving => "AERODB_SERVING",
        }
    }

    /// Returns true if this event indicates a fatal condition
    pub fn is_fatal(&self) -> bool {
        matches!(self, Event::WalCorruption | Event::RecoveryFailed)
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_events_have_string_representation() {
        let events = [
            Event::BootStart,
            Event::BootComplete,
            Event::ShutdownStart,
            Event::ShutdownComplete,
            Event::ConfigLoaded,
            Event::SchemasLoaded,
            Event::WalAppend,
            Event::WalFsync,
            Event::WalTruncate,
            Event::WalCorruption,
            Event::SnapshotStart,
            Event::SnapshotComplete,
            Event::CheckpointStart,
            Event::CheckpointComplete,
            Event::CheckpointFailed,
            Event::BackupStart,
            Event::BackupComplete,
            Event::BackupFailed,
            Event::RestoreStart,
            Event::RestoreComplete,
            Event::RestoreAborted,
            Event::RecoveryStart,
            Event::RecoveryReplayBegin,
            Event::RecoveryReplayComplete,
            Event::RecoveryIndexRebuildBegin,
            Event::RecoveryIndexRebuildComplete,
            Event::RecoveryVerifyBegin,
            Event::RecoveryVerifyComplete,
            Event::RecoveryFailed,
            Event::QueryReceived,
            Event::QueryPlanned,
            Event::QueryExecuted,
            Event::QueryRejected,
            Event::WriteBegin,
            Event::WriteCommit,
            Event::ExplainBegin,
            Event::ExplainComplete,
            Event::Serving,
        ];

        for event in events {
            let s = event.as_str();
            assert!(!s.is_empty());
            // Verify all uppercase format
            assert!(s.chars().all(|c| c.is_uppercase() || c == '_'));
        }
    }

    #[test]
    fn test_fatal_events() {
        assert!(Event::WalCorruption.is_fatal());
        assert!(Event::RecoveryFailed.is_fatal());
        assert!(!Event::BootStart.is_fatal());
        assert!(!Event::QueryExecuted.is_fatal());
    }

    #[test]
    fn test_event_display() {
        assert_eq!(format!("{}", Event::BootStart), "AERODB_STARTUP_BEGIN");
        assert_eq!(
            format!("{}", Event::CheckpointComplete),
            "CHECKPOINT_COMPLETE"
        );
    }
}
