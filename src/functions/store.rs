//! # Job Store
//!
//! Durable storage for scheduled jobs.

use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::errors::{FunctionError, FunctionResult};
use super::scheduler::ScheduledJob;

/// Trait for durable job storage
pub trait JobStore: Send + Sync + std::fmt::Debug {
    /// Load all jobs from storage
    fn load(&self) -> FunctionResult<Vec<ScheduledJob>>;

    /// Save a job
    fn save(&self, job: &ScheduledJob) -> FunctionResult<()>;

    /// Delete a job
    fn delete(&self, job_id: &Uuid) -> FunctionResult<()>;
}

/// JSON file-based job store
#[derive(Debug)]
pub struct FileJobStore {
    path: PathBuf,
}

impl FileJobStore {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    fn load_jobs(&self) -> FunctionResult<Vec<ScheduledJob>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.path)
            .map_err(|e| FunctionError::Internal(format!("Failed to read job store: {}", e)))?;

        if content.is_empty() {
            return Ok(Vec::new());
        }

        serde_json::from_str(&content)
            .map_err(|e| FunctionError::Internal(format!("Failed to parse job store: {}", e)))
    }

    fn save_jobs(&self, jobs: &[ScheduledJob]) -> FunctionResult<()> {
        let content = serde_json::to_string_pretty(jobs)
            .map_err(|e| FunctionError::Internal(format!("Failed to serialize jobs: {}", e)))?;

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                FunctionError::Internal(format!("Failed to create job store directory: {}", e))
            })?;
        }

        fs::write(&self.path, content)
            .map_err(|e| FunctionError::Internal(format!("Failed to write job store: {}", e)))
    }
}

impl JobStore for FileJobStore {
    fn load(&self) -> FunctionResult<Vec<ScheduledJob>> {
        self.load_jobs()
    }

    fn save(&self, job: &ScheduledJob) -> FunctionResult<()> {
        let mut jobs = self.load_jobs()?;

        // Update or insert
        if let Some(idx) = jobs.iter().position(|j| j.id == job.id) {
            jobs[idx] = job.clone();
        } else {
            jobs.push(job.clone());
        }

        self.save_jobs(&jobs)
    }

    fn delete(&self, job_id: &Uuid) -> FunctionResult<()> {
        let mut jobs = self.load_jobs()?;

        // Remove
        if let Some(idx) = jobs.iter().position(|j| j.id == *job_id) {
            jobs.remove(idx);
            self.save_jobs(&jobs)?;
        }

        Ok(())
    }
}

/// In-memory job store for testing
#[derive(Debug, Default)]
pub struct MemJobStore {
    jobs: std::sync::RwLock<Vec<ScheduledJob>>,
}

impl MemJobStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl JobStore for MemJobStore {
    fn load(&self) -> FunctionResult<Vec<ScheduledJob>> {
        Ok(self.jobs.read().unwrap().clone())
    }

    fn save(&self, job: &ScheduledJob) -> FunctionResult<()> {
        let mut jobs = self.jobs.write().unwrap();
        if let Some(idx) = jobs.iter().position(|j| j.id == job.id) {
            jobs[idx] = job.clone();
        } else {
            jobs.push(job.clone());
        }
        Ok(())
    }

    fn delete(&self, job_id: &Uuid) -> FunctionResult<()> {
        let mut jobs = self.jobs.write().unwrap();
        if let Some(idx) = jobs.iter().position(|j| j.id == *job_id) {
            jobs.remove(idx);
        }
        Ok(())
    }
}
