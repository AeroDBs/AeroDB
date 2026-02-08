//! Version Marker for Safe Upgrades
//!
//! HARDENING: Prevents incompatible binary upgrades from corrupting data.
//!
//! Creates `.aerodb_version` file on initialization containing:
//! - Binary version
//! - WAL format version
//! - Schema format version
//!
//! On startup, compares versions and refuses to start if incompatible.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Current versions - update these when formats change
pub const BINARY_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const WAL_FORMAT_VERSION: u16 = 1;
pub const SCHEMA_FORMAT_VERSION: u16 = 1;

/// Version marker file name
const VERSION_FILE: &str = ".aerodb_version";

/// Initialization marker file name (for atomic init detection)
const INIT_MARKER_FILE: &str = ".aerodb_initialized";

/// Version compatibility result
#[derive(Debug)]
pub enum VersionCheck {
    /// Versions are compatible, proceed
    Compatible,
    /// First initialization, create marker
    NewInstallation,
    /// Upgrade detected but compatible
    UpgradeCompatible { from: String, to: String },
    /// Incompatible - refuse startup
    Incompatible(VersionError),
}

/// Version-related errors
#[derive(Debug)]
pub enum VersionError {
    /// WAL format version mismatch
    WalFormatMismatch {
        expected: u16,
        found: u16,
        message: String,
    },
    /// Schema format version mismatch
    SchemaFormatMismatch {
        expected: u16,
        found: u16,
        message: String,
    },
    /// Downgrade attempted (newer data with older binary)
    DowngradeDetected {
        data_version: String,
        binary_version: String,
    },
    /// Partial initialization detected
    PartialInitialization {
        message: String,
    },
    /// IO error reading/writing version file
    IoError(String),
}

impl std::fmt::Display for VersionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionError::WalFormatMismatch { expected, found, message } => {
                write!(
                    f,
                    "WAL format version mismatch: expected v{}, found v{}. {}",
                    expected, found, message
                )
            }
            VersionError::SchemaFormatMismatch { expected, found, message } => {
                write!(
                    f,
                    "Schema format version mismatch: expected v{}, found v{}. {}",
                    expected, found, message
                )
            }
            VersionError::DowngradeDetected { data_version, binary_version } => {
                write!(
                    f,
                    "Downgrade detected: data written by v{} cannot be read by v{}. \
                     Downgrades are not supported. Use the original binary version or newer.",
                    data_version, binary_version
                )
            }
            VersionError::PartialInitialization { message } => {
                write!(
                    f,
                    "Partial initialization detected: {}. \
                     Run 'aerodb init --force' to reinitialize (WARNING: destroys existing data).",
                    message
                )
            }
            VersionError::IoError(msg) => {
                write!(f, "Version file error: {}", msg)
            }
        }
    }
}

impl std::error::Error for VersionError {}

/// Persisted version marker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionMarker {
    /// Binary version that created this data
    pub binary_version: String,
    /// WAL record format version
    pub wal_format_version: u16,
    /// Schema file format version
    pub schema_format_version: u16,
    /// Timestamp when marker was created
    pub created_at: String,
    /// Last binary version that accessed this data
    #[serde(default)]
    pub last_accessed_by: Option<String>,
}

impl VersionMarker {
    /// Create a new version marker with current versions
    pub fn new() -> Self {
        Self {
            binary_version: BINARY_VERSION.to_string(),
            wal_format_version: WAL_FORMAT_VERSION,
            schema_format_version: SCHEMA_FORMAT_VERSION,
            created_at: chrono::Utc::now().to_rfc3339(),
            last_accessed_by: None,
        }
    }

    /// Load version marker from data directory
    pub fn load(data_dir: &Path) -> Result<Option<Self>, VersionError> {
        let path = data_dir.join(VERSION_FILE);
        
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| VersionError::IoError(format!("Failed to read {}: {}", path.display(), e)))?;

        serde_json::from_str(&content)
            .map(Some)
            .map_err(|e| VersionError::IoError(format!("Failed to parse {}: {}", path.display(), e)))
    }

    /// Save version marker to data directory
    pub fn save(&self, data_dir: &Path) -> Result<(), VersionError> {
        let path = data_dir.join(VERSION_FILE);
        
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| VersionError::IoError(format!("Failed to serialize: {}", e)))?;

        fs::write(&path, content)
            .map_err(|e| VersionError::IoError(format!("Failed to write {}: {}", path.display(), e)))?;

        // fsync for durability
        let file = fs::File::open(&path)
            .map_err(|e| VersionError::IoError(format!("Failed to open for fsync: {}", e)))?;
        file.sync_all()
            .map_err(|e| VersionError::IoError(format!("Failed to fsync: {}", e)))?;

        Ok(())
    }

    /// Update last_accessed_by and save
    pub fn touch(&mut self, data_dir: &Path) -> Result<(), VersionError> {
        self.last_accessed_by = Some(BINARY_VERSION.to_string());
        self.save(data_dir)
    }
}

impl Default for VersionMarker {
    fn default() -> Self {
        Self::new()
    }
}

/// Version checker for startup safety
pub struct VersionChecker {
    data_dir: PathBuf,
}

impl VersionChecker {
    pub fn new(data_dir: impl AsRef<Path>) -> Self {
        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
        }
    }

    /// Check version compatibility on startup
    ///
    /// Returns:
    /// - `Compatible` if versions match
    /// - `NewInstallation` if no version file (first init)
    /// - `UpgradeCompatible` if upgrade is safe
    /// - `Incompatible` if versions don't match
    pub fn check(&self) -> VersionCheck {
        // First check for partial initialization
        if let Err(e) = self.check_initialization_state() {
            return VersionCheck::Incompatible(e);
        }

        // Load existing marker
        let marker = match VersionMarker::load(&self.data_dir) {
            Ok(Some(m)) => m,
            Ok(None) => return VersionCheck::NewInstallation,
            Err(e) => return VersionCheck::Incompatible(e),
        };

        // Check WAL format compatibility
        if marker.wal_format_version != WAL_FORMAT_VERSION {
            if marker.wal_format_version > WAL_FORMAT_VERSION {
                // Newer WAL format - downgrade
                return VersionCheck::Incompatible(VersionError::WalFormatMismatch {
                    expected: WAL_FORMAT_VERSION,
                    found: marker.wal_format_version,
                    message: format!(
                        "Data was written with WAL format v{} but this binary only supports v{}. \
                         This appears to be a downgrade attempt.",
                        marker.wal_format_version, WAL_FORMAT_VERSION
                    ),
                });
            } else {
                // Older WAL format - need migration
                return VersionCheck::Incompatible(VersionError::WalFormatMismatch {
                    expected: WAL_FORMAT_VERSION,
                    found: marker.wal_format_version,
                    message: format!(
                        "Data was written with WAL format v{} but this binary requires v{}. \
                         Run 'aerodb migrate' to upgrade the data format.",
                        marker.wal_format_version, WAL_FORMAT_VERSION
                    ),
                });
            }
        }

        // Check schema format compatibility
        if marker.schema_format_version != SCHEMA_FORMAT_VERSION {
            if marker.schema_format_version > SCHEMA_FORMAT_VERSION {
                return VersionCheck::Incompatible(VersionError::SchemaFormatMismatch {
                    expected: SCHEMA_FORMAT_VERSION,
                    found: marker.schema_format_version,
                    message: "Downgrade not supported.".to_string(),
                });
            } else {
                return VersionCheck::Incompatible(VersionError::SchemaFormatMismatch {
                    expected: SCHEMA_FORMAT_VERSION,
                    found: marker.schema_format_version,
                    message: "Run 'aerodb migrate' to upgrade.".to_string(),
                });
            }
        }

        // Check for binary version upgrades (informational)
        if marker.binary_version != BINARY_VERSION {
            return VersionCheck::UpgradeCompatible {
                from: marker.binary_version,
                to: BINARY_VERSION.to_string(),
            };
        }

        VersionCheck::Compatible
    }

    /// Check for partial initialization state
    fn check_initialization_state(&self) -> Result<(), VersionError> {
        let init_marker = self.data_dir.join(INIT_MARKER_FILE);
        let version_file = self.data_dir.join(VERSION_FILE);
        let wal_dir = self.data_dir.join("wal");
        let storage_dir = self.data_dir.join("storage");

        // If data directory doesn't exist, that's fine (new install)
        if !self.data_dir.exists() {
            return Ok(());
        }

        // Check for partial init: files exist but init marker is missing
        let has_data_files = wal_dir.exists() || storage_dir.exists() || version_file.exists();
        let has_init_marker = init_marker.exists();

        if has_data_files && !has_init_marker {
            return Err(VersionError::PartialInitialization {
                message: format!(
                    "Data files exist but initialization marker '{}' is missing. \
                     Previous initialization may have failed or files were tampered with.",
                    INIT_MARKER_FILE
                ),
            });
        }

        Ok(())
    }

    /// Create version marker and init marker after successful initialization
    pub fn mark_initialized(&self) -> Result<(), VersionError> {
        // Create version marker
        let marker = VersionMarker::new();
        marker.save(&self.data_dir)?;

        // Create init marker (empty file)
        let init_path = self.data_dir.join(INIT_MARKER_FILE);
        fs::write(&init_path, "")
            .map_err(|e| VersionError::IoError(format!("Failed to create init marker: {}", e)))?;

        // fsync init marker
        let file = fs::File::open(&init_path)
            .map_err(|e| VersionError::IoError(format!("Failed to open init marker for fsync: {}", e)))?;
        file.sync_all()
            .map_err(|e| VersionError::IoError(format!("Failed to fsync init marker: {}", e)))?;

        // fsync directory
        let dir = fs::File::open(&self.data_dir)
            .map_err(|e| VersionError::IoError(format!("Failed to open data dir for fsync: {}", e)))?;
        dir.sync_all()
            .map_err(|e| VersionError::IoError(format!("Failed to fsync data dir: {}", e)))?;

        Ok(())
    }

    /// Update version marker after successful startup
    pub fn update_access(&self) -> Result<(), VersionError> {
        if let Some(mut marker) = VersionMarker::load(&self.data_dir)? {
            marker.touch(&self.data_dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_new_installation() {
        let temp = TempDir::new().unwrap();
        let checker = VersionChecker::new(temp.path());
        
        match checker.check() {
            VersionCheck::NewInstallation => {}
            other => panic!("Expected NewInstallation, got {:?}", other),
        }
    }

    #[test]
    fn test_compatible_after_init() {
        let temp = TempDir::new().unwrap();
        let checker = VersionChecker::new(temp.path());
        
        // Initialize
        checker.mark_initialized().unwrap();
        
        // Should be compatible
        match checker.check() {
            VersionCheck::Compatible => {}
            other => panic!("Expected Compatible, got {:?}", other),
        }
    }

    #[test]
    fn test_version_marker_roundtrip() {
        let temp = TempDir::new().unwrap();
        let marker = VersionMarker::new();
        
        marker.save(temp.path()).unwrap();
        let loaded = VersionMarker::load(temp.path()).unwrap().unwrap();
        
        assert_eq!(loaded.binary_version, BINARY_VERSION);
        assert_eq!(loaded.wal_format_version, WAL_FORMAT_VERSION);
        assert_eq!(loaded.schema_format_version, SCHEMA_FORMAT_VERSION);
    }

    #[test]
    fn test_partial_init_detection() {
        let temp = TempDir::new().unwrap();
        
        // Create WAL dir but no init marker
        fs::create_dir_all(temp.path().join("wal")).unwrap();
        
        let checker = VersionChecker::new(temp.path());
        match checker.check() {
            VersionCheck::Incompatible(VersionError::PartialInitialization { .. }) => {}
            other => panic!("Expected PartialInitialization error, got {:?}", other),
        }
    }
}
