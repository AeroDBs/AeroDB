//! Crash point injection for testing durability
//!
//! Per CRASH_TESTING.md, this module provides crash point injection
//! via the `AERODB_CRASH_POINT` environment variable.
//!
//! When a crash point is enabled, AeroDB immediately terminates via
//! `std::process::abort()` - no cleanup, no unwinding, no catching.
//!
//! # Usage
//!
//! ```ignore
//! use aerodb::crash_point::maybe_crash;
//!
//! // In production code, add crash points at critical locations
//! maybe_crash("wal_after_fsync");
//! ```
//!
//! # Testing
//!
//! ```bash
//! AERODB_CRASH_POINT=wal_after_fsync cargo run
//! ```

use std::sync::OnceLock;

/// Cache the crash point name to avoid repeated env var lookups
static CRASH_POINT: OnceLock<Option<String>> = OnceLock::new();

/// Get the configured crash point (cached)
#[inline]
fn get_crash_point() -> Option<&'static str> {
    CRASH_POINT
        .get_or_init(|| std::env::var("AERODB_CRASH_POINT").ok())
        .as_deref()
}

/// Check if a specific crash point is enabled
///
/// Returns true if `AERODB_CRASH_POINT` equals the given name.
/// Zero-cost when disabled (env var not set).
#[inline]
pub fn crash_point_enabled(name: &str) -> bool {
    get_crash_point().map(|p| p == name).unwrap_or(false)
}

/// Trigger a crash if the named crash point is enabled
///
/// Per CRASH_TESTING.md:
/// - Immediately terminate the process
/// - Without cleanup
/// - Without unwinding
/// - Without catching
///
/// Uses `std::process::abort()`.
///
/// This is a no-op when `AERODB_CRASH_POINT` is not set or doesn't match.
#[inline]
pub fn maybe_crash(name: &str) {
    if crash_point_enabled(name) {
        // Log the crash for debugging
        eprintln!("[CRASH] Triggering crash at point: {}", name);
        std::process::abort();
    }
}

/// All defined crash point names
pub mod points {
    // WAL crash points
    pub const WAL_BEFORE_APPEND: &str = "wal_before_append";
    pub const WAL_AFTER_APPEND: &str = "wal_after_append";
    pub const WAL_BEFORE_FSYNC: &str = "wal_before_fsync";
    pub const WAL_AFTER_FSYNC: &str = "wal_after_fsync";
    pub const WAL_BEFORE_TRUNCATE: &str = "wal_before_truncate";
    pub const WAL_AFTER_TRUNCATE: &str = "wal_after_truncate";

    // Storage crash points
    pub const STORAGE_BEFORE_WRITE: &str = "storage_before_write";
    pub const STORAGE_AFTER_WRITE: &str = "storage_after_write";
    pub const STORAGE_BEFORE_CHECKSUM: &str = "storage_before_checksum";
    pub const STORAGE_AFTER_CHECKSUM: &str = "storage_after_checksum";

    // Snapshot crash points
    pub const SNAPSHOT_START: &str = "snapshot_start";
    pub const SNAPSHOT_AFTER_STORAGE_COPY: &str = "snapshot_after_storage_copy";
    pub const SNAPSHOT_BEFORE_MANIFEST: &str = "snapshot_before_manifest";
    pub const SNAPSHOT_AFTER_MANIFEST: &str = "snapshot_after_manifest";

    // Checkpoint crash points
    pub const CHECKPOINT_START: &str = "checkpoint_start";
    pub const CHECKPOINT_AFTER_SNAPSHOT: &str = "checkpoint_after_snapshot";
    pub const CHECKPOINT_BEFORE_WAL_TRUNCATE: &str = "checkpoint_before_wal_truncate";
    pub const CHECKPOINT_AFTER_WAL_TRUNCATE: &str = "checkpoint_after_wal_truncate";

    // Backup crash points
    pub const BACKUP_START: &str = "backup_start";
    pub const BACKUP_AFTER_SNAPSHOT_COPY: &str = "backup_after_snapshot_copy";
    pub const BACKUP_AFTER_WAL_COPY: &str = "backup_after_wal_copy";
    pub const BACKUP_BEFORE_ARCHIVE: &str = "backup_before_archive";

    // Restore crash points
    pub const RESTORE_START: &str = "restore_start";
    pub const RESTORE_AFTER_EXTRACT: &str = "restore_after_extract";
    pub const RESTORE_BEFORE_REPLACE: &str = "restore_before_replace";
    pub const RESTORE_AFTER_REPLACE: &str = "restore_after_replace";

    // Recovery crash points
    pub const RECOVERY_START: &str = "recovery_start";
    pub const RECOVERY_AFTER_WAL_REPLAY: &str = "recovery_after_wal_replay";
    pub const RECOVERY_AFTER_INDEX_REBUILD: &str = "recovery_after_index_rebuild";

    // MVCC crash points per MVCC_FAILURE_MATRIX.md
    pub const MVCC_BEFORE_COMMIT_RECORD: &str = "mvcc_before_commit_record";
    pub const MVCC_AFTER_COMMIT_RECORD: &str = "mvcc_after_commit_record";
    pub const MVCC_AFTER_COMMIT_FSYNC: &str = "mvcc_after_commit_fsync";

    // MVCC version persistence crash points per MVCC-03
    pub const MVCC_BEFORE_VERSION_WRITE: &str = "mvcc_before_version_write";
    pub const MVCC_AFTER_VERSION_WRITE: &str = "mvcc_after_version_write";
    pub const MVCC_AFTER_VERSION_FSYNC: &str = "mvcc_after_version_fsync";

    /// Get all crash point names
    pub fn all() -> &'static [&'static str] {
        &[
            WAL_BEFORE_APPEND,
            WAL_AFTER_APPEND,
            WAL_BEFORE_FSYNC,
            WAL_AFTER_FSYNC,
            WAL_BEFORE_TRUNCATE,
            WAL_AFTER_TRUNCATE,
            STORAGE_BEFORE_WRITE,
            STORAGE_AFTER_WRITE,
            STORAGE_BEFORE_CHECKSUM,
            STORAGE_AFTER_CHECKSUM,
            SNAPSHOT_START,
            SNAPSHOT_AFTER_STORAGE_COPY,
            SNAPSHOT_BEFORE_MANIFEST,
            SNAPSHOT_AFTER_MANIFEST,
            CHECKPOINT_START,
            CHECKPOINT_AFTER_SNAPSHOT,
            CHECKPOINT_BEFORE_WAL_TRUNCATE,
            CHECKPOINT_AFTER_WAL_TRUNCATE,
            BACKUP_START,
            BACKUP_AFTER_SNAPSHOT_COPY,
            BACKUP_AFTER_WAL_COPY,
            BACKUP_BEFORE_ARCHIVE,
            RESTORE_START,
            RESTORE_AFTER_EXTRACT,
            RESTORE_BEFORE_REPLACE,
            RESTORE_AFTER_REPLACE,
            RECOVERY_START,
            RECOVERY_AFTER_WAL_REPLAY,
            RECOVERY_AFTER_INDEX_REBUILD,
            MVCC_BEFORE_COMMIT_RECORD,
            MVCC_AFTER_COMMIT_RECORD,
            MVCC_AFTER_COMMIT_FSYNC,
            MVCC_BEFORE_VERSION_WRITE,
            MVCC_AFTER_VERSION_WRITE,
            MVCC_AFTER_VERSION_FSYNC,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crash_point_disabled_by_default() {
        // Without env var set, should return false
        assert!(!crash_point_enabled("test_point"));
    }

    #[test]
    fn test_all_crash_points_defined() {
        let all = points::all();
        assert_eq!(all.len(), 35);

        // Verify WAL points
        assert!(all.contains(&"wal_before_append"));
        assert!(all.contains(&"wal_after_fsync"));

        // Verify snapshot points
        assert!(all.contains(&"snapshot_start"));

        // Verify checkpoint points
        assert!(all.contains(&"checkpoint_start"));

        // Verify backup points
        assert!(all.contains(&"backup_start"));

        // Verify restore points
        assert!(all.contains(&"restore_start"));

        // Verify recovery points
        assert!(all.contains(&"recovery_start"));
    }

    #[test]
    fn test_crash_point_names_are_lowercase_with_underscores() {
        for point in points::all() {
            assert!(
                point.chars().all(|c| c.is_lowercase() || c == '_'),
                "Crash point '{}' should be lowercase with underscores",
                point
            );
        }
    }
}
