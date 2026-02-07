//! API request types
//!
//! JSON request parsing for all supported operations.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::errors::{ApiError, ApiResult};

/// Operation type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    Insert,
    Update,
    Delete,
    Query,
    Explain,
}

/// Insert request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertRequest {
    pub schema_id: String,
    pub schema_version: String,
    pub document: Value,
}

/// Update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRequest {
    pub schema_id: String,
    pub schema_version: String,
    pub document: Value,
}

/// Delete request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRequest {
    pub schema_id: String,
    pub document_id: String,
}

/// Query request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    pub schema_id: String,
    pub schema_version: String,
    #[serde(default)]
    pub filter: Option<Value>,
    #[serde(default)]
    pub sort: Option<String>,
    pub limit: usize,
}

/// Unified request envelope
#[derive(Debug, Clone)]
pub enum Request {
    Insert(InsertRequest),
    Update(UpdateRequest),
    Delete(DeleteRequest),
    Query(QueryRequest),
    Explain(QueryRequest),
}

/// Raw request for parsing
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawRequest {
    op: String,
    #[serde(default)]
    schema_id: Option<String>,
    #[serde(default)]
    schema_version: Option<String>,
    #[serde(default)]
    document: Option<Value>,
    #[serde(default)]
    document_id: Option<String>,
    #[serde(default)]
    filter: Option<Value>,
    #[serde(default)]
    sort: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

impl Request {
    /// Parse a request from JSON string
    pub fn parse(json: &str) -> ApiResult<Self> {
        let raw: RawRequest = serde_json::from_str(json)
            .map_err(|e| ApiError::invalid_request(format!("Invalid JSON: {}", e)))?;

        match raw.op.as_str() {
            "insert" => {
                let schema_id = raw
                    .schema_id
                    .ok_or_else(|| ApiError::invalid_request("Missing schema_id"))?;
                let schema_version = raw
                    .schema_version
                    .ok_or_else(|| ApiError::invalid_request("Missing schema_version"))?;
                let document = raw
                    .document
                    .ok_or_else(|| ApiError::invalid_request("Missing document"))?;

                Ok(Request::Insert(InsertRequest {
                    schema_id,
                    schema_version,
                    document,
                }))
            }
            "update" => {
                let schema_id = raw
                    .schema_id
                    .ok_or_else(|| ApiError::invalid_request("Missing schema_id"))?;
                let schema_version = raw
                    .schema_version
                    .ok_or_else(|| ApiError::invalid_request("Missing schema_version"))?;
                let document = raw
                    .document
                    .ok_or_else(|| ApiError::invalid_request("Missing document"))?;

                Ok(Request::Update(UpdateRequest {
                    schema_id,
                    schema_version,
                    document,
                }))
            }
            "delete" => {
                let schema_id = raw
                    .schema_id
                    .ok_or_else(|| ApiError::invalid_request("Missing schema_id"))?;
                let document_id = raw
                    .document_id
                    .ok_or_else(|| ApiError::invalid_request("Missing document_id"))?;

                Ok(Request::Delete(DeleteRequest {
                    schema_id,
                    document_id,
                }))
            }
            "query" => {
                let schema_id = raw
                    .schema_id
                    .ok_or_else(|| ApiError::invalid_request("Missing schema_id"))?;
                let schema_version = raw
                    .schema_version
                    .ok_or_else(|| ApiError::invalid_request("Missing schema_version"))?;
                let limit = raw
                    .limit
                    .ok_or_else(|| ApiError::invalid_request("Missing limit"))?;

                Ok(Request::Query(QueryRequest {
                    schema_id,
                    schema_version,
                    filter: raw.filter,
                    sort: raw.sort,
                    limit,
                }))
            }
            "explain" => {
                let schema_id = raw
                    .schema_id
                    .ok_or_else(|| ApiError::invalid_request("Missing schema_id"))?;
                let schema_version = raw
                    .schema_version
                    .ok_or_else(|| ApiError::invalid_request("Missing schema_version"))?;
                let limit = raw
                    .limit
                    .ok_or_else(|| ApiError::invalid_request("Missing limit"))?;

                Ok(Request::Explain(QueryRequest {
                    schema_id,
                    schema_version,
                    filter: raw.filter,
                    sort: raw.sort,
                    limit,
                }))
            }
            other => Err(ApiError::unknown_operation(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_insert() {
        let json = r#"{
            "op": "insert",
            "schema_id": "users",
            "schema_version": "v1",
            "document": {"_id": "user_1", "name": "Alice"}
        }"#;

        let req = Request::parse(json).unwrap();
        match req {
            Request::Insert(r) => {
                assert_eq!(r.schema_id, "users");
                assert_eq!(r.schema_version, "v1");
            }
            _ => panic!("Expected Insert"),
        }
    }

    #[test]
    fn test_parse_query() {
        let json = r#"{
            "op": "query",
            "schema_id": "users",
            "schema_version": "v1",
            "filter": {"age": {"$eq": 25}},
            "limit": 10
        }"#;

        let req = Request::parse(json).unwrap();
        match req {
            Request::Query(r) => {
                assert_eq!(r.schema_id, "users");
                assert_eq!(r.limit, 10);
            }
            _ => panic!("Expected Query"),
        }
    }

    #[test]
    fn test_parse_unknown_op() {
        let json = r#"{"op": "dropDatabase"}"#;
        let result = Request::parse(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().code().contains("UNKNOWN_OPERATION"));
    }

    #[test]
    fn test_parse_missing_field() {
        let json = r#"{"op": "insert"}"#;
        let result = Request::parse(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().message().contains("Missing"));
    }
}
