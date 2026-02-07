//! CRC32 checksum computation for WAL records
//!
//! Per WAL.md §202-212:
//! - Every WAL record includes a checksum
//! - Checksum covers record header, payload, and sequence number
//! - Any checksum mismatch is corruption
//!
//! Uses CRC32 (IEEE polynomial) for checksums.

use crc32fast::Hasher;

/// Computes a CRC32 checksum over the provided data.
///
/// This function is deterministic: the same input always produces the same output.
///
/// # Arguments
///
/// * `data` - The bytes to compute the checksum over
///
/// # Returns
///
/// A 32-bit CRC32 checksum value
pub fn compute_checksum(data: &[u8]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

/// Verifies that the computed checksum matches the expected checksum.
///
/// # Arguments
///
/// * `data` - The bytes to verify
/// * `expected` - The expected checksum value
///
/// # Returns
///
/// `true` if the checksum matches, `false` otherwise
pub fn verify_checksum(data: &[u8], expected: u32) -> bool {
    compute_checksum(data) == expected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_deterministic() {
        let data = b"test data for checksum verification";
        let checksum1 = compute_checksum(data);
        let checksum2 = compute_checksum(data);
        assert_eq!(checksum1, checksum2, "Checksum must be deterministic");
    }

    #[test]
    fn test_checksum_different_for_different_data() {
        let data1 = b"first payload";
        let data2 = b"second payload";
        let checksum1 = compute_checksum(data1);
        let checksum2 = compute_checksum(data2);
        assert_ne!(
            checksum1, checksum2,
            "Different data must produce different checksums"
        );
    }

    #[test]
    fn test_checksum_detects_single_bit_flip() {
        let mut data = vec![0x00, 0x01, 0x02, 0x03, 0x04];
        let original_checksum = compute_checksum(&data);

        // Flip a single bit
        data[2] ^= 0x01;
        let corrupted_checksum = compute_checksum(&data);

        assert_ne!(
            original_checksum, corrupted_checksum,
            "Single bit flip must change checksum"
        );
    }

    #[test]
    fn test_verify_checksum_success() {
        let data = b"payload to verify";
        let checksum = compute_checksum(data);
        assert!(verify_checksum(data, checksum));
    }

    #[test]
    fn test_verify_checksum_failure() {
        let data = b"payload to verify";
        let wrong_checksum = compute_checksum(data) ^ 0x1;
        assert!(!verify_checksum(data, wrong_checksum));
    }

    #[test]
    fn test_empty_data_has_consistent_checksum() {
        let empty: &[u8] = &[];
        let checksum1 = compute_checksum(empty);
        let checksum2 = compute_checksum(empty);
        assert_eq!(checksum1, checksum2);
    }
}
