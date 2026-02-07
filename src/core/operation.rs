//! Unified Operation Model
//!
//! All operations in AeroDB route through this enum.
//! This eliminates hard-coded dispatch in individual handlers.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// All operations in AeroDB route through this enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Operation {
    // Data operations
    Read(ReadOp),
    Write(WriteOp),
    Update(UpdateOp),
    Delete(DeleteOp),

    // Query operations
    Query(QueryOp),
    Explain(QueryOp),

    // Realtime operations
    Subscribe(SubscribeOp),
    Unsubscribe { subscription_id: String },
    Broadcast(BroadcastOp),

    // Function operations
    Invoke(InvokeOp),

    // File operations
    Upload(FileOp),
    Download(FileOp),
}

impl Operation {
    /// Get the collection name if this is a data operation
    pub fn collection(&self) -> Option<&str> {
        match self {
            Self::Read(r) => Some(&r.collection),
            Self::Write(w) => Some(&w.collection),
            Self::Update(u) => Some(&u.collection),
            Self::Delete(d) => Some(&d.collection),
            Self::Query(q) | Self::Explain(q) => Some(&q.collection),
            _ => None,
        }
    }

    /// Get operation name for metrics/logging
    pub fn name(&self) -> &'static str {
        match self {
            Self::Read(_) => "read",
            Self::Write(_) => "write",
            Self::Update(_) => "update",
            Self::Delete(_) => "delete",
            Self::Query(_) => "query",
            Self::Explain(_) => "explain",
            Self::Subscribe(_) => "subscribe",
            Self::Unsubscribe { .. } => "unsubscribe",
            Self::Broadcast(_) => "broadcast",
            Self::Invoke(_) => "invoke",
            Self::Upload(_) => "upload",
            Self::Download(_) => "download",
        }
    }

    /// Check if this operation requires authentication
    pub fn requires_auth(&self) -> bool {
        // Most operations require auth; public reads handled by RLS
        true
    }
}

/// Read a single document by ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadOp {
    pub collection: String,
    pub id: String,
    #[serde(default)]
    pub select: Option<Vec<String>>,
}

/// Write a new document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteOp {
    pub collection: String,
    pub document: Value,
    pub schema_id: String,
    pub schema_version: String,
}

/// Update an existing document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateOp {
    pub collection: String,
    pub id: String,
    pub updates: Value,
    #[serde(default)]
    pub schema_id: Option<String>,
    #[serde(default)]
    pub schema_version: Option<String>,
}

/// Delete a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteOp {
    pub collection: String,
    pub id: String,
    #[serde(default)]
    pub schema_id: Option<String>,
}

/// Query multiple documents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryOp {
    pub collection: String,
    #[serde(default)]
    pub filter: Option<Value>,
    #[serde(default)]
    pub select: Option<Vec<String>>,
    #[serde(default)]
    pub order: Option<Vec<OrderSpec>>,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
    #[serde(default)]
    pub schema_id: Option<String>,
    #[serde(default)]
    pub schema_version: Option<String>,
}

fn default_limit() -> usize {
    100
}

/// Sort specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderSpec {
    pub field: String,
    #[serde(default = "default_ascending")]
    pub ascending: bool,
}

fn default_ascending() -> bool {
    true
}

/// Subscribe to a channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeOp {
    pub channel: String,
    #[serde(default)]
    pub filter: Option<Value>,
}

/// Broadcast to a channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastOp {
    pub channel: String,
    pub event: String,
    pub payload: Value,
}

/// Invoke a function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeOp {
    pub function_name: String,
    #[serde(default)]
    pub payload: Value,
    #[serde(default)]
    pub async_mode: bool,
}

/// File operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOp {
    pub bucket: String,
    pub path: String,
    #[serde(default)]
    pub metadata: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_parsing() {
        let json = r#"{"op": "read", "collection": "users", "id": "user_1"}"#;
        let op: Operation = serde_json::from_str(json).unwrap();

        assert!(matches!(op, Operation::Read(_)));
        assert_eq!(op.collection(), Some("users"));
        assert_eq!(op.name(), "read");
    }

    #[test]
    fn test_query_operation() {
        let json = r#"{
            "op": "query",
            "collection": "posts",
            "filter": {"status": {"$eq": "published"}},
            "limit": 10
        }"#;
        let op: Operation = serde_json::from_str(json).unwrap();

        if let Operation::Query(q) = op {
            assert_eq!(q.collection, "posts");
            assert_eq!(q.limit, 10);
            assert!(q.filter.is_some());
        } else {
            panic!("Expected Query operation");
        }
    }

    #[test]
    fn test_write_operation() {
        let json = r#"{
            "op": "write",
            "collection": "users",
            "document": {"_id": "u1", "name": "Alice"},
            "schema_id": "users",
            "schema_version": "v1"
        }"#;
        let op: Operation = serde_json::from_str(json).unwrap();

        assert!(matches!(op, Operation::Write(_)));
    }
}
