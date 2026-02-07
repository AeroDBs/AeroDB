//! # Row-Level Security (RLS)
//!
//! Fine-grained access control at the document level.
//!
//! ## Invariants
//! - AUTH-RLS1: No silent bypass (requires explicit service role)
//! - AUTH-RLS2: Deterministic query injection
//! - AUTH-RLS3: Write validation before execution
//! - AUTH-RLS4: Filter applied at planning time

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::errors::{AuthError, AuthResult};

/// RLS context carried with each request
#[derive(Debug, Clone, Default)]
pub struct RlsContext {
    /// The authenticated user's ID (None if anonymous)
    pub user_id: Option<Uuid>,

    /// Whether the request is authenticated
    pub is_authenticated: bool,

    /// Whether using service role (bypasses RLS)
    pub is_service_role: bool,

    /// Custom claims from JWT (for advanced policies)
    pub claims: HashMap<String, serde_json::Value>,
}

impl RlsContext {
    /// Create context for an authenticated user
    pub fn authenticated(user_id: Uuid) -> Self {
        Self {
            user_id: Some(user_id),
            is_authenticated: true,
            is_service_role: false,
            claims: HashMap::new(),
        }
    }

    /// Create context for anonymous access
    pub fn anonymous() -> Self {
        Self {
            user_id: None,
            is_authenticated: false,
            is_service_role: false,
            claims: HashMap::new(),
        }
    }

    /// Create context for service role (bypasses RLS)
    pub fn service_role() -> Self {
        Self {
            user_id: None,
            is_authenticated: true,
            is_service_role: true,
            claims: HashMap::new(),
        }
    }

    /// Check if this context allows RLS bypass
    pub fn can_bypass_rls(&self) -> bool {
        self.is_service_role
    }

    /// Get the user ID or error if not authenticated
    pub fn require_user_id(&self) -> AuthResult<Uuid> {
        self.user_id.ok_or(AuthError::AuthenticationRequired)
    }
}

/// RLS policy types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RlsPolicy {
    /// No RLS - allow all access
    #[serde(rename = "none")]
    None,

    /// Ownership-based policy
    #[serde(rename = "ownership")]
    Ownership {
        /// Field containing owner ID
        owner_field: String,
    },

    /// Public read, owner write
    #[serde(rename = "public_read")]
    PublicRead {
        /// Field containing owner ID for writes
        owner_field: String,
    },

    /// Custom predicate policy (future)
    #[serde(rename = "custom")]
    Custom {
        /// Read predicate (JSON path expression)
        read_predicate: Option<String>,
        /// Write predicate (JSON path expression)
        write_predicate: Option<String>,
    },
}

impl Default for RlsPolicy {
    fn default() -> Self {
        Self::Ownership {
            owner_field: "owner_id".to_string(),
        }
    }
}

/// RLS filter to apply to queries
#[derive(Debug, Clone)]
pub struct RlsFilter {
    /// Field to filter on
    pub field: String,
    /// Value to match
    pub value: serde_json::Value,
}

/// RLS enforcer trait
pub trait RlsEnforcer: Send + Sync {
    /// Get the RLS filter to apply to a read query
    ///
    /// Returns None if no filter is needed (e.g., service role or public policy)
    fn get_read_filter(&self, collection: &str, ctx: &RlsContext) -> AuthResult<Option<RlsFilter>>;

    /// Validate a document can be written with the given context
    fn validate_write(
        &self,
        collection: &str,
        document: &serde_json::Value,
        ctx: &RlsContext,
    ) -> AuthResult<()>;

    /// Prepare a document for insertion (set owner field)
    fn prepare_insert(
        &self,
        collection: &str,
        document: &mut serde_json::Value,
        ctx: &RlsContext,
    ) -> AuthResult<()>;
}

/// Default RLS enforcer implementation
pub struct DefaultRlsEnforcer {
    /// Policies per collection
    policies: HashMap<String, RlsPolicy>,

    /// Default policy for collections without explicit policy
    default_policy: RlsPolicy,
}

impl DefaultRlsEnforcer {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
            default_policy: RlsPolicy::default(),
        }
    }

    pub fn with_policy(mut self, collection: &str, policy: RlsPolicy) -> Self {
        self.policies.insert(collection.to_string(), policy);
        self
    }

    pub fn with_default_policy(mut self, policy: RlsPolicy) -> Self {
        self.default_policy = policy;
        self
    }

    fn get_policy(&self, collection: &str) -> &RlsPolicy {
        self.policies
            .get(collection)
            .unwrap_or(&self.default_policy)
    }
}

impl Default for DefaultRlsEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

impl RlsEnforcer for DefaultRlsEnforcer {
    fn get_read_filter(&self, collection: &str, ctx: &RlsContext) -> AuthResult<Option<RlsFilter>> {
        // Service role bypasses RLS
        if ctx.is_service_role {
            return Ok(None);
        }

        let policy = self.get_policy(collection);

        match policy {
            RlsPolicy::None => Ok(None),

            RlsPolicy::Ownership { owner_field } => {
                let user_id = ctx.require_user_id()?;
                Ok(Some(RlsFilter {
                    field: owner_field.clone(),
                    value: serde_json::json!(user_id.to_string()),
                }))
            }

            RlsPolicy::PublicRead { .. } => {
                // Public read - no filter
                Ok(None)
            }

            RlsPolicy::Custom { read_predicate, .. } => {
                if read_predicate.is_some() {
                    // Custom predicates not yet implemented
                    Err(AuthError::InvalidPolicy(
                        "Custom predicates not yet supported".to_string(),
                    ))
                } else {
                    Ok(None)
                }
            }
        }
    }

    fn validate_write(
        &self,
        collection: &str,
        document: &serde_json::Value,
        ctx: &RlsContext,
    ) -> AuthResult<()> {
        // Service role bypasses RLS
        if ctx.is_service_role {
            return Ok(());
        }

        let policy = self.get_policy(collection);

        match policy {
            RlsPolicy::None => Ok(()),

            RlsPolicy::Ownership { owner_field } | RlsPolicy::PublicRead { owner_field } => {
                let user_id = ctx.require_user_id()?;

                // Check if document has owner field
                let doc_owner = document
                    .get(owner_field)
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok());

                match doc_owner {
                    Some(owner) if owner == user_id => Ok(()),
                    Some(_) => Err(AuthError::Unauthorized),
                    None => Err(AuthError::MissingOwnerField(owner_field.clone())),
                }
            }

            RlsPolicy::Custom {
                write_predicate, ..
            } => {
                if write_predicate.is_some() {
                    Err(AuthError::InvalidPolicy(
                        "Custom predicates not yet supported".to_string(),
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }

    fn prepare_insert(
        &self,
        collection: &str,
        document: &mut serde_json::Value,
        ctx: &RlsContext,
    ) -> AuthResult<()> {
        // Service role doesn't auto-set owner
        if ctx.is_service_role {
            return Ok(());
        }

        let policy = self.get_policy(collection);

        match policy {
            RlsPolicy::None => Ok(()),

            RlsPolicy::Ownership { owner_field } | RlsPolicy::PublicRead { owner_field } => {
                let user_id = ctx.require_user_id()?;

                // Set owner field if document is an object
                if let Some(obj) = document.as_object_mut() {
                    obj.insert(owner_field.clone(), serde_json::json!(user_id.to_string()));
                }

                Ok(())
            }

            RlsPolicy::Custom { .. } => Ok(()),
        }
    }
}

/// RLS decision event for observability
#[derive(Debug, Clone)]
pub enum RlsEvent {
    /// RLS filter was applied
    FilterApplied {
        collection: String,
        user_id: Uuid,
        filter_field: String,
    },

    /// Access was denied by RLS
    AccessDenied {
        collection: String,
        user_id: Option<Uuid>,
        reason: String,
    },

    /// RLS was bypassed (service role)
    Bypassed { collection: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rls_context_authenticated() {
        let user_id = Uuid::new_v4();
        let ctx = RlsContext::authenticated(user_id);

        assert!(ctx.is_authenticated);
        assert!(!ctx.is_service_role);
        assert_eq!(ctx.user_id, Some(user_id));
    }

    #[test]
    fn test_rls_context_service_role_bypass() {
        let ctx = RlsContext::service_role();

        assert!(ctx.can_bypass_rls());
    }

    #[test]
    fn test_ownership_policy_read_filter() {
        let enforcer = DefaultRlsEnforcer::new();
        let user_id = Uuid::new_v4();
        let ctx = RlsContext::authenticated(user_id);

        let filter = enforcer.get_read_filter("posts", &ctx).unwrap();

        assert!(filter.is_some());
        let filter = filter.unwrap();
        assert_eq!(filter.field, "owner_id");
        assert_eq!(filter.value, serde_json::json!(user_id.to_string()));
    }

    #[test]
    fn test_service_role_no_filter() {
        let enforcer = DefaultRlsEnforcer::new();
        let ctx = RlsContext::service_role();

        let filter = enforcer.get_read_filter("posts", &ctx).unwrap();

        assert!(filter.is_none());
    }

    #[test]
    fn test_anonymous_read_denied_with_ownership_policy() {
        let enforcer = DefaultRlsEnforcer::new();
        let ctx = RlsContext::anonymous();

        let result = enforcer.get_read_filter("posts", &ctx);

        assert!(matches!(result, Err(AuthError::AuthenticationRequired)));
    }

    #[test]
    fn test_public_read_policy_allows_anonymous() {
        let enforcer = DefaultRlsEnforcer::new().with_policy(
            "public_posts",
            RlsPolicy::PublicRead {
                owner_field: "author_id".to_string(),
            },
        );

        let ctx = RlsContext::anonymous();
        let filter = enforcer.get_read_filter("public_posts", &ctx).unwrap();

        assert!(filter.is_none());
    }

    #[test]
    fn test_write_validation_owner_match() {
        let enforcer = DefaultRlsEnforcer::new();
        let user_id = Uuid::new_v4();
        let ctx = RlsContext::authenticated(user_id);

        let doc = serde_json::json!({
            "title": "My Post",
            "owner_id": user_id.to_string()
        });

        let result = enforcer.validate_write("posts", &doc, &ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_validation_owner_mismatch() {
        let enforcer = DefaultRlsEnforcer::new();
        let user_id = Uuid::new_v4();
        let other_user_id = Uuid::new_v4();
        let ctx = RlsContext::authenticated(user_id);

        let doc = serde_json::json!({
            "title": "My Post",
            "owner_id": other_user_id.to_string()
        });

        let result = enforcer.validate_write("posts", &doc, &ctx);
        assert!(matches!(result, Err(AuthError::Unauthorized)));
    }

    #[test]
    fn test_prepare_insert_sets_owner() {
        let enforcer = DefaultRlsEnforcer::new();
        let user_id = Uuid::new_v4();
        let ctx = RlsContext::authenticated(user_id);

        let mut doc = serde_json::json!({
            "title": "New Post"
        });

        enforcer.prepare_insert("posts", &mut doc, &ctx).unwrap();

        assert_eq!(
            doc.get("owner_id").unwrap().as_str().unwrap(),
            user_id.to_string()
        );
    }
}
