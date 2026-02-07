//! Request Context
//!
//! Context carried through the execution pipeline.
//! Contains auth info, RLS filters, and observability metadata.

use std::collections::HashMap;
use std::time::Instant;

use serde_json::Value;
use uuid::Uuid;

/// Context carried through the execution pipeline
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Request ID for tracing
    pub request_id: Uuid,

    /// Authentication context
    pub auth: AuthContext,

    /// RLS filters to apply (injected by RLS middleware)
    pub rls_filters: Vec<RlsFilter>,

    /// Metadata for observability
    pub metadata: HashMap<String, Value>,

    /// Start time for duration tracking
    started_at: Instant,
}

impl RequestContext {
    /// Create a new request context
    pub fn new(auth: AuthContext) -> Self {
        Self {
            request_id: Uuid::new_v4(),
            auth,
            rls_filters: Vec::new(),
            metadata: HashMap::new(),
            started_at: Instant::now(),
        }
    }

    /// Create an anonymous context
    pub fn anonymous() -> Self {
        Self::new(AuthContext::anonymous())
    }

    /// Create a service role context (bypasses RLS)
    pub fn service_role() -> Self {
        Self::new(AuthContext::service_role())
    }

    /// Get elapsed time in milliseconds
    pub fn elapsed_ms(&self) -> u128 {
        self.started_at.elapsed().as_millis()
    }

    /// Check if RLS should be bypassed
    pub fn bypass_rls(&self) -> bool {
        self.auth.is_service_role
    }

    /// Add metadata for observability
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

impl Default for RequestContext {
    fn default() -> Self {
        Self::anonymous()
    }
}

/// Authentication context
#[derive(Debug, Clone, Default)]
pub struct AuthContext {
    /// The authenticated user's ID
    pub user_id: Option<Uuid>,

    /// Whether the request is authenticated
    pub is_authenticated: bool,

    /// Whether using service role (bypasses RLS)
    pub is_service_role: bool,

    /// Custom claims from JWT
    pub claims: HashMap<String, Value>,
}

impl AuthContext {
    /// Create context for an authenticated user
    pub fn authenticated(user_id: Uuid) -> Self {
        Self {
            user_id: Some(user_id),
            is_authenticated: true,
            is_service_role: false,
            claims: HashMap::new(),
        }
    }

    /// Create context with additional claims
    pub fn with_claims(mut self, claims: HashMap<String, Value>) -> Self {
        self.claims = claims;
        self
    }

    /// Create anonymous context
    pub fn anonymous() -> Self {
        Self::default()
    }

    /// Create service role context
    pub fn service_role() -> Self {
        Self {
            user_id: None,
            is_authenticated: true,
            is_service_role: true,
            claims: HashMap::new(),
        }
    }

    /// Get user ID or None
    pub fn user_id(&self) -> Option<Uuid> {
        self.user_id
    }

    /// Require user ID, returning error description if missing
    pub fn require_user_id(&self) -> Result<Uuid, &'static str> {
        self.user_id.ok_or("Authentication required")
    }
}

/// RLS filter to apply to queries
#[derive(Debug, Clone)]
pub struct RlsFilter {
    /// Field to filter on
    pub field: String,
    /// Filter operator
    pub operator: FilterOperator,
    /// Value to match
    pub value: Value,
}

impl RlsFilter {
    /// Create an equality filter
    pub fn eq(field: impl Into<String>, value: Value) -> Self {
        Self {
            field: field.into(),
            operator: FilterOperator::Eq,
            value,
        }
    }

    /// Create an IN filter
    pub fn in_list(field: impl Into<String>, values: Vec<Value>) -> Self {
        Self {
            field: field.into(),
            operator: FilterOperator::In,
            value: Value::Array(values),
        }
    }
}

/// Filter operators for RLS
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterOperator {
    Eq,
    Neq,
    In,
    Contains,
    Gt,
    Gte,
    Lt,
    Lte,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authenticated_context() {
        let user_id = Uuid::new_v4();
        let ctx = RequestContext::new(AuthContext::authenticated(user_id));

        assert!(ctx.auth.is_authenticated);
        assert!(!ctx.auth.is_service_role);
        assert_eq!(ctx.auth.user_id, Some(user_id));
    }

    #[test]
    fn test_service_role_bypasses_rls() {
        let ctx = RequestContext::service_role();
        assert!(ctx.bypass_rls());
    }

    #[test]
    fn test_rls_filter_creation() {
        let filter = RlsFilter::eq("owner_id", Value::String("user_123".into()));
        assert_eq!(filter.field, "owner_id");
        assert_eq!(filter.operator, FilterOperator::Eq);
    }
}
