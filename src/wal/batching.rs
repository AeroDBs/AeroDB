//! WAL Batching Optimization
//!
//! Per WAL_BATCHING.md:
//! - Multiple WAL records written in a single physical write
//! - Preserves logical record boundaries and ordering
//! - Each record individually checksummed and parseable
//!
//! Per §4.3 Batch Formation Rules:
//! - Batches formed by sequential availability of serialized records
//! - NO timers, load heuristics, or dynamic resizing
//! - Batch size explicitly bounded and configuration-defined
//!
//! Per §9.1 Disablement:
//! - Disableable via compile-time flag or startup config
//!
//! Per §6: "WAL is indistinguishable from baseline emission"

use std::io::{self, Write};

/// Configuration for WAL batching.
///
/// Per WAL_BATCHING.md §4.3:
/// - Batch size MUST be explicitly bounded
/// - Batch size MUST be deterministic
/// - Batch size MUST be configuration-defined or compile-time defined
#[derive(Debug, Clone)]
pub struct WalBatchConfig {
    /// Whether WAL batching is enabled.
    /// When false, each record is written immediately (baseline behavior).
    pub enabled: bool,
    /// Maximum number of records in a batch.
    /// Per §4.3: "Batch size MUST be explicitly bounded"
    pub max_records: usize,
    /// Maximum total bytes in a batch buffer.
    /// This provides a safety limit to prevent unbounded memory usage.
    pub max_bytes: usize,
}

impl Default for WalBatchConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Conservative default: disabled
            max_records: 16,
            max_bytes: 64 * 1024, // 64KB default
        }
    }
}

impl WalBatchConfig {
    /// Create config with batching enabled.
    pub fn enabled(max_records: usize, max_bytes: usize) -> Self {
        Self {
            enabled: true,
            max_records,
            max_bytes,
        }
    }

    /// Create config with batching disabled (baseline behavior).
    pub fn disabled() -> Self {
        Self::default()
    }
}

/// A batch of WAL records awaiting flush.
///
/// Per WAL_BATCHING.md §4.2:
/// 1. Serialize WAL record A
/// 2. Serialize WAL record B
/// 3. Serialize WAL record C
/// 4. Concatenate serialized records into contiguous buffer
/// 5. Perform one write() for the buffer
#[derive(Debug)]
pub struct WalBatch {
    /// Concatenated serialized records.
    buffer: Vec<u8>,
    /// Number of records in this batch.
    record_count: usize,
    /// Sequence numbers of records in this batch.
    sequence_numbers: Vec<u64>,
}

impl WalBatch {
    /// Create a new empty batch.
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            record_count: 0,
            sequence_numbers: Vec::new(),
        }
    }

    /// Create with pre-allocated capacity.
    pub fn with_capacity(bytes: usize, records: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(bytes),
            record_count: 0,
            sequence_numbers: Vec::with_capacity(records),
        }
    }

    /// Add a serialized record to the batch.
    ///
    /// Per WAL_BATCHING.md §4.2:
    /// - Serialization order MUST match commit order
    /// - No record may be split across buffers
    pub fn add_record(&mut self, serialized: &[u8], sequence_number: u64) {
        self.buffer.extend_from_slice(serialized);
        self.record_count += 1;
        self.sequence_numbers.push(sequence_number);
    }

    /// Get the current buffer contents.
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Get the number of records in this batch.
    pub fn record_count(&self) -> usize {
        self.record_count
    }

    /// Get the current buffer size in bytes.
    pub fn buffer_size(&self) -> usize {
        self.buffer.len()
    }

    /// Get the sequence numbers of records in this batch.
    pub fn sequence_numbers(&self) -> &[u64] {
        &self.sequence_numbers
    }

    /// Check if batch is empty.
    pub fn is_empty(&self) -> bool {
        self.record_count == 0
    }

    /// Clear the batch for reuse.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.record_count = 0;
        self.sequence_numbers.clear();
    }
}

impl Default for WalBatch {
    fn default() -> Self {
        Self::new()
    }
}

/// WAL Batcher - accumulates records for batched writing.
///
/// Per WAL_BATCHING.md §4.2:
/// - Concatenate serialized records into contiguous buffer
/// - Perform one write() for the buffer
///
/// Per §3.1:
/// - Each WAL record remains individually checksummed
/// - Each WAL record remains individually parseable
/// - Each WAL record remains individually replayable
#[derive(Debug)]
pub struct WalBatcher {
    /// Configuration.
    config: WalBatchConfig,
    /// Current batch being accumulated.
    current_batch: WalBatch,
}

impl WalBatcher {
    /// Create a new WAL batcher.
    pub fn new(config: WalBatchConfig) -> Self {
        let current_batch = if config.enabled {
            WalBatch::with_capacity(config.max_bytes, config.max_records)
        } else {
            WalBatch::new()
        };
        Self {
            config,
            current_batch,
        }
    }

    /// Check if batching is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Current batch record count.
    pub fn pending_records(&self) -> usize {
        self.current_batch.record_count()
    }

    /// Current batch buffer size.
    pub fn pending_bytes(&self) -> usize {
        self.current_batch.buffer_size()
    }

    /// Check if the batch is full and needs flushing.
    ///
    /// Per WAL_BATCHING.md §4.3:
    /// - Batch size MUST be explicitly bounded
    pub fn should_flush(&self, next_record_size: usize) -> bool {
        if !self.config.enabled {
            return true; // Baseline: always flush immediately
        }

        // Check record count limit
        if self.current_batch.record_count() >= self.config.max_records {
            return true;
        }

        // Check bytes limit (would exceed if we add next record)
        if self.current_batch.buffer_size() + next_record_size > self.config.max_bytes {
            return true;
        }

        false
    }

    /// Add a serialized record to the current batch.
    ///
    /// Returns true if the batch needs to be flushed after this add.
    pub fn add_record(&mut self, serialized: &[u8], sequence_number: u64) -> bool {
        self.current_batch.add_record(serialized, sequence_number);
        self.should_flush(0) // Check if we need to flush now
    }

    /// Write the current batch to the provided writer.
    ///
    /// Per WAL_BATCHING.md §4.2:
    /// - Perform one write() for the buffer
    ///
    /// Per §3.1:
    /// - Record boundaries are preserved in the byte stream
    pub fn flush<W: Write>(&mut self, writer: &mut W) -> io::Result<usize> {
        if self.current_batch.is_empty() {
            return Ok(0);
        }

        let bytes_written = self.current_batch.buffer_size();
        writer.write_all(self.current_batch.buffer())?;
        self.current_batch.clear();

        Ok(bytes_written)
    }

    /// Get sequence numbers of the pending batch.
    pub fn pending_sequence_numbers(&self) -> Vec<u64> {
        self.current_batch.sequence_numbers().to_vec()
    }
}

/// Batch write result for tracking.
#[derive(Debug, Clone)]
pub struct BatchWriteResult {
    /// Number of records written.
    pub records_written: usize,
    /// Number of bytes written.
    pub bytes_written: usize,
    /// Sequence numbers of written records.
    pub sequence_numbers: Vec<u64>,
}

/// Check whether a write path should use baseline or batched writing.
///
/// Per WAL_BATCHING.md §9.1:
/// - When disabled, one write per WAL record
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritePath {
    /// Baseline: one write per record
    Baseline,
    /// Batched: accumulate and write together
    Batched,
}

impl WritePath {
    /// Determine write path based on config.
    pub fn from_config(config: &WalBatchConfig) -> Self {
        if config.enabled {
            Self::Batched
        } else {
            Self::Baseline
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn test_record(seq: u64) -> Vec<u8> {
        // Simple mock record: just sequence number as bytes
        seq.to_le_bytes().to_vec()
    }

    // ==================== WalBatchConfig Tests ====================

    #[test]
    fn test_config_default_disabled() {
        let config = WalBatchConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.max_records, 16);
        assert_eq!(config.max_bytes, 64 * 1024);
    }

    #[test]
    fn test_config_enabled() {
        let config = WalBatchConfig::enabled(32, 128 * 1024);
        assert!(config.enabled);
        assert_eq!(config.max_records, 32);
        assert_eq!(config.max_bytes, 128 * 1024);
    }

    #[test]
    fn test_config_disabled() {
        let config = WalBatchConfig::disabled();
        assert!(!config.enabled);
    }

    // ==================== WalBatch Tests ====================

    #[test]
    fn test_batch_new_empty() {
        let batch = WalBatch::new();
        assert!(batch.is_empty());
        assert_eq!(batch.record_count(), 0);
        assert_eq!(batch.buffer_size(), 0);
    }

    #[test]
    fn test_batch_add_record() {
        let mut batch = WalBatch::new();
        let record = test_record(1);

        batch.add_record(&record, 1);

        assert!(!batch.is_empty());
        assert_eq!(batch.record_count(), 1);
        assert_eq!(batch.buffer_size(), 8); // u64 = 8 bytes
        assert_eq!(batch.sequence_numbers(), &[1]);
    }

    #[test]
    fn test_batch_multiple_records() {
        let mut batch = WalBatch::new();

        batch.add_record(&test_record(1), 1);
        batch.add_record(&test_record(2), 2);
        batch.add_record(&test_record(3), 3);

        assert_eq!(batch.record_count(), 3);
        assert_eq!(batch.buffer_size(), 24); // 3 * 8 bytes
        assert_eq!(batch.sequence_numbers(), &[1, 2, 3]);
    }

    #[test]
    fn test_batch_clear() {
        let mut batch = WalBatch::new();
        batch.add_record(&test_record(1), 1);
        batch.add_record(&test_record(2), 2);

        batch.clear();

        assert!(batch.is_empty());
        assert_eq!(batch.record_count(), 0);
        assert_eq!(batch.buffer_size(), 0);
    }

    #[test]
    fn test_batch_buffer_concatenation() {
        let mut batch = WalBatch::new();
        batch.add_record(&test_record(1), 1);
        batch.add_record(&test_record(2), 2);

        // Buffer should be concatenation of records
        let expected: Vec<u8> = 1u64
            .to_le_bytes()
            .iter()
            .chain(2u64.to_le_bytes().iter())
            .copied()
            .collect();

        assert_eq!(batch.buffer(), &expected[..]);
    }

    // ==================== WalBatcher Tests ====================

    #[test]
    fn test_batcher_disabled() {
        let batcher = WalBatcher::new(WalBatchConfig::disabled());
        assert!(!batcher.is_enabled());
    }

    #[test]
    fn test_batcher_enabled() {
        let batcher = WalBatcher::new(WalBatchConfig::enabled(16, 1024));
        assert!(batcher.is_enabled());
    }

    #[test]
    fn test_batcher_baseline_always_flushes() {
        let batcher = WalBatcher::new(WalBatchConfig::disabled());
        // Baseline should always indicate flush
        assert!(batcher.should_flush(8));
    }

    #[test]
    fn test_batcher_flush_on_record_limit() {
        let mut batcher = WalBatcher::new(WalBatchConfig::enabled(2, 1024));

        batcher.add_record(&test_record(1), 1);
        assert!(!batcher.should_flush(8));

        batcher.add_record(&test_record(2), 2);
        assert!(batcher.should_flush(8)); // Now at limit
    }

    #[test]
    fn test_batcher_flush_on_byte_limit() {
        let mut batcher = WalBatcher::new(WalBatchConfig::enabled(100, 20));

        batcher.add_record(&test_record(1), 1); // 8 bytes
        assert!(!batcher.should_flush(8));

        batcher.add_record(&test_record(2), 2); // 16 bytes total
                                                // Next record would make it 24 bytes, over 20 limit
        assert!(batcher.should_flush(8));
    }

    #[test]
    fn test_batcher_flush_writes_all() {
        let mut batcher = WalBatcher::new(WalBatchConfig::enabled(16, 1024));
        batcher.add_record(&test_record(1), 1);
        batcher.add_record(&test_record(2), 2);

        let mut writer = Cursor::new(Vec::new());
        let bytes = batcher.flush(&mut writer).unwrap();

        assert_eq!(bytes, 16);
        assert!(batcher.pending_records() == 0);
        assert_eq!(writer.into_inner().len(), 16);
    }

    #[test]
    fn test_batcher_flush_empty() {
        let mut batcher = WalBatcher::new(WalBatchConfig::enabled(16, 1024));

        let mut writer = Cursor::new(Vec::new());
        let bytes = batcher.flush(&mut writer).unwrap();

        assert_eq!(bytes, 0);
    }

    #[test]
    fn test_batcher_pending_sequence_numbers() {
        let mut batcher = WalBatcher::new(WalBatchConfig::enabled(16, 1024));
        batcher.add_record(&test_record(5), 5);
        batcher.add_record(&test_record(6), 6);
        batcher.add_record(&test_record(7), 7);

        let seqs = batcher.pending_sequence_numbers();
        assert_eq!(seqs, vec![5, 6, 7]);
    }

    // ==================== WritePath Tests ====================

    #[test]
    fn test_write_path_baseline() {
        let config = WalBatchConfig::disabled();
        assert_eq!(WritePath::from_config(&config), WritePath::Baseline);
    }

    #[test]
    fn test_write_path_batched() {
        let config = WalBatchConfig::enabled(16, 1024);
        assert_eq!(WritePath::from_config(&config), WritePath::Batched);
    }

    // ==================== Equivalence Tests ====================

    /// Per WAL_BATCHING.md §6:
    /// "WAL byte stream is a strict concatenation of baseline records"
    #[test]
    fn test_batch_produces_same_bytes_as_sequential() {
        // Baseline: write each record separately
        let mut baseline_writer = Cursor::new(Vec::new());
        baseline_writer.write_all(&test_record(1)).unwrap();
        baseline_writer.write_all(&test_record(2)).unwrap();
        baseline_writer.write_all(&test_record(3)).unwrap();
        let baseline_bytes = baseline_writer.into_inner();

        // Batched: write all at once
        let mut batcher = WalBatcher::new(WalBatchConfig::enabled(16, 1024));
        batcher.add_record(&test_record(1), 1);
        batcher.add_record(&test_record(2), 2);
        batcher.add_record(&test_record(3), 3);

        let mut batched_writer = Cursor::new(Vec::new());
        batcher.flush(&mut batched_writer).unwrap();
        let batched_bytes = batched_writer.into_inner();

        // Must be byte-for-byte identical
        assert_eq!(baseline_bytes, batched_bytes);
    }

    /// Per WAL_BATCHING.md §7.2:
    /// "Partial records are detected by checksum"
    #[test]
    fn test_records_individually_parseable() {
        let mut batch = WalBatch::new();
        batch.add_record(&test_record(100), 100);
        batch.add_record(&test_record(200), 200);

        let buffer = batch.buffer();

        // Should be able to parse each record from the buffer
        let record1 = u64::from_le_bytes(buffer[0..8].try_into().unwrap());
        let record2 = u64::from_le_bytes(buffer[8..16].try_into().unwrap());

        assert_eq!(record1, 100);
        assert_eq!(record2, 200);
    }
}
