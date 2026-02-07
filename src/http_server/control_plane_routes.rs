//! # Control Plane Routes
//!
//! REST API endpoints for multi-tenant management.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::control_plane::{
    billing::BillingCalculator,
    errors::{ControlPlaneError, ErrorResponse},
    metering::UsageTracker,
    provisioning::ProvisioningService,
    quota::{Quotas, QuotaEnforcer},
    registry::TenantRegistry,
    tenant::{
        CreateTenantRequest, CreateTenantResponse, Plan, Tenant, TenantDetails,
        TenantListItem, TenantStatus, UpdateTenantRequest,
    },
};

/// Control plane state
#[derive(Clone)]
pub struct ControlPlaneState {
    /// Provisioning service
    provisioning: Arc<ProvisioningService>,
    /// Usage tracker
    usage_tracker: Arc<UsageTracker>,
    /// Billing calculator
    billing: Arc<BillingCalculator>,
}

impl ControlPlaneState {
    /// Create new control plane state
    pub fn new() -> Self {
        let usage_tracker = Arc::new(UsageTracker::new());
        let registry = Arc::new(TenantRegistry::with_usage_tracker(usage_tracker.clone()));
        let provisioning = Arc::new(ProvisioningService::new(registry));

        Self {
            provisioning,
            usage_tracker,
            billing: Arc::new(BillingCalculator::new()),
        }
    }

    /// Create with custom provisioning service
    pub fn with_provisioning(provisioning: Arc<ProvisioningService>) -> Self {
        Self {
            usage_tracker: provisioning.registry().usage_tracker(),
            provisioning,
            billing: Arc::new(BillingCalculator::new()),
        }
    }
}

impl Default for ControlPlaneState {
    fn default() -> Self {
        Self::new()
    }
}

/// Build control plane routes
pub fn control_plane_routes(state: Arc<ControlPlaneState>) -> Router {
    Router::new()
        // Tenant CRUD
        .route("/v1/tenants", post(create_tenant))
        .route("/v1/tenants", get(list_tenants))
        .route("/v1/tenants/{id}", get(get_tenant))
        .route("/v1/tenants/{id}", patch(update_tenant))
        .route("/v1/tenants/{id}", delete(delete_tenant))
        // Usage & Billing
        .route("/v1/tenants/{id}/usage", get(get_usage))
        .route("/v1/tenants/{id}/invoice", get(get_invoice))
        // Quota check
        .route("/v1/tenants/{id}/quota", get(get_quota))
        .with_state(state)
}

/// Create a new tenant
async fn create_tenant(
    State(state): State<Arc<ControlPlaneState>>,
    Json(request): Json<CreateTenantRequest>,
) -> impl IntoResponse {
    match state.provisioning.create_tenant(request).await {
        Ok(response) => (StatusCode::CREATED, Json(response)).into_response(),
        Err(e) => error_response(e),
    }
}

/// List all tenants
#[derive(Deserialize)]
pub struct ListTenantsQuery {
    /// Filter by plan
    plan: Option<Plan>,
    /// Filter by status
    status: Option<TenantStatus>,
    /// Limit results
    limit: Option<usize>,
    /// Offset for pagination
    offset: Option<usize>,
}

async fn list_tenants(
    State(state): State<Arc<ControlPlaneState>>,
    Query(params): Query<ListTenantsQuery>,
) -> impl IntoResponse {
    let mut tenants = state.provisioning.list_tenants();

    // Apply filters
    if let Some(plan) = params.plan {
        tenants.retain(|t| t.plan == plan);
    }
    if let Some(status) = params.status {
        tenants.retain(|t| t.status == status);
    }

    // Apply pagination
    let offset = params.offset.unwrap_or(0);
    let limit = params.limit.unwrap_or(100).min(1000);

    let paginated: Vec<_> = tenants.into_iter().skip(offset).take(limit).collect();

    (StatusCode::OK, Json(paginated))
}

/// Get tenant details
async fn get_tenant(
    State(state): State<Arc<ControlPlaneState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.provisioning.get_tenant(id) {
        Ok(tenant) => {
            let usage = state.usage_tracker.get_current_usage(id);
            let quotas = Quotas::for_plan(&tenant.plan);

            let details = TenantDetails {
                tenant_id: tenant.tenant_id,
                name: tenant.name,
                plan: tenant.plan,
                region: tenant.region,
                isolation: tenant.isolation,
                status: tenant.status,
                database_url: tenant.database_url,
                created_at: tenant.created_at,
                updated_at: tenant.updated_at,
                quotas,
                usage,
            };

            (StatusCode::OK, Json(details)).into_response()
        }
        Err(e) => error_response(e),
    }
}

/// Update tenant
async fn update_tenant(
    State(state): State<Arc<ControlPlaneState>>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateTenantRequest>,
) -> impl IntoResponse {
    match state
        .provisioning
        .registry()
        .update(id, request)
    {
        Ok(tenant) => (StatusCode::OK, Json(tenant)).into_response(),
        Err(e) => error_response(e),
    }
}

/// Delete tenant
async fn delete_tenant(
    State(state): State<Arc<ControlPlaneState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.provisioning.delete_tenant(id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response(e),
    }
}

/// Get usage query params
#[derive(Deserialize)]
pub struct UsageQuery {
    /// Month (YYYY-MM format)
    month: Option<String>,
}

/// Get tenant usage
async fn get_usage(
    State(state): State<Arc<ControlPlaneState>>,
    Path(id): Path<Uuid>,
    Query(params): Query<UsageQuery>,
) -> impl IntoResponse {
    // Verify tenant exists
    if let Err(e) = state.provisioning.get_tenant(id) {
        return error_response(e);
    }

    let usage = match params.month {
        Some(month) => state
            .usage_tracker
            .get_usage(id, &month)
            .unwrap_or_else(|| state.usage_tracker.get_current_usage(id)),
        None => state.usage_tracker.get_current_usage(id),
    };

    (StatusCode::OK, Json(usage)).into_response()
}

/// Invoice query params
#[derive(Deserialize)]
pub struct InvoiceQuery {
    /// Month (YYYY-MM format)
    month: String,
}

/// Get invoice
async fn get_invoice(
    State(state): State<Arc<ControlPlaneState>>,
    Path(id): Path<Uuid>,
    Query(params): Query<InvoiceQuery>,
) -> impl IntoResponse {
    // Get tenant
    let tenant = match state.provisioning.get_tenant(id) {
        Ok(t) => t,
        Err(e) => return error_response(e),
    };

    // Get usage for the month
    let usage = state
        .usage_tracker
        .get_usage(id, &params.month)
        .unwrap_or_else(|| state.usage_tracker.get_current_usage(id));

    // Generate invoice
    let invoice = state
        .billing
        .generate_invoice(id, &params.month, &tenant.plan, &usage);

    (StatusCode::OK, Json(invoice)).into_response()
}

/// Get quota information
async fn get_quota(
    State(state): State<Arc<ControlPlaneState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    // Get tenant
    let tenant = match state.provisioning.get_tenant(id) {
        Ok(t) => t,
        Err(e) => return error_response(e),
    };

    let quotas = Quotas::for_plan(&tenant.plan);
    let usage = state.usage_tracker.get_current_usage(id);

    #[derive(Serialize)]
    struct QuotaResponse {
        quotas: Quotas,
        usage: crate::control_plane::metering::UsageMetrics,
        #[serde(rename = "storage_remaining")]
        storage_remaining: u64,
        #[serde(rename = "api_requests_remaining")]
        api_requests_remaining: u64,
    }

    let response = QuotaResponse {
        quotas: quotas.clone(),
        storage_remaining: quotas.storage_bytes.saturating_sub(usage.storage_bytes),
        api_requests_remaining: quotas
            .api_requests_month
            .saturating_sub(usage.api_requests),
        usage,
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// Convert error to HTTP response
fn error_response(err: ControlPlaneError) -> axum::response::Response {
    let status = match err.status_code() {
        400 => StatusCode::BAD_REQUEST,
        403 => StatusCode::FORBIDDEN,
        404 => StatusCode::NOT_FOUND,
        409 => StatusCode::CONFLICT,
        410 => StatusCode::GONE,
        429 => StatusCode::TOO_MANY_REQUESTS,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };

    let error_response = ErrorResponse::from(err);
    (status, Json(error_response)).into_response()
}

/// Quota enforcement middleware
pub struct QuotaMiddleware {
    usage_tracker: Arc<UsageTracker>,
}

impl QuotaMiddleware {
    /// Create new quota middleware
    pub fn new(usage_tracker: Arc<UsageTracker>) -> Self {
        Self { usage_tracker }
    }

    /// Check and enforce quota before a request
    pub async fn check_quota(
        &self,
        tenant_id: Uuid,
        plan: &Plan,
    ) -> Result<(), ControlPlaneError> {
        let quotas = Quotas::for_plan(plan);
        let enforcer = QuotaEnforcer::new(tenant_id.to_string(), quotas);

        // Check API request quota
        let api_count = self.usage_tracker.get_api_request_count(tenant_id);
        enforcer.enforce_api_requests(api_count)?;

        // Record this request
        self.usage_tracker.record_api_request(tenant_id);

        Ok(())
    }

    /// Check storage quota before write
    pub async fn check_storage_quota(
        &self,
        tenant_id: Uuid,
        plan: &Plan,
        additional_bytes: u64,
    ) -> Result<(), ControlPlaneError> {
        let quotas = Quotas::for_plan(plan);
        let enforcer = QuotaEnforcer::new(tenant_id.to_string(), quotas);

        let current = self.usage_tracker.get_storage_bytes(tenant_id);
        enforcer.enforce_storage(current, additional_bytes)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_create_tenant_route() {
        let state = Arc::new(ControlPlaneState::new());
        let app = control_plane_routes(state);

        let request = Request::builder()
            .method("POST")
            .uri("/v1/tenants")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"name": "test-tenant", "plan": "free", "isolation": "schema"}"#,
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_list_tenants_route() {
        let state = Arc::new(ControlPlaneState::new());
        let app = control_plane_routes(state);

        let request = Request::builder()
            .method("GET")
            .uri("/v1/tenants")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_tenant_not_found() {
        let state = Arc::new(ControlPlaneState::new());
        let app = control_plane_routes(state);

        let tenant_id = Uuid::new_v4();
        let request = Request::builder()
            .method("GET")
            .uri(&format!("/v1/tenants/{}", tenant_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
