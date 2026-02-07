//! Index Acceleration Optimization
//!
//! Per INDEX_ACCELERATION.md:
//! - Indexes remain derived, non-authoritative, rebuildable
//! - Indexes are correctness-neutral
//! - If an index can cause a wrong answer, it is invalid
//!
//! Per §4.1 Improved In-Memory Data Structures:
//! - Better hash distributions, balanced trees, cache-friendly layouts
//!
//! Per §4.3 Predicate Pre-Filtering:
//! - False positives allowed
//! - False negatives forbidden
//!
//! Per §10.1 Disablement:
//! - Disableable via compile-time flag or startup config
//!
//! Per §14: "If removing an index changes a query result, the index is incorrect."

use std::collections::{HashMap, HashSet};

/// Configuration for index acceleration.
///
/// Per INDEX_ACCELERATION.md §10.1:
/// - Disableable via compile-time flag or startup config
/// - Disablement restores baseline index structures
#[derive(Debug, Clone)]
pub struct IndexAccelConfig {
    /// Whether accelerated index structures are enabled.
    pub accelerated_structures_enabled: bool,
    /// Whether predicate pre-filtering is enabled.
    pub predicate_prefilter_enabled: bool,
    /// Whether multi-attribute indexes are enabled.
    pub multi_attribute_enabled: bool,
}

impl Default for IndexAccelConfig {
    fn default() -> Self {
        Self {
            accelerated_structures_enabled: false, // Conservative default
            predicate_prefilter_enabled: false,
            multi_attribute_enabled: false,
        }
    }
}

impl IndexAccelConfig {
    /// Create config with all accelerations enabled.
    pub fn enabled() -> Self {
        Self {
            accelerated_structures_enabled: true,
            predicate_prefilter_enabled: true,
            multi_attribute_enabled: true,
        }
    }

    /// Create config with all accelerations disabled (baseline).
    pub fn disabled() -> Self {
        Self::default()
    }
}

/// Result of predicate pre-filtering.
///
/// Per INDEX_ACCELERATION.md §4.3:
/// - Pre-filtering MUST be conservative
/// - False positives allowed
/// - False negatives forbidden
#[derive(Debug, Clone)]
pub struct PrefilterResult {
    /// Candidate document keys that MAY match the predicate.
    /// This is a SUPERSET of the true matches.
    pub candidates: HashSet<String>,
    /// Whether the filter was applied (false if index not available).
    pub filter_applied: bool,
    /// Statistics for observability.
    pub stats: PrefilterStats,
}

impl PrefilterResult {
    /// Create result indicating no prefilter was possible.
    pub fn no_filter() -> Self {
        Self {
            candidates: HashSet::new(),
            filter_applied: false,
            stats: PrefilterStats::default(),
        }
    }

    /// Create result with filtered candidates.
    pub fn filtered(candidates: HashSet<String>) -> Self {
        let count = candidates.len();
        Self {
            candidates,
            filter_applied: true,
            stats: PrefilterStats {
                candidates_returned: count,
                ..Default::default()
            },
        }
    }
}

/// Statistics for predicate pre-filtering.
///
/// Per INDEX_ACCELERATION.md §11:
/// - Metrics are passive only
/// - Metrics MUST NOT influence planner decisions
#[derive(Debug, Clone, Default)]
pub struct PrefilterStats {
    /// Number of candidate keys returned.
    pub candidates_returned: usize,
    /// Number of predicates evaluated.
    pub predicates_evaluated: usize,
    /// Whether the index was a hit.
    pub index_hit: bool,
}

/// An accelerated index entry for a single attribute.
///
/// Per INDEX_ACCELERATION.md §4.1:
/// - Index lookup returns a superset of valid candidates
/// - No valid candidate is omitted
#[derive(Debug)]
pub struct AttributeIndex {
    /// Attribute name.
    attribute: String,
    /// Value -> document keys mapping.
    /// Multiple documents can have the same value.
    value_to_keys: HashMap<String, HashSet<String>>,
    /// Total entries for statistics.
    entry_count: usize,
}

impl AttributeIndex {
    /// Create a new empty attribute index.
    pub fn new(attribute: impl Into<String>) -> Self {
        Self {
            attribute: attribute.into(),
            value_to_keys: HashMap::new(),
            entry_count: 0,
        }
    }

    /// Get the attribute name.
    pub fn attribute(&self) -> &str {
        &self.attribute
    }

    /// Insert a document into the index.
    pub fn insert(&mut self, value: impl Into<String>, doc_key: impl Into<String>) {
        let value = value.into();
        let doc_key = doc_key.into();
        self.value_to_keys.entry(value).or_default().insert(doc_key);
        self.entry_count += 1;
    }

    /// Remove a document from the index.
    pub fn remove(&mut self, value: &str, doc_key: &str) -> bool {
        if let Some(keys) = self.value_to_keys.get_mut(value) {
            let removed = keys.remove(doc_key);
            if removed {
                self.entry_count = self.entry_count.saturating_sub(1);
            }
            if keys.is_empty() {
                self.value_to_keys.remove(value);
            }
            removed
        } else {
            false
        }
    }

    /// Find all document keys with the exact value.
    ///
    /// Per INDEX_ACCELERATION.md §4.3:
    /// - False positives allowed (but we return exact matches here)
    /// - False negatives forbidden
    pub fn find_exact(&self, value: &str) -> HashSet<String> {
        self.value_to_keys.get(value).cloned().unwrap_or_default()
    }

    /// Find all document keys with values in a set.
    pub fn find_in(&self, values: &[String]) -> HashSet<String> {
        let mut result = HashSet::new();
        for value in values {
            if let Some(keys) = self.value_to_keys.get(value) {
                result.extend(keys.iter().cloned());
            }
        }
        result
    }

    /// Get the number of indexed entries.
    pub fn len(&self) -> usize {
        self.entry_count
    }

    /// Check if index is empty.
    pub fn is_empty(&self) -> bool {
        self.entry_count == 0
    }

    /// Clear the index.
    pub fn clear(&mut self) {
        self.value_to_keys.clear();
        self.entry_count = 0;
    }
}

/// Composite (multi-attribute) index.
///
/// Per INDEX_ACCELERATION.md §4.2:
/// - Indexes MUST be derivable entirely from stored documents
/// - Index build order MUST be deterministic
#[derive(Debug)]
pub struct CompositeIndex {
    /// Attribute names in order.
    attributes: Vec<String>,
    /// Composite key -> document keys mapping.
    /// Composite key is attributes joined with \0.
    composite_to_keys: HashMap<String, HashSet<String>>,
}

impl CompositeIndex {
    /// Create a new composite index for the given attributes.
    pub fn new(attributes: Vec<String>) -> Self {
        Self {
            attributes,
            composite_to_keys: HashMap::new(),
        }
    }

    /// Get the attribute names.
    pub fn attributes(&self) -> &[String] {
        &self.attributes
    }

    /// Build a composite key from attribute values.
    fn build_composite_key(&self, values: &[String]) -> String {
        values.join("\0")
    }

    /// Insert a document with the given attribute values.
    pub fn insert(&mut self, values: &[String], doc_key: impl Into<String>) {
        if values.len() != self.attributes.len() {
            return; // Ignore malformed inserts
        }
        let composite = self.build_composite_key(values);
        self.composite_to_keys
            .entry(composite)
            .or_default()
            .insert(doc_key.into());
    }

    /// Find documents with exact matching attribute values.
    pub fn find_exact(&self, values: &[String]) -> HashSet<String> {
        if values.len() != self.attributes.len() {
            return HashSet::new();
        }
        let composite = self.build_composite_key(values);
        self.composite_to_keys
            .get(&composite)
            .cloned()
            .unwrap_or_default()
    }

    /// Get the number of composite keys.
    pub fn len(&self) -> usize {
        self.composite_to_keys.len()
    }

    /// Check if index is empty.
    pub fn is_empty(&self) -> bool {
        self.composite_to_keys.is_empty()
    }
}

/// Index acceleration manager.
///
/// Per INDEX_ACCELERATION.md §3.1:
/// - Indexes may only narrow search space and suggest candidates
/// - Indexes may never decide visibility, existence, or commit ordering
#[derive(Debug)]
pub struct IndexAccelerator {
    /// Configuration.
    config: IndexAccelConfig,
    /// Single-attribute indexes.
    attribute_indexes: HashMap<String, AttributeIndex>,
    /// Composite indexes (keyed by joined attribute names).
    composite_indexes: HashMap<String, CompositeIndex>,
    /// Statistics for observability.
    stats: AcceleratorStats,
}

/// Statistics for the index accelerator.
///
/// Per INDEX_ACCELERATION.md §11:
/// - Permitted metrics are passive only
#[derive(Debug, Default, Clone)]
pub struct AcceleratorStats {
    /// Number of index lookups.
    pub lookups: u64,
    /// Number of candidates returned.
    pub candidates_returned: u64,
    /// Number of index rebuilds.
    pub rebuilds: u64,
}

impl IndexAccelerator {
    /// Create a new index accelerator.
    pub fn new(config: IndexAccelConfig) -> Self {
        Self {
            config,
            attribute_indexes: HashMap::new(),
            composite_indexes: HashMap::new(),
            stats: AcceleratorStats::default(),
        }
    }

    /// Check if acceleration is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.accelerated_structures_enabled
            || self.config.predicate_prefilter_enabled
            || self.config.multi_attribute_enabled
    }

    /// Get or create an attribute index.
    pub fn get_or_create_attribute_index(&mut self, attribute: &str) -> &mut AttributeIndex {
        self.attribute_indexes
            .entry(attribute.to_string())
            .or_insert_with(|| AttributeIndex::new(attribute))
    }

    /// Get an attribute index if it exists.
    pub fn get_attribute_index(&self, attribute: &str) -> Option<&AttributeIndex> {
        self.attribute_indexes.get(attribute)
    }

    /// Create a composite index for the given attributes.
    pub fn create_composite_index(&mut self, attributes: Vec<String>) -> &mut CompositeIndex {
        let key = attributes.join("\0");
        self.composite_indexes
            .entry(key.clone())
            .or_insert_with(|| CompositeIndex::new(attributes))
    }

    /// Prefilter candidates for an equality predicate.
    ///
    /// Per INDEX_ACCELERATION.md §4.3:
    /// - Pre-filtering MUST be conservative
    /// - False positives allowed
    /// - False negatives forbidden
    pub fn prefilter_equality(&mut self, attribute: &str, value: &str) -> PrefilterResult {
        if !self.config.predicate_prefilter_enabled {
            return PrefilterResult::no_filter();
        }

        self.stats.lookups += 1;

        if let Some(index) = self.attribute_indexes.get(attribute) {
            let candidates = index.find_exact(value);
            self.stats.candidates_returned += candidates.len() as u64;
            PrefilterResult {
                candidates,
                filter_applied: true,
                stats: PrefilterStats {
                    candidates_returned: 0, // Will be set below
                    predicates_evaluated: 1,
                    index_hit: true,
                },
            }
        } else {
            PrefilterResult::no_filter()
        }
    }

    /// Clear all indexes (for rebuild).
    ///
    /// Per INDEX_ACCELERATION.md §8.3:
    /// - Index discarded
    /// - Query falls back to baseline behavior
    pub fn clear(&mut self) {
        self.attribute_indexes.clear();
        self.composite_indexes.clear();
        self.stats.rebuilds += 1;
    }

    /// Get statistics.
    pub fn stats(&self) -> &AcceleratorStats {
        &self.stats
    }
}

/// Index path selection.
///
/// Per INDEX_ACCELERATION.md §10.1:
/// - Disablement restores baseline index structures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexPath {
    /// Baseline: no index acceleration.
    Baseline,
    /// Accelerated: improved structures and prefiltering.
    Accelerated,
}

impl IndexPath {
    /// Determine index path based on config.
    pub fn from_config(config: &IndexAccelConfig) -> Self {
        if config.accelerated_structures_enabled
            || config.predicate_prefilter_enabled
            || config.multi_attribute_enabled
        {
            Self::Accelerated
        } else {
            Self::Baseline
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== IndexAccelConfig Tests ====================

    #[test]
    fn test_config_default() {
        let config = IndexAccelConfig::default();
        assert!(!config.accelerated_structures_enabled);
        assert!(!config.predicate_prefilter_enabled);
        assert!(!config.multi_attribute_enabled);
    }

    #[test]
    fn test_config_enabled() {
        let config = IndexAccelConfig::enabled();
        assert!(config.accelerated_structures_enabled);
        assert!(config.predicate_prefilter_enabled);
        assert!(config.multi_attribute_enabled);
    }

    // ==================== AttributeIndex Tests ====================

    #[test]
    fn test_attribute_index_new() {
        let index = AttributeIndex::new("name");
        assert_eq!(index.attribute(), "name");
        assert!(index.is_empty());
    }

    #[test]
    fn test_attribute_index_insert_and_find() {
        let mut index = AttributeIndex::new("city");
        index.insert("NYC", "doc1");
        index.insert("NYC", "doc2");
        index.insert("LA", "doc3");

        let nyc = index.find_exact("NYC");
        assert_eq!(nyc.len(), 2);
        assert!(nyc.contains("doc1"));
        assert!(nyc.contains("doc2"));

        let la = index.find_exact("LA");
        assert_eq!(la.len(), 1);
        assert!(la.contains("doc3"));
    }

    #[test]
    fn test_attribute_index_find_nonexistent() {
        let index = AttributeIndex::new("city");
        let result = index.find_exact("London");
        assert!(result.is_empty());
    }

    #[test]
    fn test_attribute_index_remove() {
        let mut index = AttributeIndex::new("city");
        index.insert("NYC", "doc1");
        index.insert("NYC", "doc2");

        let removed = index.remove("NYC", "doc1");
        assert!(removed);

        let nyc = index.find_exact("NYC");
        assert_eq!(nyc.len(), 1);
        assert!(nyc.contains("doc2"));
    }

    #[test]
    fn test_attribute_index_find_in() {
        let mut index = AttributeIndex::new("status");
        index.insert("active", "doc1");
        index.insert("pending", "doc2");
        index.insert("active", "doc3");

        let values = vec!["active".to_string(), "pending".to_string()];
        let result = index.find_in(&values);
        assert_eq!(result.len(), 3);
    }

    // ==================== CompositeIndex Tests ====================

    #[test]
    fn test_composite_index_new() {
        let attrs = vec!["city".to_string(), "status".to_string()];
        let index = CompositeIndex::new(attrs.clone());
        assert_eq!(index.attributes(), &attrs[..]);
        assert!(index.is_empty());
    }

    #[test]
    fn test_composite_index_insert_and_find() {
        let mut index = CompositeIndex::new(vec!["city".to_string(), "status".to_string()]);
        index.insert(&["NYC".to_string(), "active".to_string()], "doc1");
        index.insert(&["NYC".to_string(), "active".to_string()], "doc2");
        index.insert(&["LA".to_string(), "active".to_string()], "doc3");

        let nyc_active = index.find_exact(&["NYC".to_string(), "active".to_string()]);
        assert_eq!(nyc_active.len(), 2);

        let la_active = index.find_exact(&["LA".to_string(), "active".to_string()]);
        assert_eq!(la_active.len(), 1);
    }

    #[test]
    fn test_composite_index_mismatched_values() {
        let mut index = CompositeIndex::new(vec!["a".to_string(), "b".to_string()]);

        // Wrong number of values - should be ignored
        index.insert(&["only_one".to_string()], "doc1");
        assert!(index.is_empty());

        // Find with wrong number should return empty
        let result = index.find_exact(&["only_one".to_string()]);
        assert!(result.is_empty());
    }

    // ==================== IndexAccelerator Tests ====================

    #[test]
    fn test_accelerator_disabled() {
        let acc = IndexAccelerator::new(IndexAccelConfig::disabled());
        assert!(!acc.is_enabled());
    }

    #[test]
    fn test_accelerator_enabled() {
        let acc = IndexAccelerator::new(IndexAccelConfig::enabled());
        assert!(acc.is_enabled());
    }

    #[test]
    fn test_accelerator_prefilter_disabled() {
        let mut acc = IndexAccelerator::new(IndexAccelConfig::disabled());
        let result = acc.prefilter_equality("city", "NYC");
        assert!(!result.filter_applied);
    }

    #[test]
    fn test_accelerator_prefilter_enabled() {
        let mut acc = IndexAccelerator::new(IndexAccelConfig::enabled());

        // Create and populate index
        {
            let index = acc.get_or_create_attribute_index("city");
            index.insert("NYC", "doc1");
            index.insert("NYC", "doc2");
        }

        let result = acc.prefilter_equality("city", "NYC");
        assert!(result.filter_applied);
        assert_eq!(result.candidates.len(), 2);
    }

    #[test]
    fn test_accelerator_prefilter_no_index() {
        let mut acc = IndexAccelerator::new(IndexAccelConfig::enabled());
        let result = acc.prefilter_equality("nonexistent", "value");
        assert!(!result.filter_applied);
    }

    #[test]
    fn test_accelerator_clear() {
        let mut acc = IndexAccelerator::new(IndexAccelConfig::enabled());
        acc.get_or_create_attribute_index("city")
            .insert("NYC", "doc1");

        acc.clear();

        assert!(acc.get_attribute_index("city").is_none());
        assert_eq!(acc.stats().rebuilds, 1);
    }

    // ==================== IndexPath Tests ====================

    #[test]
    fn test_index_path_baseline() {
        let config = IndexAccelConfig::disabled();
        assert_eq!(IndexPath::from_config(&config), IndexPath::Baseline);
    }

    #[test]
    fn test_index_path_accelerated() {
        let config = IndexAccelConfig::enabled();
        assert_eq!(IndexPath::from_config(&config), IndexPath::Accelerated);
    }

    // ==================== Equivalence Tests ====================

    /// Per INDEX_ACCELERATION.md §14:
    /// "If removing an index changes a query result, the index is incorrect."
    #[test]
    fn test_index_absence_no_false_negatives() {
        // Simulate: index returns candidates, absence returns all
        let mut index = AttributeIndex::new("status");
        index.insert("active", "doc1");
        index.insert("active", "doc2");
        index.insert("pending", "doc3");

        // With index: get active documents
        let with_index = index.find_exact("active");

        // Without index: would scan all, then filter
        // All "active" docs must be in with_index result
        let all_docs = vec!["doc1", "doc2", "doc3"];
        let active_docs: Vec<_> = all_docs
            .into_iter()
            .filter(|d| *d == "doc1" || *d == "doc2")
            .collect();

        // Index result must be SUPERSET
        for doc in &active_docs {
            assert!(
                with_index.contains(*doc),
                "Index missed valid candidate: {}",
                doc
            );
        }
    }
}
