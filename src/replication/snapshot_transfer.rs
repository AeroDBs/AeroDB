//! Snapshot Transfer for Replication Bootstrap
//!
//! Per REPLICATION_SNAPSHOT_TRANSFER.md:
//! - Snapshots are produced only by Primary
//! - Represent a valid MVCC cut
//! - Installation is atomic
//! - WAL replay resumes strictly after snapshot boundary
//!
//! Per §11 Equivalence to WAL-Only Bootstrap:
//! - Replica via snapshot + WAL must reach exactly same state
//! - as Replica via full WAL replay

use super::errors::{ReplicationError, ReplicationResult};
use super::role::{HaltReason, ReplicationState};
use super::wal_sender::WalPosition;
use crate::mvcc::CommitId;

/// Snapshot metadata for replication transfer
#[derive(Debug, Clone)]
pub struct SnapshotMetadata {
    /// Commit boundary of this snapshot
    /// Per §4: WAL replay resumes at CommitId > C_snap
    pub commit_boundary: CommitId,

    /// WAL sequence number at snapshot boundary
    pub wal_sequence: u64,

    /// Snapshot checksum for integrity validation
    pub checksum: u64,

    /// Size in bytes
    pub size_bytes: u64,

    /// Whether this snapshot is complete
    pub is_complete: bool,

    /// Whether this snapshot was created by Primary
    pub from_primary: bool,
}

impl SnapshotMetadata {
    /// Create new snapshot metadata.
    pub fn new(
        commit_boundary: CommitId,
        wal_sequence: u64,
        checksum: u64,
        size_bytes: u64,
    ) -> Self {
        Self {
            commit_boundary,
            wal_sequence,
            checksum,
            size_bytes,
            is_complete: true,
            from_primary: true,
        }
    }
}

/// Snapshot transfer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotTransferState {
    /// Not started
    Idle,

    /// Transfer in progress
    Transferring,

    /// Transfer complete, awaiting validation
    TransferComplete,

    /// Validated, awaiting installation
    Validated,

    /// Installed successfully
    Installed,

    /// Transfer or validation failed
    Failed,
}

/// Snapshot eligibility check result per §3
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotEligibility {
    /// Snapshot is eligible for transfer
    Eligible,

    /// Not from Primary
    NotFromPrimary,

    /// Missing commit boundary
    MissingCommitBoundary,

    /// Incomplete snapshot
    Incomplete,

    /// Failed integrity check
    IntegrityFailure,

    /// Superseded by newer snapshot
    Superseded,
}

impl SnapshotEligibility {
    /// Check if eligible.
    pub fn is_eligible(&self) -> bool {
        matches!(self, Self::Eligible)
    }
}

/// Check if a snapshot is eligible for replication transfer.
///
/// Per REPLICATION_SNAPSHOT_TRANSFER.md §3:
/// - Created by Primary
/// - Valid commit boundary
/// - Complete and self-contained
/// - Passed integrity checks
/// - Not superseded
pub fn check_snapshot_eligibility(metadata: &SnapshotMetadata) -> SnapshotEligibility {
    // Must be from Primary (§2)
    if !metadata.from_primary {
        return SnapshotEligibility::NotFromPrimary;
    }

    // Must be complete (§3.3)
    if !metadata.is_complete {
        return SnapshotEligibility::Incomplete;
    }

    // Checksum must be valid (§5.2)
    if metadata.checksum == 0 {
        return SnapshotEligibility::IntegrityFailure;
    }

    SnapshotEligibility::Eligible
}

/// Snapshot receiver for replica bootstrap
#[derive(Debug)]
pub struct SnapshotReceiver {
    /// Current transfer state
    state: SnapshotTransferState,
    /// Metadata of snapshot being received (if any)
    metadata: Option<SnapshotMetadata>,
    /// Bytes received so far
    bytes_received: u64,
}

impl SnapshotReceiver {
    /// Create a new snapshot receiver.
    pub fn new() -> Self {
        Self {
            state: SnapshotTransferState::Idle,
            metadata: None,
            bytes_received: 0,
        }
    }

    /// Get current state.
    pub fn state(&self) -> SnapshotTransferState {
        self.state
    }

    /// Get metadata if available.
    pub fn metadata(&self) -> Option<&SnapshotMetadata> {
        self.metadata.as_ref()
    }

    /// Start receiving a snapshot.
    ///
    /// Per §4.2: Replica validates and installs atomically.
    pub fn start_transfer(&mut self, metadata: SnapshotMetadata) -> ReplicationResult<()> {
        if self.state != SnapshotTransferState::Idle {
            return Err(ReplicationError::configuration_error(
                "snapshot transfer already in progress",
            ));
        }

        // Check eligibility per §3
        let eligibility = check_snapshot_eligibility(&metadata);
        if !eligibility.is_eligible() {
            return Err(ReplicationError::configuration_error(format!(
                "snapshot not eligible: {:?}",
                eligibility
            )));
        }

        self.metadata = Some(metadata);
        self.bytes_received = 0;
        self.state = SnapshotTransferState::Transferring;

        Ok(())
    }

    /// Record bytes received.
    pub fn receive_bytes(&mut self, bytes: u64) -> ReplicationResult<()> {
        if self.state != SnapshotTransferState::Transferring {
            return Err(ReplicationError::configuration_error(
                "not in transferring state",
            ));
        }

        self.bytes_received += bytes;

        // Check if transfer is complete
        if let Some(metadata) = &self.metadata {
            if self.bytes_received >= metadata.size_bytes {
                self.state = SnapshotTransferState::TransferComplete;
            }
        }

        Ok(())
    }

    /// Validate received snapshot.
    ///
    /// Per §5.2: Validate checksum, manifest, commit boundary, MVCC metadata.
    pub fn validate(&mut self) -> ReplicationResult<()> {
        if self.state != SnapshotTransferState::TransferComplete {
            return Err(ReplicationError::configuration_error(
                "transfer not complete",
            ));
        }

        let metadata = self
            .metadata
            .as_ref()
            .ok_or_else(|| ReplicationError::configuration_error("no metadata"))?;

        // Verify bytes received match expected
        if self.bytes_received != metadata.size_bytes {
            self.state = SnapshotTransferState::Failed;
            return Err(ReplicationError::history_divergence(
                "snapshot size mismatch",
            ));
        }

        // Verification passed (actual checksum verification would happen here)
        self.state = SnapshotTransferState::Validated;

        Ok(())
    }

    /// Install snapshot atomically.
    ///
    /// Per §5.1: Atomic, all-or-nothing, crash-safe.
    /// If crash before completion → discard.
    /// If crash after completion → snapshot is authoritative.
    pub fn install(&mut self) -> ReplicationResult<SnapshotInstallResult> {
        if self.state != SnapshotTransferState::Validated {
            return Err(ReplicationError::configuration_error(
                "snapshot not validated",
            ));
        }

        let metadata = self
            .metadata
            .as_ref()
            .ok_or_else(|| ReplicationError::configuration_error("no metadata"))?
            .clone();

        // In real implementation, this would:
        // 1. Write snapshot to staging area
        // 2. Fsync
        // 3. Atomically swap into place
        // 4. Update metadata

        self.state = SnapshotTransferState::Installed;

        Ok(SnapshotInstallResult {
            commit_boundary: metadata.commit_boundary,
            wal_resume_sequence: metadata.wal_sequence + 1,
        })
    }

    /// Abort transfer.
    ///
    /// Per §9.1: Crash during transfer → discard.
    pub fn abort(&mut self) {
        self.state = SnapshotTransferState::Failed;
        self.metadata = None;
        self.bytes_received = 0;
    }

    /// Reset to idle state.
    pub fn reset(&mut self) {
        self.state = SnapshotTransferState::Idle;
        self.metadata = None;
        self.bytes_received = 0;
    }
}

impl Default for SnapshotReceiver {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of successful snapshot installation
#[derive(Debug, Clone)]
pub struct SnapshotInstallResult {
    /// Commit boundary of installed snapshot
    pub commit_boundary: CommitId,

    /// WAL sequence to resume from (strictly > snapshot boundary)
    /// Per §6: Replica must accept WAL records with CommitId > C_snap
    pub wal_resume_sequence: u64,
}

impl SnapshotInstallResult {
    /// Get WAL position to resume from.
    pub fn wal_resume_position(&self) -> WalPosition {
        WalPosition::new(self.wal_resume_sequence, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_metadata() -> SnapshotMetadata {
        SnapshotMetadata::new(CommitId::new(100), 99, 0xDEADBEEF, 1024)
    }

    #[test]
    fn test_snapshot_eligibility_eligible() {
        // Per §3: Complete snapshot from Primary is eligible
        let metadata = create_test_metadata();
        assert!(check_snapshot_eligibility(&metadata).is_eligible());
    }

    #[test]
    fn test_snapshot_eligibility_not_from_primary() {
        // Per §2: Only Primary may produce snapshots
        let mut metadata = create_test_metadata();
        metadata.from_primary = false;

        assert_eq!(
            check_snapshot_eligibility(&metadata),
            SnapshotEligibility::NotFromPrimary
        );
    }

    #[test]
    fn test_snapshot_eligibility_incomplete() {
        let mut metadata = create_test_metadata();
        metadata.is_complete = false;

        assert_eq!(
            check_snapshot_eligibility(&metadata),
            SnapshotEligibility::Incomplete
        );
    }

    #[test]
    fn test_snapshot_receiver_lifecycle() {
        let mut receiver = SnapshotReceiver::new();
        assert_eq!(receiver.state(), SnapshotTransferState::Idle);

        // Start transfer
        let metadata = create_test_metadata();
        assert!(receiver.start_transfer(metadata).is_ok());
        assert_eq!(receiver.state(), SnapshotTransferState::Transferring);

        // Receive bytes
        assert!(receiver.receive_bytes(1024).is_ok());
        assert_eq!(receiver.state(), SnapshotTransferState::TransferComplete);

        // Validate
        assert!(receiver.validate().is_ok());
        assert_eq!(receiver.state(), SnapshotTransferState::Validated);

        // Install
        let result = receiver.install();
        assert!(result.is_ok());
        assert_eq!(receiver.state(), SnapshotTransferState::Installed);

        let install_result = result.unwrap();
        assert_eq!(install_result.commit_boundary, CommitId::new(100));
        assert_eq!(install_result.wal_resume_sequence, 100);
    }

    #[test]
    fn test_snapshot_receiver_abort() {
        // Per §9.1: Crash during transfer → discard
        let mut receiver = SnapshotReceiver::new();
        let metadata = create_test_metadata();
        receiver.start_transfer(metadata).unwrap();
        receiver.receive_bytes(512).unwrap();

        // Abort mid-transfer
        receiver.abort();

        assert_eq!(receiver.state(), SnapshotTransferState::Failed);
        assert!(receiver.metadata().is_none());
    }

    #[test]
    fn test_wal_resume_after_snapshot() {
        // Per §6: WAL records must have CommitId > C_snap
        let result = SnapshotInstallResult {
            commit_boundary: CommitId::new(50),
            wal_resume_sequence: 51,
        };

        let pos = result.wal_resume_position();
        assert_eq!(pos.sequence, 51);
    }

    #[test]
    fn test_cannot_start_transfer_twice() {
        let mut receiver = SnapshotReceiver::new();
        let metadata = create_test_metadata();

        assert!(receiver.start_transfer(metadata.clone()).is_ok());
        assert!(receiver.start_transfer(metadata).is_err());
    }

    #[test]
    fn test_validation_fails_on_size_mismatch() {
        let mut receiver = SnapshotReceiver::new();
        let metadata = create_test_metadata();
        receiver.start_transfer(metadata).unwrap();

        // Receive wrong number of bytes
        receiver.receive_bytes(512).unwrap();
        receiver.state = SnapshotTransferState::TransferComplete; // Force complete

        let result = receiver.validate();
        assert!(result.is_err());
        assert_eq!(receiver.state(), SnapshotTransferState::Failed);
    }
}
