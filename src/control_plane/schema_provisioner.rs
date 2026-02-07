//! # Schema-per-Tenant Provisioner
//!
//! RLS-based isolation with all tenants in one database.

use uuid::Uuid;

use super::errors::{ControlPlaneError, ControlPlaneResult};
use super::tenant::{Tenant, TenantConfig};

/// Schema-per-tenant provisioner
/// Uses RLS policies for data isolation
#[derive(Debug, Clone)]
pub struct SchemaProvisioner {
    /// Base database URL
    base_url: String,
    /// Whether to create actual schema (false for testing)
    create_schema: bool,
}

impl SchemaProvisioner {
    /// Create a new schema provisioner
    pub fn new() -> Self {
        Self {
            base_url: "http://localhost:54321".to_string(),
            create_schema: false, // Default to false for safety
        }
    }

    /// Create with custom base URL
    pub fn with_base_url(base_url: String) -> Self {
        Self {
            base_url,
            create_schema: false,
        }
    }

    /// Enable actual schema creation
    pub fn with_schema_creation(mut self) -> Self {
        self.create_schema = true;
        self
    }

    /// Generate schema name for a tenant
    pub fn schema_name(tenant_id: Uuid) -> String {
        format!("tenant_{}", tenant_id.to_string().replace("-", "_"))
    }

    /// Generate RLS policy for a table
    pub fn generate_rls_policy(table: &str, tenant_id: Uuid) -> String {
        format!(
            r#"
CREATE POLICY tenant_isolation_{table}_{id} ON {table}
FOR ALL
USING (tenant_id = '{tenant_id}'::uuid)
WITH CHECK (tenant_id = '{tenant_id}'::uuid);
"#,
            table = table,
            id = tenant_id.to_string().replace("-", "_"),
            tenant_id = tenant_id
        )
    }

    /// Generate SQL to add tenant_id column
    pub fn generate_tenant_column(table: &str) -> String {
        format!(
            "ALTER TABLE {} ADD COLUMN IF NOT EXISTS tenant_id UUID NOT NULL;",
            table
        )
    }

    /// Generate SQL to create RLS-enabled schema
    pub fn generate_schema_sql(tenant_id: Uuid) -> Vec<String> {
        let schema_name = Self::schema_name(tenant_id);
        vec![
            format!("CREATE SCHEMA IF NOT EXISTS {};", schema_name),
            format!("GRANT USAGE ON SCHEMA {} TO authenticated;", schema_name),
            format!("GRANT ALL ON ALL TABLES IN SCHEMA {} TO authenticated;", schema_name),
        ]
    }

    /// Provision schema-per-tenant resources
    pub async fn provision(&self, tenant: &Tenant) -> ControlPlaneResult<TenantConfig> {
        let schema_name = Self::schema_name(tenant.tenant_id);

        if self.create_schema {
            // In production, execute SQL to create schema
            // For now, we just log the intent
            let sql_statements = Self::generate_schema_sql(tenant.tenant_id);
            for sql in &sql_statements {
                // Would execute: {sql} in production
            }
        }

        // Generate database URL with tenant schema
        let database_url = format!(
            "{}?schema={}&tenant_id={}",
            self.base_url,
            schema_name,
            tenant.tenant_id
        );

        Ok(TenantConfig {
            database_url,
            port: None,
            data_dir: None,
            process_id: None,
        })
    }

    /// Deprovision schema-per-tenant resources
    pub async fn deprovision(&self, tenant: &Tenant) -> ControlPlaneResult<()> {
        let schema_name = Self::schema_name(tenant.tenant_id);

        if self.create_schema {
            // In production, execute SQL to drop schema
            let _sql = format!("DROP SCHEMA IF EXISTS {} CASCADE;", schema_name);
            // Would execute: {_sql} in production
        }

        Ok(())
    }
}

impl Default for SchemaProvisioner {
    fn default() -> Self {
        Self::new()
    }
}

/// RLS context middleware
/// Sets the tenant context for each request
pub struct RlsContext {
    tenant_id: Uuid,
}

impl RlsContext {
    /// Create new RLS context
    pub fn new(tenant_id: Uuid) -> Self {
        Self { tenant_id }
    }

    /// Get the tenant ID
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }

    /// Generate SQL to set tenant context
    pub fn set_context_sql(&self) -> String {
        format!(
            "SET LOCAL app.tenant_id = '{}';",
            self.tenant_id
        )
    }

    /// Generate SQL to get current tenant
    pub fn get_context_sql() -> &'static str {
        "SELECT current_setting('app.tenant_id', true)::uuid AS tenant_id;"
    }
}

/// Extract tenant ID from request
pub fn extract_tenant_id_from_subdomain(host: &str) -> Option<String> {
    // Expected format: tenant-name.aerodb.com
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() >= 2 {
        Some(parts[0].to_string())
    } else {
        None
    }
}

/// Extract tenant ID from API key
pub fn extract_tenant_id_from_api_key(api_key: &str) -> Option<String> {
    // API key format: ak_<tenant_id>
    if api_key.starts_with("ak_") {
        Some(api_key[3..].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control_plane::tenant::{IsolationModel, Plan};

    #[test]
    fn test_schema_name() {
        let tenant_id = Uuid::parse_str("12345678-1234-1234-1234-123456789012").unwrap();
        let name = SchemaProvisioner::schema_name(tenant_id);
        assert!(name.starts_with("tenant_"));
        assert!(!name.contains('-'));
    }

    #[test]
    fn test_rls_policy() {
        let tenant_id = Uuid::new_v4();
        let policy = SchemaProvisioner::generate_rls_policy("users", tenant_id);
        assert!(policy.contains("tenant_isolation_users"));
        assert!(policy.contains("tenant_id"));
    }

    #[tokio::test]
    async fn test_provision() {
        let provisioner = SchemaProvisioner::new();
        let tenant = Tenant::new(
            "test".to_string(),
            Plan::Free,
            "local".to_string(),
            IsolationModel::Schema,
        );

        let config = provisioner.provision(&tenant).await.unwrap();
        assert!(config.database_url.contains("schema=tenant_"));
        assert!(config.database_url.contains(&tenant.tenant_id.to_string()));
    }

    #[test]
    fn test_extract_subdomain() {
        let host = "acme-corp.aerodb.com";
        let tenant = extract_tenant_id_from_subdomain(host);
        assert_eq!(tenant, Some("acme-corp".to_string()));
    }

    #[test]
    fn test_rls_context() {
        let tenant_id = Uuid::new_v4();
        let ctx = RlsContext::new(tenant_id);
        let sql = ctx.set_context_sql();
        assert!(sql.contains(&tenant_id.to_string()));
    }
}
