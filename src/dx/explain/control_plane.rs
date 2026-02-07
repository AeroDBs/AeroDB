//! Phase 7 Control Plane Explanations
//!
//! Per PHASE7_OBSERVABILITY_MODEL.md:
//! - Passive views of system state
//! - Pre-execution explanations (what WILL happen)
//! - Post-execution explanations (what DID happen)
//!
//! Per PHASE7_CONFIRMATION_MODEL.md §5:
//! - System generates pre-execution explanation before confirmation
//! - Operator must understand consequences before confirming
//!
//! Explanations are read-only and have no semantic authority.
//!
//! Note: Phase 7 explanations are standalone types distinct from
//! the Phase 4 Explanation model. They serve different purposes:
//! - Phase 4: Query execution and visibility explanations
//! - Phase 7: Operator control plane action explanations

use std::time::SystemTime;
use uuid::Uuid;

/// Pre-execution explanation for a control plane command.
///
/// Per PHASE7_CONFIRMATION_MODEL.md §5.2:
/// Pre-execution explanation must describe what WILL happen.
#[derive(Debug, Clone)]
pub struct PreExecutionExplanation {
    /// Command being explained.
    pub command_name: String,

    /// Target node/replica (if applicable).
    pub target_id: Option<Uuid>,

    /// What will happen if the command is confirmed.
    pub consequences: Vec<Consequence>,

    /// Invariants that may be affected.
    pub affected_invariants: Vec<InvariantImpact>,

    /// Is this a destructive/irreversible operation?
    pub is_irreversible: bool,

    /// Risks the operator should be aware of.
    pub risks: Vec<String>,

    /// Explanation timestamp.
    pub generated_at: SystemTime,
}

impl PreExecutionExplanation {
    /// Create a pre-execution explanation for a command.
    pub fn new(command_name: impl Into<String>) -> Self {
        Self {
            command_name: command_name.into(),
            target_id: None,
            consequences: Vec::new(),
            affected_invariants: Vec::new(),
            is_irreversible: false,
            risks: Vec::new(),
            generated_at: SystemTime::now(),
        }
    }

    /// Set target ID.
    pub fn with_target(mut self, id: Uuid) -> Self {
        self.target_id = Some(id);
        self
    }

    /// Add a consequence.
    pub fn with_consequence(mut self, consequence: Consequence) -> Self {
        self.consequences.push(consequence);
        self
    }

    /// Add an affected invariant.
    pub fn with_invariant_impact(mut self, impact: InvariantImpact) -> Self {
        self.affected_invariants.push(impact);
        self
    }

    /// Mark as irreversible.
    pub fn irreversible(mut self) -> Self {
        self.is_irreversible = true;
        self
    }

    /// Add a risk.
    pub fn with_risk(mut self, risk: impl Into<String>) -> Self {
        self.risks.push(risk.into());
        self
    }

    /// Generate pre-execution explanation for request_promotion.
    pub fn for_request_promotion(replica_id: Uuid) -> Self {
        Self::new("request_promotion")
            .with_target(replica_id)
            .with_consequence(Consequence {
                description: "Node will transition from Replica to Primary role".to_string(),
                scope: ConsequenceScope::Node,
            })
            .with_consequence(Consequence {
                description: "Current primary will be demoted (if any)".to_string(),
                scope: ConsequenceScope::Cluster,
            })
            .with_invariant_impact(InvariantImpact {
                invariant_id: "P6-A1".to_string(),
                description: "Single primary invariant will be re-established".to_string(),
                impact: ImpactLevel::Enforced,
            })
            .with_risk("If current primary is still active, split-brain may occur")
    }

    /// Generate pre-execution explanation for force_promotion.
    pub fn for_force_promotion(replica_id: Uuid, acknowledged_risks: &[String]) -> Self {
        let mut exp = Self::new("force_promotion")
            .with_target(replica_id)
            .with_consequence(Consequence {
                description: "Node will be forced to Primary role, bypassing safety checks"
                    .to_string(),
                scope: ConsequenceScope::Node,
            })
            .irreversible()
            .with_invariant_impact(InvariantImpact {
                invariant_id: "P6-A1".to_string(),
                description: "Single primary invariant may be VIOLATED".to_string(),
                impact: ImpactLevel::MayViolate,
            })
            .with_risk("Current primary may still be active, causing split-brain")
            .with_risk("Data loss may occur if promotion proceeds without proper validation");

        for risk in acknowledged_risks {
            exp = exp.with_risk(format!("ACKNOWLEDGED: {}", risk));
        }

        exp
    }
}

/// A consequence of executing a command.
#[derive(Debug, Clone)]
pub struct Consequence {
    /// Human-readable description of the consequence.
    pub description: String,

    /// Scope of the consequence.
    pub scope: ConsequenceScope,
}

/// Scope of a consequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsequenceScope {
    /// Affects a single node.
    Node,

    /// Affects the entire cluster.
    Cluster,

    /// Affects replication.
    Replication,

    /// Affects data durability.
    Durability,
}

/// Impact on an invariant.
#[derive(Debug, Clone)]
pub struct InvariantImpact {
    /// Invariant identifier (e.g., "P6-A1").
    pub invariant_id: String,

    /// Description of the impact.
    pub description: String,

    /// Level of impact.
    pub impact: ImpactLevel,
}

/// Level of impact on an invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImpactLevel {
    /// Invariant will be enforced.
    Enforced,

    /// Invariant may be affected.
    MayAffect,

    /// Invariant may be violated.
    MayViolate,

    /// Invariant will be explicitly overridden.
    Override,
}

/// Post-execution explanation for what happened.
#[derive(Debug, Clone)]
pub struct PostExecutionExplanation {
    /// Command that was executed.
    pub command_name: String,

    /// Request ID.
    pub request_id: Uuid,

    /// Whether execution succeeded.
    pub success: bool,

    /// What actually happened.
    pub outcome_description: String,

    /// State changes that occurred.
    pub state_changes: Vec<StateChange>,

    /// Explanation timestamp.
    pub generated_at: SystemTime,
}

impl PostExecutionExplanation {
    /// Create a new post-execution explanation.
    pub fn new(command_name: impl Into<String>, request_id: Uuid, success: bool) -> Self {
        Self {
            command_name: command_name.into(),
            request_id,
            success,
            outcome_description: String::new(),
            state_changes: Vec::new(),
            generated_at: SystemTime::now(),
        }
    }

    /// Set outcome description.
    pub fn with_outcome(mut self, description: impl Into<String>) -> Self {
        self.outcome_description = description.into();
        self
    }

    /// Add a state change.
    pub fn with_state_change(mut self, change: StateChange) -> Self {
        self.state_changes.push(change);
        self
    }
}

/// A state change that occurred.
#[derive(Debug, Clone)]
pub struct StateChange {
    /// Component that changed.
    pub component: String,

    /// Previous value/state.
    pub before: String,

    /// New value/state.
    pub after: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pre_execution_explanation() {
        let replica_id = Uuid::new_v4();
        let exp = PreExecutionExplanation::for_request_promotion(replica_id);

        assert_eq!(exp.command_name, "request_promotion");
        assert_eq!(exp.target_id, Some(replica_id));
        assert!(!exp.is_irreversible);
        assert!(!exp.consequences.is_empty());
    }

    #[test]
    fn test_force_promotion_is_irreversible() {
        let exp = PreExecutionExplanation::for_force_promotion(
            Uuid::new_v4(),
            &["I understand the risks".to_string()],
        );

        assert!(exp.is_irreversible);
        assert!(exp.risks.iter().any(|r| r.contains("split-brain")));
    }

    #[test]
    fn test_post_execution_explanation() {
        let exp = PostExecutionExplanation::new("request_promotion", Uuid::new_v4(), true)
            .with_outcome("Promotion completed successfully")
            .with_state_change(StateChange {
                component: "node_role".to_string(),
                before: "Replica".to_string(),
                after: "Primary".to_string(),
            });

        assert!(exp.success);
        assert!(!exp.state_changes.is_empty());
    }
}
