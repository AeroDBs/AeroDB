//! # Event Dispatcher
//!
//! Non-deterministic event fan-out to subscribers.
//!
//! ## Invariant: RT-D1
//! Best-effort delivery. No guarantee of delivery or ordering.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use tokio::sync::mpsc;

use super::errors::{RealtimeError, RealtimeResult};
use super::event::DatabaseEvent;
use super::subscription::{Subscription, SubscriptionRegistry};
use crate::auth::rls::{RlsContext, RlsPolicy};

/// Event sender for a connection
pub type EventSender = mpsc::UnboundedSender<DatabaseEvent>;

/// Event receiver for a connection
pub type EventReceiver = mpsc::UnboundedReceiver<DatabaseEvent>;

/// Connection info
#[derive(Debug)]
struct Connection {
    /// Connection ID
    id: String,

    /// Event sender channel
    sender: EventSender,

    /// RLS context for this connection
    rls_context: RlsContext,
}

/// Event dispatcher that fans out events to subscribed connections
#[derive(Debug)]
pub struct Dispatcher {
    /// Active connections
    connections: RwLock<HashMap<String, Connection>>,

    /// Subscription registry
    pub subscriptions: Arc<SubscriptionRegistry>,

    /// RLS policies by collection
    rls_policies: RwLock<HashMap<String, RlsPolicy>>,
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new(Arc::new(SubscriptionRegistry::new()))
    }
}

impl Dispatcher {
    /// Create a new dispatcher
    pub fn new(subscriptions: Arc<SubscriptionRegistry>) -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            subscriptions,
            rls_policies: RwLock::new(HashMap::new()),
        }
    }

    /// Register an RLS policy for a collection
    pub fn register_rls_policy(&self, collection: &str, policy: RlsPolicy) {
        if let Ok(mut policies) = self.rls_policies.write() {
            policies.insert(collection.to_string(), policy);
        }
    }

    /// Add a connection
    pub fn connect(&self, connection_id: String, rls_context: RlsContext) -> EventReceiver {
        let (tx, rx) = mpsc::unbounded_channel();

        let connection = Connection {
            id: connection_id.clone(),
            sender: tx,
            rls_context,
        };

        if let Ok(mut connections) = self.connections.write() {
            connections.insert(connection_id, connection);
        }

        rx
    }

    /// Remove a connection
    pub fn disconnect(&self, connection_id: &str) {
        // Remove all subscriptions for this connection
        self.subscriptions.unsubscribe_all(connection_id);

        // Remove the connection
        if let Ok(mut connections) = self.connections.write() {
            connections.remove(connection_id);
        }
    }

    /// Dispatch an event to all matching subscribers
    ///
    /// This is explicitly non-deterministic (RT-D1).
    /// Events may be delivered out of order or not at all.
    pub fn dispatch(&self, event: &DatabaseEvent) -> DispatchResult {
        let mut result = DispatchResult::default();

        // Find matching subscriptions
        let subscriptions = self.subscriptions.matching(event);
        result.matched = subscriptions.len();

        // Get connections
        let connections = match self.connections.read() {
            Ok(c) => c,
            Err(_) => return result,
        };

        // Get RLS policy for this collection
        let rls_policy = self
            .rls_policies
            .read()
            .ok()
            .and_then(|p| p.get(&event.collection).cloned());

        // Dispatch to each matching subscription
        for subscription in subscriptions {
            // Check RLS
            if !self.check_rls(&subscription.rls_context, &rls_policy, event) {
                result.filtered += 1;
                continue;
            }

            // Get connection
            if let Some(conn) = connections.get(&subscription.connection_id) {
                // Send event (non-blocking)
                match conn.sender.send(event.clone()) {
                    Ok(_) => result.delivered += 1,
                    Err(_) => result.failed += 1,
                }
            } else {
                result.failed += 1;
            }
        }

        result
    }

    /// Check if event passes RLS for a given context
    fn check_rls(
        &self,
        context: &RlsContext,
        policy: &Option<RlsPolicy>,
        event: &DatabaseEvent,
    ) -> bool {
        // Service role bypasses RLS
        if context.can_bypass_rls() {
            return true;
        }

        let Some(policy) = policy else {
            return true; // No policy = allow
        };

        match policy {
            RlsPolicy::None => true,
            RlsPolicy::Ownership { owner_field } => {
                // Check if user owns the record
                let data = event.new_data.as_ref().or(event.old_data.as_ref());
                if let Some(data) = data {
                    if let Some(owner_id) = data.get(owner_field).and_then(|v| v.as_str()) {
                        if let Some(user_id) = &context.user_id {
                            return owner_id == user_id.to_string();
                        }
                    }
                }
                false
            }
            RlsPolicy::PublicRead { .. } => {
                // Allow read for everyone
                true
            }
            RlsPolicy::Custom { .. } => {
                // Custom policies not yet supported
                false
            }
        }
    }

    /// Get connection count
    pub fn connection_count(&self) -> usize {
        self.connections.read().map(|c| c.len()).unwrap_or(0)
    }
}

/// Result of dispatching an event
#[derive(Debug, Default)]
pub struct DispatchResult {
    /// Number of matching subscriptions
    pub matched: usize,
    /// Number of events delivered
    pub delivered: usize,
    /// Number of events filtered by RLS
    pub filtered: usize,
    /// Number of failed deliveries
    pub failed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn test_connect_disconnect() {
        let registry = Arc::new(SubscriptionRegistry::new());
        let dispatcher = Dispatcher::new(registry);

        let _rx = dispatcher.connect("conn-1".to_string(), RlsContext::anonymous());
        assert_eq!(dispatcher.connection_count(), 1);

        dispatcher.disconnect("conn-1");
        assert_eq!(dispatcher.connection_count(), 0);
    }

    #[tokio::test]
    async fn test_dispatch_to_subscriber() {
        let registry = Arc::new(SubscriptionRegistry::new());
        let dispatcher = Dispatcher::new(Arc::clone(&registry));

        // Connect
        let mut rx = dispatcher.connect("conn-1".to_string(), RlsContext::anonymous());

        // Subscribe
        let sub = Subscription::new(
            "conn-1".to_string(),
            "posts".to_string(),
            RlsContext::anonymous(),
        );
        registry.subscribe(sub).unwrap();

        // Create event
        let event = DatabaseEvent::insert(
            1,
            "posts".to_string(),
            "1".to_string(),
            json!({"title": "Hello"}),
            None,
        );

        // Dispatch
        let result = dispatcher.dispatch(&event);
        assert_eq!(result.matched, 1);
        assert_eq!(result.delivered, 1);

        // Receive
        let received = rx.recv().await.unwrap();
        assert_eq!(received.sequence, 1);
        assert_eq!(received.collection, "posts");
    }

    #[tokio::test]
    async fn test_rls_filtering() {
        let registry = Arc::new(SubscriptionRegistry::new());
        let dispatcher = Dispatcher::new(Arc::clone(&registry));

        // Register RLS policy
        dispatcher.register_rls_policy(
            "posts",
            RlsPolicy::Ownership {
                owner_field: "owner_id".to_string(),
            },
        );

        // Connect with specific user
        let user_id = Uuid::new_v4();
        let context = RlsContext::authenticated(user_id);
        let mut rx = dispatcher.connect("conn-1".to_string(), context.clone());

        // Subscribe
        let sub = Subscription::new("conn-1".to_string(), "posts".to_string(), context);
        registry.subscribe(sub).unwrap();

        // Event owned by someone else
        let other_user = Uuid::new_v4();
        let event = DatabaseEvent::insert(
            1,
            "posts".to_string(),
            "1".to_string(),
            json!({"title": "Hello", "owner_id": other_user.to_string()}),
            Some(other_user),
        );

        // Dispatch - should be filtered
        let result = dispatcher.dispatch(&event);
        assert_eq!(result.matched, 1);
        assert_eq!(result.filtered, 1);
        assert_eq!(result.delivered, 0);

        // Event owned by the user
        let event2 = DatabaseEvent::insert(
            2,
            "posts".to_string(),
            "2".to_string(),
            json!({"title": "Mine", "owner_id": user_id.to_string()}),
            Some(user_id),
        );

        // Dispatch - should be delivered
        let result2 = dispatcher.dispatch(&event2);
        assert_eq!(result2.matched, 1);
        assert_eq!(result2.delivered, 1);

        // Receive
        let received = rx.recv().await.unwrap();
        assert_eq!(received.sequence, 2);
    }
}
