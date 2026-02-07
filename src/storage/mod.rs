//! Document Storage subsystem for aerodb
//!
//! The storage engine holds the canonical persistent state of all documents.
//! Per STORAGE.md, this is an append-only record file with no in-place updates.
//!
//! # Design Principles
//!
//! - Append-only (no in-place updates)
//! - Checksum-verified on every read
//! - Tombstones preserved forever (Phase 0)
//! - Latest record wins for same document_id
//! - WAL-driven (storage writes occur after WAL fsync)
//!
//! # Invariants Enforced
//!
//! - D2: Checksums everywhere
//! - D3: Schema + checksum validation
//! - K1: Checksums on every record
//! - K2: Halt-on-corruption policy
//! - C1: Full-document writes

mod checksum;
mod errors;
mod reader;
mod record;
mod writer;

pub use checksum::compute_checksum;
pub use errors::{StorageError, StorageResult};
pub use reader::StorageReader;
pub use record::{DocumentRecord, StoragePayload};
pub use writer::StorageWriter;
