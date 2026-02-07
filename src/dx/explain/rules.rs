//! Rule Registry
//!
//! Per DX_EXPLANATION_MODEL.md §5.1:
//! Every rule referenced in explanations MUST:
//! - Have a stable identifier
//! - Map to INVARIANTS.md, PHASE*_INVARIANTS.md, MVCC rules, etc.
//!
//! Read-only, Phase 4, no semantic authority.

use std::collections::HashMap;

/// A registered rule definition.
#[derive(Debug, Clone)]
pub struct RuleDefinition {
    /// Stable rule identifier.
    pub rule_id: String,
    /// Source document.
    pub source_doc: String,
    /// Full description.
    pub description: String,
}

impl RuleDefinition {
    /// Create a new rule definition.
    pub fn new(
        rule_id: impl Into<String>,
        source_doc: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            source_doc: source_doc.into(),
            description: description.into(),
        }
    }
}

/// Registry of all documented rules.
///
/// Per DX_EXPLANATION_MODEL.md §5.1:
/// Free-form rules are forbidden.
#[derive(Debug, Default)]
pub struct RuleRegistry {
    rules: HashMap<String, RuleDefinition>,
}

impl RuleRegistry {
    /// Create a new registry with all known rules.
    pub fn new() -> Self {
        let mut registry = Self::default();
        registry.register_core_invariants();
        registry.register_mvcc_rules();
        registry.register_replication_rules();
        registry.register_phase3_rules();
        registry.register_phase4_rules();
        registry
    }

    /// Register a rule.
    pub fn register(&mut self, rule: RuleDefinition) {
        self.rules.insert(rule.rule_id.clone(), rule);
    }

    /// Get a rule by ID.
    pub fn get(&self, rule_id: &str) -> Option<&RuleDefinition> {
        self.rules.get(rule_id)
    }

    /// Check if a rule exists.
    pub fn contains(&self, rule_id: &str) -> bool {
        self.rules.contains_key(rule_id)
    }

    /// Get rule description for use in explanations.
    pub fn description(&self, rule_id: &str) -> String {
        self.rules
            .get(rule_id)
            .map(|r| r.description.clone())
            .unwrap_or_else(|| format!("Unknown rule: {}", rule_id))
    }

    fn register_core_invariants(&mut self) {
        // From CORE_INVARIANTS.md (formerly INVARIANTS.md)
        self.register(RuleDefinition::new(
            "D-1",
            "CORE_INVARIANTS.md",
            "No Acknowledged Write Is Ever Lost",
        ));
        self.register(RuleDefinition::new(
            "D-2",
            "CORE_INVARIANTS.md",
            "Data Corruption Is Never Ignored",
        ));
        self.register(RuleDefinition::new(
            "D-3",
            "CORE_INVARIANTS.md",
            "Reads Never Observe Invalid State",
        ));
        self.register(RuleDefinition::new(
            "R-1",
            "CORE_INVARIANTS.md",
            "WAL Precedes Acknowledgement",
        ));
        self.register(RuleDefinition::new(
            "R-2",
            "CORE_INVARIANTS.md",
            "Recovery Is Deterministic",
        ));
        self.register(RuleDefinition::new(
            "R-3",
            "CORE_INVARIANTS.md",
            "Recovery Completeness Is Verifiable",
        ));
    }

    fn register_mvcc_rules(&mut self) {
        // From MVCC_MODEL.md and MVCC_VISIBILITY_RULES.md
        self.register(RuleDefinition::new(
            "MVCC-1",
            "MVCC_MODEL.md",
            "Snapshot Isolation: reads observe consistent point-in-time view",
        ));
        self.register(RuleDefinition::new(
            "MVCC-2",
            "MVCC_MODEL.md",
            "CommitId Authority: WAL is sole authority for CommitId assignment",
        ));
        self.register(RuleDefinition::new(
            "MVCC-3",
            "MVCC_MODEL.md",
            "Version Chain Integrity: version chains are immutable once committed",
        ));
        self.register(RuleDefinition::new(
            "MVCC-VIS-1",
            "MVCC_VISIBILITY_RULES.md",
            "Version is visible if CommitId <= snapshot CommitId",
        ));
        self.register(RuleDefinition::new(
            "MVCC-VIS-2",
            "MVCC_VISIBILITY_RULES.md",
            "Uncommitted versions are never visible to other transactions",
        ));
        self.register(RuleDefinition::new(
            "MVCC-GC-1",
            "MVCC_GC_MODEL.md",
            "GC may only remove versions older than oldest active snapshot",
        ));
    }

    fn register_replication_rules(&mut self) {
        // From REPL_INVARIANTS.md
        self.register(RuleDefinition::new(
            "REP-1",
            "REPL_INVARIANTS.md",
            "Single-Writer: only Primary may accept writes",
        ));
        self.register(RuleDefinition::new(
            "REP-2",
            "REPL_INVARIANTS.md",
            "WAL Prefix: Replica_WAL == Prefix(Primary_WAL)",
        ));
        self.register(RuleDefinition::new(
            "REP-3",
            "REPL_INVARIANTS.md",
            "Read Safety: replica reads bounded by durable WAL position",
        ));
    }

    fn register_phase3_rules(&mut self) {
        // From PERF_INVARIANTS.md
        self.register(RuleDefinition::new(
            "PERF-D-1",
            "PERF_INVARIANTS.md",
            "Acknowledged Write Durability: no delay or weakening allowed",
        ));
        self.register(RuleDefinition::new(
            "PERF-DET-1",
            "PERF_INVARIANTS.md",
            "Crash Determinism: identical inputs produce identical recovery",
        ));
        self.register(RuleDefinition::new(
            "PERF-DET-2",
            "PERF_INVARIANTS.md",
            "Replay Equivalence: optimized execution is replay-equivalent",
        ));
    }

    fn register_phase4_rules(&mut self) {
        // From DX_INVARIANTS.md
        self.register(RuleDefinition::new(
            "P4-1",
            "DX_INVARIANTS.md",
            "Zero Semantic Authority: Phase 4 cannot influence database state",
        ));
        self.register(RuleDefinition::new(
            "P4-2",
            "DX_INVARIANTS.md",
            "Strict Read-Only Surfaces: all interfaces must be read-only",
        ));
        self.register(RuleDefinition::new(
            "P4-6",
            "DX_INVARIANTS.md",
            "Deterministic Observation: identical state produces identical output",
        ));
        self.register(RuleDefinition::new(
            "P4-8",
            "DX_INVARIANTS.md",
            "No Heuristic Explanations: explanations must reflect real execution",
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = RuleRegistry::new();
        assert!(registry.contains("D-1"));
        assert!(registry.contains("MVCC-1"));
        assert!(registry.contains("REP-1"));
        assert!(registry.contains("P4-1"));
    }

    #[test]
    fn test_rule_lookup() {
        let registry = RuleRegistry::new();
        let rule = registry.get("MVCC-1").unwrap();
        assert_eq!(rule.source_doc, "MVCC_MODEL.md");
    }

    #[test]
    fn test_description() {
        let registry = RuleRegistry::new();
        let desc = registry.description("D-1");
        assert!(desc.contains("Acknowledged Write"));
    }

    #[test]
    fn test_unknown_rule() {
        let registry = RuleRegistry::new();
        assert!(!registry.contains("UNKNOWN-99"));
        let desc = registry.description("UNKNOWN-99");
        assert!(desc.contains("Unknown rule"));
    }
}
