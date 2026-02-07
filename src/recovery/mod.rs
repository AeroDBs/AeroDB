//! Recovery Manager subsystem for aerodb
//!
//! Per WAL.md and STORAGE.md, recovery replays the WAL to restore state.
//!
//! # Startup Sequence (strict order)
//!
//! 1. Load schemas via schema loader
//! 2. Open WAL reader
//! 3. Open document storage
//! 4. Replay WAL from offset 0 sequentially
//! 5. Apply each WAL record via storage.apply_wal_record
//! 6. After replay completes, call index.rebuild_from_storage
//! 7. Run consistency verification
//! 8. Enter serving state
//!
//! # Invariants
//!
//! - R1: WAL is single source of truth for recovery
//! - R2: Sequential replay from byte 0
//! - K2: Halt-on-corruption policy

mod adapters;
mod errors;
mod replay;
mod startup;
mod verifier;

pub use adapters::RecoveryStorage;
pub use errors::{RecoveryError, RecoveryErrorCode, RecoveryResult};
pub use replay::{ReplayStats, StorageApply, WalRead, WalReplayer};
pub use startup::{IndexRebuild, RecoveryManager, RecoveryState};
pub use verifier::{
    ConsistencyVerifier, SchemaCheck, StorageRecordInfo, StorageScan, VerificationStats,
};
