//! HTTP request handlers for the Massive Graph API - POC implementation
//!
//! Document creation now integrated with SimpleStorage

use axum::{
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Json},
    Json as JsonExtractor,
};
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use massive_graph_core::types::{ID16, UserId};

// Response types
/// Standard API response wrapper for all endpoints
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    /// Whether the operation was successful
    pub success: bool,
    /// Response data (if successful)
    pub data: Option<T>,
    /// Optional message describing the result
    pub message: Option<String>,
    /// Optional error details
    pub error: Option<String>,
}

/// Error response for bad requests
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Whether the operation was successful (always false)
    pub success: bool,
    /// Error message
    pub error: String,
    /// Optional details about what was invalid
    pub details: Option<Value>,
}

impl<T> ApiResponse<T> {
    /// Create a successful API response with data
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            error: None,
        }
    }

    /// Create a successful API response with data and message
    pub fn success_with_message(data: T, message: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message),
            error: None,
        }
    }
}

impl ErrorResponse {
    /// Create a bad request error response
    pub fn bad_request(error: String) -> Self {
        Self {
            success: false,
            error,
            details: None,
        }
    }

    /// Create a bad request error response with details
    pub fn bad_request_with_details(error: String, details: Value) -> Self {
        Self {
            success: false,
            error,
            details: Some(details),
        }
    }
}

/// Document creation request
#[derive(Debug, Deserialize)]
pub struct CreateDocumentRequest {
    /// Optional document ID (if not provided, server generates one)
    pub id: Option<String>,
    /// Document type
    pub doc_type: String,
    /// Parent document ID (optional for root documents)
    pub parent_id: Option<String>,
    /// Document properties as JSON
    pub properties: Option<Value>,
}



/// Document information response
#[derive(Debug, Serialize)]
pub struct DocumentInfo {
    /// Document ID
    pub id: String,
    /// Document type
    pub doc_type: String,
    /// Parent document ID
    pub parent_id: Option<String>,
    /// Document properties
    pub properties: Value,
    /// Creation timestamp
    pub created_at: String,
    /// Last update timestamp
    pub updated_at: String,
    /// Document version
    pub version: u64,
}

/// System health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// Current system status
    pub status: String,
    /// System uptime duration
    pub uptime: String,
    /// Database version
    pub version: String,
}

/// System information and capabilities
#[derive(Debug, Serialize)]
pub struct InfoResponse {
    /// Database name
    pub name: String,
    /// Database version
    pub version: String,
    /// List of supported capabilities
    pub capabilities: Vec<String>,
    /// List of supported protocols
    pub protocols: Vec<String>,
}

// POC helper to get user ID - in production this would come from auth middleware
fn get_poc_user_id() -> UserId {
    UserId::from_str("tempuser000000000000000000000000").unwrap()
}

/// Custom JSON extractor that returns proper JSON error responses
pub struct JsonRequest<T>(pub T);

#[axum::async_trait]
impl<T, S> axum::extract::FromRequest<S> for JsonRequest<T>
where
    T: serde::de::DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ErrorResponse>);

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        match JsonExtractor::<T>::from_request(req, state).await {
            Ok(JsonExtractor(value)) => Ok(JsonRequest(value)),
            Err(rejection) => {
                let error_message = match rejection {
                    JsonRejection::JsonDataError(err) => {
                        tracing::error!("Invalid JSON data: {}", err);
                        "Invalid JSON data".to_string()
                    }
                    JsonRejection::JsonSyntaxError(_) => {
                        "Malformed JSON".to_string()
                    }
                    JsonRejection::MissingJsonContentType(_) => {
                        "Missing or invalid Content-Type header. Expected 'application/json'".to_string()
                    }
                    JsonRejection::BytesRejection(_) => {
                        "Failed to read request body".to_string()  
                    }
                    _ => "Invalid JSON request".to_string(),
                };
                
                tracing::warn!("JSON parsing error: {}", error_message);
                Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::bad_request(error_message))
                ))
            }
        }
    }
}

/// Generate or validate document ID
fn handle_document_id(provided_id: Option<String>) -> Result<ID16, String> {
    match provided_id {
        Some(id_str) => {
            tracing::debug!("Validating provided document ID: '{}'", id_str);
            // Validate provided ID
            if id_str.len() != 16 {
                tracing::error!("Document ID length mismatch: {} (expected 16)", id_str.len());
                return Err("Document ID must be exactly 16 characters".to_string());
            }
            ID16::from_str(&id_str).map_err(|e| {
                tracing::error!("ID16::from_str failed for '{}': {}", id_str, e);
                format!("Invalid document ID format: {}", e)
            })
        }
        None => {
            // Generate new ID
            tracing::debug!("Generating new random document ID");
            let new_id = ID16::random();
            tracing::debug!("Generated document ID: {}", new_id);
            Ok(new_id)
        }
    }
}

// Real handlers with storage integration

/// Create a new document - now with real storage integration
pub async fn create_document<S: crate::storage::StorageImpl>(
    State(storage): State<Arc<crate::storage::Store<S>>>,
    JsonRequest(request): JsonRequest<CreateDocumentRequest>,
) -> Result<(StatusCode, Json<ApiResponse<DocumentInfo>>), (StatusCode, Json<ErrorResponse>)> {
    tracing::info!("🚀 Starting create_document handler");
    tracing::debug!("Request data: {:?}", request);
    
    // POC: User ID handling is now done internally by Store
    tracing::info!("📋 Step 1: User isolation handled by storage layer");

    // Handle document ID (validate or generate)
    tracing::info!("📋 Step 2: Handling document ID");
    let doc_id = handle_document_id(request.id.clone())
        .map_err(|e| {
            tracing::error!("❌ Failed to handle document ID: {}", e);
            (StatusCode::BAD_REQUEST, Json(ErrorResponse::bad_request(e)))
        })?;
    tracing::info!("✅ Document ID handled: {}", doc_id);

    // Create document data as JSON bytes
    tracing::info!("📋 Step 3: Creating document data JSON");
    let doc_data = serde_json::json!({
        "id": doc_id.to_string(),
        "doc_type": request.doc_type.clone(),
        "parent_id": request.parent_id.clone(),
        "properties": request.properties.clone().unwrap_or(json!({})),
    });
    tracing::debug!("Document data JSON: {}", doc_data);

    tracing::info!("📋 Step 4: Serializing document data to bytes");
    let doc_bytes = serde_json::to_vec(&doc_data)
        .map_err(|e| {
            tracing::error!("❌ Failed to serialize document data: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::bad_request(format!("Failed to serialize document: {}", e))))
        })?;
    tracing::info!("✅ Document serialized, size: {} bytes", doc_bytes.len());

    // Store document using the storage layer
    tracing::info!("📋 Step 5: Calling storage.create_document");
    let user_id = get_poc_user_id();
    let result = storage.create_document(user_id, doc_id, doc_bytes);

    // Handle storage result
    tracing::info!("📋 Step 6: Processing storage result");
    match result {
        Ok(()) => {
            tracing::info!("✅ Document storage successful");
            // Success - create response with actual data
            let doc_info = DocumentInfo {
                id: doc_id.to_string(),
                doc_type: request.doc_type,
                parent_id: request.parent_id,
                properties: request.properties.unwrap_or(json!({})),
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
                version: 1,
            };
            tracing::info!("🎉 Document created successfully: {}", doc_id);

            Ok((
                StatusCode::CREATED,
                Json(ApiResponse::success(doc_info)),
            ))
        }
        Err(error_msg) => {
            tracing::error!("❌ Storage error: {}", error_msg);
            // Storage error
            Err((StatusCode::BAD_REQUEST, Json(ErrorResponse::bad_request(error_msg))))
        }
    }
}

/// Get a document by ID - fetches from storage
pub async fn get_document<S: crate::storage::StorageImpl>(
    State(storage): State<Arc<crate::storage::Store<S>>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    tracing::info!("🔍 Starting get_document handler for ID: {}", id);
    
    // POC: User ID handling is now done internally by Store
    tracing::info!("📋 Step 1: User isolation handled by storage layer");

    // Parse document ID
    tracing::info!("📋 Step 2: Parsing document ID");
    let doc_id = match ID16::from_str(&id) {
        Ok(did) => did,
        Err(e) => {
            tracing::error!("❌ Invalid document ID format '{}': {}", id, e);
            return (StatusCode::BAD_REQUEST, Json(ErrorResponse::bad_request(format!("Invalid document ID format: {}", e)))).into_response();
        }
    };
    tracing::info!("✅ Document ID parsed: {}", doc_id);

    // Get document from storage
    tracing::info!("📋 Step 3: Fetching document from storage");
    let user_id = get_poc_user_id();
    match storage.get_document(user_id, doc_id) {
        Some(doc_data) => {
            tracing::info!("✅ Document found, data size: {} bytes", doc_data.len());
            
            // Parse the document data as JSON
            tracing::debug!("📄 Parsing document data as JSON");
            let doc_json: Value = match serde_json::from_slice(&doc_data) {
                Ok(json) => json,
                Err(e) => {
                    tracing::error!("❌ Failed to parse document JSON: {}", e);
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            };
            tracing::debug!("✅ Document JSON parsed successfully");

            // Extract document information
            let doc_info = DocumentInfo {
                id: doc_json.get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&id)
                    .to_string(),
                doc_type: doc_json.get("doc_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("generic")
                    .to_string(),
                parent_id: doc_json.get("parent_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                properties: doc_json.get("properties")
                    .cloned()
                    .unwrap_or(json!({})),
                created_at: doc_json.get("created_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                updated_at: doc_json.get("updated_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                version: doc_json.get("version")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1),
            };
            
            tracing::info!("🎉 Document retrieved successfully: {}", doc_id);
            Json(ApiResponse::success(doc_info)).into_response()
        }
        None => {
            tracing::warn!("📭 Document not found: doc={}", doc_id);
            StatusCode::NOT_FOUND.into_response()
        }
    }
}


/// Delete a document - removes from storage
pub async fn delete_document<S: crate::storage::StorageImpl>(
    State(storage): State<Arc<crate::storage::Store<S>>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!("🗑️ Starting delete_document handler for ID: {}", id);
    
    // POC: User ID handling is now done internally by Store
    tracing::info!("📋 Step 1: User isolation handled by storage layer");

    // Parse document ID
    tracing::info!("📋 Step 2: Parsing document ID");
    let doc_id = ID16::from_str(&id)
        .map_err(|e| {
            tracing::error!("❌ Invalid document ID format '{}': {}", id, e);
            (StatusCode::BAD_REQUEST, Json(ErrorResponse::bad_request(format!("Invalid document ID format: {}", e))))
        })?;
    tracing::info!("✅ Document ID parsed: {}", doc_id);

    // Check if document exists before attempting deletion
    tracing::info!("📋 Step 3: Checking if document exists");
    let user_id = get_poc_user_id();
    if !storage.document_exists(user_id, doc_id) {
        tracing::warn!("📭 Document not found for deletion: doc={}", doc_id);
        return Err((StatusCode::NOT_FOUND, Json(ErrorResponse::bad_request(format!("Document {} not found", id)))));
    }
    tracing::info!("✅ Document exists, proceeding with deletion");

    // Remove document from storage
    tracing::info!("📋 Step 4: Removing document from storage");
    match storage.remove_document(user_id, doc_id) {
        Ok(()) => {
            tracing::info!("🎉 Document deleted successfully: {}", doc_id);
            Ok(StatusCode::NO_CONTENT)
        }
        Err(error_msg) => {
            tracing::error!("❌ Storage error during deletion: {}", error_msg);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::bad_request(format!("Failed to delete document: {}", error_msg)))))
        }
    }
}

/// Apply delta operations to a document - returns mock response
pub async fn apply_document_deltas<S: crate::storage::StorageImpl>(
    State(_storage): State<Arc<crate::storage::Store<S>>>,
    Path(id): Path<String>,
    JsonExtractor(deltas): JsonExtractor<Vec<Value>>,
) -> Result<(StatusCode, Json<ApiResponse<Vec<Value>>>), StatusCode> {
    // Mock delta application - just echo back the deltas with success status
    let responses: Vec<Value> = deltas
        .into_iter()
        .enumerate()
        .map(|(i, delta)| json!({
            "id": format!("delta_{}", i),
            "target_id": id,
            "operation": delta,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "success": true
        }))
        .collect();

    Ok((
        StatusCode::OK,
        Json(ApiResponse::success(responses)),
    ))
}

/// Get delta history for a document - returns mock response
pub async fn get_document_deltas(
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<Value>>>, StatusCode> {
    // Return empty delta history for now
    let deltas = vec![
        json!({
            "id": "delta_0",
            "target_id": id,
            "operation": "create",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })
    ];
    
    Ok(Json(ApiResponse::success(deltas)))
}

// System handlers

/// Health check endpoint
pub async fn health_check() -> Result<Json<HealthResponse>, StatusCode> {
    let response = HealthResponse {
        status: "healthy".to_string(),
        uptime: "0s".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    Ok(Json(response))
}

/// System information endpoint
pub async fn system_info() -> Result<Json<InfoResponse>, StatusCode> {
    let response = InfoResponse {
        name: "Massive Graph POC".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: vec![
            "documents".to_string(),
            "deltas".to_string(),
        ],
        protocols: vec![
            "http".to_string(),
        ],
    };
    Ok(Json(response))
}

/// Root API endpoint
pub async fn root_handler() -> Json<serde_json::Value> {
    Json(json!({
        "service": "Massive Graph API POC",
        "version": env!("CARGO_PKG_VERSION"),
        "status": "operational",
        "endpoints": {
            "documents": "/api/documents",
            "health": "/health",
            "info": "/info"
        }
    }))
}
