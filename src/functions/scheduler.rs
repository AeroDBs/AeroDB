//! # Function Scheduler

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use croner::Cron;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::errors::{FunctionError, FunctionResult};
use super::store::{JobStore, MemJobStore};

/// A scheduled job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    /// Job ID
    pub id: Uuid,

    /// Function name
    pub function_name: String,

    /// Cron expression
    pub cron: String,

    /// Last run time
    pub last_run: Option<DateTime<Utc>>,

    /// Next scheduled run
    pub next_run: Option<DateTime<Utc>>,

    /// Whether the job is enabled
    pub enabled: bool,
}

impl ScheduledJob {
    /// Create a new scheduled job
    pub fn new(function_name: String, cron: String) -> FunctionResult<Self> {
        // Validate cron expression
        let cron_parser = Cron::new(&cron).parse().map_err(|e| {
            FunctionError::InvalidCron(format!("Invalid cron expression '{}': {}", cron, e))
        })?;

        let next_run = cron_parser
            .find_next_occurrence(&Utc::now(), false)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                FunctionError::InvalidCron(format!("Error calculating next run: {}", e))
            })?;

        Ok(Self {
            id: Uuid::new_v4(),
            function_name,
            cron,
            last_run: None,
            next_run: Some(next_run),
            enabled: true,
        })
    }

    /// Mark as run
    pub fn mark_run(&mut self) -> FunctionResult<()> {
        self.last_run = Some(Utc::now());

        // Calculate next run
        let cron_parser = Cron::new(&self.cron).parse().map_err(|e| {
            FunctionError::InvalidCron(format!("Invalid cron expression '{}': {}", self.cron, e))
        })?;

        self.next_run = cron_parser
            .find_next_occurrence(&Utc::now(), false)
            .map(|dt| dt.with_timezone(&Utc))
            .ok();

        Ok(())
    }
}

/// Job scheduler
#[derive(Debug)]
pub struct Scheduler {
    /// Jobs by ID
    jobs: RwLock<HashMap<Uuid, ScheduledJob>>,

    /// Jobs by function name
    by_function: RwLock<HashMap<String, Uuid>>,

    /// Persistent store
    store: Arc<dyn JobStore>,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new(Arc::new(MemJobStore::default()))
    }
}

impl Scheduler {
    /// Create a new scheduler with specific store
    pub fn new(store: Arc<dyn JobStore>) -> Self {
        let jobs = store.load().unwrap_or_else(|e| {
            eprintln!("Failed to load jobs from store: {}", e);
            Vec::new()
        });

        let mut jobs_map = HashMap::new();
        let mut by_function = HashMap::new();

        for job in jobs {
            by_function.insert(job.function_name.clone(), job.id);
            jobs_map.insert(job.id, job);
        }

        Self {
            jobs: RwLock::new(jobs_map),
            by_function: RwLock::new(by_function),
            store,
        }
    }

    /// Add a scheduled job
    pub fn schedule(&self, job: ScheduledJob) -> FunctionResult<Uuid> {
        let id = job.id;
        let function_name = job.function_name.clone();

        // Persist first
        self.store.save(&job)?;

        {
            let mut jobs = self
                .jobs
                .write()
                .map_err(|_| FunctionError::Internal("Lock poisoned".into()))?;
            jobs.insert(id, job);
        }

        {
            let mut by_function = self
                .by_function
                .write()
                .map_err(|_| FunctionError::Internal("Lock poisoned".into()))?;
            by_function.insert(function_name, id);
        }

        Ok(id)
    }

    /// Cancel a job
    pub fn cancel(&self, job_id: Uuid) -> FunctionResult<()> {
        // Persist removal
        self.store.delete(&job_id)?;

        let function_name = {
            let mut jobs = self
                .jobs
                .write()
                .map_err(|_| FunctionError::Internal("Lock poisoned".into()))?;
            jobs.remove(&job_id).map(|j| j.function_name)
        };

        if let Some(name) = function_name {
            let mut by_function = self
                .by_function
                .write()
                .map_err(|_| FunctionError::Internal("Lock poisoned".into()))?;
            by_function.remove(&name);
        }

        Ok(())
    }

    /// Get jobs that are due to run
    pub fn get_due_jobs(&self) -> Vec<ScheduledJob> {
        let now = Utc::now();

        self.jobs
            .read()
            .map(|jobs| {
                jobs.values()
                    .filter(|j| j.enabled && j.next_run.map(|t| t <= now).unwrap_or(false))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Mark a job as run
    pub fn mark_run(&self, job_id: Uuid) -> FunctionResult<()> {
        let job_copy = {
            let mut jobs = self
                .jobs
                .write()
                .map_err(|_| FunctionError::Internal("Lock poisoned".into()))?;

            if let Some(job) = jobs.get_mut(&job_id) {
                job.mark_run()?;
                Some(job.clone())
            } else {
                None
            }
        };

        if let Some(job) = job_copy {
            self.store.save(&job)?;
        }

        Ok(())
    }

    /// Get job count
    pub fn len(&self) -> usize {
        self.jobs.read().map(|j| j.len()).unwrap_or(0)
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduled_job_creation() {
        let job = ScheduledJob::new("cleanup".to_string(), "0 0 * * *".to_string()).unwrap();

        assert_eq!(job.function_name, "cleanup");
        assert!(job.enabled);
    }

    #[test]
    fn test_invalid_cron() {
        let result = ScheduledJob::new("test".to_string(), "invalid".to_string());

        assert!(result.is_err());
    }

    #[test]
    fn test_scheduler() {
        let scheduler = Scheduler::new(Arc::new(MemJobStore::default()));

        let job = ScheduledJob::new("daily_task".to_string(), "0 0 * * *".to_string()).unwrap();

        let id = scheduler.schedule(job).unwrap();
        assert_eq!(scheduler.len(), 1);

        scheduler.cancel(id).unwrap();
        assert_eq!(scheduler.len(), 0);
    }

    #[test]
    fn test_get_due_jobs() {
        let scheduler = Scheduler::new(Arc::new(MemJobStore::default()));

        let mut job = ScheduledJob::new("immediate".to_string(), "* * * * *".to_string()).unwrap();
        // Force next_run to be in the past so it's due
        job.next_run = Some(Utc::now() - chrono::Duration::seconds(1));

        scheduler.schedule(job).unwrap();

        let due = scheduler.get_due_jobs();
        assert_eq!(due.len(), 1);
    }
}
