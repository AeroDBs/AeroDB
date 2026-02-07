//! # Usage Metering
//!
//! Track resource usage per tenant for billing and quota enforcement.

use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Usage metrics for a tenant
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageMetrics {
    /// Tenant ID
    pub tenant_id: Uuid,
    /// Month (YYYY-MM format)
    pub month: String,
    /// Total API requests
    pub api_requests: u64,
    /// Storage used (bytes)
    pub storage_bytes: u64,
    /// File storage used (bytes)
    pub file_storage_bytes: u64,
    /// Egress bandwidth (bytes)
    pub egress_bytes: u64,
    /// Peak realtime connections
    pub realtime_connections_peak: u64,
    /// Average realtime connections
    pub realtime_connections_avg: u64,
    /// Total function invocations
    pub function_invocations: u64,
    /// Total function execution time (ms)
    pub function_execution_ms: u64,
}

impl UsageMetrics {
    /// Create new metrics for a tenant
    pub fn new(tenant_id: Uuid) -> Self {
        Self {
            tenant_id,
            month: current_month(),
            ..Default::default()
        }
    }

    /// Create metrics for a specific month
    pub fn for_month(tenant_id: Uuid, month: String) -> Self {
        Self {
            tenant_id,
            month,
            ..Default::default()
        }
    }
}

/// Get current month in YYYY-MM format
pub fn current_month() -> String {
    let now = Utc::now();
    format!("{:04}-{:02}", now.year(), now.month())
}

/// In-memory usage tracker (for development/testing)
/// In production, use Redis or a persistent store
#[derive(Debug, Clone, Default)]
pub struct UsageTracker {
    /// Usage metrics by tenant and month
    metrics: Arc<RwLock<HashMap<(Uuid, String), UsageMetrics>>>,
    /// Current realtime connections by tenant
    realtime_connections: Arc<RwLock<HashMap<Uuid, u64>>>,
}

impl UsageTracker {
    /// Create a new usage tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create metrics for a tenant in the current month
    fn get_or_create(&self, tenant_id: Uuid) -> UsageMetrics {
        let month = current_month();
        let key = (tenant_id, month.clone());

        let read = self.metrics.read().unwrap();
        if let Some(metrics) = read.get(&key) {
            return metrics.clone();
        }
        drop(read);

        let mut write = self.metrics.write().unwrap();
        write
            .entry(key)
            .or_insert_with(|| UsageMetrics::for_month(tenant_id, month))
            .clone()
    }

    /// Record an API request
    pub fn record_api_request(&self, tenant_id: Uuid) {
        let month = current_month();
        let key = (tenant_id, month.clone());

        let mut write = self.metrics.write().unwrap();
        let metrics = write
            .entry(key)
            .or_insert_with(|| UsageMetrics::for_month(tenant_id, month));
        metrics.api_requests += 1;
    }

    /// Record storage change
    pub fn record_storage(&self, tenant_id: Uuid, delta_bytes: i64) {
        let month = current_month();
        let key = (tenant_id, month.clone());

        let mut write = self.metrics.write().unwrap();
        let metrics = write
            .entry(key)
            .or_insert_with(|| UsageMetrics::for_month(tenant_id, month));

        if delta_bytes >= 0 {
            metrics.storage_bytes = metrics.storage_bytes.saturating_add(delta_bytes as u64);
        } else {
            metrics.storage_bytes = metrics.storage_bytes.saturating_sub((-delta_bytes) as u64);
        }
    }

    /// Record file storage change
    pub fn record_file_storage(&self, tenant_id: Uuid, delta_bytes: i64) {
        let month = current_month();
        let key = (tenant_id, month.clone());

        let mut write = self.metrics.write().unwrap();
        let metrics = write
            .entry(key)
            .or_insert_with(|| UsageMetrics::for_month(tenant_id, month));

        if delta_bytes >= 0 {
            metrics.file_storage_bytes =
                metrics.file_storage_bytes.saturating_add(delta_bytes as u64);
        } else {
            metrics.file_storage_bytes =
                metrics.file_storage_bytes.saturating_sub((-delta_bytes) as u64);
        }
    }

    /// Record egress bandwidth
    pub fn record_egress(&self, tenant_id: Uuid, bytes: u64) {
        let month = current_month();
        let key = (tenant_id, month.clone());

        let mut write = self.metrics.write().unwrap();
        let metrics = write
            .entry(key)
            .or_insert_with(|| UsageMetrics::for_month(tenant_id, month));
        metrics.egress_bytes = metrics.egress_bytes.saturating_add(bytes);
    }

    /// Record realtime connection opened
    pub fn record_realtime_connect(&self, tenant_id: Uuid) {
        let month = current_month();
        let key = (tenant_id, month.clone());

        // Update current connections
        let mut connections = self.realtime_connections.write().unwrap();
        let count = connections.entry(tenant_id).or_insert(0);
        *count += 1;
        let current = *count;
        drop(connections);

        // Update peak
        let mut metrics_write = self.metrics.write().unwrap();
        let metrics = metrics_write
            .entry(key)
            .or_insert_with(|| UsageMetrics::for_month(tenant_id, month));
        if current > metrics.realtime_connections_peak {
            metrics.realtime_connections_peak = current;
        }
    }

    /// Record realtime connection closed
    pub fn record_realtime_disconnect(&self, tenant_id: Uuid) {
        let mut connections = self.realtime_connections.write().unwrap();
        if let Some(count) = connections.get_mut(&tenant_id) {
            *count = count.saturating_sub(1);
        }
    }

    /// Get current realtime connection count
    pub fn get_realtime_connections(&self, tenant_id: Uuid) -> u64 {
        let connections = self.realtime_connections.read().unwrap();
        *connections.get(&tenant_id).unwrap_or(&0)
    }

    /// Record function invocation
    pub fn record_function_invocation(&self, tenant_id: Uuid, execution_ms: u64) {
        let month = current_month();
        let key = (tenant_id, month.clone());

        let mut write = self.metrics.write().unwrap();
        let metrics = write
            .entry(key)
            .or_insert_with(|| UsageMetrics::for_month(tenant_id, month));
        metrics.function_invocations += 1;
        metrics.function_execution_ms =
            metrics.function_execution_ms.saturating_add(execution_ms);
    }

    /// Get usage for current month
    pub fn get_current_usage(&self, tenant_id: Uuid) -> UsageMetrics {
        self.get_or_create(tenant_id)
    }

    /// Get usage for a specific month
    pub fn get_usage(&self, tenant_id: Uuid, month: &str) -> Option<UsageMetrics> {
        let key = (tenant_id, month.to_string());
        let read = self.metrics.read().unwrap();
        read.get(&key).cloned()
    }

    /// Get API request count for current month
    pub fn get_api_request_count(&self, tenant_id: Uuid) -> u64 {
        self.get_current_usage(tenant_id).api_requests
    }

    /// Get storage usage
    pub fn get_storage_bytes(&self, tenant_id: Uuid) -> u64 {
        self.get_current_usage(tenant_id).storage_bytes
    }

    /// Get file storage usage
    pub fn get_file_storage_bytes(&self, tenant_id: Uuid) -> u64 {
        self.get_current_usage(tenant_id).file_storage_bytes
    }

    /// Reset metrics for a tenant (used in testing)
    #[cfg(test)]
    pub fn reset(&self, tenant_id: Uuid) {
        let month = current_month();
        let key = (tenant_id, month);

        let mut write = self.metrics.write().unwrap();
        write.remove(&key);

        let mut connections = self.realtime_connections.write().unwrap();
        connections.remove(&tenant_id);
    }
}

/// Daily usage snapshot for historical tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySnapshot {
    /// Date (YYYY-MM-DD)
    pub date: String,
    /// Tenant ID
    pub tenant_id: Uuid,
    /// API requests on this day
    pub api_requests: u64,
    /// Storage at end of day
    pub storage_bytes: u64,
    /// File storage at end of day
    pub file_storage_bytes: u64,
    /// Egress on this day
    pub egress_bytes: u64,
    /// Peak realtime connections
    pub realtime_connections_peak: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_tracking() {
        let tracker = UsageTracker::new();
        let tenant_id = Uuid::new_v4();

        // Record API requests
        tracker.record_api_request(tenant_id);
        tracker.record_api_request(tenant_id);
        assert_eq!(tracker.get_api_request_count(tenant_id), 2);

        // Record storage
        tracker.record_storage(tenant_id, 1024);
        assert_eq!(tracker.get_storage_bytes(tenant_id), 1024);

        // Record negative storage (deletion)
        tracker.record_storage(tenant_id, -512);
        assert_eq!(tracker.get_storage_bytes(tenant_id), 512);
    }

    #[test]
    fn test_realtime_connections() {
        let tracker = UsageTracker::new();
        let tenant_id = Uuid::new_v4();

        // Connect
        tracker.record_realtime_connect(tenant_id);
        tracker.record_realtime_connect(tenant_id);
        assert_eq!(tracker.get_realtime_connections(tenant_id), 2);

        // Disconnect
        tracker.record_realtime_disconnect(tenant_id);
        assert_eq!(tracker.get_realtime_connections(tenant_id), 1);

        // Check peak
        let usage = tracker.get_current_usage(tenant_id);
        assert_eq!(usage.realtime_connections_peak, 2);
    }

    #[test]
    fn test_current_month() {
        let month = current_month();
        assert!(month.len() == 7); // YYYY-MM
        assert!(month.contains('-'));
    }
}
