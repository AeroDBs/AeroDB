//! CRC32 checksum computation for snapshot files
//!
//! Per SNAPSHOT.md §6:
//! - Every snapshot includes CRC32 of storage.dat
//! - Every snapshot includes CRC32 of every schema file
//! - Checksums are verified during restore or recovery
//!
//! Uses CRC32 (IEEE polynomial) for checksums via crc32fast crate.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use crc32fast::Hasher;

use super::errors::{SnapshotError, SnapshotResult};

/// Computes a CRC32 checksum over the provided data.
///
/// This function is deterministic: the same input always produces the same output.
pub fn compute_checksum(data: &[u8]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

/// Computes a CRC32 checksum of an entire file.
///
/// Reads the file in chunks to handle large files efficiently.
///
/// # Arguments
///
/// * `path` - Path to the file to checksum
///
/// # Errors
///
/// Returns `SnapshotError::io_error` if the file cannot be read.
pub fn compute_file_checksum(path: &Path) -> SnapshotResult<u32> {
    let file = File::open(path).map_err(|e| SnapshotError::io_error_at_path(path, e))?;

    let mut reader = BufReader::new(file);
    let mut hasher = Hasher::new();
    let mut buffer = [0u8; 8192]; // 8KB buffer

    loop {
        let bytes_read = reader
            .read(&mut buffer)
            .map_err(|e| SnapshotError::io_error_at_path(path, e))?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize())
}

/// Formats a CRC32 checksum as a string per SNAPSHOT.md format.
///
/// Format: `crc32:XXXXXXXX` (lowercase hex, 8 characters, zero-padded)
pub fn format_checksum(checksum: u32) -> String {
    format!("crc32:{:08x}", checksum)
}

/// Parses a formatted checksum string back to u32.
///
/// Format expected: `crc32:XXXXXXXX`
///
/// # Errors
///
/// Returns `None` if the format is invalid.
pub fn parse_checksum(formatted: &str) -> Option<u32> {
    let stripped = formatted.strip_prefix("crc32:")?;
    u32::from_str_radix(stripped, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_checksum_deterministic() {
        let data = b"snapshot test data for checksum";
        let checksum1 = compute_checksum(data);
        let checksum2 = compute_checksum(data);
        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn test_checksum_detects_changes() {
        let data1 = b"original data";
        let data2 = b"modified data";
        let checksum1 = compute_checksum(data1);
        let checksum2 = compute_checksum(data2);
        assert_ne!(checksum1, checksum2);
    }

    #[test]
    fn test_file_checksum() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.dat");

        let test_data = b"file content for checksum test";
        std::fs::write(&file_path, test_data).unwrap();

        let file_checksum = compute_file_checksum(&file_path).unwrap();
        let memory_checksum = compute_checksum(test_data);

        assert_eq!(file_checksum, memory_checksum);
    }

    #[test]
    fn test_file_checksum_large_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("large.dat");

        // Create a file larger than the buffer size (8KB)
        let mut file = File::create(&file_path).unwrap();
        let chunk = [0xABu8; 1024];
        for _ in 0..100 {
            file.write_all(&chunk).unwrap();
        }
        file.sync_all().unwrap();
        drop(file);

        let checksum = compute_file_checksum(&file_path).unwrap();

        // Verify determinism
        let checksum2 = compute_file_checksum(&file_path).unwrap();
        assert_eq!(checksum, checksum2);
    }

    #[test]
    fn test_file_checksum_missing_file() {
        let path = Path::new("/nonexistent/path/file.dat");
        let result = compute_file_checksum(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_format_checksum() {
        assert_eq!(format_checksum(0xDEADBEEF), "crc32:deadbeef");
        assert_eq!(format_checksum(0xABCD1234), "crc32:abcd1234");
        assert_eq!(format_checksum(0x00000001), "crc32:00000001");
        assert_eq!(format_checksum(0x00000000), "crc32:00000000");
    }

    #[test]
    fn test_parse_checksum() {
        assert_eq!(parse_checksum("crc32:deadbeef"), Some(0xDEADBEEF));
        assert_eq!(parse_checksum("crc32:abcd1234"), Some(0xABCD1234));
        assert_eq!(parse_checksum("crc32:00000001"), Some(0x00000001));
        assert_eq!(parse_checksum("crc32:DEADBEEF"), Some(0xDEADBEEF)); // uppercase ok
    }

    #[test]
    fn test_parse_checksum_invalid() {
        assert_eq!(parse_checksum("invalid"), None);
        assert_eq!(parse_checksum("crc32:"), None);
        assert_eq!(parse_checksum("crc32:zzzz"), None);
        assert_eq!(parse_checksum("md5:deadbeef"), None);
    }

    #[test]
    fn test_format_parse_roundtrip() {
        let original: u32 = 0x12345678;
        let formatted = format_checksum(original);
        let parsed = parse_checksum(&formatted).unwrap();
        assert_eq!(original, parsed);
    }
}
