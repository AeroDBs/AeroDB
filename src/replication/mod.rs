//! Replication Subsystem
//!
//! Per PHASE2_REPLICATION_INVARIANTS.md:
//! - Single-writer invariant: exactly one Primary
//! - Commit authority invariant: only Primary assigns CommitId
//! - Replicas consume history, never create it
//! - Replication halts on any invariant violation
//!
//! Per REPLICATION_MODEL.md:
//! - Authority is externally configured, never inferred
//! - No leader election, no consensus, no timeouts
//! - Fail-stop on ambiguity
//!
//! # Phase 5 Implementation
//!
//! Per PHASE5_INVARIANTS.md §P5-I16:
//! - Replication MUST be disableable at startup
//! - Disabling MUST NOT affect primary behavior
//!
//! # Phase 3 Optimizations
//!
//! - Fast Read: Pre-validated snapshot reuse on replicas (optional, disabled by default)

mod authority;
mod compatibility;
mod config;
mod errors;
mod failure_matrix;
mod fast_read;
mod recovery;
mod replica_reads;
mod role;
mod snapshot_transfer;
mod wal_receiver;
mod wal_sender;

pub use authority::{
    check_commit_authority, check_dual_primary, check_write_admission, AuthorityCheck,
    WriteAdmission,
};
pub use compatibility::{
    CompatibilityAssertion, CompatibilityCheck, MvccCompatibility, Phase1Compatibility,
};
pub use config::ReplicationConfig;
pub use errors::{ReplicationError, ReplicationErrorKind, ReplicationResult};
pub use failure_matrix::{FailureOutcome, FailureState, ReplicationCrashPoint};
pub use fast_read::{
    FastReadConfig, FastReadManager, FastReadResult, FastReadStats, ReplicaReadPath,
    ReplicaSafetyState, SafetyCheck, SafetyValidator, SafetyViolation,
};
pub use recovery::{PrimaryRecovery, RecoveryValidation, ReplicaRecovery, ReplicaResumeState};
pub use replica_reads::{ReadEligibility, ReplicaReadAdmission};
pub use role::{HaltReason, ReplicationRole, ReplicationState};
pub use snapshot_transfer::{
    check_snapshot_eligibility, SnapshotEligibility, SnapshotInstallResult, SnapshotMetadata,
    SnapshotReceiver, SnapshotTransferState,
};
pub use wal_receiver::{ReceiveResult, WalReceiver};
pub use wal_sender::{WalPosition, WalRecordEnvelope, WalSender};
