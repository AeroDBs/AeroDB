//! Observability API Server
//!
//! Per DX_OBSERVABILITY_API.md §3.1:
//! - HTTP (local only)
//! - JSON responses
//! - UTF-8 encoding
//!
//! Per DX_INVARIANTS.md §P4-4 (Observability Passivity):
//! - MUST NOT influence scheduling, batching, WAL, MVCC, or replication
//!
//! Read-only, Phase 4, no semantic authority.

use super::handlers::*;
use super::response::{ApiResponse, ObservedAt};
use crate::dx::config::DxConfig;
use serde::Serialize;

/// Observability API server.
///
/// Per DX_INVARIANTS.md §P4-16:
/// - Fully disableable
/// - Removing MUST leave core behavior unchanged
#[derive(Debug)]
pub struct ObservabilityServer {
    config: DxConfig,
}

impl ObservabilityServer {
    /// Create a new observability server.
    pub fn new(config: DxConfig) -> Self {
        Self { config }
    }

    /// Check if the server should run.
    pub fn is_enabled(&self) -> bool {
        self.config.is_enabled()
    }

    /// Get bind address.
    pub fn bind_addr(&self) -> String {
        self.config.bind_addr()
    }

    /// Generate status response.
    ///
    /// Read-only, Phase 4, no semantic authority.
    pub fn get_status(
        &self,
        lifecycle: LifecycleState,
        commit_id: u64,
        wal_durable: u64,
    ) -> ApiResponse<StatusData> {
        handle_status(lifecycle, commit_id, wal_durable)
    }

    /// Generate WAL response.
    ///
    /// Read-only, Phase 4, no semantic authority.
    pub fn get_wal(&self, commit_id: u64, wal_data: WalData) -> ApiResponse<WalData> {
        handle_wal(commit_id, wal_data)
    }

    /// Generate MVCC response.
    ///
    /// Read-only, Phase 4, no semantic authority.
    pub fn get_mvcc(&self, commit_id: u64, mvcc_data: MvccData) -> ApiResponse<MvccData> {
        handle_mvcc(commit_id, mvcc_data)
    }

    /// Generate snapshots response.
    ///
    /// Read-only, Phase 4, no semantic authority.
    pub fn get_snapshots(
        &self,
        commit_id: u64,
        snapshots: Vec<SnapshotInfo>,
    ) -> ApiResponse<SnapshotsData> {
        let data = SnapshotsData { snapshots };
        ApiResponse::new(ObservedAt::live(commit_id), data)
    }

    /// Generate checkpoints response.
    ///
    /// Read-only, Phase 4, no semantic authority.
    pub fn get_checkpoints(
        &self,
        commit_id: u64,
        data: CheckpointsData,
    ) -> ApiResponse<CheckpointsData> {
        ApiResponse::new(ObservedAt::live(commit_id), data)
    }

    /// Generate indexes response.
    ///
    /// Read-only, Phase 4, no semantic authority.
    pub fn get_indexes(&self, commit_id: u64, indexes: Vec<IndexInfo>) -> ApiResponse<IndexesData> {
        let data = IndexesData { indexes };
        ApiResponse::new(ObservedAt::live(commit_id), data)
    }

    /// Generate replication response.
    ///
    /// Read-only, Phase 4, no semantic authority.
    /// Per DX_OBSERVABILITY_API.md §5.7: Must NOT attempt to contact peers.
    pub fn get_replication(
        &self,
        commit_id: u64,
        data: ReplicationData,
    ) -> ApiResponse<ReplicationData> {
        ApiResponse::new(ObservedAt::live(commit_id), data)
    }

    /// Serialize response to JSON.
    ///
    /// Per DX_OBSERVABILITY_API.md §3.1: JSON responses, UTF-8 encoding.
    pub fn to_json<T: Serialize>(response: &ApiResponse<T>) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_disabled_by_default() {
        let server = ObservabilityServer::new(DxConfig::default());
        assert!(!server.is_enabled());
    }

    #[test]
    fn test_server_enabled() {
        let server = ObservabilityServer::new(DxConfig::enabled());
        assert!(server.is_enabled());
    }

    #[test]
    fn test_bind_addr() {
        let server = ObservabilityServer::new(DxConfig::default());
        assert_eq!(server.bind_addr(), "127.0.0.1:9191");
    }

    #[test]
    fn test_status_response() {
        let server = ObservabilityServer::new(DxConfig::enabled());
        let resp = server.get_status(LifecycleState::Running, 100, 95);

        assert_eq!(resp.api_version, "v1");
        assert_eq!(resp.observed_at.commit_id, 100);
        assert_eq!(resp.data.commit_id_high_water, 100);
    }

    #[test]
    fn test_json_serialization() {
        let server = ObservabilityServer::new(DxConfig::enabled());
        let resp = server.get_status(LifecycleState::Running, 100, 95);
        let json = ObservabilityServer::to_json(&resp).unwrap();

        assert!(json.contains("\"api_version\": \"v1\""));
        assert!(json.contains("\"commit_id\": 100"));
    }

    #[test]
    fn test_mvcc_response() {
        let server = ObservabilityServer::new(DxConfig::enabled());
        let mvcc_data = MvccData {
            oldest_retained_commit_id: 50,
            latest_commit_id: 100,
            active_snapshot_count: 2,
            active_snapshot_ids: vec![1, 2],
            gc_watermark: Some(45),
        };
        let resp = server.get_mvcc(100, mvcc_data);

        assert_eq!(resp.data.latest_commit_id, 100);
        assert_eq!(resp.data.active_snapshot_count, 2);
    }

    #[test]
    fn test_replication_response() {
        let server = ObservabilityServer::new(DxConfig::enabled());
        let repl_data = ReplicationData {
            role: ReplicationRole::Primary,
            replica_state: "primary_active".to_string(),
            replica_id: None,
            read_safety: ReadSafetyStatus::Disabled,
            blocking_reason: None,
            replication_enabled: true,
            wal_prefix_commit_id: None,
            replica_lag: None,
            snapshot_bootstrap_active: false,
        };
        let resp = server.get_replication(100, repl_data);

        assert_eq!(resp.data.role, ReplicationRole::Primary);
    }
}
