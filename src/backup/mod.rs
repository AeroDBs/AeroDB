//! # Backup Module
//!
//! Backup and restore functionality for AeroDB.
//!
//! This module provides backup creation, configuration, and restore capabilities.

use std::path::Path;
use std::fs::File;
use std::io::{Read, Result as IoResult};
use serde::{Deserialize, Serialize};

/// Backup configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Enable automatic backups
    pub enabled: bool,
    /// Backup interval in hours
    pub interval_hours: u32,
    /// Maximum number of backups to retain
    pub max_backups: u32,
    /// Backup directory path
    pub backup_dir: String,
}

impl BackupConfig {
    /// Create a new backup config with defaults
    pub fn new() -> Self {
        Self {
            enabled: false,
            interval_hours: 24,
            max_backups: 7,
            backup_dir: "/var/lib/aerodb/backups".to_string(),
        }
    }
}

/// Backup manifest containing metadata about a backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    /// Backup ID
    pub backup_id: String,
    /// Snapshot ID
    pub snapshot_id: String,
    /// Creation timestamp
    pub created_at: String,
    /// Whether WAL is present
    pub wal_present: bool,
    /// Format version
    pub format_version: u32,
}

impl BackupManifest {
    /// Read manifest from a file
    pub fn read_from_file(path: &Path) -> IoResult<Self> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        serde_json::from_str(&contents).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })
    }

    /// Write manifest to a file
    pub fn write_to_file(&self, path: &Path) -> IoResult<()> {
        let contents = serde_json::to_string_pretty(self).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })?;
        std::fs::write(path, contents)?;
        Ok(())
    }
}

/// Backup status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupStatus {
    /// Last backup timestamp
    pub last_backup: Option<String>,
    /// Next scheduled backup
    pub next_backup: Option<String>,
    /// Number of available backups
    pub backup_count: u32,
    /// Total backup size in bytes
    pub total_size_bytes: u64,
}

impl Default for BackupStatus {
    fn default() -> Self {
        Self {
            last_backup: None,
            next_backup: None,
            backup_count: 0,
            total_size_bytes: 0,
        }
    }
}

/// Backup metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    /// Backup ID
    pub id: String,
    /// Creation timestamp
    pub created_at: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// Description
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_backup_config_defaults() {
        let config = BackupConfig::new();
        assert!(!config.enabled);
        assert_eq!(config.interval_hours, 24);
        assert_eq!(config.max_backups, 7);
    }

    #[test]
    fn test_backup_manifest_read_write() {
        let temp_file = NamedTempFile::new().unwrap();
        let manifest = BackupManifest {
            backup_id: "test-backup".to_string(),
            snapshot_id: "test-snapshot".to_string(),
            created_at: "2026-02-07T12:00:00Z".to_string(),
            wal_present: true,
            format_version: 1,
        };

        manifest.write_to_file(temp_file.path()).unwrap();
        let loaded = BackupManifest::read_from_file(temp_file.path()).unwrap();
        
        assert_eq!(loaded.backup_id, manifest.backup_id);
        assert_eq!(loaded.snapshot_id, manifest.snapshot_id);
        assert_eq!(loaded.format_version, 1);
    }
}
