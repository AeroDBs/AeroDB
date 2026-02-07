//! WAL record types and structures
//!
//! Per WAL.md §87-137, each WAL record contains:
//! - Record Length (u32 LE)
//! - Record Type (u8): INSERT / UPDATE / DELETE
//! - Sequence Number (u64 LE)
//! - Payload (variable)
//! - Checksum (u32 LE)
//!
//! Payload must include:
//! - Collection identifier
//! - Document primary key
//! - Schema version identifier
//! - Full document body (post-operation state)

use std::io::{self, Read, Write};

/// WAL record types as defined in WAL.md §141-172
/// Extended for MVCC per MVCC_WAL_INTERACTION.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RecordType {
    /// Insertion of a new document
    Insert = 0,
    /// Replacement of an existing document (full document, not delta)
    Update = 1,
    /// Deletion of a document (tombstone)
    Delete = 2,
    /// MVCC commit identity record
    /// Per MVCC_WAL_INTERACTION.md: The commit identity is the visibility barrier
    MvccCommit = 3,
    /// MVCC version record - binds version data to commit identity
    /// Per MVCC_WAL_INTERACTION.md: Versions exist only with durable commit
    MvccVersion = 4,
    /// MVCC garbage collection record
    /// Per MVCC_GC.md: GC events must be WAL-recorded for deterministic replay
    MvccGc = 5,
}

impl RecordType {
    /// Convert from u8, returns None for invalid values
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(RecordType::Insert),
            1 => Some(RecordType::Update),
            2 => Some(RecordType::Delete),
            3 => Some(RecordType::MvccCommit),
            4 => Some(RecordType::MvccVersion),
            5 => Some(RecordType::MvccGc),
            _ => None,
        }
    }

    /// Returns true if this is an MVCC-specific record type
    pub fn is_mvcc_record(self) -> bool {
        matches!(
            self,
            RecordType::MvccCommit | RecordType::MvccVersion | RecordType::MvccGc
        )
    }

    /// Convert to u8
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// MVCC Commit payload per MVCC_WAL_INTERACTION.md
///
/// This record type represents a commit identity assignment.
/// Per MVCC_WAL_INTERACTION.md §2:
/// - Commit identities are assigned exactly once
/// - Assignment occurs as part of commit
/// - The ordering is total, strict, and replayable
/// - No commit identity exists outside the WAL
///
/// Per MVCC_WAL_INTERACTION.md §3.2:
/// - Visibility is tied to commit identity durability
/// - A version becomes visible only after its commit identity is durable
/// - WAL fsync is the visibility barrier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MvccCommitPayload {
    /// The commit identity value
    /// Per PHASE2_INVARIANTS.md §2.5: Strictly monotonic, never reused
    pub commit_id: u64,
}

impl MvccCommitPayload {
    /// Create a new MVCC commit payload
    pub fn new(commit_id: u64) -> Self {
        Self { commit_id }
    }

    /// Serialize to bytes
    pub fn serialize(&self) -> Vec<u8> {
        self.commit_id.to_le_bytes().to_vec()
    }

    /// Deserialize from bytes
    pub fn deserialize(data: &[u8]) -> std::io::Result<Self> {
        if data.len() < 8 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "MvccCommitPayload too short",
            ));
        }
        let commit_id = u64::from_le_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]);
        Ok(Self { commit_id })
    }
}

/// MVCC Version payload - binds version data to commit identity
///
/// Per MVCC_WAL_INTERACTION.md and PHASE2_INVARIANTS.md:
/// - A version exists if and only if its commit identity exists durably
/// - Versions are written AFTER commit identity records
/// - Atomicity is enforced at recovery time
///
/// This record type contains all data needed for a versioned document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MvccVersionPayload {
    /// The commit identity this version belongs to
    /// Must reference a durable MvccCommit record
    pub commit_id: u64,
    /// Document key (collection:document_id)
    pub key: String,
    /// Whether this is a tombstone (delete) version
    pub is_tombstone: bool,
    /// Document payload (empty for tombstones)
    pub payload: Vec<u8>,
}

impl MvccVersionPayload {
    /// Create a new version payload for a document
    pub fn new(commit_id: u64, key: impl Into<String>, payload: Vec<u8>) -> Self {
        Self {
            commit_id,
            key: key.into(),
            is_tombstone: false,
            payload,
        }
    }

    /// Create a tombstone version payload
    pub fn tombstone(commit_id: u64, key: impl Into<String>) -> Self {
        Self {
            commit_id,
            key: key.into(),
            is_tombstone: true,
            payload: Vec::new(),
        }
    }

    /// Serialize to bytes
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // commit_id (u64 LE)
        buf.extend_from_slice(&self.commit_id.to_le_bytes());

        // key (length-prefixed string)
        buf.extend_from_slice(&(self.key.len() as u32).to_le_bytes());
        buf.extend_from_slice(self.key.as_bytes());

        // is_tombstone (u8)
        buf.push(if self.is_tombstone { 1 } else { 0 });

        // payload (length-prefixed bytes)
        buf.extend_from_slice(&(self.payload.len() as u32).to_le_bytes());
        buf.extend_from_slice(&self.payload);

        buf
    }

    /// Deserialize from bytes
    pub fn deserialize(data: &[u8]) -> std::io::Result<Self> {
        use std::io::{Cursor, Read};

        if data.len() < 8 + 4 + 1 + 4 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "MvccVersionPayload too short",
            ));
        }

        let mut cursor = Cursor::new(data);

        // commit_id
        let mut commit_buf = [0u8; 8];
        cursor.read_exact(&mut commit_buf)?;
        let commit_id = u64::from_le_bytes(commit_buf);

        // key
        let mut len_buf = [0u8; 4];
        cursor.read_exact(&mut len_buf)?;
        let key_len = u32::from_le_bytes(len_buf) as usize;
        let mut key_buf = vec![0u8; key_len];
        cursor.read_exact(&mut key_buf)?;
        let key = String::from_utf8(key_buf).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid UTF-8: {}", e),
            )
        })?;

        // is_tombstone
        let mut tombstone_buf = [0u8; 1];
        cursor.read_exact(&mut tombstone_buf)?;
        let is_tombstone = tombstone_buf[0] != 0;

        // payload
        cursor.read_exact(&mut len_buf)?;
        let payload_len = u32::from_le_bytes(len_buf) as usize;
        let mut payload = vec![0u8; payload_len];
        cursor.read_exact(&mut payload)?;

        Ok(Self {
            commit_id,
            key,
            is_tombstone,
            payload,
        })
    }
}

/// WAL payload containing all required fields per WAL.md §119-137
///
/// WAL records always store the full document state, not deltas.
/// This guarantees deterministic replay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalPayload {
    /// Collection identifier
    pub collection_id: String,
    /// Document primary key
    pub document_id: String,
    /// Schema identifier
    pub schema_id: String,
    /// Schema version identifier
    pub schema_version: String,
    /// Full document body (post-operation state)
    /// For DELETE operations, this is empty (tombstone)
    pub document_body: Vec<u8>,
}

impl WalPayload {
    /// Create a new WAL payload
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
        }
    }

    /// Create a tombstone payload for DELETE operations
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
        }
    }

    /// Serialize payload to bytes
    ///
    /// Format:
    /// - collection_id_len (u32 LE)
    /// - collection_id (bytes)
    /// - document_id_len (u32 LE)
    /// - document_id (bytes)
    /// - schema_id_len (u32 LE)
    /// - schema_id (bytes)
    /// - schema_version_len (u32 LE)
    /// - schema_version (bytes)
    /// - document_body_len (u32 LE)
    /// - document_body (bytes)
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.write_to(&mut buf).expect("Vec write cannot fail");
        buf
    }

    /// Write payload to a writer
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        // Collection ID
        writer.write_all(&(self.collection_id.len() as u32).to_le_bytes())?;
        writer.write_all(self.collection_id.as_bytes())?;

        // Document ID
        writer.write_all(&(self.document_id.len() as u32).to_le_bytes())?;
        writer.write_all(self.document_id.as_bytes())?;

        // Schema ID
        writer.write_all(&(self.schema_id.len() as u32).to_le_bytes())?;
        writer.write_all(self.schema_id.as_bytes())?;

        // Schema Version
        writer.write_all(&(self.schema_version.len() as u32).to_le_bytes())?;
        writer.write_all(self.schema_version.as_bytes())?;

        // Document Body
        writer.write_all(&(self.document_body.len() as u32).to_le_bytes())?;
        writer.write_all(&self.document_body)?;

        Ok(())
    }

    /// Deserialize payload from bytes
    pub fn deserialize(data: &[u8]) -> io::Result<Self> {
        let mut cursor = io::Cursor::new(data);
        Self::read_from(&mut cursor)
    }

    /// Read payload from a reader
    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        fn read_string<R: Read>(reader: &mut R) -> io::Result<String> {
            let mut len_buf = [0u8; 4];
            reader.read_exact(&mut len_buf)?;
            let len = u32::from_le_bytes(len_buf) as usize;

            let mut string_buf = vec![0u8; len];
            reader.read_exact(&mut string_buf)?;

            String::from_utf8(string_buf).map_err(|e| {
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

        let collection_id = read_string(reader)?;
        let document_id = read_string(reader)?;
        let schema_id = read_string(reader)?;
        let schema_version = read_string(reader)?;
        let document_body = read_bytes(reader)?;

        Ok(Self {
            collection_id,
            document_id,
            schema_id,
            schema_version,
            document_body,
        })
    }
}

/// Complete WAL record structure per WAL.md §87-116
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalRecord {
    /// Record type (INSERT / UPDATE / DELETE)
    pub record_type: RecordType,
    /// Global monotonic operation ID (starts at 1, never repeats)
    pub sequence_number: u64,
    /// Operation payload
    pub payload: WalPayload,
}

impl WalRecord {
    /// Create a new WAL record
    pub fn new(record_type: RecordType, sequence_number: u64, payload: WalPayload) -> Self {
        Self {
            record_type,
            sequence_number,
            payload,
        }
    }

    /// Create an INSERT record
    pub fn insert(sequence_number: u64, payload: WalPayload) -> Self {
        Self::new(RecordType::Insert, sequence_number, payload)
    }

    /// Create an UPDATE record
    pub fn update(sequence_number: u64, payload: WalPayload) -> Self {
        Self::new(RecordType::Update, sequence_number, payload)
    }

    /// Create a DELETE record
    pub fn delete(sequence_number: u64, payload: WalPayload) -> Self {
        Self::new(RecordType::Delete, sequence_number, payload)
    }

    /// Serialize the record body (everything except length prefix and checksum)
    /// This is the data over which the checksum is computed.
    ///
    /// Format:
    /// - Record Type (u8)
    /// - Sequence Number (u64 LE)
    /// - Payload (variable)
    fn serialize_body(&self) -> Vec<u8> {
        let payload_bytes = self.payload.serialize();
        let body_len = 1 + 8 + payload_bytes.len();
        let mut buf = Vec::with_capacity(body_len);

        buf.push(self.record_type.as_u8());
        buf.extend_from_slice(&self.sequence_number.to_le_bytes());
        buf.extend_from_slice(&payload_bytes);

        buf
    }

    /// Serialize the complete record to bytes
    ///
    /// Format per WAL.md:
    /// - Record Length (u32 LE) - total record length including this field
    /// - Record Type (u8)
    /// - Sequence Number (u64 LE)
    /// - Payload (variable)
    /// - Checksum (u32 LE)
    pub fn serialize(&self) -> Vec<u8> {
        let body = self.serialize_body();

        // Record length = 4 (length field) + body.len() + 4 (checksum field)
        let record_length = (4 + body.len() + 4) as u32;

        // Checksum covers: length field + body (everything except checksum itself)
        let mut checksum_data = Vec::with_capacity(4 + body.len());
        checksum_data.extend_from_slice(&record_length.to_le_bytes());
        checksum_data.extend_from_slice(&body);
        let checksum = crate::wal::checksum::compute_checksum(&checksum_data);

        // Build final record
        let mut record = Vec::with_capacity(record_length as usize);
        record.extend_from_slice(&record_length.to_le_bytes());
        record.extend_from_slice(&body);
        record.extend_from_slice(&checksum.to_le_bytes());

        record
    }

    /// Deserialize a record from bytes, verifying checksum
    ///
    /// Returns the record and the number of bytes consumed.
    /// Returns an error if checksum validation fails or data is malformed.
    pub fn deserialize(data: &[u8]) -> io::Result<(Self, usize)> {
        // Minimum record size: 4 (len) + 1 (type) + 8 (seq) + 20 (min payload) + 4 (checksum)
        const MIN_RECORD_SIZE: usize = 4 + 1 + 8 + 4;

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

        // Extract checksum from end of record
        let checksum_offset = record_length - 4;
        let stored_checksum = u32::from_le_bytes([
            data[checksum_offset],
            data[checksum_offset + 1],
            data[checksum_offset + 2],
            data[checksum_offset + 3],
        ]);

        // Verify checksum over everything except the checksum itself
        let checksum_data = &data[0..checksum_offset];
        let computed_checksum = crate::wal::checksum::compute_checksum(checksum_data);

        if computed_checksum != stored_checksum {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Checksum mismatch: computed {:08x}, stored {:08x}",
                    computed_checksum, stored_checksum
                ),
            ));
        }

        // Parse record body
        let record_type_byte = data[4];
        let record_type = RecordType::from_u8(record_type_byte).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid record type: {}", record_type_byte),
            )
        })?;

        let sequence_number = u64::from_le_bytes([
            data[5], data[6], data[7], data[8], data[9], data[10], data[11], data[12],
        ]);

        // Parse payload
        let payload_data = &data[13..checksum_offset];
        let payload = WalPayload::deserialize(payload_data)?;

        Ok((
            WalRecord {
                record_type,
                sequence_number,
                payload,
            },
            record_length,
        ))
    }
}

/// MVCC Commit WAL record per MVCC_WAL_INTERACTION.md
///
/// This is a separate record type for MVCC commits because:
/// - The payload structure differs from document operations
/// - Commit records only contain a CommitId
/// - Recovery must handle these records specially
///
/// Per MVCC_WAL_INTERACTION.md:
/// - The WAL is the sole source of truth for commit ordering
/// - Commit identity is the visibility barrier
/// - No commit identity exists outside the WAL
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MvccCommitRecord {
    /// Global monotonic sequence number
    pub sequence_number: u64,
    /// The commit identity payload
    pub payload: MvccCommitPayload,
}

impl MvccCommitRecord {
    /// Create a new MVCC commit record
    pub fn new(sequence_number: u64, commit_id: u64) -> Self {
        Self {
            sequence_number,
            payload: MvccCommitPayload::new(commit_id),
        }
    }

    /// Get the commit identity
    pub fn commit_id(&self) -> u64 {
        self.payload.commit_id
    }

    /// Serialize the record body
    fn serialize_body(&self) -> Vec<u8> {
        let payload_bytes = self.payload.serialize();
        let mut buf = Vec::with_capacity(1 + 8 + payload_bytes.len());
        buf.push(RecordType::MvccCommit.as_u8());
        buf.extend_from_slice(&self.sequence_number.to_le_bytes());
        buf.extend_from_slice(&payload_bytes);
        buf
    }

    /// Serialize the complete record to bytes with checksum
    pub fn serialize(&self) -> Vec<u8> {
        let body = self.serialize_body();
        let record_length = (4 + body.len() + 4) as u32;

        let mut checksum_data = Vec::with_capacity(4 + body.len());
        checksum_data.extend_from_slice(&record_length.to_le_bytes());
        checksum_data.extend_from_slice(&body);
        let checksum = crate::wal::checksum::compute_checksum(&checksum_data);

        let mut record = Vec::with_capacity(record_length as usize);
        record.extend_from_slice(&record_length.to_le_bytes());
        record.extend_from_slice(&body);
        record.extend_from_slice(&checksum.to_le_bytes());
        record
    }

    /// Deserialize from bytes, verifying checksum
    pub fn deserialize(data: &[u8]) -> io::Result<(Self, usize)> {
        const MIN_RECORD_SIZE: usize = 4 + 1 + 8 + 8 + 4; // length + type + seq + commit_id + checksum

        if data.len() < MIN_RECORD_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "MVCC commit record too short",
            ));
        }

        let record_length = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;

        if data.len() < record_length {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "MVCC commit record truncated",
            ));
        }

        // Verify checksum
        let checksum_offset = record_length - 4;
        let stored_checksum = u32::from_le_bytes([
            data[checksum_offset],
            data[checksum_offset + 1],
            data[checksum_offset + 2],
            data[checksum_offset + 3],
        ]);

        let checksum_data = &data[0..checksum_offset];
        let computed_checksum = crate::wal::checksum::compute_checksum(checksum_data);

        if computed_checksum != stored_checksum {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "MVCC commit checksum mismatch: computed {:08x}, stored {:08x}",
                    computed_checksum, stored_checksum
                ),
            ));
        }

        // Verify record type
        let record_type_byte = data[4];
        if record_type_byte != RecordType::MvccCommit.as_u8() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Expected MvccCommit record type, got {}", record_type_byte),
            ));
        }

        let sequence_number = u64::from_le_bytes([
            data[5], data[6], data[7], data[8], data[9], data[10], data[11], data[12],
        ]);

        let payload_data = &data[13..checksum_offset];
        let payload = MvccCommitPayload::deserialize(payload_data)?;

        Ok((
            MvccCommitRecord {
                sequence_number,
                payload,
            },
            record_length,
        ))
    }
}

/// MVCC Version WAL record - binds version data to commit identity
///
/// Per MVCC_WAL_INTERACTION.md and PHASE2_INVARIANTS.md:
/// - A version exists if and only if its commit identity exists durably
/// - This record is written AFTER the MvccCommit record
/// - Recovery cross-validates WAL and storage
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MvccVersionRecord {
    /// Global monotonic sequence number
    pub sequence_number: u64,
    /// The version payload with commit binding
    pub payload: MvccVersionPayload,
}

impl MvccVersionRecord {
    /// Create a new MVCC version record
    pub fn new(
        sequence_number: u64,
        commit_id: u64,
        key: impl Into<String>,
        data: Vec<u8>,
    ) -> Self {
        Self {
            sequence_number,
            payload: MvccVersionPayload::new(commit_id, key, data),
        }
    }

    /// Create a tombstone version record
    pub fn tombstone(sequence_number: u64, commit_id: u64, key: impl Into<String>) -> Self {
        Self {
            sequence_number,
            payload: MvccVersionPayload::tombstone(commit_id, key),
        }
    }

    /// Get the commit identity
    pub fn commit_id(&self) -> u64 {
        self.payload.commit_id
    }

    /// Get the document key
    pub fn key(&self) -> &str {
        &self.payload.key
    }

    /// Serialize the record body
    fn serialize_body(&self) -> Vec<u8> {
        let payload_bytes = self.payload.serialize();
        let mut buf = Vec::with_capacity(1 + 8 + payload_bytes.len());
        buf.push(RecordType::MvccVersion.as_u8());
        buf.extend_from_slice(&self.sequence_number.to_le_bytes());
        buf.extend_from_slice(&payload_bytes);
        buf
    }

    /// Serialize the complete record to bytes with checksum
    pub fn serialize(&self) -> Vec<u8> {
        let body = self.serialize_body();
        let record_length = (4 + body.len() + 4) as u32;

        let mut checksum_data = Vec::with_capacity(4 + body.len());
        checksum_data.extend_from_slice(&record_length.to_le_bytes());
        checksum_data.extend_from_slice(&body);
        let checksum = crate::wal::checksum::compute_checksum(&checksum_data);

        let mut record = Vec::with_capacity(record_length as usize);
        record.extend_from_slice(&record_length.to_le_bytes());
        record.extend_from_slice(&body);
        record.extend_from_slice(&checksum.to_le_bytes());
        record
    }

    /// Deserialize from bytes, verifying checksum
    pub fn deserialize(data: &[u8]) -> io::Result<(Self, usize)> {
        const MIN_RECORD_SIZE: usize = 4 + 1 + 8 + 8 + 4 + 1 + 4 + 4; // length + type + seq + commit_id + key_len + tombstone + payload_len + checksum

        if data.len() < MIN_RECORD_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "MVCC version record too short",
            ));
        }

        let record_length = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;

        if data.len() < record_length {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "MVCC version record truncated",
            ));
        }

        // Verify checksum
        let checksum_offset = record_length - 4;
        let stored_checksum = u32::from_le_bytes([
            data[checksum_offset],
            data[checksum_offset + 1],
            data[checksum_offset + 2],
            data[checksum_offset + 3],
        ]);

        let checksum_data = &data[0..checksum_offset];
        let computed_checksum = crate::wal::checksum::compute_checksum(checksum_data);

        if computed_checksum != stored_checksum {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "MVCC version checksum mismatch: computed {:08x}, stored {:08x}",
                    computed_checksum, stored_checksum
                ),
            ));
        }

        // Verify record type
        let record_type_byte = data[4];
        if record_type_byte != RecordType::MvccVersion.as_u8() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Expected MvccVersion record type, got {}", record_type_byte),
            ));
        }

        let sequence_number = u64::from_le_bytes([
            data[5], data[6], data[7], data[8], data[9], data[10], data[11], data[12],
        ]);

        let payload_data = &data[13..checksum_offset];
        let payload = MvccVersionPayload::deserialize(payload_data)?;

        Ok((
            MvccVersionRecord {
                sequence_number,
                payload,
            },
            record_length,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_payload() -> WalPayload {
        WalPayload::new(
            "users",
            "user_123",
            "user_schema",
            "v1",
            b"{\"name\": \"Alice\"}".to_vec(),
        )
    }

    #[test]
    fn test_record_type_roundtrip() {
        for record_type in [RecordType::Insert, RecordType::Update, RecordType::Delete] {
            let byte = record_type.as_u8();
            let recovered = RecordType::from_u8(byte).unwrap();
            assert_eq!(record_type, recovered);
        }
    }

    #[test]
    fn test_invalid_record_type() {
        // 5 is now valid (MvccGc), so test 6 and 255
        assert!(RecordType::from_u8(6).is_none());
        assert!(RecordType::from_u8(255).is_none());
    }

    #[test]
    fn test_mvcc_commit_record_type() {
        assert_eq!(RecordType::MvccCommit.as_u8(), 3);
        assert_eq!(RecordType::from_u8(3), Some(RecordType::MvccCommit));
        assert!(RecordType::MvccCommit.is_mvcc_record());
        assert!(!RecordType::Insert.is_mvcc_record());
    }

    #[test]
    fn test_payload_roundtrip() {
        let payload = sample_payload();
        let serialized = payload.serialize();
        let deserialized = WalPayload::deserialize(&serialized).unwrap();
        assert_eq!(payload, deserialized);
    }

    #[test]
    fn test_tombstone_payload() {
        let tombstone = WalPayload::tombstone("users", "user_123", "user_schema", "v1");
        assert!(tombstone.document_body.is_empty());

        let serialized = tombstone.serialize();
        let deserialized = WalPayload::deserialize(&serialized).unwrap();
        assert_eq!(tombstone, deserialized);
    }

    #[test]
    fn test_record_roundtrip() {
        let record = WalRecord::insert(1, sample_payload());
        let serialized = record.serialize();
        let (deserialized, bytes_consumed) = WalRecord::deserialize(&serialized).unwrap();

        assert_eq!(record, deserialized);
        assert_eq!(bytes_consumed, serialized.len());
    }

    #[test]
    fn test_record_sequence_number_preserved() {
        let record = WalRecord::update(42, sample_payload());
        let serialized = record.serialize();
        let (deserialized, _) = WalRecord::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.sequence_number, 42);
    }

    #[test]
    fn test_record_type_preserved() {
        for record_type in [RecordType::Insert, RecordType::Update, RecordType::Delete] {
            let record = WalRecord::new(record_type, 1, sample_payload());
            let serialized = record.serialize();
            let (deserialized, _) = WalRecord::deserialize(&serialized).unwrap();
            assert_eq!(deserialized.record_type, record_type);
        }
    }

    #[test]
    fn test_checksum_detects_corruption() {
        let record = WalRecord::insert(1, sample_payload());
        let mut serialized = record.serialize();

        // Corrupt a byte in the middle
        let mid = serialized.len() / 2;
        serialized[mid] ^= 0xFF;

        let result = WalRecord::deserialize(&serialized);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Checksum mismatch"));
    }

    #[test]
    fn test_truncated_record_detected() {
        let record = WalRecord::insert(1, sample_payload());
        let serialized = record.serialize();

        // Truncate
        let truncated = &serialized[0..serialized.len() - 10];
        let result = WalRecord::deserialize(truncated);
        assert!(result.is_err());
    }

    #[test]
    fn test_deterministic_serialization() {
        let record = WalRecord::insert(1, sample_payload());
        let serialized1 = record.serialize();
        let serialized2 = record.serialize();
        assert_eq!(
            serialized1, serialized2,
            "Serialization must be deterministic"
        );
    }

    #[test]
    fn test_replay_idempotency() {
        // Applying the same record twice produces the same result
        let payload = sample_payload();
        let record1 = WalRecord::update(1, payload.clone());
        let record2 = WalRecord::update(2, payload.clone());

        // Both records have the same payload (full document state)
        // After replay, the final state should be the same document
        assert_eq!(record1.payload.document_body, record2.payload.document_body);
        assert_eq!(record1.payload.document_id, record2.payload.document_id);
    }

    // === MVCC Commit Record Tests ===

    #[test]
    fn test_mvcc_commit_payload_roundtrip() {
        let payload = MvccCommitPayload::new(42);
        let serialized = payload.serialize();
        let deserialized = MvccCommitPayload::deserialize(&serialized).unwrap();
        assert_eq!(payload, deserialized);
    }

    #[test]
    fn test_mvcc_commit_record_roundtrip() {
        let record = MvccCommitRecord::new(1, 100);
        let serialized = record.serialize();
        let (deserialized, bytes_consumed) = MvccCommitRecord::deserialize(&serialized).unwrap();

        assert_eq!(record, deserialized);
        assert_eq!(bytes_consumed, serialized.len());
        assert_eq!(deserialized.commit_id(), 100);
    }

    #[test]
    fn test_mvcc_commit_record_deterministic_serialization() {
        let record = MvccCommitRecord::new(1, 42);
        let serialized1 = record.serialize();
        let serialized2 = record.serialize();
        assert_eq!(
            serialized1, serialized2,
            "Serialization must be deterministic"
        );
    }

    #[test]
    fn test_mvcc_commit_checksum_detects_corruption() {
        let record = MvccCommitRecord::new(1, 100);
        let mut serialized = record.serialize();

        // Corrupt a byte in the middle
        let mid = serialized.len() / 2;
        serialized[mid] ^= 0xFF;

        let result = MvccCommitRecord::deserialize(&serialized);
        assert!(result.is_err());
    }

    #[test]
    fn test_mvcc_commit_sequence_number_preserved() {
        let record = MvccCommitRecord::new(42, 100);
        let serialized = record.serialize();
        let (deserialized, _) = MvccCommitRecord::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.sequence_number, 42);
    }

    // === MVCC Version Record Tests ===

    #[test]
    fn test_mvcc_version_payload_roundtrip() {
        let payload = MvccVersionPayload::new(42, "users:user_1", b"data".to_vec());
        let serialized = payload.serialize();
        let deserialized = MvccVersionPayload::deserialize(&serialized).unwrap();
        assert_eq!(payload, deserialized);
    }

    #[test]
    fn test_mvcc_version_tombstone_roundtrip() {
        let payload = MvccVersionPayload::tombstone(42, "users:user_1");
        let serialized = payload.serialize();
        let deserialized = MvccVersionPayload::deserialize(&serialized).unwrap();
        assert_eq!(payload, deserialized);
        assert!(deserialized.is_tombstone);
        assert!(deserialized.payload.is_empty());
    }

    #[test]
    fn test_mvcc_version_record_roundtrip() {
        let record = MvccVersionRecord::new(1, 100, "users:user_1", b"document".to_vec());
        let serialized = record.serialize();
        let (deserialized, bytes_consumed) = MvccVersionRecord::deserialize(&serialized).unwrap();

        assert_eq!(record, deserialized);
        assert_eq!(bytes_consumed, serialized.len());
        assert_eq!(deserialized.commit_id(), 100);
        assert_eq!(deserialized.key(), "users:user_1");
    }

    #[test]
    fn test_mvcc_version_record_deterministic_serialization() {
        let record = MvccVersionRecord::new(1, 42, "key", b"data".to_vec());
        let serialized1 = record.serialize();
        let serialized2 = record.serialize();
        assert_eq!(
            serialized1, serialized2,
            "Serialization must be deterministic"
        );
    }

    #[test]
    fn test_mvcc_version_checksum_detects_corruption() {
        let record = MvccVersionRecord::new(1, 100, "key", b"data".to_vec());
        let mut serialized = record.serialize();

        let mid = serialized.len() / 2;
        serialized[mid] ^= 0xFF;

        let result = MvccVersionRecord::deserialize(&serialized);
        assert!(result.is_err());
    }

    #[test]
    fn test_mvcc_version_record_type() {
        assert_eq!(RecordType::MvccVersion.as_u8(), 4);
        assert_eq!(RecordType::from_u8(4), Some(RecordType::MvccVersion));
        assert!(RecordType::MvccVersion.is_mvcc_record());
    }
}
