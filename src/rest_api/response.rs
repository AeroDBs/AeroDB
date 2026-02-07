//! # Response Formatting
//!
//! Standard response types for REST API.

use serde::Serialize;

/// List response with pagination
#[derive(Debug, Clone, Serialize)]
pub struct ListResponse<T: Serialize> {
    pub data: Vec<T>,
    pub count: usize,
    pub limit: usize,
    pub offset: usize,
}

impl<T: Serialize> ListResponse<T> {
    pub fn new(data: Vec<T>, limit: usize, offset: usize) -> Self {
        let count = data.len();
        Self {
            data,
            count,
            limit,
            offset,
        }
    }
}

/// Single record response
#[derive(Debug, Clone, Serialize)]
pub struct SingleResponse<T: Serialize> {
    pub data: T,
}

impl<T: Serialize> SingleResponse<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

/// Insert response with created records
#[derive(Debug, Clone, Serialize)]
pub struct InsertResponse<T: Serialize> {
    pub data: Vec<T>,
    pub count: usize,
}

impl<T: Serialize> InsertResponse<T> {
    pub fn new(data: Vec<T>) -> Self {
        let count = data.len();
        Self { data, count }
    }

    pub fn single(data: T) -> Self {
        Self {
            data: vec![data],
            count: 1,
        }
    }
}

/// Update response
#[derive(Debug, Clone, Serialize)]
pub struct UpdateResponse<T: Serialize> {
    pub data: T,
}

impl<T: Serialize> UpdateResponse<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

/// Delete response
#[derive(Debug, Clone, Serialize)]
pub struct DeleteResponse {
    pub deleted: bool,
}

impl DeleteResponse {
    pub fn success() -> Self {
        Self { deleted: true }
    }
}

/// Count-only response (for HEAD requests)
#[derive(Debug, Clone, Serialize)]
pub struct CountResponse {
    pub count: usize,
}

impl CountResponse {
    pub fn new(count: usize) -> Self {
        Self { count }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_list_response_serialization() {
        let response = ListResponse::new(vec![json!({"id": 1}), json!({"id": 2})], 20, 0);

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["count"], 2);
        assert_eq!(json["limit"], 20);
        assert_eq!(json["offset"], 0);
    }

    #[test]
    fn test_single_response_serialization() {
        let response = SingleResponse::new(json!({"id": 1, "name": "Test"}));

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["data"]["id"], 1);
    }
}
