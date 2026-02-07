//! Performance Optimizations
//!
//! This module contains Phase 3 performance optimizations.
//!
//! All optimizations are:
//! - Correctness-preserving
//! - Disabled by default
//! - Discardable without data migration

mod memory_layout;

pub use memory_layout::{
    Arena, CacheAligned, CacheLineAligned, MemoryLayoutConfig, MemoryLayoutStats, MemoryPath,
    PackedKeyValue, CACHE_LINE_SIZE,
};
