//! Backpressure Mechanisms
//!
//! HARDENING: Protects system from overload by limiting concurrent operations.
//!
//! Per Production Hardening Analysis:
//! - Connection limits with explicit rejection
//! - Queue depth limits
//! - No silent degradation

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

/// Backpressure configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackpressureConfig {
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Maximum pending operations in queue
    pub max_queue_depth: usize,
    /// Maximum concurrent operations per connection
    pub max_ops_per_connection: usize,
    /// Queue timeout before rejection (ms)
    pub queue_timeout_ms: u64,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            max_connections: 1000,
            max_queue_depth: 10000,
            max_ops_per_connection: 100,
            queue_timeout_ms: 30000, // 30 seconds
        }
    }
}

/// Backpressure errors
#[derive(Debug)]
pub enum BackpressureError {
    /// Connection limit reached
    ConnectionLimitReached {
        current: usize,
        limit: usize,
    },
    /// Queue is full
    QueueFull {
        current: usize,
        limit: usize,
    },
    /// Operation timed out waiting in queue
    QueueTimeout {
        waited_ms: u64,
        timeout_ms: u64,
    },
    /// Too many operations from this connection
    TooManyOpsPerConnection {
        current: usize,
        limit: usize,
    },
}

impl std::fmt::Display for BackpressureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackpressureError::ConnectionLimitReached { current, limit } => {
                write!(
                    f,
                    "Connection limit reached: {} active, {} max. Try again later.",
                    current, limit
                )
            }
            BackpressureError::QueueFull { current, limit } => {
                write!(
                    f,
                    "Operation queue full: {} pending, {} max. Try again later.",
                    current, limit
                )
            }
            BackpressureError::QueueTimeout { waited_ms, timeout_ms } => {
                write!(
                    f,
                    "Operation timed out waiting in queue: waited {}ms, timeout {}ms.",
                    waited_ms, timeout_ms
                )
            }
            BackpressureError::TooManyOpsPerConnection { current, limit } => {
                write!(
                    f,
                    "Too many concurrent operations from this connection: {} active, {} max.",
                    current, limit
                )
            }
        }
    }
}

impl std::error::Error for BackpressureError {}

impl BackpressureError {
    /// HTTP status code for this error
    pub fn http_status_code(&self) -> u16 {
        match self {
            BackpressureError::ConnectionLimitReached { .. } => 503,
            BackpressureError::QueueFull { .. } => 503,
            BackpressureError::QueueTimeout { .. } => 504, // Gateway Timeout
            BackpressureError::TooManyOpsPerConnection { .. } => 429, // Too Many Requests
        }
    }

    /// Is this error recoverable by retrying later?
    pub fn is_retryable(&self) -> bool {
        true // All backpressure errors are retryable
    }

    /// Suggested retry delay in ms
    pub fn retry_after_ms(&self) -> u64 {
        match self {
            BackpressureError::ConnectionLimitReached { .. } => 1000,
            BackpressureError::QueueFull { .. } => 500,
            BackpressureError::QueueTimeout { .. } => 100,
            BackpressureError::TooManyOpsPerConnection { .. } => 100,
        }
    }
}

/// Backpressure manager
///
/// HARDENING: Central control for system load management.
#[derive(Debug)]
pub struct BackpressureManager {
    config: BackpressureConfig,
    current_connections: AtomicUsize,
    current_queue_depth: AtomicUsize,
}

impl BackpressureManager {
    pub fn new(config: BackpressureConfig) -> Self {
        Self {
            config,
            current_connections: AtomicUsize::new(0),
            current_queue_depth: AtomicUsize::new(0),
        }
    }

    /// Try to acquire a connection slot
    ///
    /// Returns a guard that releases the slot on drop.
    pub fn try_acquire_connection(&self) -> Result<ConnectionGuard, BackpressureError> {
        let current = self.current_connections.fetch_add(1, Ordering::AcqRel);
        
        if current >= self.config.max_connections {
            // Roll back the increment
            self.current_connections.fetch_sub(1, Ordering::Release);
            return Err(BackpressureError::ConnectionLimitReached {
                current,
                limit: self.config.max_connections,
            });
        }

        Ok(ConnectionGuard {
            counter: Arc::new(ConnectionCounterInner {
                current_connections: &self.current_connections as *const _ as usize,
            }),
            config: self.config.clone(),
            ops_count: AtomicUsize::new(0),
        })
    }

    /// Try to acquire a queue slot for an operation
    pub fn try_enqueue(&self) -> Result<QueueGuard, BackpressureError> {
        let current = self.current_queue_depth.fetch_add(1, Ordering::AcqRel);
        
        if current >= self.config.max_queue_depth {
            self.current_queue_depth.fetch_sub(1, Ordering::Release);
            return Err(BackpressureError::QueueFull {
                current,
                limit: self.config.max_queue_depth,
            });
        }

        Ok(QueueGuard {
            manager: self,
            enqueued_at: Instant::now(),
            timeout_ms: self.config.queue_timeout_ms,
        })
    }

    /// Get current connection count
    pub fn current_connections(&self) -> usize {
        self.current_connections.load(Ordering::Acquire)
    }

    /// Get current queue depth
    pub fn current_queue_depth(&self) -> usize {
        self.current_queue_depth.load(Ordering::Acquire)
    }

    /// Get configuration
    pub fn config(&self) -> &BackpressureConfig {
        &self.config
    }

    /// Get system load status
    pub fn load_status(&self) -> LoadStatus {
        let conn_percent = (self.current_connections() as f64 / self.config.max_connections as f64) * 100.0;
        let queue_percent = (self.current_queue_depth() as f64 / self.config.max_queue_depth as f64) * 100.0;
        
        let max_percent = conn_percent.max(queue_percent);
        
        if max_percent >= 90.0 {
            LoadStatus::Critical
        } else if max_percent >= 75.0 {
            LoadStatus::Warning
        } else {
            LoadStatus::Normal
        }
    }
}

/// System load status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadStatus {
    /// System is operating normally
    Normal,
    /// System is approaching limits
    Warning,
    /// System is at or near capacity
    Critical,
}

// Internal helper to track connection counter pointer
struct ConnectionCounterInner {
    current_connections: usize,
}

/// RAII guard for connection slot
pub struct ConnectionGuard {
    counter: Arc<ConnectionCounterInner>,
    config: BackpressureConfig,
    ops_count: AtomicUsize,
}

impl ConnectionGuard {
    /// Try to start an operation on this connection
    pub fn try_start_operation(&self) -> Result<OperationGuard<'_>, BackpressureError> {
        let current = self.ops_count.fetch_add(1, Ordering::AcqRel);
        
        if current >= self.config.max_ops_per_connection {
            self.ops_count.fetch_sub(1, Ordering::Release);
            return Err(BackpressureError::TooManyOpsPerConnection {
                current,
                limit: self.config.max_ops_per_connection,
            });
        }

        Ok(OperationGuard { guard: self })
    }

    /// Get current operation count for this connection
    pub fn current_ops(&self) -> usize {
        self.ops_count.load(Ordering::Acquire)
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        // SAFETY: We stored the pointer as usize, convert back
        let ptr = self.counter.current_connections as *const AtomicUsize;
        if !ptr.is_null() {
            unsafe {
                (*ptr).fetch_sub(1, Ordering::Release);
            }
        }
    }
}

/// RAII guard for an operation
pub struct OperationGuard<'a> {
    guard: &'a ConnectionGuard,
}

impl<'a> Drop for OperationGuard<'a> {
    fn drop(&mut self) {
        self.guard.ops_count.fetch_sub(1, Ordering::Release);
    }
}

/// RAII guard for queue slot
pub struct QueueGuard<'a> {
    manager: &'a BackpressureManager,
    enqueued_at: Instant,
    timeout_ms: u64,
}

impl<'a> QueueGuard<'a> {
    /// Check if operation has timed out waiting in queue
    pub fn check_timeout(&self) -> Result<(), BackpressureError> {
        let elapsed = self.enqueued_at.elapsed();
        let timeout = Duration::from_millis(self.timeout_ms);
        
        if elapsed > timeout {
            return Err(BackpressureError::QueueTimeout {
                waited_ms: elapsed.as_millis() as u64,
                timeout_ms: self.timeout_ms,
            });
        }
        
        Ok(())
    }

    /// Time spent waiting in queue (ms)
    pub fn wait_time_ms(&self) -> u64 {
        self.enqueued_at.elapsed().as_millis() as u64
    }
}

impl<'a> Drop for QueueGuard<'a> {
    fn drop(&mut self) {
        self.manager.current_queue_depth.fetch_sub(1, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_limit() {
        let config = BackpressureConfig {
            max_connections: 2,
            ..Default::default()
        };
        let manager = BackpressureManager::new(config);
        
        let _conn1 = manager.try_acquire_connection().unwrap();
        let _conn2 = manager.try_acquire_connection().unwrap();
        
        // Third should fail
        match manager.try_acquire_connection() {
            Err(BackpressureError::ConnectionLimitReached { .. }) => {}
            Ok(_) => panic!("Expected ConnectionLimitReached, got Ok"),
            Err(e) => panic!("Expected ConnectionLimitReached, got {}", e),
        }
    }

    #[test]
    fn test_connection_release() {
        let config = BackpressureConfig {
            max_connections: 1,
            ..Default::default()
        };
        let manager = BackpressureManager::new(config);
        
        {
            let _conn = manager.try_acquire_connection().unwrap();
            assert_eq!(manager.current_connections(), 1);
        }
        
        // After drop, should be able to acquire again
        assert_eq!(manager.current_connections(), 0);
        let _conn = manager.try_acquire_connection().unwrap();
    }

    #[test]
    fn test_queue_limit() {
        let config = BackpressureConfig {
            max_queue_depth: 2,
            ..Default::default()
        };
        let manager = BackpressureManager::new(config);
        
        let _q1 = manager.try_enqueue().unwrap();
        let _q2 = manager.try_enqueue().unwrap();
        
        // Third should fail
        assert!(matches!(
            manager.try_enqueue(),
            Err(BackpressureError::QueueFull { .. })
        ));
    }

    #[test]
    fn test_ops_per_connection() {
        let config = BackpressureConfig {
            max_ops_per_connection: 2,
            ..Default::default()
        };
        let manager = BackpressureManager::new(config);
        
        let conn = manager.try_acquire_connection().unwrap();
        let _op1 = conn.try_start_operation().unwrap();
        let _op2 = conn.try_start_operation().unwrap();
        
        // Third should fail
        assert!(matches!(
            conn.try_start_operation(),
            Err(BackpressureError::TooManyOpsPerConnection { .. })
        ));
    }

    #[test]
    fn test_load_status() {
        let config = BackpressureConfig {
            max_connections: 10,
            max_queue_depth: 10,
            ..Default::default()
        };
        let manager = BackpressureManager::new(config);
        
        assert_eq!(manager.load_status(), LoadStatus::Normal);
        
        // Acquire 8 connections (80%)
        let _conns: Vec<_> = (0..8)
            .map(|_| manager.try_acquire_connection().unwrap())
            .collect();
        
        assert_eq!(manager.load_status(), LoadStatus::Warning);
    }

    #[test]
    fn test_error_http_codes() {
        assert_eq!(
            BackpressureError::ConnectionLimitReached { current: 0, limit: 0 }.http_status_code(),
            503
        );
        assert_eq!(
            BackpressureError::QueueFull { current: 0, limit: 0 }.http_status_code(),
            503
        );
        assert_eq!(
            BackpressureError::QueueTimeout { waited_ms: 0, timeout_ms: 0 }.http_status_code(),
            504
        );
        assert_eq!(
            BackpressureError::TooManyOpsPerConnection { current: 0, limit: 0 }.http_status_code(),
            429
        );
    }
}
