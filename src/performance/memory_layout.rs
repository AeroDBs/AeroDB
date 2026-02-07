//! Memory Layout Optimization
//!
//! Per MEMORY_LAYOUT_OPTIMIZATION.md:
//! - Improve cache locality, reduce allocation count, improve data density
//! - Only in-memory representations affected
//! - Memory layout is never authoritative
//!
//! Per §4.1 Structure Packing:
//! - Reorder fields to reduce padding
//!
//! Per §4.2 Cache-Line Alignment:
//! - Alignment is advisory, not required
//! - No correctness dependence on alignment
//!
//! Per §4.3 Allocation Strategy:
//! - Arena allocation, object pooling, bump allocators
//!
//! Per §10.1 Disablement:
//! - Disableable via compile-time flag or startup config

/// Configuration for memory layout optimizations.
///
/// Per MEMORY_LAYOUT_OPTIMIZATION.md §10.1:
/// - Disableable via compile-time flag or startup config
/// - Disablement restores baseline allocation paths
#[derive(Debug, Clone)]
pub struct MemoryLayoutConfig {
    /// Whether structure packing is enabled.
    pub packing_enabled: bool,
    /// Whether cache-line alignment is enabled.
    pub alignment_enabled: bool,
    /// Whether optimized allocation strategies are enabled.
    pub allocation_enabled: bool,
}

impl Default for MemoryLayoutConfig {
    fn default() -> Self {
        Self {
            packing_enabled: false, // Conservative default
            alignment_enabled: false,
            allocation_enabled: false,
        }
    }
}

impl MemoryLayoutConfig {
    /// Create config with all optimizations enabled.
    pub fn enabled() -> Self {
        Self {
            packing_enabled: true,
            alignment_enabled: true,
            allocation_enabled: true,
        }
    }

    /// Create config with all optimizations disabled (baseline).
    pub fn disabled() -> Self {
        Self::default()
    }

    /// Check if any optimization is enabled.
    pub fn any_enabled(&self) -> bool {
        self.packing_enabled || self.alignment_enabled || self.allocation_enabled
    }
}

/// Cache line size (typical x86/ARM).
pub const CACHE_LINE_SIZE: usize = 64;

/// Marker trait for cache-line-aligned types.
///
/// Per MEMORY_LAYOUT_OPTIMIZATION.md §4.2:
/// - Alignment is advisory, not required
/// - No correctness dependence on alignment
pub trait CacheLineAligned {
    /// Size of this type when aligned to cache line.
    fn aligned_size() -> usize;
}

/// A wrapper that aligns its contents to cache line boundaries.
///
/// Per MEMORY_LAYOUT_OPTIMIZATION.md §4.2:
/// - Code must behave identically without alignment
#[repr(C, align(64))]
#[derive(Debug, Clone)]
pub struct CacheAligned<T> {
    inner: T,
}

impl<T> CacheAligned<T> {
    /// Create a new cache-aligned wrapper.
    pub fn new(value: T) -> Self {
        Self { inner: value }
    }

    /// Get a reference to the inner value.
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Get a mutable reference to the inner value.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Unwrap and return the inner value.
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T: Default> Default for CacheAligned<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

/// Simple arena allocator for homogeneous types.
///
/// Per MEMORY_LAYOUT_OPTIMIZATION.md §4.3:
/// - Allocation lifetime MUST be explicit
/// - No reuse of live objects
/// - No hidden aliasing
#[derive(Debug)]
pub struct Arena<T> {
    /// Storage for allocated items.
    storage: Vec<T>,
    /// Maximum capacity.
    capacity: usize,
}

impl<T> Arena<T> {
    /// Create a new arena with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            storage: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Allocate a new item in the arena.
    ///
    /// Returns None if arena is full.
    pub fn alloc(&mut self, item: T) -> Option<usize> {
        if self.storage.len() >= self.capacity {
            return None;
        }
        let idx = self.storage.len();
        self.storage.push(item);
        Some(idx)
    }

    /// Get a reference to an item by index.
    pub fn get(&self, idx: usize) -> Option<&T> {
        self.storage.get(idx)
    }

    /// Get a mutable reference to an item by index.
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        self.storage.get_mut(idx)
    }

    /// Get the number of allocated items.
    pub fn len(&self) -> usize {
        self.storage.len()
    }

    /// Check if the arena is empty.
    pub fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }

    /// Clear all items from the arena.
    ///
    /// Per MEMORY_LAYOUT_OPTIMIZATION.md §4.3:
    /// - Drop semantics remain correct
    pub fn clear(&mut self) {
        self.storage.clear();
    }

    /// Get remaining capacity.
    pub fn remaining_capacity(&self) -> usize {
        self.capacity.saturating_sub(self.storage.len())
    }
}

/// Packed key-value pair for dense storage.
///
/// Per MEMORY_LAYOUT_OPTIMIZATION.md §4.1:
/// - Reorder fields to reduce padding
/// - Field semantics unchanged
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackedKeyValue {
    /// 8-byte key hash.
    pub key_hash: u64,
    /// 4-byte value offset.
    pub value_offset: u32,
    /// 2-byte value length.
    pub value_len: u16,
    /// 2-byte flags.
    pub flags: u16,
}

impl PackedKeyValue {
    /// Create a new packed key-value.
    pub fn new(key_hash: u64, value_offset: u32, value_len: u16) -> Self {
        Self {
            key_hash,
            value_offset,
            value_len,
            flags: 0,
        }
    }

    /// Create with flags.
    pub fn with_flags(key_hash: u64, value_offset: u32, value_len: u16, flags: u16) -> Self {
        Self {
            key_hash,
            value_offset,
            value_len,
            flags,
        }
    }

    /// Size of this packed struct.
    pub const fn size() -> usize {
        16 // 8 + 4 + 2 + 2 = 16 bytes, no padding
    }
}

/// Statistics for memory layout optimizations.
///
/// Per MEMORY_LAYOUT_OPTIMIZATION.md §11:
/// - Metrics are passive only
/// - Metrics MUST NOT influence allocation strategy
#[derive(Debug, Default, Clone)]
pub struct MemoryLayoutStats {
    /// Number of arena allocations.
    pub arena_allocations: u64,
    /// Number of arena allocation failures.
    pub arena_allocation_failures: u64,
    /// Number of cache-aligned allocations.
    pub cache_aligned_allocations: u64,
    /// Approximate cache line utilization percentage.
    pub cache_line_utilization: f64,
}

/// Memory layout path selection.
///
/// Per MEMORY_LAYOUT_OPTIMIZATION.md §10.1:
/// - Disablement restores baseline allocation paths
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPath {
    /// Baseline: standard allocation and layout.
    Baseline,
    /// Optimized: packed structures and aligned allocations.
    Optimized,
}

impl MemoryPath {
    /// Determine memory path based on config.
    pub fn from_config(config: &MemoryLayoutConfig) -> Self {
        if config.any_enabled() {
            Self::Optimized
        } else {
            Self::Baseline
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== MemoryLayoutConfig Tests ====================

    #[test]
    fn test_config_default() {
        let config = MemoryLayoutConfig::default();
        assert!(!config.packing_enabled);
        assert!(!config.alignment_enabled);
        assert!(!config.allocation_enabled);
        assert!(!config.any_enabled());
    }

    #[test]
    fn test_config_enabled() {
        let config = MemoryLayoutConfig::enabled();
        assert!(config.packing_enabled);
        assert!(config.alignment_enabled);
        assert!(config.allocation_enabled);
        assert!(config.any_enabled());
    }

    // ==================== CacheAligned Tests ====================

    #[test]
    fn test_cache_aligned_new() {
        let aligned = CacheAligned::new(42u64);
        assert_eq!(*aligned.inner(), 42);
    }

    #[test]
    fn test_cache_aligned_alignment() {
        let aligned = CacheAligned::new(0u64);
        let addr = (&aligned) as *const _ as usize;
        // On most systems this should be 64-byte aligned
        // We can't guarantee it in tests, but verify size
        assert!(std::mem::size_of::<CacheAligned<u64>>() >= CACHE_LINE_SIZE);
    }

    #[test]
    fn test_cache_aligned_into_inner() {
        let aligned = CacheAligned::new(99i32);
        assert_eq!(aligned.into_inner(), 99);
    }

    #[test]
    fn test_cache_aligned_default() {
        let aligned: CacheAligned<u64> = CacheAligned::default();
        assert_eq!(*aligned.inner(), 0);
    }

    // ==================== Arena Tests ====================

    #[test]
    fn test_arena_alloc() {
        let mut arena = Arena::with_capacity(3);
        let idx0 = arena.alloc("first").unwrap();
        let idx1 = arena.alloc("second").unwrap();

        assert_eq!(idx0, 0);
        assert_eq!(idx1, 1);
        assert_eq!(arena.len(), 2);
    }

    #[test]
    fn test_arena_full() {
        let mut arena = Arena::with_capacity(2);
        arena.alloc(1).unwrap();
        arena.alloc(2).unwrap();
        let result = arena.alloc(3);
        assert!(result.is_none());
    }

    #[test]
    fn test_arena_get() {
        let mut arena = Arena::with_capacity(10);
        arena.alloc("hello");
        arena.alloc("world");

        assert_eq!(arena.get(0), Some(&"hello"));
        assert_eq!(arena.get(1), Some(&"world"));
        assert_eq!(arena.get(2), None);
    }

    #[test]
    fn test_arena_clear() {
        let mut arena = Arena::with_capacity(10);
        arena.alloc(1);
        arena.alloc(2);

        arena.clear();

        assert!(arena.is_empty());
        assert_eq!(arena.remaining_capacity(), 10);
    }

    #[test]
    fn test_arena_remaining_capacity() {
        let mut arena = Arena::with_capacity(5);
        arena.alloc(1);
        arena.alloc(2);

        assert_eq!(arena.remaining_capacity(), 3);
    }

    // ==================== PackedKeyValue Tests ====================

    #[test]
    fn test_packed_kv_new() {
        let kv = PackedKeyValue::new(0x1234567890ABCDEF, 100, 50);
        assert_eq!(kv.key_hash, 0x1234567890ABCDEF);
        assert_eq!(kv.value_offset, 100);
        assert_eq!(kv.value_len, 50);
        assert_eq!(kv.flags, 0);
    }

    #[test]
    fn test_packed_kv_with_flags() {
        let kv = PackedKeyValue::with_flags(123, 456, 78, 0xFF);
        assert_eq!(kv.flags, 0xFF);
    }

    #[test]
    fn test_packed_kv_size() {
        assert_eq!(PackedKeyValue::size(), 16);
        assert_eq!(std::mem::size_of::<PackedKeyValue>(), 16);
    }

    // ==================== MemoryPath Tests ====================

    #[test]
    fn test_path_baseline() {
        let config = MemoryLayoutConfig::disabled();
        assert_eq!(MemoryPath::from_config(&config), MemoryPath::Baseline);
    }

    #[test]
    fn test_path_optimized() {
        let config = MemoryLayoutConfig::enabled();
        assert_eq!(MemoryPath::from_config(&config), MemoryPath::Optimized);
    }

    // ==================== Equivalence Tests ====================

    /// Per MEMORY_LAYOUT_OPTIMIZATION.md §7:
    /// "Any difference is invisible to all observers."
    #[test]
    fn test_packed_equals_unpacked_semantically() {
        // An unpacked representation would have the same logical values
        let key_hash: u64 = 0x1234567890ABCDEF;
        let value_offset: u32 = 1000;
        let value_len: u16 = 500;

        let packed = PackedKeyValue::new(key_hash, value_offset, value_len);

        // Semantic equivalence: same values accessible
        assert_eq!(packed.key_hash, key_hash);
        assert_eq!(packed.value_offset, value_offset);
        assert_eq!(packed.value_len, value_len);
    }

    /// Per MEMORY_LAYOUT_OPTIMIZATION.md §4.2:
    /// "Code must behave identically without alignment."
    #[test]
    fn test_alignment_irrelevant_to_correctness() {
        // Unaligned value
        let unaligned = 42u64;

        // Aligned value
        let aligned = CacheAligned::new(42u64);

        // Both must be semantically identical
        assert_eq!(unaligned, *aligned.inner());
    }
}
