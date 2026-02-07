//! WAL Sender Abstraction
//!
//! Per REPLICATION_LOG_FLOW.md §4:
//! - Primary emits WAL records
//! - Records must be sent verbatim
//! - Order must be preserved
//! - No re-encoding, reordering, or inference

use super::errors::{ReplicationError, ReplicationResult};
use crate::wal::WalRecord;

/// WAL position tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WalPosition {
    /// Sequence number of the WAL record
    pub sequence: u64,
    /// Byte offset in WAL file
    pub offset: u64,
}

impl WalPosition {
    /// Create a new WAL position.
    pub fn new(sequence: u64, offset: u64) -> Self {
        Self { sequence, offset }
    }

    /// Genesis position (start of WAL).
    pub fn genesis() -> Self {
        Self {
            sequence: 0,
            offset: 0,
        }
    }

    /// Advance to next position.
    pub fn advance(&self, record_size: u64) -> Self {
        Self {
            sequence: self.sequence + 1,
            offset: self.offset + record_size,
        }
    }
}

/// WAL sender state per REPLICATION_LOG_FLOW.md §4.1
#[derive(Debug)]
pub struct WalSender {
    /// Current position in WAL being sent
    current_position: WalPosition,
    /// Last acknowledged position by replica
    ack_position: WalPosition,
    /// Whether sender is active
    active: bool,
}

impl WalSender {
    /// Create a new WAL sender starting from a position.
    pub fn new(start_position: WalPosition) -> Self {
        Self {
            current_position: start_position,
            ack_position: start_position,
            active: false,
        }
    }

    /// Create a new WAL sender from genesis.
    pub fn from_genesis() -> Self {
        Self::new(WalPosition::genesis())
    }

    /// Start the sender.
    pub fn start(&mut self) {
        self.active = true;
    }

    /// Stop the sender.
    pub fn stop(&mut self) {
        self.active = false;
    }

    /// Check if sender is active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Get current sending position.
    pub fn current_position(&self) -> WalPosition {
        self.current_position
    }

    /// Get last acknowledged position.
    pub fn ack_position(&self) -> WalPosition {
        self.ack_position
    }

    /// Prepare a record for sending.
    ///
    /// Per REPLICATION_LOG_FLOW.md §2.1:
    /// - WAL records are sent verbatim
    /// - No re-encoding allowed
    pub fn prepare_record(&self, record: &WalRecord) -> ReplicationResult<WalRecordEnvelope> {
        if !self.active {
            return Err(ReplicationError::configuration_error(
                "WAL sender is not active",
            ));
        }

        Ok(WalRecordEnvelope::new(
            self.current_position,
            record.clone(),
        ))
    }

    /// Mark a record as sent and advance position.
    pub fn record_sent(&mut self, record_size: u64) {
        self.current_position = self.current_position.advance(record_size);
    }

    /// Handle acknowledgment from replica.
    pub fn handle_ack(&mut self, acked_position: WalPosition) -> ReplicationResult<()> {
        // Ack must be monotonically increasing
        if acked_position < self.ack_position {
            return Err(ReplicationError::history_divergence(
                "received acknowledgment for already-acked position",
            ));
        }

        // Ack cannot be ahead of what we sent
        if acked_position > self.current_position {
            return Err(ReplicationError::history_divergence(
                "received acknowledgment for position not yet sent",
            ));
        }

        self.ack_position = acked_position;
        Ok(())
    }
}

/// Envelope for WAL record transmission
///
/// Per Stage 3: Includes checksum for validation before application.
#[derive(Debug, Clone)]
pub struct WalRecordEnvelope {
    /// Position of this record
    pub position: WalPosition,
    /// The WAL record (verbatim)
    pub record: WalRecord,
    /// CRC32 checksum of the record for validation
    pub checksum: u32,
}

impl WalRecordEnvelope {
    /// Create a new envelope with computed checksum.
    pub fn new(position: WalPosition, record: WalRecord) -> Self {
        // Simple checksum based on sequence for now
        // Real implementation would compute CRC32 of serialized record
        let checksum = compute_record_checksum(&record, position.sequence);
        Self {
            position,
            record,
            checksum,
        }
    }

    /// Validate the envelope's checksum.
    ///
    /// Per Stage 3: Must validate before application.
    pub fn validate_checksum(&self) -> bool {
        let expected = compute_record_checksum(&self.record, self.position.sequence);
        self.checksum == expected
    }
}

/// Compute a checksum for a WAL record.
fn compute_record_checksum(record: &WalRecord, sequence: u64) -> u32 {
    // Simple hash combining sequence and record fields
    let mut hash = sequence as u32;
    hash = hash.wrapping_add(record.sequence_number as u32);
    hash = hash
        .wrapping_mul(31)
        .wrapping_add(record.payload.collection_id.len() as u32);
    hash = hash
        .wrapping_mul(31)
        .wrapping_add(record.payload.document_id.len() as u32);
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wal_position_advance() {
        let pos = WalPosition::new(10, 1000);
        let next = pos.advance(50);

        assert_eq!(next.sequence, 11);
        assert_eq!(next.offset, 1050);
    }

    #[test]
    fn test_sender_starts_inactive() {
        let sender = WalSender::from_genesis();
        assert!(!sender.is_active());
    }

    #[test]
    fn test_sender_can_start_and_stop() {
        let mut sender = WalSender::from_genesis();
        sender.start();
        assert!(sender.is_active());
        sender.stop();
        assert!(!sender.is_active());
    }

    #[test]
    fn test_ack_must_be_monotonic() {
        let mut sender = WalSender::new(WalPosition::new(5, 500));
        sender.start();

        // First ack at position 5
        assert!(sender.handle_ack(WalPosition::new(5, 500)).is_ok());

        // Cannot ack an earlier position
        assert!(sender.handle_ack(WalPosition::new(3, 300)).is_err());
    }

    #[test]
    fn test_ack_cannot_exceed_sent() {
        let mut sender = WalSender::new(WalPosition::new(5, 500));
        sender.start();

        // Cannot ack a position we haven't sent yet
        assert!(sender.handle_ack(WalPosition::new(10, 1000)).is_err());
    }
}
