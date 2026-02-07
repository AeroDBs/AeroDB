//! Backup Scheduler for AeroDB.
//!
//! Provides scheduling logic for automatic backups.

use chrono::{DateTime, Duration, Utc};

use crate::backup::BackupConfig;

/// Backup scheduler that tracks when backups should occur.
///
/// The scheduler does not run backups itself - it only provides
/// timing information. The actual backup execution should be
/// triggered by an external scheduler (e.g., tokio interval).
pub struct BackupScheduler {
    config: BackupConfig,
    last_backup_time: Option<DateTime<Utc>>,
}

impl BackupScheduler {
    /// Create a new backup scheduler.
    pub fn new(config: BackupConfig) -> Self {
        Self {
            config,
            last_backup_time: None,
        }
    }

    /// Create scheduler with known last backup time.
    pub fn with_last_backup(config: BackupConfig, last_backup: DateTime<Utc>) -> Self {
        Self {
            config,
            last_backup_time: Some(last_backup),
        }
    }

    /// Check if a backup is due based on interval_hours.
    pub fn is_backup_due(&self) -> bool {
        if !self.config.enabled {
            return false;
        }

        match self.last_backup_time {
            None => true, // No backup yet, always due
            Some(last) => {
                let interval = Duration::hours(self.config.interval_hours as i64);
                Utc::now() >= last + interval
            }
        }
    }

    /// Calculate the next backup time.
    ///
    /// Returns None if backups are disabled.
    pub fn next_backup_time(&self) -> Option<DateTime<Utc>> {
        if !self.config.enabled {
            return None;
        }

        let interval = Duration::hours(self.config.interval_hours as i64);

        match self.last_backup_time {
            None => Some(Utc::now()),
            Some(last) => Some(last + interval),
        }
    }

    /// Update the last backup time to now.
    pub fn mark_backup_complete(&mut self) {
        self.last_backup_time = Some(Utc::now());
    }

    /// Update the last backup time to a specific time.
    pub fn set_last_backup_time(&mut self, time: DateTime<Utc>) {
        self.last_backup_time = Some(time);
    }

    /// Get the last backup time.
    pub fn last_backup_time(&self) -> Option<DateTime<Utc>> {
        self.last_backup_time
    }

    /// Check if backups are enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the backup interval in hours.
    pub fn interval_hours(&self) -> u32 {
        self.config.interval_hours
    }

    /// Time until next backup (from now).
    ///
    /// Returns None if backups are disabled or already due.
    pub fn time_until_next_backup(&self) -> Option<Duration> {
        if !self.config.enabled {
            return None;
        }

        self.next_backup_time().and_then(|next| {
            let now = Utc::now();
            if next > now {
                Some(next - now)
            } else {
                None // Already due
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config(enabled: bool, interval_hours: u32) -> BackupConfig {
        BackupConfig {
            enabled,
            interval_hours,
            max_backups: 7,
            backup_dir: "/tmp/backups".to_string(),
        }
    }

    #[test]
    fn test_scheduler_new() {
        let config = create_test_config(true, 24);
        let scheduler = BackupScheduler::new(config);
        
        assert!(scheduler.is_enabled());
        assert_eq!(scheduler.interval_hours(), 24);
        assert!(scheduler.last_backup_time().is_none());
    }

    #[test]
    fn test_backup_due_no_previous() {
        let config = create_test_config(true, 24);
        let scheduler = BackupScheduler::new(config);
        
        assert!(scheduler.is_backup_due());
    }

    #[test]
    fn test_backup_not_due_recent() {
        let config = create_test_config(true, 24);
        let last_backup = Utc::now() - Duration::hours(1);
        let scheduler = BackupScheduler::with_last_backup(config, last_backup);
        
        assert!(!scheduler.is_backup_due());
    }

    #[test]
    fn test_backup_due_overdue() {
        let config = create_test_config(true, 24);
        let last_backup = Utc::now() - Duration::hours(25);
        let scheduler = BackupScheduler::with_last_backup(config, last_backup);
        
        assert!(scheduler.is_backup_due());
    }

    #[test]
    fn test_backup_disabled() {
        let config = create_test_config(false, 24);
        let scheduler = BackupScheduler::new(config);
        
        assert!(!scheduler.is_enabled());
        assert!(!scheduler.is_backup_due());
        assert!(scheduler.next_backup_time().is_none());
    }

    #[test]
    fn test_next_backup_time() {
        let config = create_test_config(true, 24);
        let last_backup = Utc::now() - Duration::hours(1);
        let scheduler = BackupScheduler::with_last_backup(config, last_backup);
        
        let next = scheduler.next_backup_time().unwrap();
        assert!(next > Utc::now());
    }

    #[test]
    fn test_mark_backup_complete() {
        let config = create_test_config(true, 24);
        let mut scheduler = BackupScheduler::new(config);
        
        assert!(scheduler.last_backup_time().is_none());
        
        scheduler.mark_backup_complete();
        
        assert!(scheduler.last_backup_time().is_some());
        assert!(!scheduler.is_backup_due());
    }

    #[test]
    fn test_time_until_next_backup() {
        let config = create_test_config(true, 24);
        let last_backup = Utc::now() - Duration::hours(1);
        let scheduler = BackupScheduler::with_last_backup(config, last_backup);
        
        let remaining = scheduler.time_until_next_backup().unwrap();
        assert!(remaining.num_hours() >= 22);
        assert!(remaining.num_hours() <= 23);
    }

    #[test]
    fn test_time_until_next_backup_already_due() {
        let config = create_test_config(true, 24);
        let last_backup = Utc::now() - Duration::hours(25);
        let scheduler = BackupScheduler::with_last_backup(config, last_backup);
        
        assert!(scheduler.time_until_next_backup().is_none());
    }
}
