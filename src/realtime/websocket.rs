//! # WebSocket Server for Real-Time Subscriptions
//!
//! Provides WebSocket connectivity for real-time event delivery.
//! This is the network layer on top of the Dispatcher.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Message, Result as WsResult},
};
use uuid::Uuid;

use super::dispatcher::{Dispatcher, EventReceiver};
use super::errors::{RealtimeError, RealtimeResult};
use super::event::DatabaseEvent;
use super::subscription::{Subscription, SubscriptionFilter};
use crate::auth::rls::RlsContext;

// Logging macros (must be defined before use)
macro_rules! log_info {
    ($($arg:tt)*) => {
        eprintln!("[INFO] {}", format!($($arg)*));
    };
}

macro_rules! log_error {
    ($($arg:tt)*) => {
        eprintln!("[ERROR] {}", format!($($arg)*));
    };
}

/// WebSocket server configuration
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    /// Bind address
    pub bind_addr: String,

    /// Maximum connections per IP
    pub max_connections_per_ip: usize,

    /// Heartbeat interval in seconds
    pub heartbeat_interval_secs: u64,

    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:4000".to_string(),
            max_connections_per_ip: 100,
            heartbeat_interval_secs: 30,
            connection_timeout_secs: 60,
        }
    }
}

/// WebSocket message from client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Subscribe to a channel
    Subscribe {
        channel: String,
        #[serde(default)]
        filter: Option<SubscriptionFilter>,
    },

    /// Unsubscribe from a channel
    Unsubscribe { channel: String },

    /// Heartbeat/ping
    Heartbeat {
        #[serde(default)]
        ref_id: Option<String>,
    },

    /// Authentication
    Auth { token: String },
}

/// WebSocket message to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Subscription confirmed
    Subscribed {
        channel: String,
        subscription_id: String,
    },

    /// Unsubscription confirmed
    Unsubscribed { channel: String },

    /// Database event
    Event {
        channel: String,
        event: DatabaseEvent,
    },

    /// Heartbeat response
    Heartbeat {
        ref_id: Option<String>,
        server_time: i64,
    },

    /// Error message
    Error { message: String, code: String },

    /// System message
    System { message: String },
}

/// Connection state
struct ConnectionState {
    id: String,
    rls_context: RlsContext,
    subscriptions: Vec<String>,
    authenticated: bool,
}

/// WebSocket server
pub struct WebSocketServer {
    config: WebSocketConfig,
    dispatcher: Arc<Dispatcher>,
    shutdown_tx: broadcast::Sender<()>,
    connections: Arc<RwLock<HashMap<String, mpsc::Sender<ServerMessage>>>>,
}

impl WebSocketServer {
    /// Create a new WebSocket server
    pub fn new(config: WebSocketConfig, dispatcher: Arc<Dispatcher>) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            config,
            dispatcher,
            shutdown_tx,
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start the WebSocket server
    pub async fn run(&self) -> RealtimeResult<()> {
        let addr: SocketAddr = self
            .config
            .bind_addr
            .parse()
            .map_err(|e| RealtimeError::ConfigError(format!("Invalid bind address: {}", e)))?;

        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| RealtimeError::ConfigError(format!("Failed to bind: {}", e)))?;

        log_info!("WebSocket server listening on {}", addr);

        let mut shutdown_rx = self.shutdown_tx.subscribe();

        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, peer_addr)) => {
                            let dispatcher = Arc::clone(&self.dispatcher);
                            let connections = Arc::clone(&self.connections);
                            let config = self.config.clone();

                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_connection(
                                    stream,
                                    peer_addr,
                                    dispatcher,
                                    connections,
                                    config,
                                ).await {
                                    log_error!("WebSocket error for {}: {}", peer_addr, e);
                                }
                            });
                        }
                        Err(e) => {
                            log_error!("Accept failed: {}", e);
                        }
                    }
                }

                _ = shutdown_rx.recv() => {
                    log_info!("WebSocket server shutting down");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle a single WebSocket connection
    async fn handle_connection(
        stream: TcpStream,
        peer_addr: SocketAddr,
        dispatcher: Arc<Dispatcher>,
        connections: Arc<RwLock<HashMap<String, mpsc::Sender<ServerMessage>>>>,
        config: WebSocketConfig,
    ) -> RealtimeResult<()> {
        let ws_stream = accept_async(stream).await.map_err(|e| {
            RealtimeError::ConnectionError(format!("WebSocket handshake failed: {}", e))
        })?;

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Generate connection ID
        let connection_id = Uuid::new_v4().to_string();
        log_info!(
            "New WebSocket connection: {} from {}",
            connection_id,
            peer_addr
        );

        // Create message channel
        let (msg_tx, mut msg_rx) = mpsc::channel::<ServerMessage>(256);

        // Register connection
        {
            let mut conns = connections.write().await;
            conns.insert(connection_id.clone(), msg_tx.clone());
        }

        // Default RLS context (unauthenticated)
        let mut rls_context = RlsContext::anonymous();
        let mut event_receiver: Option<EventReceiver> = None;
        let mut subscribed_channels: Vec<String> = Vec::new();

        // Send welcome message
        let welcome = ServerMessage::System {
            message: format!("Connected. Connection ID: {}", connection_id),
        };
        let _ = msg_tx.send(welcome).await;

        // Heartbeat interval
        let heartbeat_interval = tokio::time::Duration::from_secs(config.heartbeat_interval_secs);
        let mut heartbeat_timer = tokio::time::interval(heartbeat_interval);

        loop {
            tokio::select! {
                // Handle incoming WebSocket messages
                msg = ws_receiver.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            match serde_json::from_str::<ClientMessage>(&text) {
                                Ok(client_msg) => {
                                    match Self::process_client_message(
                                        &connection_id,
                                        client_msg,
                                        &mut rls_context,
                                        &mut event_receiver,
                                        &mut subscribed_channels,
                                        &dispatcher,
                                        &msg_tx,
                                    ).await {
                                        Ok(()) => {}
                                        Err(e) => {
                                            let err_msg = ServerMessage::Error {
                                                message: e.to_string(),
                                                code: "PROCESSING_ERROR".to_string(),
                                            };
                                            let _ = msg_tx.send(err_msg).await;
                                        }
                                    }
                                }
                                Err(e) => {
                                    let err_msg = ServerMessage::Error {
                                        message: format!("Invalid message format: {}", e),
                                        code: "INVALID_MESSAGE".to_string(),
                                    };
                                    let _ = msg_tx.send(err_msg).await;
                                }
                            }
                        }
                        Some(Ok(Message::Binary(_))) => {
                            // Binary messages not supported
                            let err_msg = ServerMessage::Error {
                                message: "Binary messages not supported".to_string(),
                                code: "UNSUPPORTED".to_string(),
                            };
                            let _ = msg_tx.send(err_msg).await;
                        }
                        Some(Ok(Message::Ping(data))) => {
                            if let Err(e) = ws_sender.send(Message::Pong(data)).await {
                                log_error!("Failed to send pong: {}", e);
                                break;
                            }
                        }
                        Some(Ok(Message::Close(_))) | None => {
                            log_info!("Connection closed: {}", connection_id);
                            break;
                        }
                        Some(Err(e)) => {
                            log_error!("WebSocket receive error: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }

                // Handle outgoing messages
                Some(server_msg) = msg_rx.recv() => {
                    match serde_json::to_string(&server_msg) {
                        Ok(json) => {
                            if let Err(e) = ws_sender.send(Message::Text(json)).await {
                                log_error!("Failed to send message: {}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            log_error!("Failed to serialize message: {}", e);
                        }
                    }
                }

                // Handle events from dispatcher
                event = async {
                    if let Some(ref mut rx) = event_receiver {
                        rx.recv().await
                    } else {
                        std::future::pending().await
                    }
                } => {
                    if let Some(event) = event {
                        let event_msg = ServerMessage::Event {
                            channel: event.collection.clone(),
                            event,
                        };
                        let _ = msg_tx.send(event_msg).await;
                    }
                }

                // Periodic heartbeat
                _ = heartbeat_timer.tick() => {
                    let heartbeat = ServerMessage::Heartbeat {
                        ref_id: None,
                        server_time: chrono::Utc::now().timestamp(),
                    };
                    let _ = msg_tx.send(heartbeat).await;
                }
            }
        }

        // Cleanup
        dispatcher.disconnect(&connection_id);
        {
            let mut conns = connections.write().await;
            conns.remove(&connection_id);
        }

        log_info!("Connection {} cleaned up", connection_id);
        Ok(())
    }

    /// Process a client message
    async fn process_client_message(
        connection_id: &str,
        message: ClientMessage,
        rls_context: &mut RlsContext,
        event_receiver: &mut Option<EventReceiver>,
        subscribed_channels: &mut Vec<String>,
        dispatcher: &Arc<Dispatcher>,
        msg_tx: &mpsc::Sender<ServerMessage>,
    ) -> RealtimeResult<()> {
        match message {
            ClientMessage::Subscribe { channel, filter: _ } => {
                // Connect to dispatcher if not already
                if event_receiver.is_none() {
                    let rx = dispatcher.connect(connection_id.to_string(), rls_context.clone());
                    *event_receiver = Some(rx);
                }

                // Subscribe to channel
                let subscription = Subscription::new(
                    connection_id.to_string(),
                    channel.clone(),
                    rls_context.clone(),
                );

                dispatcher.subscriptions.register(subscription.clone())?;
                subscribed_channels.push(channel.clone());

                let response = ServerMessage::Subscribed {
                    channel: channel.clone(),
                    subscription_id: subscription.id.to_string(),
                };
                let _ = msg_tx.send(response).await;
            }

            ClientMessage::Unsubscribe { channel } => {
                dispatcher
                    .subscriptions
                    .unsubscribe_by_channel(connection_id, &channel)?;
                subscribed_channels.retain(|c| c != &channel);

                let response = ServerMessage::Unsubscribed { channel };
                let _ = msg_tx.send(response).await;
            }

            ClientMessage::Heartbeat { ref_id } => {
                let response = ServerMessage::Heartbeat {
                    ref_id,
                    server_time: chrono::Utc::now().timestamp(),
                };
                let _ = msg_tx.send(response).await;
            }

            ClientMessage::Auth { token } => {
                // In production, validate JWT and extract RLS context
                // For now, mark as authenticated with the token as user_id
                match validate_token(&token) {
                    Ok(context) => {
                        *rls_context = context;
                        let response = ServerMessage::System {
                            message: "Authenticated successfully".to_string(),
                        };
                        let _ = msg_tx.send(response).await;
                    }
                    Err(e) => {
                        let response = ServerMessage::Error {
                            message: e.to_string(),
                            code: "AUTH_FAILED".to_string(),
                        };
                        let _ = msg_tx.send(response).await;
                    }
                }
            }
        }

        Ok(())
    }

    /// Shutdown the server
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }

    /// Get connection count
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }
}

/// Validate JWT token and return RLS context
fn validate_token(token: &str) -> RealtimeResult<RlsContext> {
    use crate::auth::jwt::{JwtConfig, JwtManager};

    if token.is_empty() {
        return Err(RealtimeError::AuthError("Empty token".to_string()));
    }

    // Create JWT manager with default config for validation
    // In production, this would be injected as a dependency
    let jwt_manager = JwtManager::new(JwtConfig::default());

    // Validate the JWT token
    match jwt_manager.validate_token(token) {
        Ok(claims) => {
            // Extract user_id from claims
            match JwtManager::get_user_id(&claims) {
                Ok(user_id) => Ok(RlsContext::authenticated(user_id)),
                Err(_) => {
                    // Token is valid but has no user_id claim - allow anonymous authenticated
                    Ok(RlsContext::anonymous())
                }
            }
        }
        Err(e) => Err(RealtimeError::AuthError(format!(
            "Token validation failed: {}",
            e
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = WebSocketConfig::default();
        assert_eq!(config.bind_addr, "0.0.0.0:4000");
        assert_eq!(config.heartbeat_interval_secs, 30);
    }

    #[test]
    fn test_client_message_parse() {
        let json = r#"{"type": "subscribe", "channel": "users"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();

        match msg {
            ClientMessage::Subscribe { channel, .. } => {
                assert_eq!(channel, "users");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_server_message_serialize() {
        let msg = ServerMessage::Heartbeat {
            ref_id: Some("123".to_string()),
            server_time: 1234567890,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("heartbeat"));
        assert!(json.contains("123"));
    }

    #[test]
    fn test_validate_token() {
        let user_id = Uuid::new_v4();
        let result = validate_token(&user_id.to_string()).unwrap();
        assert_eq!(result.user_id, Some(user_id));
        assert!(result.is_authenticated);
    }

    #[test]
    fn test_validate_empty_token() {
        assert!(validate_token("").is_err());
    }
}
