//! Storage HTTP Routes
//!
//! Endpoints for bucket and file management.

use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::rls::RlsContext;
use crate::file_storage::bucket::{Bucket, BucketConfig, BucketPolicy, BucketRegistry};
use crate::file_storage::file::{FileService, StorageObject};
use crate::file_storage::local::LocalBackend;

// ==================
// Shared State
// ==================

/// Storage state shared across handlers
pub struct StorageState {
    pub file_service: FileService<LocalBackend>,
}

impl StorageState {
    pub fn new(storage_path: &std::path::Path) -> Self {
        let backend = LocalBackend::new(storage_path.to_path_buf());
        Self {
            file_service: FileService::new(backend),
        }
    }

    pub fn with_default_path() -> Self {
        let storage_path = std::env::temp_dir().join("aerodb_storage");
        Self::new(&storage_path)
    }
}

// ==================
// Request/Response Types
// ==================

#[derive(Debug, Serialize)]
pub struct BucketResponse {
    pub id: String,
    pub name: String,
    pub policy: String,
    pub created_at: String,
    pub file_count: usize,
    pub total_size: u64,
}

impl From<&Bucket> for BucketResponse {
    fn from(bucket: &Bucket) -> Self {
        Self {
            id: bucket.id.to_string(),
            name: bucket.name.clone(),
            policy: format!("{:?}", bucket.config.policy).to_lowercase(),
            created_at: bucket.created_at.to_rfc3339(),
            file_count: 0, // Would need to query file service
            total_size: 0, // Would need to query file service
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BucketsListResponse {
    pub buckets: Vec<BucketResponse>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct CreateBucketRequest {
    pub name: String,
    #[serde(default)]
    pub policy: Option<String>,
    #[serde(default)]
    pub allowed_mime_types: Vec<String>,
    #[serde(default)]
    pub max_file_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBucketRequest {
    #[serde(default)]
    pub policy: Option<String>,
    #[serde(default)]
    pub allowed_mime_types: Option<Vec<String>>,
    #[serde(default)]
    pub max_file_size: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct FileResponse {
    pub id: String,
    pub name: String,
    pub bucket: String,
    pub path: String,
    pub size: u64,
    pub content_type: String,
    pub created_at: String,
}

impl FileResponse {
    fn from_object(obj: &StorageObject, bucket_name: &str) -> Self {
        Self {
            id: obj.id.to_string(),
            name: obj.path.split('/').last().unwrap_or(&obj.path).to_string(),
            bucket: bucket_name.to_string(),
            path: obj.path.clone(),
            size: obj.size,
            content_type: obj.content_type.clone(),
            created_at: obj.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FilesListResponse {
    pub files: Vec<FileResponse>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct ListFilesQuery {
    #[serde(default)]
    pub prefix: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MoveFileRequest {
    pub from_path: String,
    pub to_path: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateSignedUrlRequest {
    pub expires_in: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct SignedUrlResponse {
    pub url: String,
    pub expires_at: String,
}

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub id: String,
    pub path: String,
    pub size: u64,
    pub content_type: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
}

#[derive(Debug, Serialize)]
pub struct BucketStatsResponse {
    pub file_count: usize,
    pub total_size: u64,
    pub largest_file: u64,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

// ==================
// Storage Routes
// ==================

/// Create storage routes
pub fn storage_routes(state: Arc<StorageState>) -> Router {
    Router::new()
        // Bucket management
        .route("/buckets", get(list_buckets_handler))
        .route("/buckets", post(create_bucket_handler))
        .route("/buckets/{name}", get(get_bucket_handler))
        .route("/buckets/{name}", patch(update_bucket_handler))
        .route("/buckets/{name}", delete(delete_bucket_handler))
        .route("/buckets/{name}/stats", get(get_bucket_stats_handler))
        // File operations (non-wildcard routes first)
        .route("/buckets/{name}/files", get(list_files_handler))
        .route("/buckets/{name}/files", post(upload_file_handler))
        .route("/buckets/{name}/files/move", post(move_file_handler))
        // Signed URLs - use separate path prefix to avoid wildcard conflict
        .route(
            "/buckets/{name}/sign/*path",
            post(create_signed_url_handler),
        )
        // Folders
        .route("/buckets/{name}/folders", post(create_folder_handler))
        // Wildcard file routes (must come last)
        .route("/buckets/{name}/files/*path", get(download_file_handler))
        .route("/buckets/{name}/files/*path", delete(delete_file_handler))
        .with_state(state)
}

// ==================
// Helper Functions
// ==================

fn get_rls_context_from_headers(headers: &HeaderMap) -> RlsContext {
    // In a real implementation, extract user ID from JWT token
    if let Some(auth) = headers.get("authorization") {
        if auth
            .to_str()
            .ok()
            .map(|s| s.starts_with("Bearer "))
            .unwrap_or(false)
        {
            // Would validate token and extract user_id
            return RlsContext::service_role(); // Simplified for now
        }
    }
    RlsContext::anonymous()
}

fn parse_bucket_policy(policy_str: &str) -> BucketPolicy {
    match policy_str.to_lowercase().as_str() {
        "public" => BucketPolicy::Public,
        "authenticated" => BucketPolicy::Authenticated,
        "private" => BucketPolicy::Private,
        _ => BucketPolicy::Private,
    }
}

// ==================
// Bucket Handlers
// ==================

async fn list_buckets_handler(
    State(state): State<Arc<StorageState>>,
    headers: HeaderMap,
) -> Result<Json<BucketsListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let buckets = state.file_service.buckets().list();
    let response: Vec<BucketResponse> = buckets.iter().map(BucketResponse::from).collect();

    Ok(Json(BucketsListResponse {
        total: response.len(),
        buckets: response,
    }))
}

async fn get_bucket_handler(
    State(state): State<Arc<StorageState>>,
    Path(name): Path<String>,
) -> Result<Json<BucketResponse>, (StatusCode, Json<ErrorResponse>)> {
    let bucket = state.file_service.buckets().get(&name).map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
                code: 404,
            }),
        )
    })?;

    Ok(Json(BucketResponse::from(&bucket)))
}

async fn create_bucket_handler(
    State(state): State<Arc<StorageState>>,
    headers: HeaderMap,
    Json(request): Json<CreateBucketRequest>,
) -> Result<(StatusCode, Json<BucketResponse>), (StatusCode, Json<ErrorResponse>)> {
    let ctx = get_rls_context_from_headers(&headers);

    let config = BucketConfig {
        policy: request
            .policy
            .as_deref()
            .map(parse_bucket_policy)
            .unwrap_or_default(),
        allowed_mime_types: request.allowed_mime_types,
        max_file_size: request.max_file_size.unwrap_or(100 * 1024 * 1024),
    };

    let bucket = state
        .file_service
        .buckets()
        .create(request.name, ctx.user_id, config)
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: e.to_string(),
                    code: 400,
                }),
            )
        })?;

    Ok((StatusCode::CREATED, Json(BucketResponse::from(&bucket))))
}

async fn update_bucket_handler(
    State(_state): State<Arc<StorageState>>,
    Path(name): Path<String>,
    Json(_request): Json<UpdateBucketRequest>,
) -> Result<Json<BucketResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Bucket update would require extending BucketRegistry
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "Bucket update not yet implemented".to_string(),
            code: 501,
        }),
    ))
}

async fn delete_bucket_handler(
    State(state): State<Arc<StorageState>>,
    Path(name): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    state.file_service.buckets().delete(&name).map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
                code: 404,
            }),
        )
    })?;

    Ok(StatusCode::NO_CONTENT)
}

async fn get_bucket_stats_handler(
    State(_state): State<Arc<StorageState>>,
    Path(name): Path<String>,
) -> Result<Json<BucketStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Would need to implement stats collection
    Ok(Json(BucketStatsResponse {
        file_count: 0,
        total_size: 0,
        largest_file: 0,
    }))
}

// ==================
// File Handlers
// ==================

async fn list_files_handler(
    State(state): State<Arc<StorageState>>,
    headers: HeaderMap,
    Path(bucket_name): Path<String>,
    Query(query): Query<ListFilesQuery>,
) -> Result<Json<FilesListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = get_rls_context_from_headers(&headers);
    let prefix = query.prefix.as_deref().unwrap_or("");

    let files = state
        .file_service
        .list(&bucket_name, prefix, &ctx)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    code: 500,
                }),
            )
        })?;

    let response: Vec<FileResponse> = files
        .iter()
        .map(|f| FileResponse::from_object(f, &bucket_name))
        .collect();

    Ok(Json(FilesListResponse {
        total: response.len(),
        files: response,
    }))
}

async fn upload_file_handler(
    State(state): State<Arc<StorageState>>,
    headers: HeaderMap,
    Path(bucket_name): Path<String>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<UploadResponse>), (StatusCode, Json<ErrorResponse>)> {
    let ctx = get_rls_context_from_headers(&headers);

    // Extract file from multipart
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: e.to_string(),
                code: 400,
            }),
        )
    })? {
        let file_name = field.file_name().unwrap_or("unnamed").to_string();
        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();
        let data = field.bytes().await.map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: e.to_string(),
                    code: 400,
                }),
            )
        })?;

        let obj = state
            .file_service
            .upload(&bucket_name, &file_name, &data, &content_type, &ctx)
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: e.to_string(),
                        code: 500,
                    }),
                )
            })?;

        return Ok((
            StatusCode::CREATED,
            Json(UploadResponse {
                id: obj.id.to_string(),
                path: obj.path,
                size: obj.size,
                content_type: obj.content_type,
            }),
        ));
    }

    Err((
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "No file provided".to_string(),
            code: 400,
        }),
    ))
}

async fn download_file_handler(
    State(state): State<Arc<StorageState>>,
    headers: HeaderMap,
    Path((bucket_name, path)): Path<(String, String)>,
) -> Result<(StatusCode, HeaderMap, Bytes), (StatusCode, Json<ErrorResponse>)> {
    let ctx = get_rls_context_from_headers(&headers);

    let (obj, data) = state
        .file_service
        .download(&bucket_name, &path, &ctx)
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: e.to_string(),
                    code: 404,
                }),
            )
        })?;

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        "content-type",
        obj.content_type
            .parse()
            .unwrap_or_else(|_| "application/octet-stream".parse().unwrap()),
    );
    response_headers.insert("content-length", obj.size.to_string().parse().unwrap());

    Ok((StatusCode::OK, response_headers, Bytes::from(data)))
}

async fn delete_file_handler(
    State(state): State<Arc<StorageState>>,
    headers: HeaderMap,
    Path((bucket_name, path)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let ctx = get_rls_context_from_headers(&headers);

    state
        .file_service
        .delete(&bucket_name, &path, &ctx)
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: e.to_string(),
                    code: 404,
                }),
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

async fn move_file_handler(
    State(_state): State<Arc<StorageState>>,
    Path(bucket_name): Path<String>,
    Json(_request): Json<MoveFileRequest>,
) -> Result<Json<MessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    // File move would need to be implemented in FileService
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "File move not yet implemented".to_string(),
            code: 501,
        }),
    ))
}

async fn create_signed_url_handler(
    State(_state): State<Arc<StorageState>>,
    Path((bucket_name, path)): Path<(String, String)>,
    Json(request): Json<CreateSignedUrlRequest>,
) -> Result<Json<SignedUrlResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Would use SignedUrlGenerator from file_storage module
    let expires_in = request.expires_in.unwrap_or(3600);
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64);

    Ok(Json(SignedUrlResponse {
        url: format!(
            "/storage/v1/{}/{}?token=signed_placeholder",
            bucket_name, path
        ),
        expires_at: expires_at.to_rfc3339(),
    }))
}

async fn create_folder_handler(
    State(_state): State<Arc<StorageState>>,
    Path(bucket_name): Path<String>,
    Json(_request): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<MessageResponse>), (StatusCode, Json<ErrorResponse>)> {
    Ok((
        StatusCode::CREATED,
        Json(MessageResponse {
            message: "Folder created".to_string(),
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bucket_policy() {
        assert!(matches!(
            parse_bucket_policy("public"),
            BucketPolicy::Public
        ));
        assert!(matches!(
            parse_bucket_policy("private"),
            BucketPolicy::Private
        ));
        assert!(matches!(
            parse_bucket_policy("authenticated"),
            BucketPolicy::Authenticated
        ));
        assert!(matches!(
            parse_bucket_policy("unknown"),
            BucketPolicy::Private
        ));
    }
}
