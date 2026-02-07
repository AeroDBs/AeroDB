//! # Migration Checksum
//!
//! MANIFESTO ALIGNMENT: Deterministic checksum for tamper detection.
//!
//! Per Design Manifesto: "fail loudly, execute predictably, leave no surprises."
//!
//! This module provides CRC32 checksums for migration files to detect
//! any manual modifications after creation. This ensures migrations
//! remain deterministic and auditable.

use crc32fast::Hasher;

/// Compute CRC32 checksum for migration content
///
/// MANIFESTO ALIGNMENT: Deterministic checksum generation.
/// Same content always produces same checksum.
pub fn compute_checksum(content: &str) -> String {
    let mut hasher = Hasher::new();
    hasher.update(content.as_bytes());
    let checksum = hasher.finalize();
    format!("crc32:{:08X}", checksum)
}

/// Verify checksum matches content
///
/// MANIFESTO ALIGNMENT: Explicit verification, no silent corruption.
pub fn verify_checksum(content: &str, expected: &str) -> bool {
    let actual = compute_checksum(content);
    actual == expected
}

/// Extract checksum value from formatted string
///
/// Parses "crc32:ABC12345" format.
pub fn parse_checksum(formatted: &str) -> Option<u32> {
    if !formatted.starts_with("crc32:") {
        return None;
    }

    let hex_part = &formatted[6..];
    u32::from_str_radix(hex_part, 16).ok()
}

/// Generate checksum for migration file content
///
/// This function:
/// 1. Removes existing checksum line (if present)
/// 2. Computes checksum of remaining content
/// 3. Returns the checksum string
pub fn generate_checksum_for_file(content: &str) -> String {
    // Remove checksum line for computation
    let content_without_checksum: String = content
        .lines()
        .filter(|line| !line.starts_with("checksum:"))
        .collect::<Vec<_>>()
        .join("\n");

    compute_checksum(&content_without_checksum)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_checksum_deterministic() {
        let content = "hello world";
        let c1 = compute_checksum(content);
        let c2 = compute_checksum(content);
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_compute_checksum_format() {
        let checksum = compute_checksum("test");
        assert!(checksum.starts_with("crc32:"));
        assert_eq!(checksum.len(), 6 + 8); // "crc32:" + 8 hex digits
    }

    #[test]
    fn test_verify_checksum_valid() {
        let content = "migration content";
        let checksum = compute_checksum(content);
        assert!(verify_checksum(content, &checksum));
    }

    #[test]
    fn test_verify_checksum_invalid() {
        let content = "migration content";
        let checksum = compute_checksum(content);
        assert!(!verify_checksum("modified content", &checksum));
    }

    #[test]
    fn test_parse_checksum_valid() {
        let result = parse_checksum("crc32:ABC12345");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), 0xABC12345);
    }

    #[test]
    fn test_parse_checksum_invalid_format() {
        assert!(parse_checksum("md5:ABC12345").is_none());
        assert!(parse_checksum("ABC12345").is_none());
    }

    #[test]
    fn test_generate_checksum_excludes_checksum_line() {
        let content = "version: 1\nchecksum: crc32:OLD\nname: test";
        let checksum = generate_checksum_for_file(content);

        // Should compute checksum WITHOUT the checksum line
        let expected = compute_checksum("version: 1\nname: test");
        assert_eq!(checksum, expected);
    }
}
