//! # AeroDB File Storage Module
//!
//! Phase 11: File Storage
//!
//! S3-compatible file storage with RLS-based access control.

pub mod backend;
pub mod bucket;
pub mod errors;
pub mod file;
pub mod local;
pub mod metadata;
pub mod permissions;
pub mod signed_url;

pub use backend::StorageBackend;
pub use bucket::{Bucket, BucketConfig};
pub use errors::{StorageError, StorageResult};
pub use file::{FileService, StorageObject};
pub use local::LocalBackend;
pub use metadata::{InMemoryMetadataStore, MetadataStore};
pub use permissions::StoragePermissions;
pub use signed_url::SignedUrlGenerator;
