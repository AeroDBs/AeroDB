//! Document storage record types
//!
//! Per STORAGE.md §6.2, the document record format is:
//!
//! ```text
//! +------------------+
//! | Record Length    | (u32 LE)
//! +------------------+
//! | Document ID      | (length-prefixed string)
//! +------------------+
//! | Schema ID        | (length-prefixed string)
//! +------------------+
//! | Schema Version   | (length-prefixed string)
//! +------------------+
//! | Tombstone Flag   | (u8: 0 = live, 1 = deleted)
//! +------------------+
//! | Document Payload | (length-prefixed bytes)
//! +------------------+
//! | Checksum         | (u32 LE)
//! +------------------+
//! ```
//!
//! Checksum covers all bytes except the checksum itself.

use std::io::{self, Read};

/// Payload structure for a document to be stored.
///
/// This contains all the metadata needed for a document record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoragePayload {
    /// Collection identifier (from WAL)
    pub collection_id: String,
    /// Document primary key
    pub document_id: String,
    /// Schema identifier
    pub schema_id: String,
    /// Schema version identifier
    pub schema_version: String,
    /// Full document body (empty for tombstones)
    pub document_body: Vec<u8>,
    /// Whether this is a tombstone (DELETE)
    pub is_tombstone: bool,
}

impl StoragePayload {
    /// Create a new storage payload for a live document
    pub fn new(
        collection_id: impl Into<String>,
        document_id: impl Into<String>,
        schema_id: impl Into<String>,
        schema_version: impl Into<String>,
        document_body: Vec<u8>,
    ) -> Self {
        Self {
            collection_id: collection_id.into(),
            document_id: document_id.into(),
            schema_id: schema_id.into(),
            schema_version: schema_version.into(),
            document_body,
            is_tombstone: false,
        }
    }

    /// Create a tombstone payload for a deleted document
    pub fn tombstone(
        collection_id: impl Into<String>,
        document_id: impl Into<String>,
        schema_id: impl Into<String>,
        schema_version: impl Into<String>,
    ) -> Self {
        Self {
            collection_id: collection_id.into(),
            document_id: document_id.into(),
            schema_id: schema_id.into(),
            schema_version: schema_version.into(),
            document_body: Vec::new(),
            is_tombstone: true,
        }
    }

    /// Create a storage payload from a WAL record
    pub fn from_wal_record(wal_record: &crate::wal::WalRecord) -> Self {
        let is_tombstone = wal_record.record_type == crate::wal::RecordType::Delete;
        Self {
            collection_id: wal_record.payload.collection_id.clone(),
            document_id: wal_record.payload.document_id.clone(),
            schema_id: wal_record.payload.schema_id.clone(),
            schema_version: wal_record.payload.schema_version.clone(),
            document_body: if is_tombstone {
                Vec::new()
            } else {
                wal_record.payload.document_body.clone()
            },
            is_tombstone,
        }
    }
}

/// Complete document record as stored on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentRecord {
    /// Document primary key (composite: collection_id:document_id)
    pub document_id: String,
    /// Schema identifier
    pub schema_id: String,
    /// Schema version identifier
    pub schema_version: String,
    /// Whether this is a tombstone (deleted document)
    pub is_tombstone: bool,
    /// Document payload (empty for tombstones)
    pub document_body: Vec<u8>,
}

impl DocumentRecord {
    /// Create a new document record from a storage payload
    pub fn from_payload(payload: &StoragePayload) -> Self {
        // Composite key: collection_id:document_id
        let composite_id = format!("{}:{}", payload.collection_id, payload.document_id);
        Self {
            document_id: composite_id,
            schema_id: payload.schema_id.clone(),
            schema_version: payload.schema_version.clone(),
            is_tombstone: payload.is_tombstone,
            document_body: payload.document_body.clone(),
        }
    }

    /// Serialize the record body (everything except length prefix and checksum).
    /// This is the data over which the checksum is computed.
    fn serialize_body(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // Document ID (length-prefixed)
        buf.extend_from_slice(&(self.document_id.len() as u32).to_le_bytes());
        buf.extend_from_slice(self.document_id.as_bytes());

        // Schema ID (length-prefixed)
        buf.extend_from_slice(&(self.schema_id.len() as u32).to_le_bytes());
        buf.extend_from_slice(self.schema_id.as_bytes());

        // Schema Version (length-prefixed)
        buf.extend_from_slice(&(self.schema_version.len() as u32).to_le_bytes());
        buf.extend_from_slice(self.schema_version.as_bytes());

        // Tombstone flag
        buf.push(if self.is_tombstone { 1 } else { 0 });

        // Document body (length-prefixed)
        buf.extend_from_slice(&(self.document_body.len() as u32).to_le_bytes());
        buf.extend_from_slice(&self.document_body);

        buf
    }

    /// Serialize the complete record to bytes.
    ///
    /// Format:
    /// - Record Length (u32 LE) - total record length including this field
    /// - Body (variable)
    /// - Checksum (u32 LE)
    pub fn serialize(&self) -> Vec<u8> {
        let body = self.serialize_body();

        // Record length = 4 (length) + body.len() + 4 (checksum)
        let record_length = (4 + body.len() + 4) as u32;

        // Checksum covers: length + body
        let mut checksum_data = Vec::with_capacity(4 + body.len());
        checksum_data.extend_from_slice(&record_length.to_le_bytes());
        checksum_data.extend_from_slice(&body);
        let checksum = super::checksum::compute_checksum(&checksum_data);

        // Build final record
        let mut record = Vec::with_capacity(record_length as usize);
        record.extend_from_slice(&record_length.to_le_bytes());
        record.extend_from_slice(&body);
        record.extend_from_slice(&checksum.to_le_bytes());

        record
    }

    /// Deserialize a record from bytes, verifying checksum.
    ///
    /// Returns the record and the number of bytes consumed.
    pub fn deserialize(data: &[u8]) -> io::Result<(Self, usize)> {
        const MIN_RECORD_SIZE: usize = 4 + 4 + 4 + 4 + 1 + 4 + 4; // len + 3 strings + tombstone + body + checksum

        if data.len() < MIN_RECORD_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Record too short",
            ));
        }

        // Read record length
        let record_length = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;

        if record_length < MIN_RECORD_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid record length: {}", record_length),
            ));
        }

        if data.len() < record_length {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!(
                    "Record truncated: expected {} bytes, got {}",
                    record_length,
                    data.len()
                ),
            ));
        }

        // Extract and verify checksum
        let checksum_offset = record_length - 4;
        let stored_checksum = u32::from_le_bytes([
            data[checksum_offset],
            data[checksum_offset + 1],
            data[checksum_offset + 2],
            data[checksum_offset + 3],
        ]);

        let checksum_data = &data[0..checksum_offset];
        let computed_checksum = super::checksum::compute_checksum(checksum_data);

        if computed_checksum != stored_checksum {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Checksum mismatch: computed {:08x}, stored {:08x}",
                    computed_checksum, stored_checksum
                ),
            ));
        }

        // Parse body
        let mut cursor = std::io::Cursor::new(&data[4..checksum_offset]);

        fn read_string<R: Read>(reader: &mut R) -> io::Result<String> {
            let mut len_buf = [0u8; 4];
            reader.read_exact(&mut len_buf)?;
            let len = u32::from_le_bytes(len_buf) as usize;

            let mut buf = vec![0u8; len];
            reader.read_exact(&mut buf)?;

            String::from_utf8(buf).map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("Invalid UTF-8: {}", e))
            })
        }

        fn read_bytes<R: Read>(reader: &mut R) -> io::Result<Vec<u8>> {
            let mut len_buf = [0u8; 4];
            reader.read_exact(&mut len_buf)?;
            let len = u32::from_le_bytes(len_buf) as usize;

            let mut buf = vec![0u8; len];
            reader.read_exact(&mut buf)?;

            Ok(buf)
        }

        let document_id = read_string(&mut cursor)?;
        let schema_id = read_string(&mut cursor)?;
        let schema_version = read_string(&mut cursor)?;

        let mut tombstone_buf = [0u8; 1];
        cursor.read_exact(&mut tombstone_buf)?;
        let is_tombstone = tombstone_buf[0] != 0;

        let document_body = read_bytes(&mut cursor)?;

        Ok((
            Self {
                document_id,
                schema_id,
                schema_version,
                is_tombstone,
                document_body,
            },
            record_length,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_payload() -> StoragePayload {
        StoragePayload::new(
            "users",
            "user_123",
            "user_schema",
            "v1",
            b"{\"name\": \"Alice\"}".to_vec(),
        )
    }

    #[test]
    fn test_payload_from_wal_record_insert() {
        let wal_payload = crate::wal::WalPayload::new(
            "users",
            "user_123",
            "user_schema",
            "v1",
            b"{\"name\": \"Bob\"}".to_vec(),
        );
        let wal_record = crate::wal::WalRecord::insert(1, wal_payload);

        let storage_payload = StoragePayload::from_wal_record(&wal_record);
        assert_eq!(storage_payload.document_id, "user_123");
        assert_eq!(storage_payload.schema_id, "user_schema");
        assert!(!storage_payload.is_tombstone);
        assert!(!storage_payload.document_body.is_empty());
    }

    #[test]
    fn test_payload_from_wal_record_delete() {
        let wal_payload =
            crate::wal::WalPayload::tombstone("users", "user_123", "user_schema", "v1");
        let wal_record = crate::wal::WalRecord::delete(1, wal_payload);

        let storage_payload = StoragePayload::from_wal_record(&wal_record);
        assert!(storage_payload.is_tombstone);
        assert!(storage_payload.document_body.is_empty());
    }

    #[test]
    fn test_record_roundtrip() {
        let record = DocumentRecord::from_payload(&sample_payload());
        let serialized = record.serialize();
        let (deserialized, bytes_consumed) = DocumentRecord::deserialize(&serialized).unwrap();

        assert_eq!(record, deserialized);
        assert_eq!(bytes_consumed, serialized.len());
    }

    #[test]
    fn test_tombstone_record_roundtrip() {
        let payload = StoragePayload::tombstone("users", "user_123", "user_schema", "v1");
        let record = DocumentRecord::from_payload(&payload);

        assert!(record.is_tombstone);
        assert!(record.document_body.is_empty());

        let serialized = record.serialize();
        let (deserialized, _) = DocumentRecord::deserialize(&serialized).unwrap();

        assert_eq!(record, deserialized);
        assert!(deserialized.is_tombstone);
    }

    #[test]
    fn test_checksum_detects_corruption() {
        let record = DocumentRecord::from_payload(&sample_payload());
        let mut serialized = record.serialize();

        // Corrupt a byte
        let mid = serialized.len() / 2;
        serialized[mid] ^= 0xFF;

        let result = DocumentRecord::deserialize(&serialized);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Checksum mismatch"));
    }

    #[test]
    fn test_deterministic_serialization() {
        let record = DocumentRecord::from_payload(&sample_payload());
        let s1 = record.serialize();
        let s2 = record.serialize();
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_composite_document_id() {
        let payload = sample_payload();
        let record = DocumentRecord::from_payload(&payload);
        assert_eq!(record.document_id, "users:user_123");
    }
}
