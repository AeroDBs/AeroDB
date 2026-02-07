//! Metrics registry for AeroDB
//!
//! Per OBSERVABILITY.md:
//! - Counters only (no gauges, no histograms)
//! - Monotonic increase
//! - Reset only on process start
//! - Thread-safe but lock-minimal

use std::sync::atomic::{AtomicU64, Ordering};

/// Metrics registry containing all operational counters
///
/// Per OBSERVABILITY.md §5, all values are exact.
///
/// # Thread Safety
///
/// All counters use atomic operations for thread-safe increments.
/// Uses Relaxed ordering for minimal overhead (eventual consistency is fine for metrics).
#[derive(Debug, Default)]
pub struct MetricsRegistry {
    /// Total bytes written to WAL
    wal_bytes_written: AtomicU64,
    /// Total WAL records written
    wal_records_written: AtomicU64,
    /// WAL truncation count
    wal_truncations: AtomicU64,
    /// Snapshot count
    snapshots_created: AtomicU64,
    /// Checkpoint count
    checkpoints_created: AtomicU64,
    /// Backup count
    backups_created: AtomicU64,
    /// Restore count
    restores_performed: AtomicU64,
    /// Successful query count
    queries_executed: AtomicU64,
    /// Rejected query count
    queries_rejected: AtomicU64,
    /// Recovery run count
    recovery_runs: AtomicU64,
    /// Recovery failure count
    recovery_failures: AtomicU64,
    /// Document count (current)
    documents: AtomicU64,
    /// Write operation count
    writes: AtomicU64,
}

impl MetricsRegistry {
    /// Create a new metrics registry with all counters at zero
    pub fn new() -> Self {
        Self::default()
    }

    // WAL metrics

    /// Increment WAL bytes written
    pub fn add_wal_bytes(&self, bytes: u64) {
        self.wal_bytes_written.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Increment WAL records written
    pub fn increment_wal_records(&self) {
        self.wal_records_written.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment WAL truncations
    pub fn increment_wal_truncations(&self) {
        self.wal_truncations.fetch_add(1, Ordering::Relaxed);
    }

    /// Get WAL bytes written
    pub fn wal_bytes(&self) -> u64 {
        self.wal_bytes_written.load(Ordering::Relaxed)
    }

    // Snapshot/Checkpoint metrics

    /// Increment snapshots created
    pub fn increment_snapshots(&self) {
        self.snapshots_created.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment checkpoints created
    pub fn increment_checkpoints(&self) {
        self.checkpoints_created.fetch_add(1, Ordering::Relaxed);
    }

    // Backup/Restore metrics

    /// Increment backups created
    pub fn increment_backups(&self) {
        self.backups_created.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment restores performed
    pub fn increment_restores(&self) {
        self.restores_performed.fetch_add(1, Ordering::Relaxed);
    }

    // Query metrics

    /// Increment queries executed
    pub fn increment_queries_executed(&self) {
        self.queries_executed.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment queries rejected
    pub fn increment_queries_rejected(&self) {
        self.queries_rejected.fetch_add(1, Ordering::Relaxed);
    }

    // Recovery metrics

    /// Increment recovery runs
    pub fn increment_recovery_runs(&self) {
        self.recovery_runs.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment recovery failures
    pub fn increment_recovery_failures(&self) {
        self.recovery_failures.fetch_add(1, Ordering::Relaxed);
    }

    // Document metrics

    /// Set document count
    pub fn set_documents(&self, count: u64) {
        self.documents.store(count, Ordering::Relaxed);
    }

    /// Increment document count
    pub fn increment_documents(&self) {
        self.documents.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement document count
    pub fn decrement_documents(&self) {
        self.documents.fetch_sub(1, Ordering::Relaxed);
    }

    /// Increment writes
    pub fn increment_writes(&self) {
        self.writes.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current snapshot of all metrics as JSON
    ///
    /// Per OBSERVABILITY.md §5, returns exact values.
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"wal_bytes":{},"wal_records":{},"wal_truncations":{},"snapshots":{},"checkpoints":{},"backups":{},"restores":{},"queries_executed":{},"queries_rejected":{},"recovery_runs":{},"recovery_failures":{},"documents":{},"writes":{}}}"#,
            self.wal_bytes_written.load(Ordering::Relaxed),
            self.wal_records_written.load(Ordering::Relaxed),
            self.wal_truncations.load(Ordering::Relaxed),
            self.snapshots_created.load(Ordering::Relaxed),
            self.checkpoints_created.load(Ordering::Relaxed),
            self.backups_created.load(Ordering::Relaxed),
            self.restores_performed.load(Ordering::Relaxed),
            self.queries_executed.load(Ordering::Relaxed),
            self.queries_rejected.load(Ordering::Relaxed),
            self.recovery_runs.load(Ordering::Relaxed),
            self.recovery_failures.load(Ordering::Relaxed),
            self.documents.load(Ordering::Relaxed),
            self.writes.load(Ordering::Relaxed),
        )
    }

    /// Get all metrics as a snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            wal_bytes: self.wal_bytes_written.load(Ordering::Relaxed),
            wal_records: self.wal_records_written.load(Ordering::Relaxed),
            wal_truncations: self.wal_truncations.load(Ordering::Relaxed),
            snapshots: self.snapshots_created.load(Ordering::Relaxed),
            checkpoints: self.checkpoints_created.load(Ordering::Relaxed),
            backups: self.backups_created.load(Ordering::Relaxed),
            restores: self.restores_performed.load(Ordering::Relaxed),
            queries_executed: self.queries_executed.load(Ordering::Relaxed),
            queries_rejected: self.queries_rejected.load(Ordering::Relaxed),
            recovery_runs: self.recovery_runs.load(Ordering::Relaxed),
            recovery_failures: self.recovery_failures.load(Ordering::Relaxed),
            documents: self.documents.load(Ordering::Relaxed),
            writes: self.writes.load(Ordering::Relaxed),
        }
    }
}

/// A point-in-time snapshot of all metrics
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricsSnapshot {
    pub wal_bytes: u64,
    pub wal_records: u64,
    pub wal_truncations: u64,
    pub snapshots: u64,
    pub checkpoints: u64,
    pub backups: u64,
    pub restores: u64,
    pub queries_executed: u64,
    pub queries_rejected: u64,
    pub recovery_runs: u64,
    pub recovery_failures: u64,
    pub documents: u64,
    pub writes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_registry_has_zero_values() {
        let registry = MetricsRegistry::new();
        let snapshot = registry.snapshot();

        assert_eq!(snapshot.wal_bytes, 0);
        assert_eq!(snapshot.wal_records, 0);
        assert_eq!(snapshot.snapshots, 0);
        assert_eq!(snapshot.queries_executed, 0);
    }

    #[test]
    fn test_increment_wal_bytes() {
        let registry = MetricsRegistry::new();

        registry.add_wal_bytes(100);
        assert_eq!(registry.wal_bytes(), 100);

        registry.add_wal_bytes(50);
        assert_eq!(registry.wal_bytes(), 150);
    }

    #[test]
    fn test_increment_counters() {
        let registry = MetricsRegistry::new();

        registry.increment_wal_records();
        registry.increment_wal_records();
        registry.increment_snapshots();
        registry.increment_checkpoints();
        registry.increment_backups();
        registry.increment_restores();
        registry.increment_queries_executed();
        registry.increment_queries_rejected();
        registry.increment_recovery_runs();
        registry.increment_recovery_failures();

        let snapshot = registry.snapshot();
        assert_eq!(snapshot.wal_records, 2);
        assert_eq!(snapshot.snapshots, 1);
        assert_eq!(snapshot.checkpoints, 1);
        assert_eq!(snapshot.backups, 1);
        assert_eq!(snapshot.restores, 1);
        assert_eq!(snapshot.queries_executed, 1);
        assert_eq!(snapshot.queries_rejected, 1);
        assert_eq!(snapshot.recovery_runs, 1);
        assert_eq!(snapshot.recovery_failures, 1);
    }

    #[test]
    fn test_document_count() {
        let registry = MetricsRegistry::new();

        registry.set_documents(100);
        assert_eq!(registry.snapshot().documents, 100);

        registry.increment_documents();
        assert_eq!(registry.snapshot().documents, 101);

        registry.decrement_documents();
        assert_eq!(registry.snapshot().documents, 100);
    }

    #[test]
    fn test_to_json() {
        let registry = MetricsRegistry::new();
        registry.add_wal_bytes(1234);
        registry.increment_queries_executed();

        let json = registry.to_json();

        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["wal_bytes"], 1234);
        assert_eq!(parsed["queries_executed"], 1);
    }

    #[test]
    fn test_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let registry = Arc::new(MetricsRegistry::new());
        let mut handles = vec![];

        // Spawn multiple threads incrementing counters
        for _ in 0..10 {
            let reg = Arc::clone(&registry);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    reg.increment_wal_records();
                    reg.increment_queries_executed();
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let snapshot = registry.snapshot();
        assert_eq!(snapshot.wal_records, 1000);
        assert_eq!(snapshot.queries_executed, 1000);
    }

    #[test]
    fn test_monotonic_increase() {
        let registry = MetricsRegistry::new();

        let mut prev = registry.snapshot().wal_bytes;
        for _ in 0..10 {
            registry.add_wal_bytes(10);
            let current = registry.snapshot().wal_bytes;
            assert!(current >= prev);
            prev = current;
        }
    }
}
