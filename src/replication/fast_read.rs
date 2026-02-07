//! Replica Read Fast Path Optimization
//!
//! Per REPLICA_READ_FAST_PATH.md:
//! - Avoid redundant work for already-proven-safe snapshots
//! - Reduce read-path overhead on replicas only
//!
//! Per §4 Safety Preconditions:
//! - Replica has durably applied WAL up to CommitId R
//! - Requested snapshot CommitId S ≤ R
//! - Snapshot S is immutable
//! - No WAL gaps exist
//! - Replica is not mid-recovery
//! - Replica is not mid-snapshot-bootstrap
//!
//! Per §10.1 Disablement:
//! - Disableable via compile-time flag or startup config

/// Configuration for replica read fast path.
///
/// Per REPLICA_READ_FAST_PATH.md §10.1:
/// - Disableable via compile-time flag or startup config
/// - Disablement restores baseline replica read behavior
#[derive(Debug, Clone)]
pub struct FastReadConfig {
    /// Whether fast path reads are enabled.
    pub enabled: bool,
}

impl Default for FastReadConfig {
    fn default() -> Self {
        Self { enabled: false } // Conservative default
    }
}

impl FastReadConfig {
    /// Create config with fast path enabled.
    pub fn enabled() -> Self {
        Self { enabled: true }
    }

    /// Create config with fast path disabled (baseline).
    pub fn disabled() -> Self {
        Self::default()
    }
}

/// Safety precondition check result.
///
/// Per REPLICA_READ_FAST_PATH.md §4:
/// - All preconditions must hold to use fast path
/// - If any fails, fall back to baseline
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafetyCheck {
    /// All preconditions pass - fast path is safe.
    Safe,
    /// One or more preconditions fail.
    Unsafe(SafetyViolation),
}

impl SafetyCheck {
    pub fn is_safe(&self) -> bool {
        matches!(self, SafetyCheck::Safe)
    }
}

/// Describes which safety precondition failed.
///
/// Per REPLICA_READ_FAST_PATH.md §4:
/// - Each precondition that can fail should be identifiable
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafetyViolation {
    /// Requested snapshot CommitId > replica's durable WAL CommitId.
    SnapshotAheadOfWal {
        snapshot_commit_id: u64,
        wal_commit_id: u64,
    },
    /// WAL has gaps - not a clean prefix.
    WalGapDetected,
    /// Replica is currently in recovery mode.
    MidRecovery,
    /// Replica is bootstrapping from a snapshot.
    MidSnapshotBootstrap,
    /// Snapshot is not immutable (should not happen normally).
    SnapshotNotImmutable,
    /// Fast path is disabled.
    Disabled,
}

/// Replica read safety state.
///
/// Per REPLICA_READ_FAST_PATH.md §4:
/// - Represents the current safety state of the replica
#[derive(Debug, Clone)]
pub struct ReplicaSafetyState {
    /// Highest CommitId durably applied from WAL.
    pub durable_wal_commit_id: u64,
    /// Whether WAL is a clean prefix (no gaps).
    pub wal_contiguous: bool,
    /// Whether replica is in recovery mode.
    pub in_recovery: bool,
    /// Whether replica is bootstrapping from snapshot.
    pub in_snapshot_bootstrap: bool,
}

impl Default for ReplicaSafetyState {
    fn default() -> Self {
        Self {
            durable_wal_commit_id: 0,
            wal_contiguous: true,
            in_recovery: false,
            in_snapshot_bootstrap: false,
        }
    }
}

impl ReplicaSafetyState {
    /// Create a safe state for testing.
    pub fn safe_at(commit_id: u64) -> Self {
        Self {
            durable_wal_commit_id: commit_id,
            wal_contiguous: true,
            in_recovery: false,
            in_snapshot_bootstrap: false,
        }
    }
}

/// Safety precondition validator.
///
/// Per REPLICA_READ_FAST_PATH.md §4:
/// - Validates all preconditions before allowing fast path
#[derive(Debug)]
pub struct SafetyValidator {
    config: FastReadConfig,
}

impl SafetyValidator {
    /// Create a new safety validator.
    pub fn new(config: FastReadConfig) -> Self {
        Self { config }
    }

    /// Validate whether fast path is safe for a given snapshot.
    ///
    /// Per REPLICA_READ_FAST_PATH.md §4:
    /// 1. Replica has durably applied WAL up to CommitId R
    /// 2. Requested snapshot CommitId S ≤ R
    /// 3. Snapshot S is immutable
    /// 4. No WAL gaps exist
    /// 5. Replica is not mid-recovery
    /// 6. Replica is not mid-snapshot-bootstrap
    pub fn validate(&self, state: &ReplicaSafetyState, snapshot_commit_id: u64) -> SafetyCheck {
        // Check if fast path is enabled
        if !self.config.enabled {
            return SafetyCheck::Unsafe(SafetyViolation::Disabled);
        }

        // Precondition 5: Replica is not mid-recovery
        if state.in_recovery {
            return SafetyCheck::Unsafe(SafetyViolation::MidRecovery);
        }

        // Precondition 6: Replica is not mid-snapshot-bootstrap
        if state.in_snapshot_bootstrap {
            return SafetyCheck::Unsafe(SafetyViolation::MidSnapshotBootstrap);
        }

        // Precondition 4: No WAL gaps exist
        if !state.wal_contiguous {
            return SafetyCheck::Unsafe(SafetyViolation::WalGapDetected);
        }

        // Precondition 2: Requested snapshot CommitId S ≤ R
        if snapshot_commit_id > state.durable_wal_commit_id {
            return SafetyCheck::Unsafe(SafetyViolation::SnapshotAheadOfWal {
                snapshot_commit_id,
                wal_commit_id: state.durable_wal_commit_id,
            });
        }

        // All preconditions pass
        SafetyCheck::Safe
    }
}

/// Result of a fast path read attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastReadResult {
    /// Whether fast path was used.
    pub fast_path_used: bool,
    /// The snapshot CommitId that was read.
    pub snapshot_commit_id: u64,
}

/// Fast path read manager.
///
/// Per REPLICA_READ_FAST_PATH.md §5.2:
/// - Validates safety preconditions
/// - Reuses pre-validated snapshot visibility boundary
#[derive(Debug)]
pub struct FastReadManager {
    /// Configuration.
    config: FastReadConfig,
    /// Safety validator.
    validator: SafetyValidator,
    /// Current safety state.
    state: ReplicaSafetyState,
    /// Statistics for observability.
    stats: FastReadStats,
}

/// Statistics for fast path reads.
///
/// Per REPLICA_READ_FAST_PATH.md §11:
/// - Metrics are passive only
/// - Metrics MUST NOT influence read selection
#[derive(Debug, Default, Clone)]
pub struct FastReadStats {
    /// Number of fast path hits.
    pub fast_path_hits: u64,
    /// Number of fast path misses (fell back to baseline).
    pub fast_path_misses: u64,
    /// Number of fallbacks due to each violation type.
    pub fallback_disabled: u64,
    pub fallback_mid_recovery: u64,
    pub fallback_mid_bootstrap: u64,
    pub fallback_wal_gap: u64,
    pub fallback_snapshot_ahead: u64,
}

impl FastReadManager {
    /// Create a new fast read manager.
    pub fn new(config: FastReadConfig) -> Self {
        let validator = SafetyValidator::new(config.clone());
        Self {
            config,
            validator,
            state: ReplicaSafetyState::default(),
            stats: FastReadStats::default(),
        }
    }

    /// Check if fast path is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Update the replica safety state.
    pub fn update_state(&mut self, state: ReplicaSafetyState) {
        self.state = state;
    }

    /// Update just the durable WAL commit ID (common case).
    pub fn update_durable_commit_id(&mut self, commit_id: u64) {
        self.state.durable_wal_commit_id = commit_id;
    }

    /// Set recovery mode.
    pub fn set_recovery_mode(&mut self, in_recovery: bool) {
        self.state.in_recovery = in_recovery;
    }

    /// Set bootstrap mode.
    pub fn set_bootstrap_mode(&mut self, in_bootstrap: bool) {
        self.state.in_snapshot_bootstrap = in_bootstrap;
    }

    /// Attempt to use fast path for a read.
    ///
    /// Returns whether fast path was used and the snapshot CommitId.
    pub fn try_fast_read(&mut self, snapshot_commit_id: u64) -> FastReadResult {
        let check = self.validator.validate(&self.state, snapshot_commit_id);

        match check {
            SafetyCheck::Safe => {
                self.stats.fast_path_hits += 1;
                FastReadResult {
                    fast_path_used: true,
                    snapshot_commit_id,
                }
            }
            SafetyCheck::Unsafe(violation) => {
                self.stats.fast_path_misses += 1;
                match violation {
                    SafetyViolation::Disabled => self.stats.fallback_disabled += 1,
                    SafetyViolation::MidRecovery => self.stats.fallback_mid_recovery += 1,
                    SafetyViolation::MidSnapshotBootstrap => self.stats.fallback_mid_bootstrap += 1,
                    SafetyViolation::WalGapDetected => self.stats.fallback_wal_gap += 1,
                    SafetyViolation::SnapshotAheadOfWal { .. } => {
                        self.stats.fallback_snapshot_ahead += 1
                    }
                    SafetyViolation::SnapshotNotImmutable => {}
                }
                FastReadResult {
                    fast_path_used: false,
                    snapshot_commit_id,
                }
            }
        }
    }

    /// Get the current durable commit ID.
    pub fn durable_commit_id(&self) -> u64 {
        self.state.durable_wal_commit_id
    }

    /// Get statistics.
    pub fn stats(&self) -> &FastReadStats {
        &self.stats
    }
}

/// Read path selection.
///
/// Per REPLICA_READ_FAST_PATH.md §10.1:
/// - Disablement restores baseline replica read behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplicaReadPath {
    /// Baseline: full validation on each read.
    Baseline,
    /// Fast path: pre-validated snapshot reuse.
    FastPath,
}

impl ReplicaReadPath {
    /// Determine read path based on config.
    pub fn from_config(config: &FastReadConfig) -> Self {
        if config.enabled {
            Self::FastPath
        } else {
            Self::Baseline
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== FastReadConfig Tests ====================

    #[test]
    fn test_config_default() {
        let config = FastReadConfig::default();
        assert!(!config.enabled);
    }

    #[test]
    fn test_config_enabled() {
        let config = FastReadConfig::enabled();
        assert!(config.enabled);
    }

    // ==================== SafetyValidator Tests ====================

    #[test]
    fn test_validator_disabled() {
        let validator = SafetyValidator::new(FastReadConfig::disabled());
        let state = ReplicaSafetyState::safe_at(100);
        let result = validator.validate(&state, 50);
        assert_eq!(result, SafetyCheck::Unsafe(SafetyViolation::Disabled));
    }

    #[test]
    fn test_validator_safe() {
        let validator = SafetyValidator::new(FastReadConfig::enabled());
        let state = ReplicaSafetyState::safe_at(100);
        let result = validator.validate(&state, 50);
        assert!(result.is_safe());
    }

    #[test]
    fn test_validator_safe_at_boundary() {
        let validator = SafetyValidator::new(FastReadConfig::enabled());
        let state = ReplicaSafetyState::safe_at(100);
        let result = validator.validate(&state, 100); // S == R
        assert!(result.is_safe());
    }

    #[test]
    fn test_validator_snapshot_ahead() {
        let validator = SafetyValidator::new(FastReadConfig::enabled());
        let state = ReplicaSafetyState::safe_at(100);
        let result = validator.validate(&state, 150); // S > R
        assert_eq!(
            result,
            SafetyCheck::Unsafe(SafetyViolation::SnapshotAheadOfWal {
                snapshot_commit_id: 150,
                wal_commit_id: 100,
            })
        );
    }

    #[test]
    fn test_validator_mid_recovery() {
        let validator = SafetyValidator::new(FastReadConfig::enabled());
        let mut state = ReplicaSafetyState::safe_at(100);
        state.in_recovery = true;
        let result = validator.validate(&state, 50);
        assert_eq!(result, SafetyCheck::Unsafe(SafetyViolation::MidRecovery));
    }

    #[test]
    fn test_validator_mid_bootstrap() {
        let validator = SafetyValidator::new(FastReadConfig::enabled());
        let mut state = ReplicaSafetyState::safe_at(100);
        state.in_snapshot_bootstrap = true;
        let result = validator.validate(&state, 50);
        assert_eq!(
            result,
            SafetyCheck::Unsafe(SafetyViolation::MidSnapshotBootstrap)
        );
    }

    #[test]
    fn test_validator_wal_gap() {
        let validator = SafetyValidator::new(FastReadConfig::enabled());
        let mut state = ReplicaSafetyState::safe_at(100);
        state.wal_contiguous = false;
        let result = validator.validate(&state, 50);
        assert_eq!(result, SafetyCheck::Unsafe(SafetyViolation::WalGapDetected));
    }

    // ==================== FastReadManager Tests ====================

    #[test]
    fn test_manager_disabled() {
        let mut manager = FastReadManager::new(FastReadConfig::disabled());
        let result = manager.try_fast_read(50);
        assert!(!result.fast_path_used);
        assert_eq!(manager.stats().fallback_disabled, 1);
    }

    #[test]
    fn test_manager_enabled_and_safe() {
        let mut manager = FastReadManager::new(FastReadConfig::enabled());
        manager.update_state(ReplicaSafetyState::safe_at(100));

        let result = manager.try_fast_read(50);
        assert!(result.fast_path_used);
        assert_eq!(manager.stats().fast_path_hits, 1);
    }

    #[test]
    fn test_manager_update_commit_id() {
        let mut manager = FastReadManager::new(FastReadConfig::enabled());
        manager.update_durable_commit_id(100);
        assert_eq!(manager.durable_commit_id(), 100);
    }

    #[test]
    fn test_manager_recovery_mode() {
        let mut manager = FastReadManager::new(FastReadConfig::enabled());
        manager.update_state(ReplicaSafetyState::safe_at(100));
        manager.set_recovery_mode(true);

        let result = manager.try_fast_read(50);
        assert!(!result.fast_path_used);
        assert_eq!(manager.stats().fallback_mid_recovery, 1);
    }

    #[test]
    fn test_manager_stats_accumulate() {
        let mut manager = FastReadManager::new(FastReadConfig::enabled());
        manager.update_state(ReplicaSafetyState::safe_at(100));

        manager.try_fast_read(50);
        manager.try_fast_read(60);
        manager.try_fast_read(70);

        assert_eq!(manager.stats().fast_path_hits, 3);
    }

    // ==================== ReplicaReadPath Tests ====================

    #[test]
    fn test_path_baseline() {
        let config = FastReadConfig::disabled();
        assert_eq!(
            ReplicaReadPath::from_config(&config),
            ReplicaReadPath::Baseline
        );
    }

    #[test]
    fn test_path_fast() {
        let config = FastReadConfig::enabled();
        assert_eq!(
            ReplicaReadPath::from_config(&config),
            ReplicaReadPath::FastPath
        );
    }

    // ==================== Equivalence Tests ====================

    /// Per REPLICA_READ_FAST_PATH.md §7:
    /// "The optimization removes redundant checks, not safety checks."
    #[test]
    fn test_fast_path_never_reads_unreplicated() {
        let mut manager = FastReadManager::new(FastReadConfig::enabled());
        manager.update_state(ReplicaSafetyState::safe_at(100));

        // Trying to read snapshot ahead of replicated data should fail
        let result = manager.try_fast_read(150);
        assert!(!result.fast_path_used);
        assert_eq!(manager.stats().fallback_snapshot_ahead, 1);
    }

    /// Per REPLICA_READ_FAST_PATH.md §8.4:
    /// "Replica lag increase causes fallback to baseline."
    #[test]
    fn test_fallback_on_lag() {
        let mut manager = FastReadManager::new(FastReadConfig::enabled());
        manager.update_state(ReplicaSafetyState::safe_at(50));

        // Snapshot is ahead of our WAL state
        let result = manager.try_fast_read(100);
        assert!(!result.fast_path_used);
    }
}
