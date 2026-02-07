//! # Subscription Management
//!
//! Client subscription registry and filtering.

use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::errors::{RealtimeError, RealtimeResult};
use super::event::DatabaseEvent;
use crate::auth::rls::RlsContext;

/// Filter operator for subscription predicates
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilterOp {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    In,
}

/// Subscription filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionFilter {
    /// Field to filter on
    pub field: String,
    /// Operator
    pub op: FilterOp,
    /// Value to compare
    pub value: serde_json::Value,
}

impl SubscriptionFilter {
    /// Check if an event matches this filter
    pub fn matches(&self, event: &DatabaseEvent) -> bool {
        // Get the value from new_data or old_data
        let data = event.new_data.as_ref().or(event.old_data.as_ref());

        let Some(data) = data else {
            return false;
        };

        let Some(field_value) = data.get(&self.field) else {
            return false;
        };

        match self.op {
            FilterOp::Eq => field_value == &self.value,
            FilterOp::Neq => field_value != &self.value,
            FilterOp::Gt => {
                if let (Some(a), Some(b)) = (field_value.as_f64(), self.value.as_f64()) {
                    a > b
                } else {
                    false
                }
            }
            FilterOp::Gte => {
                if let (Some(a), Some(b)) = (field_value.as_f64(), self.value.as_f64()) {
                    a >= b
                } else {
                    false
                }
            }
            FilterOp::Lt => {
                if let (Some(a), Some(b)) = (field_value.as_f64(), self.value.as_f64()) {
                    a < b
                } else {
                    false
                }
            }
            FilterOp::Lte => {
                if let (Some(a), Some(b)) = (field_value.as_f64(), self.value.as_f64()) {
                    a <= b
                } else {
                    false
                }
            }
            FilterOp::In => {
                if let Some(arr) = self.value.as_array() {
                    arr.contains(field_value)
                } else {
                    false
                }
            }
        }
    }
}

/// A subscription to database changes
#[derive(Debug, Clone)]
pub struct Subscription {
    /// Unique subscription ID
    pub id: String,

    /// Connection ID
    pub connection_id: String,

    /// Topic (e.g., "realtime:public:posts")
    pub topic: String,

    /// Collection name
    pub collection: String,

    /// Event types to subscribe to (None = all)
    pub event_types: Option<HashSet<String>>,

    /// Filters to apply
    pub filters: Vec<SubscriptionFilter>,

    /// RLS context for this subscription
    pub rls_context: RlsContext,
}

impl Subscription {
    /// Create a new subscription
    pub fn new(connection_id: String, collection: String, rls_context: RlsContext) -> Self {
        let id = Uuid::new_v4().to_string();
        let topic = format!("realtime:public:{}", collection);

        Self {
            id,
            connection_id,
            topic,
            collection,
            event_types: None,
            filters: Vec::new(),
            rls_context,
        }
    }

    /// Add a filter
    pub fn with_filter(mut self, filter: SubscriptionFilter) -> Self {
        self.filters.push(filter);
        self
    }

    /// Set event types
    pub fn with_events(mut self, events: HashSet<String>) -> Self {
        self.event_types = Some(events);
        self
    }

    /// Check if an event matches this subscription
    pub fn matches(&self, event: &DatabaseEvent) -> bool {
        // Check collection
        if event.collection != self.collection {
            return false;
        }

        // Check event type
        if let Some(ref types) = self.event_types {
            if !types.contains(&event.event_type.to_string()) {
                return false;
            }
        }

        // Check filters
        for filter in &self.filters {
            if !filter.matches(event) {
                return false;
            }
        }

        true
    }
}

/// Registry of active subscriptions
#[derive(Debug, Default)]
pub struct SubscriptionRegistry {
    /// Subscriptions by ID
    by_id: RwLock<HashMap<String, Subscription>>,

    /// Subscription IDs by topic
    by_topic: RwLock<HashMap<String, HashSet<String>>>,

    /// Subscription IDs by connection
    by_connection: RwLock<HashMap<String, HashSet<String>>>,

    /// Maximum subscriptions per connection
    max_per_connection: usize,
}

impl SubscriptionRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            by_id: RwLock::new(HashMap::new()),
            by_topic: RwLock::new(HashMap::new()),
            by_connection: RwLock::new(HashMap::new()),
            max_per_connection: 100,
        }
    }

    /// Add a subscription
    pub fn subscribe(&self, subscription: Subscription) -> RealtimeResult<String> {
        // Check limit
        if let Ok(by_conn) = self.by_connection.read() {
            if let Some(subs) = by_conn.get(&subscription.connection_id) {
                if subs.len() >= self.max_per_connection {
                    return Err(RealtimeError::TooManySubscriptions(self.max_per_connection));
                }
            }
        }

        let id = subscription.id.clone();
        let topic = subscription.topic.clone();
        let connection_id = subscription.connection_id.clone();

        // Add to all indexes
        if let Ok(mut by_id) = self.by_id.write() {
            by_id.insert(id.clone(), subscription);
        }

        if let Ok(mut by_topic) = self.by_topic.write() {
            by_topic.entry(topic).or_default().insert(id.clone());
        }

        if let Ok(mut by_conn) = self.by_connection.write() {
            by_conn.entry(connection_id).or_default().insert(id.clone());
        }

        Ok(id)
    }

    /// Remove a subscription
    pub fn unsubscribe(&self, subscription_id: &str) -> RealtimeResult<()> {
        let subscription = {
            let mut by_id = self
                .by_id
                .write()
                .map_err(|_| RealtimeError::Internal("Lock poisoned".into()))?;
            by_id.remove(subscription_id)
        };

        if let Some(sub) = subscription {
            if let Ok(mut by_topic) = self.by_topic.write() {
                if let Some(ids) = by_topic.get_mut(&sub.topic) {
                    ids.remove(subscription_id);
                }
            }

            if let Ok(mut by_conn) = self.by_connection.write() {
                if let Some(ids) = by_conn.get_mut(&sub.connection_id) {
                    ids.remove(subscription_id);
                }
            }

            Ok(())
        } else {
            Err(RealtimeError::SubscriptionNotFound(
                subscription_id.to_string(),
            ))
        }
    }

    /// Remove all subscriptions for a connection
    pub fn unsubscribe_all(&self, connection_id: &str) {
        let sub_ids: Vec<String> = {
            if let Ok(by_conn) = self.by_connection.read() {
                by_conn
                    .get(connection_id)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .collect()
            } else {
                return;
            }
        };

        for id in sub_ids {
            let _ = self.unsubscribe(&id);
        }
    }

    /// Get subscriptions matching an event
    pub fn matching(&self, event: &DatabaseEvent) -> Vec<Subscription> {
        let topic = event.topic();

        let sub_ids: Vec<String> = {
            if let Ok(by_topic) = self.by_topic.read() {
                by_topic
                    .get(&topic)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .collect()
            } else {
                return Vec::new();
            }
        };

        let mut result = Vec::new();
        if let Ok(by_id) = self.by_id.read() {
            for id in sub_ids {
                if let Some(sub) = by_id.get(&id) {
                    if sub.matches(event) {
                        result.push(sub.clone());
                    }
                }
            }
        }

        result
    }

    /// Get subscription count
    pub fn len(&self) -> usize {
        self.by_id.read().map(|m| m.len()).unwrap_or(0)
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Alias for subscribe (used by WebSocket server)
    pub fn register(&self, subscription: Subscription) -> RealtimeResult<String> {
        self.subscribe(subscription)
    }

    /// Unsubscribe by channel for a specific connection
    pub fn unsubscribe_by_channel(&self, connection_id: &str, channel: &str) -> RealtimeResult<()> {
        let sub_ids: Vec<String> = {
            if let Ok(by_conn) = self.by_connection.read() {
                by_conn
                    .get(connection_id)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .collect()
            } else {
                return Ok(());
            }
        };

        // Find subscription matching this channel
        let mut found = false;
        if let Ok(by_id) = self.by_id.read() {
            for id in sub_ids {
                if let Some(sub) = by_id.get(&id) {
                    if sub.collection == channel {
                        drop(by_id); // Release read lock before unsubscribe
                        self.unsubscribe(&id)?;
                        found = true;
                        break;
                    }
                }
            }
        }

        if found {
            Ok(())
        } else {
            Err(RealtimeError::SubscriptionNotFound(format!(
                "{}:{}",
                connection_id, channel
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_rls() -> RlsContext {
        RlsContext::anonymous()
    }

    #[test]
    fn test_subscription_creation() {
        let sub = Subscription::new("conn-1".to_string(), "posts".to_string(), create_test_rls());

        assert_eq!(sub.collection, "posts");
        assert_eq!(sub.topic, "realtime:public:posts");
    }

    #[test]
    fn test_filter_eq() {
        let filter = SubscriptionFilter {
            field: "status".to_string(),
            op: FilterOp::Eq,
            value: json!("published"),
        };

        let event = DatabaseEvent::insert(
            1,
            "posts".to_string(),
            "1".to_string(),
            json!({"status": "published"}),
            None,
        );

        assert!(filter.matches(&event));

        let event2 = DatabaseEvent::insert(
            2,
            "posts".to_string(),
            "2".to_string(),
            json!({"status": "draft"}),
            None,
        );

        assert!(!filter.matches(&event2));
    }

    #[test]
    fn test_filter_in() {
        let filter = SubscriptionFilter {
            field: "status".to_string(),
            op: FilterOp::In,
            value: json!(["published", "featured"]),
        };

        let event = DatabaseEvent::insert(
            1,
            "posts".to_string(),
            "1".to_string(),
            json!({"status": "published"}),
            None,
        );

        assert!(filter.matches(&event));
    }

    #[test]
    fn test_subscription_matching() {
        let sub = Subscription::new("conn-1".to_string(), "posts".to_string(), create_test_rls());

        let event = DatabaseEvent::insert(1, "posts".to_string(), "1".to_string(), json!({}), None);

        assert!(sub.matches(&event));

        let other_event =
            DatabaseEvent::insert(2, "comments".to_string(), "1".to_string(), json!({}), None);

        assert!(!sub.matches(&other_event));
    }

    #[test]
    fn test_registry_subscribe_unsubscribe() {
        let registry = SubscriptionRegistry::new();

        let sub = Subscription::new("conn-1".to_string(), "posts".to_string(), create_test_rls());

        let id = registry.subscribe(sub).unwrap();
        assert_eq!(registry.len(), 1);

        registry.unsubscribe(&id).unwrap();
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_matching() {
        let registry = SubscriptionRegistry::new();

        let sub = Subscription::new("conn-1".to_string(), "posts".to_string(), create_test_rls());
        registry.subscribe(sub).unwrap();

        let event = DatabaseEvent::insert(1, "posts".to_string(), "1".to_string(), json!({}), None);

        let matching = registry.matching(&event);
        assert_eq!(matching.len(), 1);
    }

    #[test]
    fn test_unsubscribe_all() {
        let registry = SubscriptionRegistry::new();

        for i in 0..5 {
            let sub = Subscription::new(
                "conn-1".to_string(),
                format!("collection-{}", i),
                create_test_rls(),
            );
            registry.subscribe(sub).unwrap();
        }

        assert_eq!(registry.len(), 5);

        registry.unsubscribe_all("conn-1");
        assert_eq!(registry.len(), 0);
    }
}
