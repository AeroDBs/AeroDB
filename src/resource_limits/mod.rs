//! Resource Limits and Tracking
//!
//! Centralized resource management for production hardening.
//!
//! # Hardening Principles
//!
//! - Refuse before OOM
//! - Refuse before disk full
//! - Refuse before FD exhaustion
//! - No kernel reliance for limits
//!
//! # Configuration
//!
//! All limits are configurable via aerodb.toml [resource_limits] section.

use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

mod errors;
pub use errors::{ResourceError, ResourceResult, ResourceType};

/// Resource limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimitsConfig {
    /// Minimum free disk bytes before refusing writes (default: 1GB)
    pub min_free_disk_bytes: u64,
    /// Maximum memory bytes allowed (default: 4GB)
    pub max_memory_bytes: u64,
    /// Maximum file descriptors (default: 90% of ulimit)
    pub max_file_descriptors: usize,
    /// Maximum documents in a result set
    pub max_result_set_docs: usize,
    /// Warning threshold percentage (default: 75%)
    pub warning_threshold_percent: u8,
    /// Critical threshold percentage (default: 90%)
    pub critical_threshold_percent: u8,
}

impl Default for ResourceLimitsConfig {
    fn default() -> Self {
        Self {
            min_free_disk_bytes: 1024 * 1024 * 1024, // 1GB
            max_memory_bytes: 4 * 1024 * 1024 * 1024, // 4GB
            max_file_descriptors: 1000,
            max_result_set_docs: 10000,
            warning_threshold_percent: 75,
            critical_threshold_percent: 90,
        }
    }
}

/// System health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// All resources healthy
    Normal,
    /// One or more resources approaching limits
    Warning,
    /// One or more resources at critical threshold
    Critical,
    /// System in read-only mode due to resource exhaustion
    ReadOnly,
}

/// Current resource status snapshot
#[derive(Debug, Clone)]
pub struct ResourceStatus {
    pub disk_usage_bytes: u64,
    pub disk_total_bytes: u64,
    pub disk_free_bytes: u64,
    pub memory_usage_bytes: u64,
    pub memory_limit_bytes: u64,
    pub open_file_descriptors: usize,
    pub fd_limit: usize,
    pub health_status: HealthStatus,
    pub read_only_mode: bool,
}

/// Memory allocation tracker
///
/// Tracks memory allocations and enforces limits.
/// Does NOT replace the allocator - tracks logical allocations.
#[derive(Debug)]
pub struct MemoryTracker {
    allocated: AtomicU64,
    limit: u64,
}

impl MemoryTracker {
    pub fn new(limit: u64) -> Self {
        Self {
            allocated: AtomicU64::new(0),
            limit,
        }
    }

    /// Try to allocate memory, returns error if would exceed limit
    pub fn try_allocate(&self, size: u64) -> ResourceResult<()> {
        let current = self.allocated.load(Ordering::Acquire);
        if current + size > self.limit {
            return Err(ResourceError::MemoryExhausted {
                current,
                requested: size,
                limit: self.limit,
            });
        }
        self.allocated.fetch_add(size, Ordering::Release);
        Ok(())
    }

    /// Release previously allocated memory
    pub fn release(&self, size: u64) {
        self.allocated.fetch_sub(size, Ordering::Release);
    }

    /// Get current allocation
    pub fn current(&self) -> u64 {
        self.allocated.load(Ordering::Acquire)
    }

    /// Get limit
    pub fn limit(&self) -> u64 {
        self.limit
    }

    /// Get usage percentage
    pub fn usage_percent(&self) -> u8 {
        if self.limit == 0 {
            return 0;
        }
        ((self.current() as f64 / self.limit as f64) * 100.0) as u8
    }
}

/// File descriptor tracker
#[derive(Debug)]
pub struct FileDescriptorTracker {
    open_count: AtomicUsize,
    limit: usize,
}

impl FileDescriptorTracker {
    pub fn new(limit: usize) -> Self {
        Self {
            open_count: AtomicUsize::new(0),
            limit,
        }
    }

    /// Try to open a file descriptor
    pub fn try_open(&self) -> ResourceResult<()> {
        let current = self.open_count.load(Ordering::Acquire);
        if current >= self.limit {
            return Err(ResourceError::FileDescriptorLimit {
                current,
                limit: self.limit,
            });
        }
        self.open_count.fetch_add(1, Ordering::Release);
        Ok(())
    }

    /// Release a file descriptor
    pub fn close(&self) {
        let prev = self.open_count.fetch_sub(1, Ordering::Release);
        // Prevent underflow
        if prev == 0 {
            self.open_count.store(0, Ordering::Release);
        }
    }

    /// Current open count
    pub fn current(&self) -> usize {
        self.open_count.load(Ordering::Acquire)
    }

    /// Get limit
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Get usage percentage
    pub fn usage_percent(&self) -> u8 {
        if self.limit == 0 {
            return 0;
        }
        ((self.current() as f64 / self.limit as f64) * 100.0) as u8
    }
}

/// Disk space checker
#[derive(Debug)]
pub struct DiskSpaceChecker {
    data_path: std::path::PathBuf,
    min_free_bytes: u64,
}

impl DiskSpaceChecker {
    pub fn new(data_path: impl AsRef<Path>, min_free_bytes: u64) -> Self {
        Self {
            data_path: data_path.as_ref().to_path_buf(),
            min_free_bytes,
        }
    }

    /// Check if there's enough disk space for a write
    pub fn check_space(&self, required_bytes: u64) -> ResourceResult<()> {
        let free = self.get_free_space()?;
        let needed = self.min_free_bytes + required_bytes;

        if free < needed {
            return Err(ResourceError::DiskFull {
                available: free,
                required: needed,
            });
        }
        Ok(())
    }

    /// Get current free space
    pub fn get_free_space(&self) -> ResourceResult<u64> {
        // Use statvfs on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let meta = std::fs::metadata(&self.data_path).map_err(|e| {
                ResourceError::IoError(format!("Failed to get disk stats: {}", e))
            })?;

            // Use nix or libc for statvfs - for now, use a simpler approach
            // In production, implement proper statvfs
            let output = std::process::Command::new("df")
                .arg("-B1")
                .arg(&self.data_path)
                .output()
                .map_err(|e| ResourceError::IoError(format!("Failed to run df: {}", e)))?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = stdout.lines().collect();
            if lines.len() < 2 {
                return Err(ResourceError::IoError(
                    "Unexpected df output format".to_string(),
                ));
            }

            let parts: Vec<&str> = lines[1].split_whitespace().collect();
            if parts.len() < 4 {
                return Err(ResourceError::IoError(
                    "Unexpected df output format".to_string(),
                ));
            }

            parts[3]
                .parse::<u64>()
                .map_err(|e| ResourceError::IoError(format!("Failed to parse free space: {}", e)))
        }

        #[cfg(not(unix))]
        {
            // Fallback for non-Unix systems
            Ok(u64::MAX)
        }
    }

    /// Get total disk space
    pub fn get_total_space(&self) -> ResourceResult<u64> {
        #[cfg(unix)]
        {
            let output = std::process::Command::new("df")
                .arg("-B1")
                .arg(&self.data_path)
                .output()
                .map_err(|e| ResourceError::IoError(format!("Failed to run df: {}", e)))?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = stdout.lines().collect();
            if lines.len() < 2 {
                return Err(ResourceError::IoError(
                    "Unexpected df output format".to_string(),
                ));
            }

            let parts: Vec<&str> = lines[1].split_whitespace().collect();
            if parts.len() < 2 {
                return Err(ResourceError::IoError(
                    "Unexpected df output format".to_string(),
                ));
            }

            parts[1]
                .parse::<u64>()
                .map_err(|e| ResourceError::IoError(format!("Failed to parse total space: {}", e)))
        }

        #[cfg(not(unix))]
        {
            Ok(u64::MAX)
        }
    }

    /// Get usage percentage
    pub fn usage_percent(&self) -> ResourceResult<u8> {
        let total = self.get_total_space()?;
        let free = self.get_free_space()?;
        if total == 0 {
            return Ok(0);
        }
        let used = total.saturating_sub(free);
        Ok(((used as f64 / total as f64) * 100.0) as u8)
    }
}

/// Centralized resource manager
#[derive(Debug)]
pub struct ResourceManager {
    config: ResourceLimitsConfig,
    memory: Arc<MemoryTracker>,
    file_descriptors: Arc<FileDescriptorTracker>,
    disk: Arc<DiskSpaceChecker>,
    read_only_mode: std::sync::atomic::AtomicBool,
}

impl ResourceManager {
    pub fn new(config: ResourceLimitsConfig, data_path: impl AsRef<Path>) -> Self {
        Self {
            memory: Arc::new(MemoryTracker::new(config.max_memory_bytes)),
            file_descriptors: Arc::new(FileDescriptorTracker::new(config.max_file_descriptors)),
            disk: Arc::new(DiskSpaceChecker::new(data_path, config.min_free_disk_bytes)),
            read_only_mode: std::sync::atomic::AtomicBool::new(false),
            config,
        }
    }

    /// Check if writes are allowed (not in read-only mode)
    pub fn writes_allowed(&self) -> bool {
        !self.read_only_mode.load(Ordering::Acquire)
    }

    /// Enter read-only mode
    pub fn enter_read_only_mode(&self) {
        self.read_only_mode.store(true, Ordering::Release);
        eprintln!("[WARN] System entering READ-ONLY mode due to resource exhaustion");
    }

    /// Exit read-only mode
    pub fn exit_read_only_mode(&self) {
        self.read_only_mode.store(false, Ordering::Release);
        eprintln!("[INFO] System exiting read-only mode");
    }

    /// Check disk space before write
    pub fn check_disk_space(&self, required_bytes: u64) -> ResourceResult<()> {
        self.disk.check_space(required_bytes)
    }

    /// Try to allocate memory
    pub fn try_allocate_memory(&self, size: u64) -> ResourceResult<()> {
        self.memory.try_allocate(size)
    }

    /// Release memory
    pub fn release_memory(&self, size: u64) {
        self.memory.release(size)
    }

    /// Try to open a file descriptor
    pub fn try_open_fd(&self) -> ResourceResult<()> {
        self.file_descriptors.try_open()
    }

    /// Close a file descriptor
    pub fn close_fd(&self) {
        self.file_descriptors.close()
    }

    /// Get current resource status
    pub fn get_status(&self) -> ResourceResult<ResourceStatus> {
        let disk_free = self.disk.get_free_space().unwrap_or(0);
        let disk_total = self.disk.get_total_space().unwrap_or(0);
        let disk_usage = disk_total.saturating_sub(disk_free);

        let memory_usage = self.memory.current();
        let memory_limit = self.memory.limit();

        let fd_current = self.file_descriptors.current();
        let fd_limit = self.file_descriptors.limit();

        // Determine health status
        let health_status = self.calculate_health_status(
            disk_usage,
            disk_total,
            memory_usage,
            memory_limit,
            fd_current,
            fd_limit,
        );

        Ok(ResourceStatus {
            disk_usage_bytes: disk_usage,
            disk_total_bytes: disk_total,
            disk_free_bytes: disk_free,
            memory_usage_bytes: memory_usage,
            memory_limit_bytes: memory_limit,
            open_file_descriptors: fd_current,
            fd_limit,
            health_status,
            read_only_mode: !self.writes_allowed(),
        })
    }

    fn calculate_health_status(
        &self,
        disk_usage: u64,
        disk_total: u64,
        memory_usage: u64,
        memory_limit: u64,
        fd_current: usize,
        fd_limit: usize,
    ) -> HealthStatus {
        if !self.writes_allowed() {
            return HealthStatus::ReadOnly;
        }

        let disk_percent = if disk_total > 0 {
            ((disk_usage as f64 / disk_total as f64) * 100.0) as u8
        } else {
            0
        };

        let memory_percent = if memory_limit > 0 {
            ((memory_usage as f64 / memory_limit as f64) * 100.0) as u8
        } else {
            0
        };

        let fd_percent = if fd_limit > 0 {
            ((fd_current as f64 / fd_limit as f64) * 100.0) as u8
        } else {
            0
        };

        let max_percent = disk_percent.max(memory_percent).max(fd_percent);

        if max_percent >= self.config.critical_threshold_percent {
            HealthStatus::Critical
        } else if max_percent >= self.config.warning_threshold_percent {
            HealthStatus::Warning
        } else {
            HealthStatus::Normal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_tracker_basic() {
        let tracker = MemoryTracker::new(1000);
        assert!(tracker.try_allocate(500).is_ok());
        assert_eq!(tracker.current(), 500);
        assert!(tracker.try_allocate(600).is_err()); // Would exceed
        tracker.release(500);
        assert_eq!(tracker.current(), 0);
    }

    #[test]
    fn test_fd_tracker_basic() {
        let tracker = FileDescriptorTracker::new(10);
        for _ in 0..10 {
            assert!(tracker.try_open().is_ok());
        }
        assert!(tracker.try_open().is_err()); // At limit
        tracker.close();
        assert!(tracker.try_open().is_ok());
    }

    #[test]
    fn test_usage_percent() {
        let tracker = MemoryTracker::new(100);
        tracker.try_allocate(75).unwrap();
        assert_eq!(tracker.usage_percent(), 75);
    }
}
