//! Crash testing utilities
//!
//! Per CRASH_TESTING.md, these utilities support:
//! - Creating temp directories
//! - Validating post-crash state
//! - Checking invariants

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Create a temporary data directory for crash testing
pub fn create_temp_data_dir(prefix: &str) -> PathBuf {
    let temp_dir = std::env::temp_dir();
    let unique_name = format!("aerodb_crash_test_{}_{}", prefix, std::process::id());
    let data_dir = temp_dir.join(unique_name);

    if data_dir.exists() {
        let _ = fs::remove_dir_all(&data_dir);
    }

    fs::create_dir_all(&data_dir).expect("Failed to create temp data dir");
    fs::create_dir_all(data_dir.join("wal")).expect("Failed to create wal dir");
    fs::create_dir_all(data_dir.join("data")).expect("Failed to create data dir");
    fs::create_dir_all(data_dir.join("snapshots")).expect("Failed to create snapshots dir");
    fs::create_dir_all(data_dir.join("metadata/schemas")).expect("Failed to create schemas dir");

    data_dir
}

/// Cleanup a temp data directory
pub fn cleanup_temp_data_dir(data_dir: &Path) {
    if data_dir.exists() {
        let _ = fs::remove_dir_all(data_dir);
    }
}

/// Validate WAL integrity after crash
pub fn validate_wal_integrity(data_dir: &Path) -> Result<(), String> {
    let wal_dir = data_dir.join("wal");

    if !wal_dir.exists() {
        return Err("WAL directory does not exist".to_string());
    }

    // Check wal.log exists
    let wal_log = wal_dir.join("wal.log");
    if wal_log.exists() {
        // Try to read to verify not corrupted
        let mut file = File::open(&wal_log).map_err(|e| format!("Cannot open wal.log: {}", e))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| format!("Cannot read wal.log: {}", e))?;
    }

    Ok(())
}

/// Validate snapshot integrity after crash
pub fn validate_snapshot_integrity(data_dir: &Path) -> Result<(), String> {
    let snapshots_dir = data_dir.join("snapshots");

    if !snapshots_dir.exists() {
        // No snapshots is valid
        return Ok(());
    }

    // Check each snapshot has a manifest
    for entry in
        fs::read_dir(&snapshots_dir).map_err(|e| format!("Cannot read snapshots dir: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Cannot read entry: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            let manifest = path.join("manifest.json");
            if !manifest.exists() {
                // Incomplete snapshot - this is valid after crash
                // The snapshot should be ignored during recovery
                continue;
            }

            // Validate manifest is valid JSON
            let mut file =
                File::open(&manifest).map_err(|e| format!("Cannot open manifest: {}", e))?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .map_err(|e| format!("Cannot read manifest: {}", e))?;

            let _: serde_json::Value = serde_json::from_str(&contents)
                .map_err(|e| format!("Invalid manifest JSON: {}", e))?;
        }
    }

    Ok(())
}

/// Validate no partial files exist
pub fn validate_no_partial_files(data_dir: &Path) -> Result<(), String> {
    // Check for .tmp files that would indicate partial writes
    fn check_dir(dir: &Path) -> Result<(), String> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in
            fs::read_dir(dir).map_err(|e| format!("Cannot read dir {}: {}", dir.display(), e))?
        {
            let entry = entry.map_err(|e| format!("Cannot read entry: {}", e))?;
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if name.ends_with(".tmp") || name.ends_with(".partial") {
                return Err(format!("Partial file found: {}", path.display()));
            }

            if path.is_dir() {
                check_dir(&path)?;
            }
        }

        Ok(())
    }

    check_dir(data_dir)
}

/// Validate backup archive integrity
pub fn validate_backup_integrity(backup_path: &Path) -> Result<(), String> {
    if !backup_path.exists() {
        // No backup created is valid during crash
        return Ok(());
    }

    // Try to open as tar archive
    let file = File::open(backup_path).map_err(|e| format!("Cannot open backup: {}", e))?;

    let mut archive = tar::Archive::new(file);
    let entries = archive
        .entries()
        .map_err(|e| format!("Cannot read backup entries: {}", e))?;

    // Count entries to verify archive is readable
    let mut count = 0;
    for entry in entries {
        entry.map_err(|e| format!("Corrupt backup entry: {}", e))?;
        count += 1;
    }

    if count == 0 {
        return Err("Backup archive is empty".to_string());
    }

    Ok(())
}

/// Write test data to WAL for crash testing
pub fn write_test_wal_record(data_dir: &Path, data: &[u8]) -> Result<(), String> {
    let wal_log = data_dir.join("wal/wal.log");

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&wal_log)
        .map_err(|e| format!("Cannot open wal.log: {}", e))?;

    file.write_all(data)
        .map_err(|e| format!("Cannot write to wal.log: {}", e))?;

    file.sync_all()
        .map_err(|e| format!("Cannot fsync wal.log: {}", e))?;

    Ok(())
}

/// Read test data from WAL
pub fn read_wal_contents(data_dir: &Path) -> Result<Vec<u8>, String> {
    let wal_log = data_dir.join("wal/wal.log");

    if !wal_log.exists() {
        return Ok(Vec::new());
    }

    let mut file = File::open(&wal_log).map_err(|e| format!("Cannot open wal.log: {}", e))?;

    let mut contents = Vec::new();
    file.read_to_end(&mut contents)
        .map_err(|e| format!("Cannot read wal.log: {}", e))?;

    Ok(contents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_temp_data_dir() {
        let data_dir = create_temp_data_dir("test");
        assert!(data_dir.exists());
        assert!(data_dir.join("wal").exists());
        assert!(data_dir.join("data").exists());
        assert!(data_dir.join("snapshots").exists());
        cleanup_temp_data_dir(&data_dir);
        assert!(!data_dir.exists());
    }

    #[test]
    fn test_validate_wal_integrity() {
        let data_dir = create_temp_data_dir("wal_test");

        // Empty WAL is valid
        let result = validate_wal_integrity(&data_dir);
        assert!(result.is_ok());

        // Write some data
        write_test_wal_record(&data_dir, b"test data").unwrap();
        let result = validate_wal_integrity(&data_dir);
        assert!(result.is_ok());

        cleanup_temp_data_dir(&data_dir);
    }

    #[test]
    fn test_validate_no_partial_files() {
        let data_dir = create_temp_data_dir("partial_test");

        // No partial files is valid
        let result = validate_no_partial_files(&data_dir);
        assert!(result.is_ok());

        // Create a partial file
        File::create(data_dir.join("test.tmp")).unwrap();
        let result = validate_no_partial_files(&data_dir);
        assert!(result.is_err());

        cleanup_temp_data_dir(&data_dir);
    }
}
