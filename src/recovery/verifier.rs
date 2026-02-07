//! Consistency verifier for recovery
//!
//! Verifies storage integrity after WAL replay.
//!
//! Per STORAGE.md and SCHEMA.md:
//! - Scan storage sequentially
//! - Validate checksum on every record
//! - Ensure no invalid schema references exist

use super::errors::{RecoveryError, RecoveryResult};

/// Trait for schema existence checking
pub trait SchemaCheck {
    /// Check if schema ID exists
    fn schema_exists(&self, schema_id: &str) -> bool;
    /// Check if schema version exists
    fn schema_version_exists(&self, schema_id: &str, version: &str) -> bool;
}

/// Storage record for verification
#[derive(Debug, Clone)]
pub struct StorageRecordInfo {
    /// Document ID
    pub document_id: String,
    /// Schema ID
    pub schema_id: String,
    /// Schema version
    pub schema_version: String,
    /// Offset in storage file
    pub offset: u64,
    /// Is tombstone
    pub is_tombstone: bool,
}

/// Trait for scanning storage
pub trait StorageScan {
    /// Read the next storage record for verification
    /// Returns None if at end
    /// Returns Err if checksum fails
    fn scan_next(&mut self) -> RecoveryResult<Option<StorageRecordInfo>>;

    /// Reset to beginning of storage
    fn reset(&mut self) -> RecoveryResult<()>;
}

/// Verification statistics
#[derive(Debug, Clone, Default)]
pub struct VerificationStats {
    /// Number of records verified
    pub records_verified: u64,
    /// Number of tombstones
    pub tombstones: u64,
    /// Number of live documents
    pub live_documents: u64,
}

/// Consistency verifier that checks storage integrity
pub struct ConsistencyVerifier;

impl ConsistencyVerifier {
    /// Verify storage consistency after replay.
    ///
    /// This method:
    /// 1. Scans storage sequentially
    /// 2. Validates checksum on every record
    /// 3. Ensures no invalid schema references exist
    ///
    /// Returns FATAL error on any corruption or invalid reference.
    pub fn verify<S: StorageScan, C: SchemaCheck>(
        storage: &mut S,
        schema_registry: &C,
    ) -> RecoveryResult<VerificationStats> {
        storage.reset()?;

        let mut stats = VerificationStats::default();

        loop {
            // Read next record (checksum validated by scanner)
            let record = match storage.scan_next() {
                Ok(Some(r)) => r,
                Ok(None) => break, // End of storage
                Err(e) => {
                    // Storage corruption detected - abort immediately
                    return Err(RecoveryError::storage_corruption(
                        e.offset().unwrap_or(0),
                        e.message(),
                    ));
                }
            };

            stats.records_verified += 1;

            if record.is_tombstone {
                stats.tombstones += 1;
                continue;
            }

            stats.live_documents += 1;

            // Verify schema exists
            if !schema_registry.schema_exists(&record.schema_id) {
                return Err(RecoveryError::schema_missing(
                    &record.schema_id,
                    &record.schema_version,
                ));
            }

            if !schema_registry.schema_version_exists(&record.schema_id, &record.schema_version) {
                return Err(RecoveryError::schema_missing(
                    &record.schema_id,
                    &record.schema_version,
                ));
            }
        }

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    struct MockStorage {
        records: Vec<StorageRecordInfo>,
        position: usize,
        corrupt_at: Option<usize>,
    }

    impl MockStorage {
        fn new(records: Vec<StorageRecordInfo>) -> Self {
            Self {
                records,
                position: 0,
                corrupt_at: None,
            }
        }

        fn with_corruption_at(mut self, index: usize) -> Self {
            self.corrupt_at = Some(index);
            self
        }
    }

    impl StorageScan for MockStorage {
        fn scan_next(&mut self) -> RecoveryResult<Option<StorageRecordInfo>> {
            if self.position >= self.records.len() {
                return Ok(None);
            }

            if self.corrupt_at == Some(self.position) {
                return Err(RecoveryError::storage_corruption(
                    self.records[self.position].offset,
                    "checksum mismatch",
                ));
            }

            let record = self.records[self.position].clone();
            self.position += 1;
            Ok(Some(record))
        }

        fn reset(&mut self) -> RecoveryResult<()> {
            self.position = 0;
            Ok(())
        }
    }

    struct MockSchemaRegistry {
        schemas: HashSet<(String, String)>,
    }

    impl MockSchemaRegistry {
        fn new() -> Self {
            let mut schemas = HashSet::new();
            schemas.insert(("users".to_string(), "v1".to_string()));
            schemas.insert(("posts".to_string(), "v1".to_string()));
            Self { schemas }
        }
    }

    impl SchemaCheck for MockSchemaRegistry {
        fn schema_exists(&self, schema_id: &str) -> bool {
            self.schemas.iter().any(|(id, _)| id == schema_id)
        }

        fn schema_version_exists(&self, schema_id: &str, version: &str) -> bool {
            self.schemas
                .contains(&(schema_id.to_string(), version.to_string()))
        }
    }

    fn make_record(id: &str, schema_id: &str, version: &str, offset: u64) -> StorageRecordInfo {
        StorageRecordInfo {
            document_id: id.to_string(),
            schema_id: schema_id.to_string(),
            schema_version: version.to_string(),
            offset,
            is_tombstone: false,
        }
    }

    fn make_tombstone(id: &str, offset: u64) -> StorageRecordInfo {
        StorageRecordInfo {
            document_id: id.to_string(),
            schema_id: "".to_string(),
            schema_version: "".to_string(),
            offset,
            is_tombstone: true,
        }
    }

    #[test]
    fn test_successful_verification() {
        let records = vec![
            make_record("user_1", "users", "v1", 0),
            make_record("user_2", "users", "v1", 100),
            make_tombstone("user_3", 200),
        ];

        let mut storage = MockStorage::new(records);
        let schema = MockSchemaRegistry::new();

        let stats = ConsistencyVerifier::verify(&mut storage, &schema).unwrap();

        assert_eq!(stats.records_verified, 3);
        assert_eq!(stats.live_documents, 2);
        assert_eq!(stats.tombstones, 1);
    }

    #[test]
    fn test_storage_corruption_aborts() {
        let records = vec![
            make_record("user_1", "users", "v1", 0),
            make_record("user_2", "users", "v1", 100),
        ];

        let mut storage = MockStorage::new(records).with_corruption_at(1);
        let schema = MockSchemaRegistry::new();

        let result = ConsistencyVerifier::verify(&mut storage, &schema);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code().code(), "AERO_STORAGE_CORRUPTION");
    }

    #[test]
    fn test_missing_schema_aborts() {
        let records = vec![
            make_record("user_1", "users", "v1", 0),
            make_record("order_1", "orders", "v1", 100), // orders schema doesn't exist
        ];

        let mut storage = MockStorage::new(records);
        let schema = MockSchemaRegistry::new();

        let result = ConsistencyVerifier::verify(&mut storage, &schema);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().code().code(),
            "AERO_RECOVERY_SCHEMA_MISSING"
        );
    }

    #[test]
    fn test_empty_storage() {
        let mut storage = MockStorage::new(vec![]);
        let schema = MockSchemaRegistry::new();

        let stats = ConsistencyVerifier::verify(&mut storage, &schema).unwrap();

        assert_eq!(stats.records_verified, 0);
    }
}
