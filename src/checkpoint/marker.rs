//! Checkpoint marker file handling
//!
//! Per CHECKPOINT.md §5, the checkpoint marker file records:
//! - snapshot_id: The ID of the associated snapshot
//! - created_at: RFC3339 timestamp
//! - wal_truncated: Whether WAL was successfully truncated
//! - format_version: Always 1 for Phase 1
//!
//! Location: `<data_dir>/checkpoint.json`
//!
//! The marker is written AFTER snapshot fsync and BEFORE WAL truncation.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::errors::{CheckpointError, CheckpointResult};

/// Checkpoint marker data structure per CHECKPOINT.md §5
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointMarker {
    /// Snapshot ID referenced by this checkpoint (RFC3339 basic format)
    pub snapshot_id: String,

    /// Timestamp when checkpoint was created (RFC3339 format)
    pub created_at: String,

    /// Whether WAL was successfully truncated
    pub wal_truncated: bool,

    /// Format version (always 1 for Phase 1)
    pub format_version: u8,
}

impl CheckpointMarker {
    /// Creates a new checkpoint marker (before WAL truncation)
    ///
    /// Per CHECKPOINT.md §5:
    /// - Written AFTER snapshot fsync
    /// - Written BEFORE WAL truncation
    ///
    /// Initially `wal_truncated` is false; it is updated after truncation.
    pub fn new(snapshot_id: &str, created_at: &str) -> Self {
        Self {
            snapshot_id: snapshot_id.to_string(),
            created_at: created_at.to_string(),
            wal_truncated: false,
            format_version: 1,
        }
    }

    /// Creates a checkpoint marker with truncation status set
    pub fn with_truncation(snapshot_id: &str, created_at: &str, truncated: bool) -> Self {
        Self {
            snapshot_id: snapshot_id.to_string(),
            created_at: created_at.to_string(),
            wal_truncated: truncated,
            format_version: 1,
        }
    }

    /// Serializes the marker to JSON
    pub fn to_json(&self) -> CheckpointResult<String> {
        serde_json::to_string_pretty(self).map_err(|e| {
            CheckpointError::marker_failed(
                format!("Failed to serialize checkpoint marker: {}", e),
                std::io::Error::new(std::io::ErrorKind::InvalidData, e),
            )
        })
    }

    /// Deserializes the marker from JSON
    pub fn from_json(json: &str) -> CheckpointResult<Self> {
        serde_json::from_str(json).map_err(|e| {
            CheckpointError::failed(format!("Failed to parse checkpoint marker: {}", e))
        })
    }

    /// Writes the marker to a file with fsync
    ///
    /// Per CHECKPOINT.md, the marker must be durable before proceeding.
    pub fn write_to_file(&self, path: &Path) -> CheckpointResult<()> {
        let json = self.to_json()?;

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| {
                    CheckpointError::marker_failed(
                        format!("Failed to create marker directory: {}", parent.display()),
                        e,
                    )
                })?;
            }
        }

        // Write to file
        let mut file = File::create(path).map_err(|e| {
            CheckpointError::marker_failed(
                format!("Failed to create marker file: {}", path.display()),
                e,
            )
        })?;

        file.write_all(json.as_bytes()).map_err(|e| {
            CheckpointError::marker_failed(
                format!("Failed to write marker file: {}", path.display()),
                e,
            )
        })?;

        // fsync is mandatory
        file.sync_all().map_err(|e| {
            CheckpointError::marker_failed(
                format!("Failed to fsync marker file: {}", path.display()),
                e,
            )
        })?;

        // fsync parent directory
        if let Some(parent) = path.parent() {
            let dir = OpenOptions::new().read(true).open(parent).map_err(|e| {
                CheckpointError::marker_failed(
                    format!(
                        "Failed to open marker directory for fsync: {}",
                        parent.display()
                    ),
                    e,
                )
            })?;

            dir.sync_all().map_err(|e| {
                CheckpointError::marker_failed(
                    format!("Failed to fsync marker directory: {}", parent.display()),
                    e,
                )
            })?;
        }

        Ok(())
    }

    /// Reads a marker from a file
    pub fn read_from_file(path: &Path) -> CheckpointResult<Self> {
        let mut file = File::open(path).map_err(|e| {
            CheckpointError::failed_with_source(
                format!("Failed to open marker file: {}", path.display()),
                e,
            )
        })?;

        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|e| {
            CheckpointError::failed_with_source(
                format!("Failed to read marker file: {}", path.display()),
                e,
            )
        })?;

        Self::from_json(&contents)
    }

    /// Checks if a marker file exists
    pub fn exists(path: &Path) -> bool {
        path.exists()
    }
}

/// Returns the path to the checkpoint marker file
pub fn marker_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("checkpoint.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_marker_creation() {
        let marker = CheckpointMarker::new("20260204T163000Z", "2026-02-04T16:30:00Z");

        assert_eq!(marker.snapshot_id, "20260204T163000Z");
        assert_eq!(marker.created_at, "2026-02-04T16:30:00Z");
        assert!(!marker.wal_truncated);
        assert_eq!(marker.format_version, 1);
    }

    #[test]
    fn test_marker_with_truncation() {
        let marker =
            CheckpointMarker::with_truncation("20260204T163000Z", "2026-02-04T16:30:00Z", true);

        assert!(marker.wal_truncated);
    }

    #[test]
    fn test_marker_json_roundtrip() {
        let marker =
            CheckpointMarker::with_truncation("20260204T163000Z", "2026-02-04T16:30:00Z", true);

        let json = marker.to_json().unwrap();
        let parsed = CheckpointMarker::from_json(&json).unwrap();

        assert_eq!(marker, parsed);
    }

    #[test]
    fn test_marker_json_format_matches_spec() {
        let marker =
            CheckpointMarker::with_truncation("20260204T113000Z", "2026-02-04T11:30:00Z", true);

        let json = marker.to_json().unwrap();

        // Per CHECKPOINT.md §5, verify expected fields
        assert!(json.contains("\"snapshot_id\""));
        assert!(json.contains("\"created_at\""));
        assert!(json.contains("\"wal_truncated\""));
        assert!(json.contains("\"format_version\""));
        assert!(json.contains("\"20260204T113000Z\""));
        assert!(json.contains("\"2026-02-04T11:30:00Z\""));
        assert!(json.contains("true")); // wal_truncated
        assert!(json.contains("1")); // format_version
    }

    #[test]
    fn test_marker_write_and_read() {
        let temp_dir = TempDir::new().unwrap();
        let marker_path = temp_dir.path().join("checkpoint.json");

        let marker =
            CheckpointMarker::with_truncation("20260204T163000Z", "2026-02-04T16:30:00Z", true);

        // Write
        marker.write_to_file(&marker_path).unwrap();

        // Verify file exists
        assert!(marker_path.exists());

        // Read back
        let read_marker = CheckpointMarker::read_from_file(&marker_path).unwrap();

        assert_eq!(marker, read_marker);
    }

    #[test]
    fn test_marker_exists() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("checkpoint.json");

        assert!(!CheckpointMarker::exists(&path));

        let marker = CheckpointMarker::new("test", "test");
        marker.write_to_file(&path).unwrap();

        assert!(CheckpointMarker::exists(&path));
    }

    #[test]
    fn test_marker_path_function() {
        let data_dir = Path::new("/data");
        let path = marker_path(data_dir);

        assert_eq!(path, Path::new("/data/checkpoint.json"));
    }

    #[test]
    fn test_format_version_always_one() {
        let marker1 = CheckpointMarker::new("id1", "time1");
        let marker2 = CheckpointMarker::with_truncation("id2", "time2", true);

        assert_eq!(marker1.format_version, 1);
        assert_eq!(marker2.format_version, 1);
    }

    #[test]
    fn test_marker_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("nonexistent.json");

        let result = CheckpointMarker::read_from_file(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_marker_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("bad.json");

        fs::write(&path, "not valid json").unwrap();

        let result = CheckpointMarker::read_from_file(&path);
        assert!(result.is_err());
    }
}
