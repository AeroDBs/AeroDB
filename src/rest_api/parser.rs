//! # Query Parameter Parser
//!
//! Parses REST API query parameters into structured queries.

use std::collections::HashMap;

use super::errors::{RestError, RestResult};
use super::filter::{FilterExpr, FilterOperator};

/// Maximum number of records that can be returned
pub const MAX_LIMIT: usize = 1000;

/// Default limit if not specified
pub const DEFAULT_LIMIT: usize = 100;

/// Parsed query parameters
#[derive(Debug, Clone)]
pub struct QueryParams {
    /// Fields to select (None = all)
    pub select: Option<Vec<String>>,

    /// Filter expressions
    pub filters: Vec<FilterExpr>,

    /// Order by clauses
    pub order: Vec<OrderBy>,

    /// Number of records to return
    pub limit: usize,

    /// Number of records to skip
    pub offset: usize,
}

impl Default for QueryParams {
    fn default() -> Self {
        Self {
            select: None,
            filters: Vec::new(),
            order: Vec::new(),
            limit: DEFAULT_LIMIT,
            offset: 0,
        }
    }
}

/// Order by clause
#[derive(Debug, Clone)]
pub struct OrderBy {
    pub field: String,
    pub ascending: bool,
}

impl QueryParams {
    /// Parse query parameters from a HashMap
    pub fn parse(params: &HashMap<String, String>) -> RestResult<Self> {
        let mut result = QueryParams {
            limit: DEFAULT_LIMIT,
            ..Default::default()
        };

        for (key, value) in params {
            match key.as_str() {
                "select" => {
                    result.select = Some(parse_select(value)?);
                }
                "order" => {
                    result.order = parse_order(value)?;
                }
                "limit" => {
                    result.limit = parse_limit(value)?;
                }
                "offset" => {
                    result.offset = parse_offset(value)?;
                }
                _ => {
                    // Treat as filter
                    if let Some(filter) = parse_filter(key, value)? {
                        result.filters.push(filter);
                    }
                }
            }
        }

        // Enforce maximum limit
        if result.limit > MAX_LIMIT {
            return Err(RestError::LimitExceeded(result.limit, MAX_LIMIT));
        }

        Ok(result)
    }

    /// Check if this query is bounded
    pub fn is_bounded(&self) -> bool {
        self.limit > 0 && self.limit <= MAX_LIMIT
    }
}

/// Parse select parameter (comma-separated field list)
fn parse_select(value: &str) -> RestResult<Vec<String>> {
    if value == "*" {
        return Ok(vec!["*".to_string()]);
    }

    let fields: Vec<String> = value
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if fields.is_empty() {
        return Err(RestError::InvalidQueryParam(
            "select cannot be empty".to_string(),
        ));
    }

    Ok(fields)
}

/// Parse order parameter (comma-separated field.direction)
fn parse_order(value: &str) -> RestResult<Vec<OrderBy>> {
    let mut orders = Vec::new();

    for part in value.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let (field, ascending) = if let Some(dot_pos) = part.rfind('.') {
            let field = &part[..dot_pos];
            let direction = &part[dot_pos + 1..];

            let ascending = match direction.to_lowercase().as_str() {
                "asc" => true,
                "desc" => false,
                _ => {
                    return Err(RestError::InvalidQueryParam(format!(
                        "Invalid order direction: {}",
                        direction
                    )))
                }
            };

            (field.to_string(), ascending)
        } else {
            // Default to ascending
            (part.to_string(), true)
        };

        orders.push(OrderBy { field, ascending });
    }

    Ok(orders)
}

/// Parse limit parameter
fn parse_limit(value: &str) -> RestResult<usize> {
    value
        .parse()
        .map_err(|_| RestError::InvalidQueryParam(format!("Invalid limit: {}", value)))
}

/// Parse offset parameter
fn parse_offset(value: &str) -> RestResult<usize> {
    value
        .parse()
        .map_err(|_| RestError::InvalidQueryParam(format!("Invalid offset: {}", value)))
}

/// Parse a filter expression from key=value
fn parse_filter(field: &str, value: &str) -> RestResult<Option<FilterExpr>> {
    // Check for operator prefix
    let (operator, actual_value) = if let Some(dot_pos) = value.find('.') {
        let op_str = &value[..dot_pos];
        let val = &value[dot_pos + 1..];

        let op = match op_str {
            "eq" => FilterOperator::Eq,
            "neq" => FilterOperator::Neq,
            "gt" => FilterOperator::Gt,
            "gte" => FilterOperator::Gte,
            "lt" => FilterOperator::Lt,
            "lte" => FilterOperator::Lte,
            "like" => FilterOperator::Like,
            "in" => FilterOperator::In,
            "is" => FilterOperator::Is,
            _ => {
                // No known operator, treat as eq with the whole value
                return Ok(Some(FilterExpr {
                    field: field.to_string(),
                    operator: FilterOperator::Eq,
                    value: parse_filter_value(value)?,
                }));
            }
        };

        (op, val)
    } else {
        // No operator, default to eq
        (FilterOperator::Eq, value)
    };

    Ok(Some(FilterExpr {
        field: field.to_string(),
        operator,
        value: parse_filter_value(actual_value)?,
    }))
}

/// Parse a filter value (handles lists for 'in' operator)
fn parse_filter_value(value: &str) -> RestResult<serde_json::Value> {
    // Check for list syntax: (a,b,c)
    if value.starts_with('(') && value.ends_with(')') {
        let inner = &value[1..value.len() - 1];
        let items: Vec<serde_json::Value> = inner
            .split(',')
            .map(|s| serde_json::Value::String(s.trim().to_string()))
            .collect();
        return Ok(serde_json::Value::Array(items));
    }

    // Check for null
    if value == "null" {
        return Ok(serde_json::Value::Null);
    }

    // Check for boolean
    if value == "true" {
        return Ok(serde_json::Value::Bool(true));
    }
    if value == "false" {
        return Ok(serde_json::Value::Bool(false));
    }

    // Check for number
    if let Ok(n) = value.parse::<i64>() {
        return Ok(serde_json::Value::Number(n.into()));
    }
    if let Ok(n) = value.parse::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(n) {
            return Ok(serde_json::Value::Number(num));
        }
    }

    // Default to string
    Ok(serde_json::Value::String(value.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_select() {
        let fields = parse_select("id,name,email").unwrap();
        assert_eq!(fields, vec!["id", "name", "email"]);

        let all = parse_select("*").unwrap();
        assert_eq!(all, vec!["*"]);
    }

    #[test]
    fn test_parse_order() {
        let orders = parse_order("created_at.desc,name.asc").unwrap();
        assert_eq!(orders.len(), 2);
        assert_eq!(orders[0].field, "created_at");
        assert!(!orders[0].ascending);
        assert_eq!(orders[1].field, "name");
        assert!(orders[1].ascending);
    }

    #[test]
    fn test_parse_limit() {
        assert_eq!(parse_limit("50").unwrap(), 50);
        assert!(parse_limit("abc").is_err());
    }

    #[test]
    fn test_parse_filter() {
        let filter = parse_filter("age", "gt.18").unwrap().unwrap();
        assert_eq!(filter.field, "age");
        assert_eq!(filter.operator, FilterOperator::Gt);
        assert_eq!(filter.value, serde_json::json!(18));

        let eq_filter = parse_filter("name", "John").unwrap().unwrap();
        assert_eq!(eq_filter.operator, FilterOperator::Eq);
        assert_eq!(eq_filter.value, serde_json::json!("John"));
    }

    #[test]
    fn test_parse_in_filter() {
        let filter = parse_filter("status", "in.(active,pending,done)")
            .unwrap()
            .unwrap();
        assert_eq!(filter.operator, FilterOperator::In);
        assert_eq!(
            filter.value,
            serde_json::json!(["active", "pending", "done"])
        );
    }

    #[test]
    fn test_full_query_params() {
        let mut params = HashMap::new();
        params.insert("select".to_string(), "id,name".to_string());
        params.insert("order".to_string(), "name.asc".to_string());
        params.insert("limit".to_string(), "20".to_string());
        params.insert("offset".to_string(), "10".to_string());
        params.insert("status".to_string(), "eq.active".to_string());

        let query = QueryParams::parse(&params).unwrap();

        assert_eq!(
            query.select,
            Some(vec!["id".to_string(), "name".to_string()])
        );
        assert_eq!(query.order.len(), 1);
        assert_eq!(query.limit, 20);
        assert_eq!(query.offset, 10);
        assert_eq!(query.filters.len(), 1);
    }

    #[test]
    fn test_limit_exceeded() {
        let mut params = HashMap::new();
        params.insert("limit".to_string(), "5000".to_string());

        let result = QueryParams::parse(&params);
        assert!(matches!(result, Err(RestError::LimitExceeded(5000, 1000))));
    }
}
