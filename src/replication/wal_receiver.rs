//! WAL Receiver Abstraction
//!
//! Per REPLICATION_LOG_FLOW.md:
//! - Replica receives WAL records
//! - Replica appends them to its WAL
//! - Strict order must be preserved
//! - Gaps are fatal and halt replication
//!
//! Per §3.2 Prefix Application Rule:
//! - Replica_WAL == Prefix(Primary_WAL)
//! - Replicas may lag, but must never skip, reorder, or invent

use super::errors::{ReplicationError, ReplicationResult};
use super::role::{HaltReason, ReplicationState};
use super::wal_sender::{WalPosition, WalRecordEnvelope};
use crate::wal::WalRecord;

/// WAL receiver state per REPLICATION_LOG_FLOW.md
#[derive(Debug)]
pub struct WalReceiver {
    /// Last applied position
    applied_position: WalPosition,
    /// Expected next sequence number
    expected_sequence: u64,
    /// Whether receiver is active
    active: bool,
}

impl WalReceiver {
    /// Create a new WAL receiver starting from a position.
    pub fn new(start_position: WalPosition) -> Self {
        Self {
            applied_position: start_position,
            expected_sequence: start_position.sequence,
            active: false,
        }
    }

    /// Create a new WAL receiver from genesis.
    pub fn from_genesis() -> Self {
        Self::new(WalPosition::genesis())
    }

    /// Create a receiver after snapshot restore.
    ///
    /// Per REPLICATION_LOG_FLOW.md §7:
    /// - WAL replay must begin strictly after snapshot boundary
    pub fn after_snapshot(snapshot_commit_sequence: u64, snapshot_offset: u64) -> Self {
        let position = WalPosition::new(snapshot_commit_sequence, snapshot_offset);
        Self {
            applied_position: position,
            expected_sequence: snapshot_commit_sequence + 1,
            active: false,
        }
    }

    /// Start the receiver.
    pub fn start(&mut self) {
        self.active = true;
    }

    /// Stop the receiver.
    pub fn stop(&mut self) {
        self.active = false;
    }

    /// Check if receiver is active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Get last applied position.
    pub fn applied_position(&self) -> WalPosition {
        self.applied_position
    }

    /// Get expected next sequence.
    pub fn expected_sequence(&self) -> u64 {
        self.expected_sequence
    }

    /// Validate and receive a WAL record envelope.
    ///
    /// Per REPLICATION_LOG_FLOW.md §5:
    /// - Gaps are detected by sequence metadata
    /// - Gaps are fatal
    ///
    /// Per §8:
    /// - WAL record integrity must be verified
    /// - CommitId monotonicity must be checked
    pub fn receive(&mut self, envelope: &WalRecordEnvelope) -> ReceiveResult {
        if !self.active {
            return ReceiveResult::NotActive;
        }

        // Per §5.1: Check for gaps
        if envelope.position.sequence < self.expected_sequence {
            // Duplicate - already received
            return ReceiveResult::Duplicate;
        }

        if envelope.position.sequence > self.expected_sequence {
            // Gap detected!
            // Per §5.2: Replica must stop applying WAL
            return ReceiveResult::GapDetected {
                expected: self.expected_sequence,
                received: envelope.position.sequence,
            };
        }

        // Sequence matches expected - this is the happy path
        // Per Stage 3: Validate checksum before accepting
        if !envelope.validate_checksum() {
            return ReceiveResult::ChecksumInvalid;
        }

        ReceiveResult::Accepted
    }

    /// Apply a received record (after validation).
    ///
    /// Per REPLICATION_LOG_FLOW.md §4.2:
    /// - Record is considered replicated only when durably appended
    pub fn apply(&mut self, envelope: &WalRecordEnvelope, record_size: u64) {
        self.applied_position = envelope.position.advance(record_size);
        self.expected_sequence = envelope.position.sequence + 1;
    }

    /// Check if a position is behind (would need catch-up).
    pub fn is_behind(&self, primary_position: WalPosition) -> bool {
        self.applied_position < primary_position
    }

    /// Calculate lag in sequence numbers.
    pub fn sequence_lag(&self, primary_sequence: u64) -> u64 {
        primary_sequence.saturating_sub(self.applied_position.sequence)
    }
}

/// Result of receiving a WAL record
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiveResult {
    /// Record accepted and ready to apply
    Accepted,

    /// Receiver is not active
    NotActive,

    /// Record is a duplicate (already received)
    Duplicate,

    /// Gap detected - fatal per REPLICATION_LOG_FLOW.md §5.2
    GapDetected { expected: u64, received: u64 },

    /// Checksum validation failed - fatal per Stage 3
    ChecksumInvalid,
}

impl ReceiveResult {
    /// Check if result is accepted.
    pub fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted)
    }

    /// Check if result is a gap (fatal).
    pub fn is_gap(&self) -> bool {
        matches!(self, Self::GapDetected { .. })
    }

    /// Check if result is a checksum failure (fatal).
    pub fn is_checksum_invalid(&self) -> bool {
        matches!(self, Self::ChecksumInvalid)
    }

    /// Check if result is fatal (gap or checksum failure).
    pub fn is_fatal(&self) -> bool {
        self.is_gap() || self.is_checksum_invalid()
    }

    /// Convert to halt reason.
    pub fn to_halt_reason(&self) -> Option<HaltReason> {
        match self {
            Self::GapDetected { .. } => Some(HaltReason::WalGapDetected),
            Self::ChecksumInvalid => Some(HaltReason::WalCorruption),
            _ => None,
        }
    }

    /// Convert to replication result.
    pub fn to_result(&self) -> ReplicationResult<()> {
        match self {
            Self::Accepted => Ok(()),
            Self::NotActive => Err(ReplicationError::configuration_error(
                "WAL receiver is not active",
            )),
            Self::Duplicate => Ok(()), // Duplicates are idempotent
            Self::GapDetected { expected, received } => Err(ReplicationError::wal_gap(format!(
                "WAL gap detected: expected sequence {}, received {}",
                expected, received
            ))),
            Self::ChecksumInvalid => Err(ReplicationError::wal_integrity_failed(
                "WAL record checksum validation failed",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receiver_starts_inactive() {
        let receiver = WalReceiver::from_genesis();
        assert!(!receiver.is_active());
    }

    #[test]
    fn test_receiver_rejects_when_inactive() {
        let mut receiver = WalReceiver::from_genesis();
        let envelope = WalRecordEnvelope::new(WalPosition::genesis(), create_test_record());

        assert_eq!(receiver.receive(&envelope), ReceiveResult::NotActive);
    }

    #[test]
    fn test_receiver_accepts_expected_sequence() {
        let mut receiver = WalReceiver::from_genesis();
        receiver.start();

        let envelope = WalRecordEnvelope::new(WalPosition::genesis(), create_test_record());

        assert_eq!(receiver.receive(&envelope), ReceiveResult::Accepted);
    }

    #[test]
    fn test_receiver_detects_gap() {
        // Per REPLICATION_LOG_FLOW.md §5
        let mut receiver = WalReceiver::from_genesis();
        receiver.start();

        // Skip sequence 0, send sequence 2
        let envelope = WalRecordEnvelope::new(WalPosition::new(2, 200), create_test_record());

        let result = receiver.receive(&envelope);
        assert!(result.is_gap());

        match result {
            ReceiveResult::GapDetected { expected, received } => {
                assert_eq!(expected, 0);
                assert_eq!(received, 2);
            }
            _ => panic!("expected gap"),
        }
    }

    #[test]
    fn test_receiver_handles_duplicate() {
        let mut receiver = WalReceiver::new(WalPosition::new(5, 500));
        receiver.start();

        // Send sequence 3 (already applied)
        let envelope = WalRecordEnvelope::new(WalPosition::new(3, 300), create_test_record());

        assert_eq!(receiver.receive(&envelope), ReceiveResult::Duplicate);
    }

    #[test]
    fn test_receiver_apply_advances_position() {
        let mut receiver = WalReceiver::from_genesis();
        receiver.start();

        let envelope = WalRecordEnvelope::new(WalPosition::genesis(), create_test_record());

        assert!(receiver.receive(&envelope).is_accepted());
        receiver.apply(&envelope, 50);

        assert_eq!(receiver.expected_sequence(), 1);
        assert_eq!(receiver.applied_position().sequence, 1);
    }

    #[test]
    fn test_gap_is_fatal() {
        // Per REPLICATION_LOG_FLOW.md §5.2
        let result = ReceiveResult::GapDetected {
            expected: 5,
            received: 10,
        };

        assert!(result.is_gap());
        assert_eq!(result.to_halt_reason(), Some(HaltReason::WalGapDetected));
        assert!(result.to_result().is_err());
    }

    fn create_test_record() -> WalRecord {
        use crate::wal::{RecordType, WalPayload};
        WalRecord {
            sequence_number: 0,
            record_type: RecordType::Insert,
            payload: WalPayload {
                collection_id: "test".to_string(),
                document_id: "doc1".to_string(),
                schema_id: "schema1".to_string(),
                schema_version: "v1".to_string(),
                document_body: vec![],
            },
        }
    }
}
