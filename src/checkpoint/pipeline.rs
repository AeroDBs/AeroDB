//! Checkpoint Pipelining Optimization
//!
//! Per CHECKPOINT_PIPELINING.md:
//! - Overlapping non-authoritative preparation work with normal operation
//! - Deferring all authoritative durability decisions to baseline ordering
//!
//! Per §4.2 Pipelined Checkpoint Path:
//! - Phase A (Preparation): Pipeline-eligible, tentative artifacts
//! - Phase B (Authority): Non-pipelined, strictly ordered
//!
//! Per §9.1 Disablement:
//! - Disableable via compile-time flag or startup config
//! - Disablement restores fully sequential checkpoint behavior

/// Configuration for checkpoint pipelining.
///
/// Per CHECKPOINT_PIPELINING.md §9.1:
/// - Disableable via compile-time flag or startup config
/// - Disablement restores fully sequential checkpoint behavior
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Whether checkpoint pipelining is enabled.
    pub enabled: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self { enabled: false } // Conservative default
    }
}

impl PipelineConfig {
    /// Create config with pipelining enabled.
    pub fn enabled() -> Self {
        Self { enabled: true }
    }

    /// Create config with pipelining disabled (baseline).
    pub fn disabled() -> Self {
        Self::default()
    }
}

/// Phase A: Preparation work that is pipeline-eligible.
///
/// Per CHECKPOINT_PIPELINING.md §4.2:
/// - Snapshot CommitId selection
/// - Snapshot visibility freeze
/// - Snapshot enumeration
/// - Snapshot file writes (not yet authoritative)
///
/// Per §4.3: Phase A work MUST be restart-discardable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhaseA {
    /// Select a CommitId for the checkpoint.
    SelectCommitId,
    /// Freeze snapshot visibility at the CommitId.
    FreezeVisibility,
    /// Enumerate persistent state for snapshot.
    EnumerateState,
    /// Write tentative snapshot files.
    WriteTentativeSnapshot,
}

impl PhaseA {
    /// All Phase A steps in order.
    pub fn steps() -> &'static [PhaseA] {
        &[
            PhaseA::SelectCommitId,
            PhaseA::FreezeVisibility,
            PhaseA::EnumerateState,
            PhaseA::WriteTentativeSnapshot,
        ]
    }

    /// Check if this step can overlap with normal operations.
    ///
    /// Per §4.2: All Phase A steps may overlap with normal reads/writes.
    pub fn can_overlap(&self) -> bool {
        true // All Phase A steps are pipelineable
    }

    /// Check if this step produces restart-discardable artifacts.
    ///
    /// Per §4.3: Phase A work MUST be restart-discardable.
    pub fn is_restart_discardable(&self) -> bool {
        true
    }
}

/// Phase B: Authority steps that are non-pipelined.
///
/// Per CHECKPOINT_PIPELINING.md §4.2:
/// - Snapshot fsync
/// - Checkpoint marker write
/// - Checkpoint marker fsync
/// - WAL truncation
///
/// Per §4.3: Phase B work MUST preserve baseline ordering exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhaseB {
    /// fsync the snapshot files.
    SnapshotFsync,
    /// Write the checkpoint marker.
    WriteMarker,
    /// fsync the checkpoint marker.
    MarkerFsync,
    /// Truncate the WAL.
    WalTruncation,
}

impl PhaseB {
    /// All Phase B steps in order.
    pub fn steps() -> &'static [PhaseB] {
        &[
            PhaseB::SnapshotFsync,
            PhaseB::WriteMarker,
            PhaseB::MarkerFsync,
            PhaseB::WalTruncation,
        ]
    }

    /// Check if this step can overlap with normal operations.
    ///
    /// Per §4.2: Phase B steps are strictly ordered.
    pub fn can_overlap(&self) -> bool {
        false // No Phase B step may overlap
    }

    /// Check if this step has recovery authority.
    ///
    /// Per §4.2: Phase B steps define durability and recovery authority.
    pub fn has_recovery_authority(&self) -> bool {
        true
    }
}

/// State of a pipelined checkpoint.
///
/// Per CHECKPOINT_PIPELINING.md §4.3:
/// - No read or write may observe Phase-A artifacts as authoritative
/// - No recovery logic may consult Phase-A artifacts
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineState {
    /// No checkpoint in progress.
    Idle,
    /// Executing Phase A (preparation).
    PhaseA {
        /// Current step in Phase A.
        step: usize,
        /// CommitId selected for this checkpoint.
        commit_id: u64,
    },
    /// Phase A complete, transitioning to Phase B.
    Transitioning {
        /// CommitId for this checkpoint.
        commit_id: u64,
    },
    /// Executing Phase B (authority).
    PhaseB {
        /// Current step in Phase B.
        step: usize,
        /// CommitId for this checkpoint.
        commit_id: u64,
    },
    /// Checkpoint complete.
    Complete {
        /// CommitId of the completed checkpoint.
        commit_id: u64,
    },
    /// Checkpoint failed/aborted.
    Aborted {
        /// Reason for abort.
        reason: String,
    },
}

impl PipelineState {
    /// Check if this state is in Phase A.
    pub fn is_phase_a(&self) -> bool {
        matches!(self, PipelineState::PhaseA { .. })
    }

    /// Check if this state is in Phase B.
    pub fn is_phase_b(&self) -> bool {
        matches!(self, PipelineState::PhaseB { .. })
    }

    /// Check if a checkpoint is in progress.
    pub fn is_in_progress(&self) -> bool {
        matches!(
            self,
            PipelineState::PhaseA { .. }
                | PipelineState::Transitioning { .. }
                | PipelineState::PhaseB { .. }
        )
    }

    /// Get the CommitId if one is set.
    pub fn commit_id(&self) -> Option<u64> {
        match self {
            PipelineState::PhaseA { commit_id, .. }
            | PipelineState::Transitioning { commit_id }
            | PipelineState::PhaseB { commit_id, .. }
            | PipelineState::Complete { commit_id } => Some(*commit_id),
            _ => None,
        }
    }
}

/// Result of a Phase A step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseAResult {
    /// The step that completed.
    pub step: PhaseA,
    /// Whether the step succeeded.
    pub success: bool,
    /// Optional artifact path (for tentative snapshot).
    pub artifact_path: Option<String>,
}

/// Result of a Phase B step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseBResult {
    /// The step that completed.
    pub step: PhaseB,
    /// Whether the step succeeded.
    pub success: bool,
}

/// Checkpoint pipeline manager.
///
/// Per CHECKPOINT_PIPELINING.md §4.2:
/// - Manages pipelined checkpoint execution
/// - Ensures Phase B ordering is never violated
#[derive(Debug)]
pub struct CheckpointPipeline {
    /// Configuration.
    config: PipelineConfig,
    /// Current pipeline state.
    state: PipelineState,
    /// Statistics for observability.
    stats: PipelineStats,
}

/// Statistics for checkpoint pipelining.
///
/// Per CHECKPOINT_PIPELINING.md §10:
/// - Metrics are passive only
/// - Metrics MUST NOT influence scheduling
#[derive(Debug, Default, Clone)]
pub struct PipelineStats {
    /// Number of checkpoints started.
    pub checkpoints_started: u64,
    /// Number of checkpoints completed.
    pub checkpoints_completed: u64,
    /// Number of checkpoints aborted.
    pub checkpoints_aborted: u64,
    /// Total Phase A duration (simulated).
    pub phase_a_steps_completed: u64,
    /// Total Phase B duration (simulated).
    pub phase_b_steps_completed: u64,
}

impl CheckpointPipeline {
    /// Create a new checkpoint pipeline.
    pub fn new(config: PipelineConfig) -> Self {
        Self {
            config,
            state: PipelineState::Idle,
            stats: PipelineStats::default(),
        }
    }

    /// Check if pipelining is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the current state.
    pub fn state(&self) -> &PipelineState {
        &self.state
    }

    /// Start a new checkpoint at the given CommitId.
    ///
    /// Returns error if a checkpoint is already in progress.
    pub fn start(&mut self, commit_id: u64) -> Result<(), CheckpointPipelineError> {
        if self.state.is_in_progress() {
            return Err(CheckpointPipelineError::CheckpointInProgress);
        }

        self.state = PipelineState::PhaseA { step: 0, commit_id };
        self.stats.checkpoints_started += 1;
        Ok(())
    }

    /// Advance Phase A to the next step.
    pub fn advance_phase_a(&mut self) -> Result<PhaseAResult, CheckpointPipelineError> {
        match &self.state {
            PipelineState::PhaseA { step, commit_id } => {
                let steps = PhaseA::steps();
                if *step >= steps.len() {
                    // Transition to Phase B
                    self.state = PipelineState::Transitioning {
                        commit_id: *commit_id,
                    };
                    return Err(CheckpointPipelineError::PhaseAComplete);
                }

                let current_step = steps[*step].clone();
                self.stats.phase_a_steps_completed += 1;

                // Advance to next step
                self.state = PipelineState::PhaseA {
                    step: step + 1,
                    commit_id: *commit_id,
                };

                Ok(PhaseAResult {
                    step: current_step,
                    success: true,
                    artifact_path: None,
                })
            }
            _ => Err(CheckpointPipelineError::NotInPhaseA),
        }
    }

    /// Begin Phase B execution.
    ///
    /// Per §4.3: Phase B work MUST preserve baseline ordering exactly.
    pub fn begin_phase_b(&mut self) -> Result<(), CheckpointPipelineError> {
        match &self.state {
            PipelineState::Transitioning { commit_id } => {
                self.state = PipelineState::PhaseB {
                    step: 0,
                    commit_id: *commit_id,
                };
                Ok(())
            }
            PipelineState::PhaseA { step, commit_id } => {
                // Complete remaining Phase A steps first
                let steps = PhaseA::steps();
                if *step < steps.len() {
                    return Err(CheckpointPipelineError::PhaseAIncomplete);
                }
                self.state = PipelineState::PhaseB {
                    step: 0,
                    commit_id: *commit_id,
                };
                Ok(())
            }
            _ => Err(CheckpointPipelineError::InvalidStateTransition),
        }
    }

    /// Advance Phase B to the next step.
    ///
    /// Per §4.3: Phase B steps are strictly ordered.
    pub fn advance_phase_b(&mut self) -> Result<PhaseBResult, CheckpointPipelineError> {
        match &self.state {
            PipelineState::PhaseB { step, commit_id } => {
                let steps = PhaseB::steps();
                if *step >= steps.len() {
                    // Checkpoint complete
                    self.state = PipelineState::Complete {
                        commit_id: *commit_id,
                    };
                    self.stats.checkpoints_completed += 1;
                    return Err(CheckpointPipelineError::PhaseBComplete);
                }

                let current_step = steps[*step].clone();
                self.stats.phase_b_steps_completed += 1;

                // Advance to next step
                self.state = PipelineState::PhaseB {
                    step: step + 1,
                    commit_id: *commit_id,
                };

                Ok(PhaseBResult {
                    step: current_step,
                    success: true,
                })
            }
            _ => Err(CheckpointPipelineError::NotInPhaseB),
        }
    }

    /// Abort the current checkpoint.
    ///
    /// Per §7.1: Crash during Phase A results in no authoritative checkpoint.
    pub fn abort(&mut self, reason: impl Into<String>) {
        if self.state.is_in_progress() {
            self.stats.checkpoints_aborted += 1;
        }
        self.state = PipelineState::Aborted {
            reason: reason.into(),
        };
    }

    /// Reset to idle state (after abort or completion).
    pub fn reset(&mut self) {
        self.state = PipelineState::Idle;
    }

    /// Get statistics.
    pub fn stats(&self) -> &PipelineStats {
        &self.stats
    }
}

/// Errors for checkpoint pipeline operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckpointPipelineError {
    /// A checkpoint is already in progress.
    CheckpointInProgress,
    /// Not currently in Phase A.
    NotInPhaseA,
    /// Not currently in Phase B.
    NotInPhaseB,
    /// Phase A is not complete.
    PhaseAIncomplete,
    /// Phase A has completed, transition to Phase B.
    PhaseAComplete,
    /// Phase B has completed, checkpoint done.
    PhaseBComplete,
    /// Invalid state transition.
    InvalidStateTransition,
}

/// Pipeline path selection.
///
/// Per CHECKPOINT_PIPELINING.md §9.1:
/// - Disablement restores fully sequential checkpoint behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointPath {
    /// Baseline: fully sequential.
    Sequential,
    /// Pipelined: Phase A overlapped.
    Pipelined,
}

impl CheckpointPath {
    /// Determine checkpoint path based on config.
    pub fn from_config(config: &PipelineConfig) -> Self {
        if config.enabled {
            Self::Pipelined
        } else {
            Self::Sequential
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== PipelineConfig Tests ====================

    #[test]
    fn test_config_default() {
        let config = PipelineConfig::default();
        assert!(!config.enabled);
    }

    #[test]
    fn test_config_enabled() {
        let config = PipelineConfig::enabled();
        assert!(config.enabled);
    }

    // ==================== PhaseA Tests ====================

    #[test]
    fn test_phase_a_steps() {
        let steps = PhaseA::steps();
        assert_eq!(steps.len(), 4);
        assert_eq!(steps[0], PhaseA::SelectCommitId);
        assert_eq!(steps[3], PhaseA::WriteTentativeSnapshot);
    }

    #[test]
    fn test_phase_a_can_overlap() {
        for step in PhaseA::steps() {
            assert!(step.can_overlap());
        }
    }

    #[test]
    fn test_phase_a_restart_discardable() {
        for step in PhaseA::steps() {
            assert!(step.is_restart_discardable());
        }
    }

    // ==================== PhaseB Tests ====================

    #[test]
    fn test_phase_b_steps() {
        let steps = PhaseB::steps();
        assert_eq!(steps.len(), 4);
        assert_eq!(steps[0], PhaseB::SnapshotFsync);
        assert_eq!(steps[3], PhaseB::WalTruncation);
    }

    #[test]
    fn test_phase_b_cannot_overlap() {
        for step in PhaseB::steps() {
            assert!(!step.can_overlap());
        }
    }

    #[test]
    fn test_phase_b_has_recovery_authority() {
        for step in PhaseB::steps() {
            assert!(step.has_recovery_authority());
        }
    }

    // ==================== PipelineState Tests ====================

    #[test]
    fn test_state_idle() {
        let state = PipelineState::Idle;
        assert!(!state.is_in_progress());
        assert!(state.commit_id().is_none());
    }

    #[test]
    fn test_state_phase_a() {
        let state = PipelineState::PhaseA {
            step: 0,
            commit_id: 100,
        };
        assert!(state.is_phase_a());
        assert!(state.is_in_progress());
        assert_eq!(state.commit_id(), Some(100));
    }

    #[test]
    fn test_state_phase_b() {
        let state = PipelineState::PhaseB {
            step: 0,
            commit_id: 100,
        };
        assert!(state.is_phase_b());
        assert!(state.is_in_progress());
    }

    // ==================== CheckpointPipeline Tests ====================

    #[test]
    fn test_pipeline_disabled() {
        let pipeline = CheckpointPipeline::new(PipelineConfig::disabled());
        assert!(!pipeline.is_enabled());
    }

    #[test]
    fn test_pipeline_start() {
        let mut pipeline = CheckpointPipeline::new(PipelineConfig::enabled());
        let result = pipeline.start(100);
        assert!(result.is_ok());
        assert!(pipeline.state().is_phase_a());
    }

    #[test]
    fn test_pipeline_double_start() {
        let mut pipeline = CheckpointPipeline::new(PipelineConfig::enabled());
        pipeline.start(100).unwrap();
        let result = pipeline.start(200);
        assert_eq!(result, Err(CheckpointPipelineError::CheckpointInProgress));
    }

    #[test]
    fn test_pipeline_phase_a_advance() {
        let mut pipeline = CheckpointPipeline::new(PipelineConfig::enabled());
        pipeline.start(100).unwrap();

        // Advance through all Phase A steps
        for _ in PhaseA::steps() {
            let result = pipeline.advance_phase_a();
            assert!(result.is_ok());
        }

        // Next advance should indicate completion
        let result = pipeline.advance_phase_a();
        assert_eq!(result, Err(CheckpointPipelineError::PhaseAComplete));
    }

    #[test]
    fn test_pipeline_transition_to_phase_b() {
        let mut pipeline = CheckpointPipeline::new(PipelineConfig::enabled());
        pipeline.start(100).unwrap();

        // Complete Phase A
        for _ in PhaseA::steps() {
            pipeline.advance_phase_a().ok();
        }

        // Begin Phase B
        let result = pipeline.begin_phase_b();
        assert!(result.is_ok());
        assert!(pipeline.state().is_phase_b());
    }

    #[test]
    fn test_pipeline_phase_b_advance() {
        let mut pipeline = CheckpointPipeline::new(PipelineConfig::enabled());
        pipeline.start(100).unwrap();

        // Complete Phase A
        for _ in PhaseA::steps() {
            pipeline.advance_phase_a().ok();
        }
        pipeline.begin_phase_b().unwrap();

        // Advance through all Phase B steps
        for _ in PhaseB::steps() {
            let result = pipeline.advance_phase_b();
            assert!(result.is_ok());
        }

        // Next advance should indicate completion
        let result = pipeline.advance_phase_b();
        assert_eq!(result, Err(CheckpointPipelineError::PhaseBComplete));
        assert_eq!(pipeline.state().commit_id(), Some(100));
    }

    #[test]
    fn test_pipeline_abort() {
        let mut pipeline = CheckpointPipeline::new(PipelineConfig::enabled());
        pipeline.start(100).unwrap();
        pipeline.abort("test abort");

        match pipeline.state() {
            PipelineState::Aborted { reason } => assert_eq!(reason, "test abort"),
            _ => panic!("Expected Aborted state"),
        }
    }

    #[test]
    fn test_pipeline_stats() {
        let mut pipeline = CheckpointPipeline::new(PipelineConfig::enabled());
        pipeline.start(100).unwrap();

        for _ in PhaseA::steps() {
            pipeline.advance_phase_a().ok();
        }

        assert_eq!(pipeline.stats().checkpoints_started, 1);
        assert_eq!(pipeline.stats().phase_a_steps_completed, 4);
    }

    // ==================== CheckpointPath Tests ====================

    #[test]
    fn test_path_sequential() {
        let config = PipelineConfig::disabled();
        assert_eq!(
            CheckpointPath::from_config(&config),
            CheckpointPath::Sequential
        );
    }

    #[test]
    fn test_path_pipelined() {
        let config = PipelineConfig::enabled();
        assert_eq!(
            CheckpointPath::from_config(&config),
            CheckpointPath::Pipelined
        );
    }

    // ==================== Crash Equivalence Tests ====================

    /// Per CHECKPOINT_PIPELINING.md §7.1:
    /// "Crash during Phase A results in no authoritative checkpoint."
    #[test]
    fn test_crash_during_phase_a() {
        let mut pipeline = CheckpointPipeline::new(PipelineConfig::enabled());
        pipeline.start(100).unwrap();
        pipeline.advance_phase_a().ok();

        // Simulate crash by aborting
        pipeline.abort("simulated crash");

        // No authoritative checkpoint - state is aborted
        assert!(matches!(pipeline.state(), PipelineState::Aborted { .. }));
        assert_eq!(pipeline.stats().checkpoints_aborted, 1);
    }

    /// Per CHECKPOINT_PIPELINING.md §7.3:
    /// "fsync incomplete → snapshot not durable"
    #[test]
    fn test_crash_during_phase_b_fsync() {
        let mut pipeline = CheckpointPipeline::new(PipelineConfig::enabled());
        pipeline.start(100).unwrap();

        for _ in PhaseA::steps() {
            pipeline.advance_phase_a().ok();
        }
        pipeline.begin_phase_b().unwrap();

        // Execute first step (SnapshotFsync), then crash
        pipeline.advance_phase_b().ok();
        pipeline.abort("crash during phase B");

        // Checkpoint not complete
        assert!(matches!(pipeline.state(), PipelineState::Aborted { .. }));
    }
}
