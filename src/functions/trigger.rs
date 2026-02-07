//! # Trigger Types

use serde::{Deserialize, Serialize};

/// HTTP methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl Default for HttpMethod {
    fn default() -> Self {
        Self::Post
    }
}

/// Database event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DbEventType {
    Insert,
    Update,
    Delete,
}

/// Trigger types for functions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TriggerType {
    /// HTTP trigger
    Http {
        path: String,
        #[serde(default)]
        method: HttpMethod,
    },

    /// Database trigger
    Database {
        collection: String,
        event: DbEventType,
    },

    /// Scheduled trigger (cron)
    Schedule { cron: String },

    /// Webhook trigger
    Webhook {
        #[serde(skip_serializing)]
        secret: String,
    },
}

impl TriggerType {
    /// Create an HTTP trigger
    pub fn http(path: String) -> Self {
        Self::Http {
            path,
            method: HttpMethod::Post,
        }
    }

    /// Create a database trigger
    pub fn database(collection: String, event: DbEventType) -> Self {
        Self::Database { collection, event }
    }

    /// Create a schedule trigger
    pub fn schedule(cron: String) -> Self {
        Self::Schedule { cron }
    }

    /// Create a webhook trigger
    pub fn webhook(secret: String) -> Self {
        Self::Webhook { secret }
    }

    /// Get trigger identifier for matching
    pub fn identifier(&self) -> String {
        match self {
            TriggerType::Http { path, method } => format!("http:{}:{:?}", path, method),
            TriggerType::Database { collection, event } => format!("db:{}:{:?}", collection, event),
            TriggerType::Schedule { cron } => format!("cron:{}", cron),
            TriggerType::Webhook { .. } => "webhook".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_trigger() {
        let trigger = TriggerType::http("/api/hello".to_string());
        assert!(matches!(trigger, TriggerType::Http { .. }));
        assert!(trigger.identifier().contains("http:"));
    }

    #[test]
    fn test_database_trigger() {
        let trigger = TriggerType::database("users".to_string(), DbEventType::Insert);
        assert!(matches!(trigger, TriggerType::Database { .. }));
    }

    #[test]
    fn test_schedule_trigger() {
        let trigger = TriggerType::schedule("0 * * * *".to_string());
        assert!(matches!(trigger, TriggerType::Schedule { .. }));
    }
}
