//! # Real-Time Errors
//!
//! Error types for the real-time module.

use thiserror::Error;

/// Result type for real-time operations
pub type RealtimeResult<T> = Result<T, RealtimeError>;

/// Real-time errors
#[derive(Debug, Clone, Error)]
pub enum RealtimeError {
    // ==================
    // Connection Errors
    // ==================
    /// Connection closed
    #[error("Connection closed")]
    ConnectionClosed,

    /// Connection timeout
    #[error("Connection timeout")]
    ConnectionTimeout,

    /// Invalid message format
    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    // ==================
    // Subscription Errors
    // ==================
    /// Invalid topic format
    #[error("Invalid topic: {0}")]
    InvalidTopic(String),

    /// Subscription not found
    #[error("Subscription not found: {0}")]
    SubscriptionNotFound(String),

    /// Too many subscriptions
    #[error("Too many subscriptions (max: {0})")]
    TooManySubscriptions(usize),

    // ==================
    // Authorization Errors
    // ==================
    /// Not authorized to subscribe
    #[error("Not authorized to subscribe to this topic")]
    Unauthorized,

    /// Authentication required
    #[error("Authentication required")]
    AuthenticationRequired,

    // ==================
    // Broadcast Errors
    // ==================
    /// Channel not found
    #[error("Channel not found: {0}")]
    ChannelNotFound(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    /// Message too large
    #[error("Message too large (max: {0} bytes)")]
    MessageTooLarge(usize),

    // ==================
    // Presence Errors
    // ==================
    /// Not tracking presence
    #[error("Not tracking presence in this channel")]
    NotTracking,

    // ==================
    // Internal Errors
    // ==================
    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Connection error
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Authentication error
    #[error("Authentication error: {0}")]
    AuthError(String),
}

impl RealtimeError {
    /// Returns the close code for WebSocket
    pub fn close_code(&self) -> u16 {
        match self {
            RealtimeError::ConnectionClosed => 1000,
            RealtimeError::ConnectionTimeout => 1001,
            RealtimeError::InvalidMessage(_) => 1003,
            RealtimeError::InvalidTopic(_) => 4000,
            RealtimeError::SubscriptionNotFound(_) => 4001,
            RealtimeError::TooManySubscriptions(_) => 4002,
            RealtimeError::Unauthorized => 4003,
            RealtimeError::AuthenticationRequired => 4004,
            RealtimeError::ChannelNotFound(_) => 4010,
            RealtimeError::RateLimitExceeded => 4020,
            RealtimeError::MessageTooLarge(_) => 4021,
            RealtimeError::NotTracking => 4030,
            RealtimeError::Internal(_) => 4500,
            RealtimeError::ConfigError(_) => 4501,
            RealtimeError::ConnectionError(_) => 4502,
            RealtimeError::AuthError(_) => 4003,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_close_codes() {
        assert_eq!(RealtimeError::ConnectionClosed.close_code(), 1000);
        assert_eq!(RealtimeError::Unauthorized.close_code(), 4003);
        assert_eq!(RealtimeError::RateLimitExceeded.close_code(), 4020);
    }
}
