//! Admission Control
//!
//! HARDENING: Rate limiting and load shedding for write operations.
//!
//! Implements a token bucket algorithm for write rate limiting.
//! - Configurable max writes per second
//! - Burst capacity handling
//! - Per-tenant quotas (extensible)

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use serde::{Deserialize, Serialize};

/// Admission control configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdmissionControlConfig {
    /// Max writes per second (0 = unlimited)
    pub max_writes_per_second: u32,
    
    /// Max concurrent queries (0 = unlimited)
    pub max_concurrent_queries: u32,
}

impl Default for AdmissionControlConfig {
    fn default() -> Self {
        Self {
            max_writes_per_second: 0,
            max_concurrent_queries: 100,
        }
    }
}

/// Token bucket for rate limiting
#[derive(Debug)]
struct TokenBucket {
    capacity: f64,
    tokens: f64,
    rate: f64,
    last_update: Instant,
}

impl TokenBucket {
    fn new(rate: f64, capacity: f64) -> Self {
        Self {
            capacity,
            tokens: capacity,
            rate,
            last_update: Instant::now(),
        }
    }

    fn try_acquire(&mut self, amount: f64) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        
        // Replenish tokens
        self.tokens = (self.tokens + self.rate * elapsed).min(self.capacity);
        self.last_update = now;

        if self.tokens >= amount {
            self.tokens -= amount;
            true
        } else {
            false
        }
    }
}

/// Global admission controller
pub struct AdmissionController {
    config: AdmissionControlConfig,
    write_bucket: Mutex<Option<TokenBucket>>,
    active_queries: AtomicU64,
}

impl AdmissionController {
    pub fn new(config: AdmissionControlConfig) -> Self {
        let write_bucket = if config.max_writes_per_second > 0 {
            Some(TokenBucket::new(
                config.max_writes_per_second as f64,
                config.max_writes_per_second as f64, // Burst = 1 sec worth
            ))
        } else {
            None
        };

        Self {
            config,
            write_bucket: Mutex::new(write_bucket),
            active_queries: AtomicU64::new(0),
        }
    }

    /// Try to acquire permission for a write operation
    pub fn try_acquire_write(&self) -> bool {
        let mut bucket = self.write_bucket.lock().unwrap();
        if let Some(bucket) = bucket.as_mut() {
            bucket.try_acquire(1.0)
        } else {
            true // Unlimited
        }
    }

    /// Try to acquire permission for a query
    pub fn try_acquire_query(&self) -> bool {
        if self.config.max_concurrent_queries == 0 {
            return true;
        }

        let current = self.active_queries.load(Ordering::Relaxed);
        if current >= self.config.max_concurrent_queries as u64 {
            return false;
        }

        // Optimistic increment
        let prev = self.active_queries.fetch_add(1, Ordering::SeqCst);
        if prev >= self.config.max_concurrent_queries as u64 {
            // Rolled over limit, back off
            self.active_queries.fetch_sub(1, Ordering::SeqCst);
            false
        } else {
            true
        }
    }

    /// Release query permission
    pub fn release_query(&self) {
        if self.config.max_concurrent_queries > 0 {
            self.active_queries.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

/// RAII guard for query execution
pub struct QueryGuard<'a> {
    controller: &'a AdmissionController,
}

impl<'a> Drop for QueryGuard<'a> {
    fn drop(&mut self) {
        self.controller.release_query();
    }
}

impl AdmissionController {
    pub fn acquire_query_guard(&self) -> Option<QueryGuard> {
        if self.try_acquire_query() {
            Some(QueryGuard { controller: self })
        } else {
            None
        }
    }
}
