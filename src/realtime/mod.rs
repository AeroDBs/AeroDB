//! # AeroDB Real-Time Module
//!
//! Phase 10: Real-Time Subscriptions
//!
//! This module provides real-time event subscriptions over WebSocket,
//! with deterministic event generation and best-effort delivery.
//!
//! ## Architecture
//!
//! - **Event Log** (deterministic): WAL → Event transformation
//! - **Dispatcher** (non-deterministic): WebSocket event delivery
//! - **Subscriptions**: Client subscription management
//! - **Broadcast**: Pub/sub channels
//! - **Presence**: User presence tracking
//! - **WebSocket**: Network layer for connections

pub mod broadcast;
pub mod dispatcher;
pub mod errors;
pub mod event;
pub mod event_log;
pub mod presence;
pub mod subscription;
pub mod websocket;

pub use broadcast::BroadcastChannel;
pub use dispatcher::Dispatcher;
pub use errors::{RealtimeError, RealtimeResult};
pub use event::{BroadcastEvent, DatabaseEvent, EventType};
pub use event_log::EventLog;
pub use presence::PresenceTracker;
pub use subscription::{Subscription, SubscriptionFilter, SubscriptionRegistry};
pub use websocket::{WebSocketConfig, WebSocketServer};
