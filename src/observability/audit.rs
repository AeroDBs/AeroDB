//! Phase 7 Audit Logging
//!
//! Per PHASE7_AUDITABILITY.md:
//! - Every command attempt MUST be logged
//! - Every outcome (success, rejection, failure) MUST be logged
//! - Audit log is append-only and durable
//! - Crash-safe: writes are synced before acknowledgement
//!
//! Per PHASE7_INVARIANTS.md §P7-O3:
//! - States and action history are written to persistent append-only audit logs.
//! - No background purging or retention policies (those are external concerns).

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use uuid::Uuid;

/// Audit action type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditAction {
    /// Command was requested.
    CommandRequested,

    /// Confirmation was requested.
    ConfirmationRequested,

    /// Confirmation was provided.
    ConfirmationProvided,

    /// Confirmation was rejected by operator.
    ConfirmationRejected,

    /// Confirmation expired.
    ConfirmationExpired,

    /// Command was executed successfully.
    CommandExecuted,

    /// Command was rejected (validation failure).
    CommandRejected,

    /// Command failed during execution.
    CommandFailed,

    /// Authority check performed.
    AuthorityCheck,
}

impl AuditAction {
    /// Returns the action name string.
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditAction::CommandRequested => "COMMAND_REQUESTED",
            AuditAction::ConfirmationRequested => "CONFIRMATION_REQUESTED",
            AuditAction::ConfirmationProvided => "CONFIRMATION_PROVIDED",
            AuditAction::ConfirmationRejected => "CONFIRMATION_REJECTED",
            AuditAction::ConfirmationExpired => "CONFIRMATION_EXPIRED",
            AuditAction::CommandExecuted => "COMMAND_EXECUTED",
            AuditAction::CommandRejected => "COMMAND_REJECTED",
            AuditAction::CommandFailed => "COMMAND_FAILED",
            AuditAction::AuthorityCheck => "AUTHORITY_CHECK",
        }
    }
}

impl fmt::Display for AuditAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Audit record outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditOutcome {
    /// Action succeeded.
    Success,

    /// Action was rejected.
    Rejected,

    /// Action failed.
    Failed,

    /// Action is pending (e.g., awaiting confirmation).
    Pending,
}

impl AuditOutcome {
    /// Returns the outcome string.
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditOutcome::Success => "SUCCESS",
            AuditOutcome::Rejected => "REJECTED",
            AuditOutcome::Failed => "FAILED",
            AuditOutcome::Pending => "PENDING",
        }
    }
}

impl fmt::Display for AuditOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A single audit record.
///
/// Per PHASE7_AUDITABILITY.md §3:
/// Each record MUST include: timestamp, action, authority, target, outcome.
#[derive(Debug, Clone)]
pub struct AuditRecord {
    /// Unique record ID.
    pub id: Uuid,

    /// Timestamp when the action occurred.
    pub timestamp: SystemTime,

    /// The action that occurred.
    pub action: AuditAction,

    /// Command name (if applicable).
    pub command_name: Option<String>,

    /// Request ID for correlation.
    pub request_id: Option<Uuid>,

    /// Target node/replica ID (if applicable).
    pub target_id: Option<Uuid>,

    /// Authority level of the requester.
    pub authority_level: Option<String>,

    /// Operator identity (if known).
    pub operator_id: Option<String>,

    /// Confirmation token ID (if applicable).
    pub confirmation_token: Option<Uuid>,

    /// Outcome of the action.
    pub outcome: AuditOutcome,

    /// Error message (if outcome is Rejected or Failed).
    pub error_message: Option<String>,

    /// Referenced invariant (if applicable).
    pub invariant: Option<String>,
}

impl AuditRecord {
    /// Create a new audit record.
    pub fn new(action: AuditAction, outcome: AuditOutcome) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            action,
            command_name: None,
            request_id: None,
            target_id: None,
            authority_level: None,
            operator_id: None,
            confirmation_token: None,
            outcome,
            error_message: None,
            invariant: None,
        }
    }

    /// Set command name.
    pub fn with_command(mut self, name: impl Into<String>) -> Self {
        self.command_name = Some(name.into());
        self
    }

    /// Set request ID.
    pub fn with_request_id(mut self, id: Uuid) -> Self {
        self.request_id = Some(id);
        self
    }

    /// Set target ID.
    pub fn with_target(mut self, id: Uuid) -> Self {
        self.target_id = Some(id);
        self
    }

    /// Set authority level.
    pub fn with_authority(mut self, level: impl Into<String>) -> Self {
        self.authority_level = Some(level.into());
        self
    }

    /// Set operator ID.
    pub fn with_operator(mut self, id: impl Into<String>) -> Self {
        self.operator_id = Some(id.into());
        self
    }

    /// Set confirmation token.
    pub fn with_confirmation_token(mut self, id: Uuid) -> Self {
        self.confirmation_token = Some(id);
        self
    }

    /// Set error message.
    pub fn with_error(mut self, message: impl Into<String>) -> Self {
        self.error_message = Some(message.into());
        self
    }

    /// Set invariant reference.
    pub fn with_invariant(mut self, invariant: impl Into<String>) -> Self {
        self.invariant = Some(invariant.into());
        self
    }

    /// Serialize to JSON line (for append-only logging).
    pub fn to_json(&self) -> String {
        // Manual JSON to avoid dependency; simple and deterministic
        let timestamp = self
            .timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut json = format!(
            r#"{{"id":"{}","ts":{},"action":"{}","outcome":"{}""#,
            self.id, timestamp, self.action, self.outcome
        );

        if let Some(ref cmd) = self.command_name {
            json.push_str(&format!(r#","cmd":"{}""#, escape_json(cmd)));
        }
        if let Some(ref rid) = self.request_id {
            json.push_str(&format!(r#","req_id":"{}""#, rid));
        }
        if let Some(ref tid) = self.target_id {
            json.push_str(&format!(r#","target":"{}""#, tid));
        }
        if let Some(ref auth) = self.authority_level {
            json.push_str(&format!(r#","auth":"{}""#, escape_json(auth)));
        }
        if let Some(ref op) = self.operator_id {
            json.push_str(&format!(r#","operator":"{}""#, escape_json(op)));
        }
        if let Some(ref tok) = self.confirmation_token {
            json.push_str(&format!(r#","token":"{}""#, tok));
        }
        if let Some(ref err) = self.error_message {
            json.push_str(&format!(r#","error":"{}""#, escape_json(err)));
        }
        if let Some(ref inv) = self.invariant {
            json.push_str(&format!(r#","invariant":"{}""#, escape_json(inv)));
        }

        json.push('}');
        json
    }
}

/// Escape special JSON characters.
fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Audit log trait.
///
/// Per PHASE7_AUDITABILITY.md §4:
/// Audit log MUST be append-only and durable.
pub trait AuditLog: Send + Sync {
    /// Append a record to the audit log.
    ///
    /// This MUST be synchronous and durable.
    /// The record MUST be visible after this call returns.
    fn append(&self, record: &AuditRecord) -> io::Result<()>;

    /// Sync the audit log to durable storage.
    fn sync(&self) -> io::Result<()>;
}

/// File-based audit log implementation.
///
/// Per PHASE7_AUDITABILITY.md §5:
/// - Append-only file format
/// - fsync after each write for durability
/// - One JSON record per line
pub struct FileAuditLog {
    path: PathBuf,
    writer: Arc<Mutex<BufWriter<File>>>,
}

impl FileAuditLog {
    /// Open or create an audit log file.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new().create(true).append(true).open(&path)?;

        Ok(Self {
            path,
            writer: Arc::new(Mutex::new(BufWriter::new(file))),
        })
    }

    /// Get the audit log path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl AuditLog for FileAuditLog {
    fn append(&self, record: &AuditRecord) -> io::Result<()> {
        let json = record.to_json();
        let mut writer = self.writer.lock().unwrap();
        writeln!(writer, "{}", json)?;
        writer.flush()?;
        // Sync to disk for durability
        writer.get_ref().sync_all()
    }

    fn sync(&self) -> io::Result<()> {
        let writer = self.writer.lock().unwrap();
        writer.get_ref().sync_all()
    }
}

/// In-memory audit log for testing.
#[derive(Debug, Default)]
pub struct MemoryAuditLog {
    records: Arc<Mutex<Vec<AuditRecord>>>,
}

impl MemoryAuditLog {
    /// Create a new in-memory audit log.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all recorded entries.
    pub fn records(&self) -> Vec<AuditRecord> {
        self.records.lock().unwrap().clone()
    }

    /// Get the number of records.
    pub fn len(&self) -> usize {
        self.records.lock().unwrap().len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.records.lock().unwrap().is_empty()
    }
}

impl AuditLog for MemoryAuditLog {
    fn append(&self, record: &AuditRecord) -> io::Result<()> {
        self.records.lock().unwrap().push(record.clone());
        Ok(())
    }

    fn sync(&self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_audit_record_creation() {
        let record = AuditRecord::new(AuditAction::CommandRequested, AuditOutcome::Pending)
            .with_command("request_promotion")
            .with_request_id(Uuid::new_v4())
            .with_authority("OPERATOR");

        assert_eq!(record.action, AuditAction::CommandRequested);
        assert_eq!(record.outcome, AuditOutcome::Pending);
        assert_eq!(record.command_name, Some("request_promotion".to_string()));
    }

    #[test]
    fn test_audit_record_json() {
        let record = AuditRecord::new(AuditAction::CommandExecuted, AuditOutcome::Success)
            .with_command("request_promotion");

        let json = record.to_json();
        assert!(json.contains("COMMAND_EXECUTED"));
        assert!(json.contains("SUCCESS"));
        assert!(json.contains("request_promotion"));
    }

    #[test]
    fn test_memory_audit_log() {
        let log = MemoryAuditLog::new();

        let record1 = AuditRecord::new(AuditAction::CommandRequested, AuditOutcome::Pending);
        let record2 = AuditRecord::new(AuditAction::CommandExecuted, AuditOutcome::Success);

        log.append(&record1).unwrap();
        log.append(&record2).unwrap();

        assert_eq!(log.len(), 2);
        let records = log.records();
        assert_eq!(records[0].action, AuditAction::CommandRequested);
        assert_eq!(records[1].action, AuditAction::CommandExecuted);
    }

    #[test]
    fn test_file_audit_log() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("audit.log");

        let log = FileAuditLog::open(&path).unwrap();

        let record = AuditRecord::new(AuditAction::CommandExecuted, AuditOutcome::Success)
            .with_command("inspect_cluster_state");

        log.append(&record).unwrap();

        // Read back the log
        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("COMMAND_EXECUTED"));
        assert!(contents.contains("inspect_cluster_state"));
    }

    #[test]
    fn test_escape_json() {
        assert_eq!(escape_json("hello"), "hello");
        assert_eq!(escape_json("hello\"world"), "hello\\\"world");
        assert_eq!(escape_json("line\nbreak"), "line\\nbreak");
    }
}
