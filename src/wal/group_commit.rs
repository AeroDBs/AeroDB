//! Group Commit Optimization
//!
//! Per GROUP_COMMIT.md:
//! - Allow multiple commits to share a single fsync
//! - Preserve exact durability, ordering, and visibility semantics
//! - No commit acknowledged before fsync returns
//!
//! Per §4.3 Group Formation Rules:
//! - Groups formed ONLY by concurrent arrival
//! - NO timers, delays, load thresholds, or background batching
//!
//! Per §9.1 Disablement:
//! - Disableable via compile-time flag or startup config
//!
//! Per §6: "The only difference is which commits wait on which fsync,
//!          which is not observable."

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

use super::errors::{WalError, WalResult};
use super::record::{RecordType, WalPayload};

/// Configuration for group commit.
///
/// Per GROUP_COMMIT.md §9.1:
/// - Disableable via compile-time flag or startup config
/// - When disabled, each commit performs its own fsync
#[derive(Debug, Clone)]
pub struct GroupCommitConfig {
    /// Whether group commit is enabled.
    /// When false, behavior is identical to baseline (one fsync per commit).
    pub enabled: bool,
}

impl Default for GroupCommitConfig {
    fn default() -> Self {
        Self { enabled: false } // Conservative default: disabled
    }
}

impl GroupCommitConfig {
    /// Create config with group commit enabled.
    pub fn enabled() -> Self {
        Self { enabled: true }
    }

    /// Create config with group commit disabled (baseline behavior).
    pub fn disabled() -> Self {
        Self { enabled: false }
    }
}

/// State of a pending commit in the group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingCommitState {
    /// Waiting to be appended to WAL.
    Queued,
    /// Appended to WAL, waiting for fsync.
    Appended,
    /// Fsync complete, commit is durable.
    Durable,
    /// Commit failed.
    Failed,
}

/// A pending commit waiting for group commit.
///
/// Per GROUP_COMMIT.md §3.1:
/// - Each commit is logically independent
/// - Separately represented in WAL
/// - Separately ordered
/// - Separately acknowledged
#[derive(Debug)]
pub struct PendingCommit {
    /// Record type for this commit.
    pub record_type: RecordType,
    /// Payload for this commit.
    pub payload: WalPayload,
    /// Assigned sequence number (after append).
    pub sequence_number: Option<u64>,
    /// Current state of this pending commit.
    pub state: PendingCommitState,
}

impl PendingCommit {
    /// Create a new pending commit.
    pub fn new(record_type: RecordType, payload: WalPayload) -> Self {
        Self {
            record_type,
            payload,
            sequence_number: None,
            state: PendingCommitState::Queued,
        }
    }

    /// Mark as appended with assigned sequence number.
    pub fn mark_appended(&mut self, seq: u64) {
        self.sequence_number = Some(seq);
        self.state = PendingCommitState::Appended;
    }

    /// Mark as durable (fsync complete).
    pub fn mark_durable(&mut self) {
        debug_assert!(self.state == PendingCommitState::Appended);
        self.state = PendingCommitState::Durable;
    }

    /// Mark as failed.
    pub fn mark_failed(&mut self) {
        self.state = PendingCommitState::Failed;
    }
}

/// A group of commits that will share a single fsync.
///
/// Per GROUP_COMMIT.md §4.2:
/// 1. Append WAL record for commit A
/// 2. Append WAL record for commit B
/// 3. Append WAL record for commit C
/// 4. fsync WAL once
/// 5. Assign CommitIds to A, B, C (in append order)
/// 6. Acknowledge A, B, C (in order)
#[derive(Debug, Default)]
pub struct CommitGroup {
    /// Commits in this group, in append order.
    commits: VecDeque<PendingCommit>,
    /// Whether all commits have been appended.
    all_appended: bool,
    /// Whether fsync has completed.
    fsync_complete: bool,
    /// Error if fsync failed.
    fsync_error: Option<String>,
}

impl CommitGroup {
    /// Create a new empty commit group.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a commit to the group.
    pub fn add_commit(&mut self, commit: PendingCommit) {
        debug_assert!(!self.all_appended, "cannot add after appending");
        self.commits.push_back(commit);
    }

    /// Number of commits in this group.
    pub fn len(&self) -> usize {
        self.commits.len()
    }

    /// Whether the group is empty.
    pub fn is_empty(&self) -> bool {
        self.commits.is_empty()
    }

    /// Mark a commit as appended.
    pub fn mark_commit_appended(&mut self, index: usize, seq: u64) {
        if let Some(commit) = self.commits.get_mut(index) {
            commit.mark_appended(seq);
        }
    }

    /// Mark all commits as appended.
    pub fn mark_all_appended(&mut self) {
        self.all_appended = true;
    }

    /// Mark fsync as complete (success).
    pub fn mark_fsync_complete(&mut self) {
        debug_assert!(self.all_appended);
        self.fsync_complete = true;
        for commit in &mut self.commits {
            if commit.state == PendingCommitState::Appended {
                commit.mark_durable();
            }
        }
    }

    /// Mark fsync as failed.
    pub fn mark_fsync_failed(&mut self, error: String) {
        self.fsync_error = Some(error);
        for commit in &mut self.commits {
            commit.mark_failed();
        }
    }

    /// Whether fsync has completed.
    pub fn is_fsync_complete(&self) -> bool {
        self.fsync_complete
    }

    /// Get fsync error if any.
    pub fn fsync_error(&self) -> Option<&str> {
        self.fsync_error.as_deref()
    }

    /// Get sequence numbers for all durable commits.
    pub fn durable_sequence_numbers(&self) -> Vec<u64> {
        self.commits
            .iter()
            .filter(|c| c.state == PendingCommitState::Durable)
            .filter_map(|c| c.sequence_number)
            .collect()
    }

    /// Drain all commits from the group.
    pub fn drain(&mut self) -> impl Iterator<Item = PendingCommit> + '_ {
        self.commits.drain(..)
    }
}

/// Result of submitting a commit to the group commit manager.
#[derive(Debug)]
pub struct GroupCommitResult {
    /// Sequence number assigned to the commit.
    pub sequence_number: u64,
    /// Whether this commit triggered the group fsync.
    pub was_leader: bool,
    /// Number of commits in the group.
    pub group_size: usize,
}

/// Group Commit Manager
///
/// Per GROUP_COMMIT.md §4.2:
/// - Allows multiple concurrent commits to share a single fsync
/// - Each commit is independently durable after fsync
///
/// Per §3.1:
/// - "Multiple logically independent commits to wait on the same fsync call,
///    provided that each commit's WAL record is fully written before that fsync."
///
/// Invariant Preservation:
/// - D-1: No commit acknowledged before fsync (enforced by wait_for_fsync)
/// - DET-1: WAL contents identical to baseline
/// - MVCC-2: CommitId assigned after fsync
#[derive(Debug)]
pub struct GroupCommitManager {
    /// Configuration.
    config: GroupCommitConfig,
    /// Inner state protected by mutex.
    inner: Mutex<GroupCommitInner>,
    /// Condition variable for fsync completion.
    fsync_complete: Condvar,
}

/// Inner state of the group commit manager.
#[derive(Debug)]
struct GroupCommitInner {
    /// Current group being formed.
    current_group: CommitGroup,
    /// Next group epoch (for distinguishing groups).
    epoch: u64,
    /// Current epoch's fsync status.
    current_epoch_fsync_complete: bool,
    /// Current epoch's fsync error.
    current_epoch_error: Option<String>,
}

impl GroupCommitManager {
    /// Create a new group commit manager.
    pub fn new(config: GroupCommitConfig) -> Self {
        Self {
            config,
            inner: Mutex::new(GroupCommitInner {
                current_group: CommitGroup::new(),
                epoch: 0,
                current_epoch_fsync_complete: false,
                current_epoch_error: None,
            }),
            fsync_complete: Condvar::new(),
        }
    }

    /// Check if group commit is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Submit a commit to the group.
    ///
    /// Returns the epoch this commit belongs to.
    /// The caller must:
    /// 1. Append the record to WAL
    /// 2. Call `mark_appended`
    /// 3. Call `perform_fsync_if_leader` or wait for leader
    /// 4. Call `wait_for_fsync`
    pub fn submit_commit(&self, commit: PendingCommit) -> (u64, usize) {
        let mut inner = self.inner.lock().unwrap();
        let epoch = inner.epoch;
        let index = inner.current_group.len();
        inner.current_group.add_commit(commit);
        (epoch, index)
    }

    /// Mark a commit as appended to WAL.
    pub fn mark_appended(&self, epoch: u64, index: usize, seq: u64) {
        let mut inner = self.inner.lock().unwrap();
        if inner.epoch == epoch {
            inner.current_group.mark_commit_appended(index, seq);
        }
    }

    /// Check if this commit should lead the fsync.
    ///
    /// Per GROUP_COMMIT.md §4.3: "If only one commit is present, behavior is identical to baseline."
    ///
    /// The leader is the commit that observes no other pending appends.
    /// This is a simple heuristic: if the group is ready to fsync, the caller is the leader.
    pub fn should_lead_fsync(&self, epoch: u64) -> bool {
        let inner = self.inner.lock().unwrap();
        inner.epoch == epoch && !inner.current_epoch_fsync_complete
    }

    /// Signal that fsync has completed for the current group.
    pub fn signal_fsync_complete(&self, epoch: u64) {
        let mut inner = self.inner.lock().unwrap();
        if inner.epoch == epoch {
            inner.current_group.mark_all_appended();
            inner.current_group.mark_fsync_complete();
            inner.current_epoch_fsync_complete = true;
            inner.epoch += 1;
            inner.current_group = CommitGroup::new();
            inner.current_epoch_fsync_complete = false;
        }
        self.fsync_complete.notify_all();
    }

    /// Signal that fsync failed.
    pub fn signal_fsync_failed(&self, epoch: u64, error: String) {
        let mut inner = self.inner.lock().unwrap();
        if inner.epoch == epoch {
            inner.current_group.mark_fsync_failed(error.clone());
            inner.current_epoch_error = Some(error);
            inner.epoch += 1;
            inner.current_group = CommitGroup::new();
            inner.current_epoch_fsync_complete = false;
        }
        self.fsync_complete.notify_all();
    }

    /// Wait for fsync to complete for a given epoch.
    ///
    /// Per GROUP_COMMIT.md §4.2: "No commit is acknowledged before fsync returns"
    pub fn wait_for_fsync(&self, commit_epoch: u64) -> WalResult<()> {
        let mut inner = self.inner.lock().unwrap();

        // If epoch has advanced, fsync already completed
        while inner.epoch == commit_epoch && !inner.current_epoch_fsync_complete {
            inner = self.fsync_complete.wait(inner).unwrap();
        }

        // Check for error
        if let Some(ref error) = inner.current_epoch_error {
            return Err(WalError::fsync_failed(
                format!("Group commit fsync failed: {}", error),
                std::io::Error::new(std::io::ErrorKind::Other, error.clone()),
            ));
        }

        Ok(())
    }

    /// Get current group size (for testing/observability).
    ///
    /// Per GROUP_COMMIT.md §10: Metrics are passive only.
    pub fn current_group_size(&self) -> usize {
        let inner = self.inner.lock().unwrap();
        inner.current_group.len()
    }
}

/// Check whether a write path should use baseline or group commit.
///
/// Per GROUP_COMMIT.md §9.1:
/// - When disabled, each commit performs its own fsync
/// - No shared fsync paths exist
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitPath {
    /// Baseline: one fsync per commit
    Baseline,
    /// Group commit: share fsync with concurrent commits
    GroupCommit,
}

impl CommitPath {
    /// Determine commit path based on config.
    pub fn from_config(config: &GroupCommitConfig) -> Self {
        if config.enabled {
            Self::GroupCommit
        } else {
            Self::Baseline
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_payload() -> WalPayload {
        WalPayload::new(
            "test_collection",
            "test_doc",
            "test_schema",
            "v1",
            b"test data".to_vec(),
        )
    }

    // ==================== GroupCommitConfig Tests ====================

    #[test]
    fn test_config_default_disabled() {
        let config = GroupCommitConfig::default();
        assert!(!config.enabled);
    }

    #[test]
    fn test_config_enabled() {
        let config = GroupCommitConfig::enabled();
        assert!(config.enabled);
    }

    #[test]
    fn test_config_disabled() {
        let config = GroupCommitConfig::disabled();
        assert!(!config.enabled);
    }

    // ==================== PendingCommit Tests ====================

    #[test]
    fn test_pending_commit_new() {
        let commit = PendingCommit::new(RecordType::Insert, test_payload());
        assert_eq!(commit.state, PendingCommitState::Queued);
        assert!(commit.sequence_number.is_none());
    }

    #[test]
    fn test_pending_commit_transitions() {
        let mut commit = PendingCommit::new(RecordType::Insert, test_payload());

        // Queued -> Appended
        commit.mark_appended(42);
        assert_eq!(commit.state, PendingCommitState::Appended);
        assert_eq!(commit.sequence_number, Some(42));

        // Appended -> Durable
        commit.mark_durable();
        assert_eq!(commit.state, PendingCommitState::Durable);
    }

    #[test]
    fn test_pending_commit_failed() {
        let mut commit = PendingCommit::new(RecordType::Insert, test_payload());
        commit.mark_failed();
        assert_eq!(commit.state, PendingCommitState::Failed);
    }

    // ==================== CommitGroup Tests ====================

    #[test]
    fn test_commit_group_empty() {
        let group = CommitGroup::new();
        assert!(group.is_empty());
        assert_eq!(group.len(), 0);
    }

    #[test]
    fn test_commit_group_add_commits() {
        let mut group = CommitGroup::new();
        group.add_commit(PendingCommit::new(RecordType::Insert, test_payload()));
        group.add_commit(PendingCommit::new(RecordType::Update, test_payload()));

        assert!(!group.is_empty());
        assert_eq!(group.len(), 2);
    }

    #[test]
    fn test_commit_group_fsync_complete() {
        let mut group = CommitGroup::new();
        group.add_commit(PendingCommit::new(RecordType::Insert, test_payload()));

        group.mark_commit_appended(0, 1);
        group.mark_all_appended();
        group.mark_fsync_complete();

        assert!(group.is_fsync_complete());
        assert!(group.fsync_error().is_none());

        let seqs = group.durable_sequence_numbers();
        assert_eq!(seqs, vec![1]);
    }

    #[test]
    fn test_commit_group_fsync_failed() {
        let mut group = CommitGroup::new();
        group.add_commit(PendingCommit::new(RecordType::Insert, test_payload()));

        group.mark_commit_appended(0, 1);
        group.mark_fsync_failed("disk error".to_string());

        assert!(!group.is_fsync_complete());
        assert_eq!(group.fsync_error(), Some("disk error"));

        let seqs = group.durable_sequence_numbers();
        assert!(seqs.is_empty()); // Failed commits are not durable
    }

    // ==================== GroupCommitManager Tests ====================

    #[test]
    fn test_manager_disabled() {
        let manager = GroupCommitManager::new(GroupCommitConfig::disabled());
        assert!(!manager.is_enabled());
    }

    #[test]
    fn test_manager_enabled() {
        let manager = GroupCommitManager::new(GroupCommitConfig::enabled());
        assert!(manager.is_enabled());
    }

    #[test]
    fn test_manager_submit_commit() {
        let manager = GroupCommitManager::new(GroupCommitConfig::enabled());
        let commit = PendingCommit::new(RecordType::Insert, test_payload());

        let (epoch, index) = manager.submit_commit(commit);
        assert_eq!(epoch, 0);
        assert_eq!(index, 0);
        assert_eq!(manager.current_group_size(), 1);
    }

    #[test]
    fn test_manager_multiple_commits() {
        let manager = GroupCommitManager::new(GroupCommitConfig::enabled());

        let (epoch1, idx1) =
            manager.submit_commit(PendingCommit::new(RecordType::Insert, test_payload()));
        let (epoch2, idx2) =
            manager.submit_commit(PendingCommit::new(RecordType::Update, test_payload()));
        let (epoch3, idx3) =
            manager.submit_commit(PendingCommit::new(RecordType::Delete, test_payload()));

        assert_eq!(epoch1, epoch2);
        assert_eq!(epoch2, epoch3);
        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 2);
        assert_eq!(manager.current_group_size(), 3);
    }

    #[test]
    fn test_manager_fsync_complete_advances_epoch() {
        let manager = GroupCommitManager::new(GroupCommitConfig::enabled());

        let (epoch1, _) =
            manager.submit_commit(PendingCommit::new(RecordType::Insert, test_payload()));
        manager.signal_fsync_complete(epoch1);

        let (epoch2, _) =
            manager.submit_commit(PendingCommit::new(RecordType::Insert, test_payload()));
        assert_eq!(epoch2, epoch1 + 1);
    }

    #[test]
    fn test_manager_wait_for_fsync() {
        let manager = Arc::new(GroupCommitManager::new(GroupCommitConfig::enabled()));

        let (epoch, _) =
            manager.submit_commit(PendingCommit::new(RecordType::Insert, test_payload()));

        // Signal completion before waiting
        manager.signal_fsync_complete(epoch);

        // Wait should return immediately
        let result = manager.wait_for_fsync(epoch);
        assert!(result.is_ok());
    }

    // ==================== CommitPath Tests ====================

    #[test]
    fn test_commit_path_baseline() {
        let config = GroupCommitConfig::disabled();
        assert_eq!(CommitPath::from_config(&config), CommitPath::Baseline);
    }

    #[test]
    fn test_commit_path_group_commit() {
        let config = GroupCommitConfig::enabled();
        assert_eq!(CommitPath::from_config(&config), CommitPath::GroupCommit);
    }

    // ==================== Equivalence Tests ====================

    /// Per GROUP_COMMIT.md §6:
    /// "If only one commit is present, behavior is identical to baseline."
    #[test]
    fn test_single_commit_equivalent_to_baseline() {
        let manager = GroupCommitManager::new(GroupCommitConfig::enabled());

        // Single commit should have same behavior
        let (epoch, index) =
            manager.submit_commit(PendingCommit::new(RecordType::Insert, test_payload()));
        manager.mark_appended(epoch, index, 1);

        // Leader should be true for single commit
        assert!(manager.should_lead_fsync(epoch));

        manager.signal_fsync_complete(epoch);

        let result = manager.wait_for_fsync(epoch);
        assert!(result.is_ok());
    }

    /// Per GROUP_COMMIT.md §7: Crash equivalence
    #[test]
    fn test_crash_before_append_equivalent() {
        // If crash happens before any append, commit is lost in both paths
        // This test validates state machine allows this scenario
        let commit = PendingCommit::new(RecordType::Insert, test_payload());
        assert_eq!(commit.state, PendingCommitState::Queued);
        // No further action - crash here means commit lost
    }

    /// Per GROUP_COMMIT.md §7: After append, before fsync
    #[test]
    fn test_crash_after_append_before_fsync_equivalent() {
        let mut group = CommitGroup::new();
        group.add_commit(PendingCommit::new(RecordType::Insert, test_payload()));
        group.mark_commit_appended(0, 1);
        group.mark_all_appended();
        // Crash before fsync - commit not durable
        assert!(!group.is_fsync_complete());
        // On recovery, this record would be dropped because no fsync
    }
}
