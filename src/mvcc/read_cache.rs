//! Read Path Optimization
//!
//! Per READ_PATH_OPTIMIZATION.md:
//! - Reduces cost of determining visible version without changing result
//! - All optimizations are read-only, snapshot-scoped, deterministic, discardable
//!
//! Per §4.1 Snapshot-Local Visibility Caching:
//! - Cache valid ONLY for one snapshot
//! - Cache MUST be discarded when snapshot ends
//! - Cache entries are immutable
//!
//! Per §4.2 Deterministic Short-Circuit Traversal:
//! - Stop traversal once first visible version is found
//!
//! Per §9.1 Disablement:
//! - Disableable via compile-time flag or startup config

use std::collections::HashMap;

/// Configuration for read path optimizations.
///
/// Per READ_PATH_OPTIMIZATION.md §10.1:
/// - Disableable via compile-time flag or startup config
/// - Disablement restores baseline traversal
#[derive(Debug, Clone)]
pub struct ReadPathConfig {
    /// Whether visibility caching is enabled.
    pub visibility_cache_enabled: bool,
    /// Whether short-circuit traversal is enabled.
    pub short_circuit_enabled: bool,
    /// Maximum entries in the visibility cache per snapshot.
    pub cache_max_entries: usize,
}

impl Default for ReadPathConfig {
    fn default() -> Self {
        Self {
            visibility_cache_enabled: false, // Conservative default: disabled
            short_circuit_enabled: true,     // This is baseline-equivalent optimization
            cache_max_entries: 1000,
        }
    }
}

impl ReadPathConfig {
    /// Create config with all optimizations enabled.
    pub fn enabled() -> Self {
        Self {
            visibility_cache_enabled: true,
            short_circuit_enabled: true,
            cache_max_entries: 1000,
        }
    }

    /// Create config with all optimizations disabled (baseline behavior).
    pub fn disabled() -> Self {
        Self {
            visibility_cache_enabled: false,
            short_circuit_enabled: false,
            cache_max_entries: 0,
        }
    }
}

/// Cached visibility result, suitable for snapshot-local caching.
///
/// Per READ_PATH_OPTIMIZATION.md §4.1:
/// - Keyed by document identifier and snapshot CommitId
/// - Cache entries are immutable
///
/// Note: This is distinct from mvcc::visibility::CachedVisibility which
/// holds borrowed references. CachedVisibility holds owned CommitIds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CachedVisibility {
    /// Version is visible at the given CommitId.
    Visible {
        /// The CommitId of the visible version.
        version_commit_id: u64,
    },
    /// No version is visible (document doesn't exist in this snapshot).
    NotVisible,
    /// Document was deleted (tombstone visible).
    Deleted {
        /// The CommitId of the tombstone.
        tombstone_commit_id: u64,
    },
}

/// Key for visibility cache lookups.
///
/// Per READ_PATH_OPTIMIZATION.md §4.1:
/// - Keyed by document identifier and snapshot CommitId
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VisibilityCacheKey {
    /// Document identifier (collection + doc_id).
    pub document_key: String,
    /// Snapshot CommitId (the upper bound for visibility).
    pub snapshot_commit_id: u64,
}

impl VisibilityCacheKey {
    /// Create a new cache key.
    pub fn new(document_key: impl Into<String>, snapshot_commit_id: u64) -> Self {
        Self {
            document_key: document_key.into(),
            snapshot_commit_id,
        }
    }
}

/// Snapshot-local visibility cache.
///
/// Per READ_PATH_OPTIMIZATION.md §4.1:
/// - Cache is valid ONLY for one snapshot
/// - Cache MUST be discarded when snapshot ends
/// - Cache entries are immutable
///
/// Per §3.1: "All optimizations are read-only, snapshot-scoped, deterministic, discardable"
#[derive(Debug)]
pub struct SnapshotVisibilityCache {
    /// The snapshot CommitId this cache is bound to.
    snapshot_commit_id: u64,
    /// Cached visibility results.
    cache: HashMap<String, CachedVisibility>,
    /// Maximum entries allowed.
    max_entries: usize,
    /// Cache statistics (passive only per §11).
    stats: CacheStats,
}

/// Cache statistics for observability.
///
/// Per READ_PATH_OPTIMIZATION.md §11:
/// - Metrics MUST NOT influence caching
/// - Metrics are passive only
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    /// Number of cache hits.
    pub hits: u64,
    /// Number of cache misses.
    pub misses: u64,
    /// Number of entries evicted due to capacity.
    pub evictions: u64,
}

impl SnapshotVisibilityCache {
    /// Create a new cache bound to a specific snapshot.
    pub fn new(snapshot_commit_id: u64, max_entries: usize) -> Self {
        Self {
            snapshot_commit_id,
            cache: HashMap::with_capacity(max_entries.min(100)),
            max_entries,
            stats: CacheStats::default(),
        }
    }

    /// Get the snapshot CommitId this cache is bound to.
    pub fn snapshot_commit_id(&self) -> u64 {
        self.snapshot_commit_id
    }

    /// Look up a cached visibility result.
    ///
    /// Returns None if not cached.
    pub fn get(&mut self, document_key: &str) -> Option<&CachedVisibility> {
        if let Some(result) = self.cache.get(document_key) {
            self.stats.hits += 1;
            Some(result)
        } else {
            self.stats.misses += 1;
            None
        }
    }

    /// Cache a visibility result.
    ///
    /// Per READ_PATH_OPTIMIZATION.md §4.1:
    /// - Cache entries are immutable (no updates)
    pub fn insert(&mut self, document_key: impl Into<String>, result: CachedVisibility) {
        if self.cache.len() >= self.max_entries {
            // Simple eviction: just skip insertion
            // A more sophisticated LRU could be added, but keeping it simple
            self.stats.evictions += 1;
            return;
        }

        let key = document_key.into();
        // Only insert if not already present (immutable entries)
        if !self.cache.contains_key(&key) {
            self.cache.insert(key, result);
        }
    }

    /// Check if a key is cached.
    pub fn contains(&self, document_key: &str) -> bool {
        self.cache.contains_key(document_key)
    }

    /// Get the number of cached entries.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Get cache statistics.
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Clear the cache (for testing or manual invalidation).
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

/// Traversal decision for version chain.
///
/// Per READ_PATH_OPTIMIZATION.md §4.2:
/// - Stop traversal once first visible version is found
/// - Order MUST be explicit
/// - Visibility MUST be checked
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraversalDecision {
    /// Continue traversing the version chain.
    Continue,
    /// Stop traversing - visible version found.
    StopVisible,
    /// Stop traversing - no visible version exists.
    StopNotVisible,
}

/// Short-circuit traversal helper.
///
/// Per READ_PATH_OPTIMIZATION.md §4.2:
/// - If version chains are ordered by CommitId
/// - Stop traversal once first visible version is found
/// - No speculative skipping
pub struct ShortCircuitTraversal {
    /// Whether short-circuit is enabled.
    enabled: bool,
    /// Number of versions checked.
    versions_checked: u64,
    /// Whether traversal has stopped.
    stopped: bool,
}

impl ShortCircuitTraversal {
    /// Create a new traversal helper.
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            versions_checked: 0,
            stopped: false,
        }
    }

    /// Check if traversal should continue after checking a version.
    ///
    /// Per READ_PATH_OPTIMIZATION.md §4.2:
    /// - "Later versions cannot be visible"
    /// - "Earlier versions are superseded"
    pub fn should_continue(
        &mut self,
        version_commit_id: u64,
        snapshot_commit_id: u64,
        is_visible: bool,
    ) -> TraversalDecision {
        self.versions_checked += 1;

        if self.stopped {
            return TraversalDecision::StopNotVisible;
        }

        if is_visible {
            if self.enabled {
                self.stopped = true;
            }
            return TraversalDecision::StopVisible;
        }

        // If this version's CommitId is greater than snapshot, continue looking for earlier versions
        // If version is older than snapshot but not visible (e.g., uncommitted), continue
        if version_commit_id > snapshot_commit_id && self.enabled {
            // This version is in the future - continue to find older versions
            TraversalDecision::Continue
        } else {
            TraversalDecision::Continue
        }
    }

    /// Get the number of versions checked.
    pub fn versions_checked(&self) -> u64 {
        self.versions_checked
    }
}

/// Read path mode selection.
///
/// Per READ_PATH_OPTIMIZATION.md §10.1:
/// - Disablement restores baseline traversal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadPath {
    /// Baseline: full traversal, no caching.
    Baseline,
    /// Optimized: caching and short-circuit enabled.
    Optimized,
}

impl ReadPath {
    /// Determine read path based on config.
    pub fn from_config(config: &ReadPathConfig) -> Self {
        if config.visibility_cache_enabled || config.short_circuit_enabled {
            Self::Optimized
        } else {
            Self::Baseline
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== ReadPathConfig Tests ====================

    #[test]
    fn test_config_default() {
        let config = ReadPathConfig::default();
        assert!(!config.visibility_cache_enabled);
        assert!(config.short_circuit_enabled); // Safe baseline-equivalent opt
        assert_eq!(config.cache_max_entries, 1000);
    }

    #[test]
    fn test_config_enabled() {
        let config = ReadPathConfig::enabled();
        assert!(config.visibility_cache_enabled);
        assert!(config.short_circuit_enabled);
    }

    #[test]
    fn test_config_disabled() {
        let config = ReadPathConfig::disabled();
        assert!(!config.visibility_cache_enabled);
        assert!(!config.short_circuit_enabled);
    }

    // ==================== VisibilityCacheKey Tests ====================

    #[test]
    fn test_cache_key_new() {
        let key = VisibilityCacheKey::new("doc1", 100);
        assert_eq!(key.document_key, "doc1");
        assert_eq!(key.snapshot_commit_id, 100);
    }

    #[test]
    fn test_cache_key_equality() {
        let key1 = VisibilityCacheKey::new("doc1", 100);
        let key2 = VisibilityCacheKey::new("doc1", 100);
        let key3 = VisibilityCacheKey::new("doc1", 200);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    // ==================== SnapshotVisibilityCache Tests ====================

    #[test]
    fn test_cache_new() {
        let cache = SnapshotVisibilityCache::new(100, 1000);
        assert_eq!(cache.snapshot_commit_id(), 100);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = SnapshotVisibilityCache::new(100, 1000);

        cache.insert(
            "doc1",
            CachedVisibility::Visible {
                version_commit_id: 50,
            },
        );

        let result = cache.get("doc1");
        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            &CachedVisibility::Visible {
                version_commit_id: 50
            }
        );
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = SnapshotVisibilityCache::new(100, 1000);

        let result = cache.get("nonexistent");
        assert!(result.is_none());
        assert_eq!(cache.stats().misses, 1);
    }

    #[test]
    fn test_cache_hit_stats() {
        let mut cache = SnapshotVisibilityCache::new(100, 1000);
        cache.insert("doc1", CachedVisibility::NotVisible);

        let _ = cache.get("doc1");
        let _ = cache.get("doc1");

        assert_eq!(cache.stats().hits, 2);
    }

    #[test]
    fn test_cache_immutable_entries() {
        let mut cache = SnapshotVisibilityCache::new(100, 1000);

        // Insert first value
        cache.insert(
            "doc1",
            CachedVisibility::Visible {
                version_commit_id: 50,
            },
        );

        // Try to insert different value for same key
        cache.insert("doc1", CachedVisibility::NotVisible);

        // Should still have original value (immutable)
        let result = cache.get("doc1");
        assert_eq!(
            result.unwrap(),
            &CachedVisibility::Visible {
                version_commit_id: 50
            }
        );
    }

    #[test]
    fn test_cache_capacity_limit() {
        let mut cache = SnapshotVisibilityCache::new(100, 2);

        cache.insert("doc1", CachedVisibility::NotVisible);
        cache.insert("doc2", CachedVisibility::NotVisible);
        cache.insert("doc3", CachedVisibility::NotVisible); // Should be skipped

        assert_eq!(cache.len(), 2);
        assert_eq!(cache.stats().evictions, 1);
    }

    #[test]
    fn test_cache_deleted_result() {
        let mut cache = SnapshotVisibilityCache::new(100, 1000);

        cache.insert(
            "doc1",
            CachedVisibility::Deleted {
                tombstone_commit_id: 75,
            },
        );

        let result = cache.get("doc1");
        assert_eq!(
            result.unwrap(),
            &CachedVisibility::Deleted {
                tombstone_commit_id: 75
            }
        );
    }

    // ==================== ShortCircuitTraversal Tests ====================

    #[test]
    fn test_traversal_enabled_stops_on_visible() {
        let mut traversal = ShortCircuitTraversal::new(true);

        // First version not visible
        let decision = traversal.should_continue(200, 150, false);
        assert_eq!(decision, TraversalDecision::Continue);

        // Second version is visible - should stop
        let decision = traversal.should_continue(100, 150, true);
        assert_eq!(decision, TraversalDecision::StopVisible);

        assert_eq!(traversal.versions_checked(), 2);
    }

    #[test]
    fn test_traversal_disabled_continues() {
        let mut traversal = ShortCircuitTraversal::new(false);

        // Even on visible, returns StopVisible but doesn't set stopped flag
        let decision = traversal.should_continue(100, 150, true);
        assert_eq!(decision, TraversalDecision::StopVisible);
    }

    #[test]
    fn test_traversal_after_stopped() {
        let mut traversal = ShortCircuitTraversal::new(true);

        // Find visible version
        traversal.should_continue(100, 150, true);

        // Further calls should return StopNotVisible
        let decision = traversal.should_continue(50, 150, true);
        assert_eq!(decision, TraversalDecision::StopNotVisible);
    }

    // ==================== ReadPath Tests ====================

    #[test]
    fn test_read_path_baseline() {
        let config = ReadPathConfig::disabled();
        assert_eq!(ReadPath::from_config(&config), ReadPath::Baseline);
    }

    #[test]
    fn test_read_path_optimized() {
        let config = ReadPathConfig::enabled();
        assert_eq!(ReadPath::from_config(&config), ReadPath::Optimized);
    }

    // ==================== Equivalence Tests ====================

    /// Per READ_PATH_OPTIMIZATION.md §7:
    /// "For any snapshot and document, the same version is selected"
    #[test]
    fn test_cache_returns_same_result() {
        // Simulate baseline computation
        let baseline_result = CachedVisibility::Visible {
            version_commit_id: 42,
        };

        // Cache the result
        let mut cache = SnapshotVisibilityCache::new(100, 1000);
        cache.insert("doc1", baseline_result.clone());

        // Cached result must equal baseline
        let cached = cache.get("doc1").unwrap();
        assert_eq!(cached, &baseline_result);
    }

    /// Per READ_PATH_OPTIMIZATION.md §4.1:
    /// "Cache is valid ONLY for one snapshot"
    #[test]
    fn test_cache_snapshot_scoped() {
        let mut cache1 = SnapshotVisibilityCache::new(100, 1000);
        let mut cache2 = SnapshotVisibilityCache::new(200, 1000);

        cache1.insert(
            "doc1",
            CachedVisibility::Visible {
                version_commit_id: 50,
            },
        );

        // Different snapshot should not have the entry
        assert!(cache2.get("doc1").is_none());
    }
}
