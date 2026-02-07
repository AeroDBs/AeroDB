//! CRC32 checksum computation for storage records
//!
//! Per STORAGE.md §6.4 and §11:
//! - Every read validates checksum
//! - Any checksum failure on read → operation abort
//! - During recovery → startup abort
//!
//! Uses CRC32 (IEEE polynomial) for checksums.

use crc32fast::Hasher;

/// Computes a CRC32 checksum over the provided data.
///
/// This function is deterministic: the same input always produces the same output.
pub fn compute_checksum(data: &[u8]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

/// Verifies that the computed checksum matches the expected checksum.
pub fn verify_checksum(data: &[u8], expected: u32) -> bool {
    compute_checksum(data) == expected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_deterministic() {
        let data = b"document storage test data";
        let checksum1 = compute_checksum(data);
        let checksum2 = compute_checksum(data);
        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn test_checksum_detects_corruption() {
        let mut data = vec![0x00, 0x01, 0x02, 0x03, 0x04];
        let original = compute_checksum(&data);
        data[2] ^= 0x01;
        let corrupted = compute_checksum(&data);
        assert_ne!(original, corrupted);
    }

    #[test]
    fn test_verify_checksum() {
        let data = b"test payload";
        let checksum = compute_checksum(data);
        assert!(verify_checksum(data, checksum));
        assert!(!verify_checksum(data, checksum ^ 1));
    }
}
