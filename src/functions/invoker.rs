//! # Function Invoker

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use std::sync::Arc;

use super::errors::{FunctionError, FunctionResult};
use super::function::Function;
use super::runtime::{ExecutionContext, RuntimeConfig, WasmRuntime, WasmtimeRuntime};
use super::trigger::TriggerType;

/// Invocation context passed to functions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvocationContext {
    /// Invocation ID
    pub id: Uuid,

    /// Function ID
    pub function_id: Uuid,

    /// Trigger that caused invocation
    pub trigger: TriggerType,

    /// Request payload
    pub payload: Value,

    /// User ID (if authenticated)
    pub user_id: Option<Uuid>,

    /// Invocation timestamp
    pub timestamp: DateTime<Utc>,
}

impl InvocationContext {
    /// Create a new invocation context
    pub fn new(function: &Function, payload: Value, user_id: Option<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            function_id: function.id,
            trigger: function.trigger.clone(),
            payload,
            user_id,
            timestamp: Utc::now(),
        }
    }
}

/// Result of function invocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvocationResult {
    /// Invocation ID
    pub id: Uuid,

    /// Success flag
    pub success: bool,

    /// Return value (if successful)
    pub result: Option<Value>,

    /// Error message (if failed)
    pub error: Option<String>,

    /// Execution duration in milliseconds
    pub duration_ms: u64,

    /// Logs produced
    pub logs: Vec<String>,
}

impl InvocationResult {
    /// Create a successful result
    pub fn success(id: Uuid, result: Value, duration_ms: u64) -> Self {
        Self {
            id,
            success: true,
            result: Some(result),
            error: None,
            duration_ms,
            logs: Vec::new(),
        }
    }

    /// Create a failed result
    pub fn failure(id: Uuid, error: String, duration_ms: u64) -> Self {
        Self {
            id,
            success: false,
            result: None,
            error: Some(error),
            duration_ms,
            logs: Vec::new(),
        }
    }
}

/// Function invoker
#[derive(Debug, Clone)]
pub struct Invoker {
    runtime: Arc<WasmtimeRuntime>,
    config: RuntimeConfig,
}

impl Default for Invoker {
    fn default() -> Self {
        Self::new()
    }
}

impl Invoker {
    /// Create a new invoker
    pub fn new() -> Self {
        Self {
            runtime: Arc::new(WasmtimeRuntime::default()),
            config: RuntimeConfig::default(),
        }
    }

    /// Invoke a function
    ///
    /// Note: Actual WASM execution is stubbed. This simulates
    /// successful invocation for testing purposes.
    pub fn invoke(
        &self,
        function: &Function,
        context: InvocationContext,
    ) -> FunctionResult<InvocationResult> {
        // Check if function is enabled
        if !function.enabled {
            return Err(FunctionError::RuntimeError("Function is disabled".into()));
        }

        // Create execution context
        let mut exec_context = ExecutionContext::new(function, context.user_id);

        // Copy environment variables if any
        // ... (env logic would go here)

        // Execute via runtime
        let result = self
            .runtime
            .execute(function, context.payload, exec_context, &self.config)?;

        // Map ExecutionResult to InvocationResult
        if result.success {
            Ok(InvocationResult::success(
                context.id,
                result.result.unwrap_or(serde_json::Value::Null),
                result.duration_ms,
            ))
        } else {
            Ok(InvocationResult::failure(
                context.id,
                result.error.unwrap_or("Unknown error".to_string()),
                result.duration_ms,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invocation_context() {
        let func = Function::new(
            "test".to_string(),
            TriggerType::http("/test".to_string()),
            vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00], // Valid empty WASM module
        );

        let context = InvocationContext::new(&func, serde_json::json!({"message": "hello"}), None);

        assert_eq!(context.function_id, func.id);
    }

    #[test]
    fn test_invoke() {
        let invoker = Invoker::new();

        let func = Function::new(
            "echo".to_string(),
            TriggerType::http("/echo".to_string()),
            vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00],
        );

        let context = InvocationContext::new(&func, serde_json::json!({"input": "test"}), None);

        let result = invoker.invoke(&func, context).unwrap();
        assert!(result.success);
        assert!(result.result.is_some());
    }

    #[test]
    fn test_disabled_function() {
        let invoker = Invoker::new();

        let mut func = Function::new(
            "disabled".to_string(),
            TriggerType::http("/disabled".to_string()),
            vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00],
        );
        func.enabled = false;

        let context = InvocationContext::new(&func, serde_json::json!({}), None);

        assert!(invoker.invoke(&func, context).is_err());
    }
}
