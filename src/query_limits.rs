//! Query Resource Limits
//!
//! HARDENING: Enforce limits on individual query execution.
//!
//! - Max result set size
//! - Query timeout

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryLimitsConfig {
    /// Max documents in result set
    pub max_result_set_docs: usize,
    
    /// Default query timeout in ms
    pub query_timeout_ms: u64,
}

impl Default for QueryLimitsConfig {
    fn default() -> Self {
        Self {
            max_result_set_docs: 10000,
            query_timeout_ms: 30000, // 30s
        }
    }
}
