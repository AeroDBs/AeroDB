//! Result types for query execution

use serde_json::Value;

/// A single document in the result set
#[derive(Debug, Clone)]
pub struct ResultDocument {
    /// Document ID
    pub id: String,
    /// Schema ID
    pub schema_id: String,
    /// Schema version
    pub schema_version: String,
    /// Document body as JSON
    pub body: Value,
    /// Offset in storage file (for debugging/testing)
    pub storage_offset: u64,
}

impl ResultDocument {
    /// Creates a new result document
    pub fn new(
        id: impl Into<String>,
        schema_id: impl Into<String>,
        schema_version: impl Into<String>,
        body: Value,
        storage_offset: u64,
    ) -> Self {
        Self {
            id: id.into(),
            schema_id: schema_id.into(),
            schema_version: schema_version.into(),
            body,
            storage_offset,
        }
    }

    /// Returns the document ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the document body
    pub fn body(&self) -> &Value {
        &self.body
    }
}

/// Result of query execution
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Documents in result order
    pub documents: Vec<ResultDocument>,
    /// Number of documents scanned
    pub scanned_count: usize,
    /// Number of documents returned
    pub returned_count: usize,
    /// Whether limit was applied
    pub limit_applied: bool,
}

impl ExecutionResult {
    /// Creates an empty result
    pub fn empty() -> Self {
        Self {
            documents: Vec::new(),
            scanned_count: 0,
            returned_count: 0,
            limit_applied: false,
        }
    }

    /// Returns true if no documents matched
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Returns the number of results
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Returns an iterator over the documents
    pub fn iter(&self) -> impl Iterator<Item = &ResultDocument> {
        self.documents.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_result_document() {
        let doc = ResultDocument::new("user_123", "users", "v1", json!({"name": "Alice"}), 1000);
        assert_eq!(doc.id(), "user_123");
        assert_eq!(doc.schema_id, "users");
    }

    #[test]
    fn test_execution_result_empty() {
        let result = ExecutionResult::empty();
        assert!(result.is_empty());
        assert_eq!(result.len(), 0);
    }
}
