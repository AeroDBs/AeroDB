//! # Function Definition

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

use super::trigger::TriggerType;

/// Function configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionConfig {
    /// Execution timeout in milliseconds
    pub timeout_ms: u64,

    /// Memory limit in MB
    pub memory_mb: u32,

    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Maximum retries for failed invocations
    #[serde(default = "default_retries")]
    pub max_retries: u32,
}

fn default_retries() -> u32 {
    3
}

impl Default for FunctionConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 10_000, // 10 seconds
            memory_mb: 64,
            env: HashMap::new(),
            max_retries: default_retries(),
        }
    }
}

/// A serverless function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    /// Unique function ID
    pub id: Uuid,

    /// Function name (unique)
    pub name: String,

    /// Function description
    #[serde(default)]
    pub description: String,

    /// Trigger type
    pub trigger: TriggerType,

    /// WASM module hash (for caching)
    pub wasm_hash: String,

    /// WASM module bytes (stored separately in production)
    #[serde(skip)]
    pub wasm_bytes: Vec<u8>,

    /// Function configuration
    pub config: FunctionConfig,

    /// Whether function is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Created timestamp
    pub created_at: DateTime<Utc>,

    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

fn default_enabled() -> bool {
    true
}

impl Function {
    /// Create a new function
    pub fn new(name: String, trigger: TriggerType, wasm_bytes: Vec<u8>) -> Self {
        let now = Utc::now();
        let mut hasher = Sha256::new();
        hasher.update(&wasm_bytes);
        let wasm_hash = format!("{:x}", hasher.finalize());

        Self {
            id: Uuid::new_v4(),
            name,
            description: String::new(),
            trigger,
            wasm_hash,
            wasm_bytes,
            config: FunctionConfig::default(),
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create with custom config
    pub fn with_config(mut self, config: FunctionConfig) -> Self {
        self.config = config;
        self
    }

    /// Update WASM bytes
    pub fn update_wasm(&mut self, wasm_bytes: Vec<u8>) {
        let mut hasher = Sha256::new();
        hasher.update(&wasm_bytes);
        self.wasm_hash = format!("{:x}", hasher.finalize());
        self.wasm_bytes = wasm_bytes;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_creation() {
        let func = Function::new(
            "hello".to_string(),
            TriggerType::http("/hello".to_string()),
            vec![0, 1, 2, 3],
        );

        assert_eq!(func.name, "hello");
        assert!(func.enabled);
        assert!(!func.wasm_hash.is_empty());
    }

    #[test]
    fn test_update_wasm() {
        let mut func = Function::new(
            "test".to_string(),
            TriggerType::http("/test".to_string()),
            vec![1, 2, 3],
        );

        let old_hash = func.wasm_hash.clone();
        func.update_wasm(vec![4, 5, 6]);

        assert_ne!(func.wasm_hash, old_hash);
    }
}
