//! Realtime HTTP Routes and WebSocket Handler
//!
//! Endpoints for subscriptions, broadcasting, and WebSocket connections.

use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;
use uuid::Uuid;

// ==================
// Shared State
// ==================

/// Realtime state shared across handlers
pub struct RealtimeState {
    pub active_connections: Arc<RwLock<usize>>,
    pub subscriptions: Arc<RwLock<Vec<SubscriptionInfo>>>,
}

#[derive(Debug, Clone)]
pub struct SubscriptionInfo {
    pub id: String,
    pub connection_id: String,
    pub channel: String,
    pub created_at: String,
}

impl RealtimeState {
    pub fn new() -> Self {
        Self {
            active_connections: Arc::new(RwLock::new(0)),
            subscriptions: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl Default for RealtimeState {
    fn default() -> Self {
        Self::new()
    }
}

// ==================
// Request/Response Types
// ==================

#[derive(Debug, Serialize)]
pub struct SubscriptionResponse {
    pub id: String,
    pub channel: String,
    pub filter: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionsListResponse {
    pub subscriptions: Vec<SubscriptionResponse>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct CreateSubscriptionRequest {
    pub channel: String,
    #[serde(default)]
    pub filter: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct BroadcastRequest {
    pub channel: String,
    pub event: String,
    pub payload: Value,
}

#[derive(Debug, Serialize)]
pub struct BroadcastResponse {
    pub subscribers_notified: usize,
}

#[derive(Debug, Serialize)]
pub struct RealtimeStatsResponse {
    pub active_connections: usize,
    pub total_subscriptions: usize,
    pub channels: Vec<String>,
    pub messages_per_minute: u64,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSocketMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl WebSocketMessage {
    pub fn subscribed(channel: String) -> Self {
        Self {
            msg_type: "subscribed".to_string(),
            channel: Some(channel),
            event: None,
            payload: None,
            error: None,
        }
    }

    pub fn error(msg: String) -> Self {
        Self {
            msg_type: "error".to_string(),
            channel: None,
            event: None,
            payload: None,
            error: Some(msg),
        }
    }

    pub fn pong() -> Self {
        Self {
            msg_type: "pong".to_string(),
            channel: None,
            event: None,
            payload: None,
            error: None,
        }
    }
}

// ==================
// Realtime Routes
// ==================

/// Create realtime routes with WebSocket support
pub fn realtime_routes(state: Arc<RealtimeState>) -> Router {
    Router::new()
        // HTTP endpoints for managing subscriptions
        .route("/subscriptions", get(list_subscriptions_handler))
        .route(
            "/subscriptions/{id}",
            delete(disconnect_subscription_handler),
        )
        // Broadcast endpoint
        .route("/broadcast", post(broadcast_handler))
        // Stats
        .route("/stats", get(get_stats_handler))
        // WebSocket endpoint
        .route("/ws", get(websocket_handler))
        .with_state(state)
}

// ==================
// WebSocket Handler
// ==================

/// Handle WebSocket upgrade request
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<RealtimeState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

/// Handle individual WebSocket connection
async fn handle_websocket(socket: WebSocket, state: Arc<RealtimeState>) {
    // Track connection
    {
        let mut count = state.active_connections.write().await;
        *count += 1;
    }

    let (mut sender, mut receiver) = socket.split();
    let connection_id = Uuid::new_v4().to_string();

    // Send welcome message
    let welcome = WebSocketMessage {
        msg_type: "connected".to_string(),
        channel: None,
        event: None,
        payload: Some(serde_json::json!({ "connection_id": connection_id })),
        error: None,
    };
    if let Ok(json) = serde_json::to_string(&welcome) {
        let _ = sender.send(Message::Text(json.into())).await;
    }

    // Handle incoming messages
    while let Some(result) = receiver.next().await {
        match result {
            Ok(Message::Text(text)) => {
                if let Ok(msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                    let response = handle_ws_message(msg, &state, &connection_id).await;
                    if let Ok(json) = serde_json::to_string(&response) {
                        if sender.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                } else {
                    let error = WebSocketMessage::error("Invalid message format".to_string());
                    if let Ok(json) = serde_json::to_string(&error) {
                        let _ = sender.send(Message::Text(json.into())).await;
                    }
                }
            }
            Ok(Message::Ping(data)) => {
                let _ = sender.send(Message::Pong(data)).await;
            }
            Ok(Message::Close(_)) => break,
            Err(_) => break,
            _ => {}
        }
    }

    // Cleanup subscriptions for this connection
    {
        let mut subs = state.subscriptions.write().await;
        subs.retain(|s| s.connection_id != connection_id);
    }

    // Decrement connection count
    {
        let mut count = state.active_connections.write().await;
        *count = count.saturating_sub(1);
    }
}

/// Handle a parsed WebSocket message
async fn handle_ws_message(
    msg: WebSocketMessage,
    state: &RealtimeState,
    connection_id: &str,
) -> WebSocketMessage {
    match msg.msg_type.as_str() {
        "subscribe" => {
            if let Some(channel) = msg.channel {
                let sub = SubscriptionInfo {
                    id: Uuid::new_v4().to_string(),
                    connection_id: connection_id.to_string(),
                    channel: channel.clone(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                };
                state.subscriptions.write().await.push(sub);
                WebSocketMessage::subscribed(channel)
            } else {
                WebSocketMessage::error("Channel required for subscribe".to_string())
            }
        }
        "unsubscribe" => {
            if let Some(channel) = msg.channel.clone() {
                let mut subs = state.subscriptions.write().await;
                subs.retain(|s| !(s.connection_id == connection_id && s.channel == channel));
                WebSocketMessage {
                    msg_type: "unsubscribed".to_string(),
                    channel: msg.channel,
                    event: None,
                    payload: None,
                    error: None,
                }
            } else {
                WebSocketMessage::error("Channel required for unsubscribe".to_string())
            }
        }
        "broadcast" => {
            if let (Some(channel), Some(event), Some(payload)) =
                (msg.channel, msg.event, msg.payload)
            {
                // Count subscribers for this channel
                let subs = state.subscriptions.read().await;
                let count = subs.iter().filter(|s| s.channel == channel).count();
                WebSocketMessage {
                    msg_type: "broadcast_ack".to_string(),
                    channel: Some(channel),
                    event: Some(event),
                    payload: Some(serde_json::json!({ "subscribers_notified": count })),
                    error: None,
                }
            } else {
                WebSocketMessage::error(
                    "Channel, event, and payload required for broadcast".to_string(),
                )
            }
        }
        "ping" => WebSocketMessage::pong(),
        _ => WebSocketMessage::error(format!("Unknown message type: {}", msg.msg_type)),
    }
}

// ==================
// HTTP Handlers
// ==================

/// List all active subscriptions
async fn list_subscriptions_handler(
    State(state): State<Arc<RealtimeState>>,
) -> Result<Json<SubscriptionsListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let subs = state.subscriptions.read().await;
    let responses: Vec<SubscriptionResponse> = subs
        .iter()
        .map(|s| SubscriptionResponse {
            id: s.id.clone(),
            channel: s.channel.clone(),
            filter: None,
            created_at: s.created_at.clone(),
        })
        .collect();
    let total = responses.len();

    Ok(Json(SubscriptionsListResponse {
        subscriptions: responses,
        total,
    }))
}

/// Disconnect a specific subscription
async fn disconnect_subscription_handler(
    State(state): State<Arc<RealtimeState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let mut subs = state.subscriptions.write().await;
    let initial_len = subs.len();
    subs.retain(|s| s.id != id);

    if subs.len() < initial_len {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Subscription not found".to_string(),
                code: 404,
            }),
        ))
    }
}

/// Broadcast a message to a channel
async fn broadcast_handler(
    State(state): State<Arc<RealtimeState>>,
    Json(request): Json<BroadcastRequest>,
) -> Result<Json<BroadcastResponse>, (StatusCode, Json<ErrorResponse>)> {
    let subs = state.subscriptions.read().await;
    let count = subs.iter().filter(|s| s.channel == request.channel).count();

    Ok(Json(BroadcastResponse {
        subscribers_notified: count,
    }))
}

/// Get realtime statistics
async fn get_stats_handler(
    State(state): State<Arc<RealtimeState>>,
) -> Result<Json<RealtimeStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let connections = *state.active_connections.read().await;
    let subs = state.subscriptions.read().await;
    let total_subscriptions = subs.len();

    // Get unique channels
    let channels: Vec<String> = subs
        .iter()
        .map(|s| s.channel.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    Ok(Json(RealtimeStatsResponse {
        active_connections: connections,
        total_subscriptions,
        channels,
        messages_per_minute: 0,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_message_creation() {
        let msg = WebSocketMessage::subscribed("test-channel".to_string());
        assert_eq!(msg.msg_type, "subscribed");
        assert_eq!(msg.channel, Some("test-channel".to_string()));
    }

    #[test]
    fn test_realtime_state_creation() {
        let state = RealtimeState::new();
        // State should be created successfully
    }
}
