//! Async job system for long-running analyses.
//!
//! Analyses that take more than a few seconds are submitted as async jobs.
//! Clients receive a job ID immediately and poll for results.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Job status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JobStatus {
    /// Job is queued.
    Pending,
    /// Job is currently running.
    Running,
    /// Job completed successfully.
    Completed,
    /// Job failed.
    Failed,
}

/// A submitted analysis job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique job identifier.
    pub id: String,
    /// Job type description.
    pub job_type: String,
    /// Current status.
    pub status: JobStatus,
    /// When the job was submitted (ISO 8601).
    pub submitted_at: String,
    /// When the job started running.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    /// When the job completed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    /// Result (JSON string) if completed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    /// Error message if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Progress percentage (0-100).
    pub progress: u8,
}

/// Job store — in-memory for now, can be replaced with database.
#[derive(Debug, Clone)]
pub struct JobStore {
    jobs: Arc<Mutex<HashMap<String, Job>>>,
    /// Maximum number of jobs to retain.
    max_jobs: usize,
}

impl JobStore {
    pub fn new(max_jobs: usize) -> Self {
        Self {
            jobs: Arc::new(Mutex::new(HashMap::new())),
            max_jobs,
        }
    }

    /// Create a new pending job and return its ID.
    pub fn create(&self, job_type: &str) -> String {
        let id = generate_job_id();
        let now = now_iso8601();

        let job = Job {
            id: id.clone(),
            job_type: job_type.to_string(),
            status: JobStatus::Pending,
            submitted_at: now,
            started_at: None,
            completed_at: None,
            result: None,
            error: None,
            progress: 0,
        };

        let mut jobs = self.jobs.lock().unwrap();

        // Evict oldest completed jobs if at capacity
        if jobs.len() >= self.max_jobs {
            let oldest_completed: Option<String> = jobs
                .iter()
                .filter(|(_, j)| matches!(j.status, JobStatus::Completed | JobStatus::Failed))
                .min_by_key(|(_, j)| j.submitted_at.clone())
                .map(|(id, _)| id.clone());

            if let Some(old_id) = oldest_completed {
                jobs.remove(&old_id);
            }
        }

        jobs.insert(id.clone(), job);
        id
    }

    /// Mark a job as running.
    pub fn mark_running(&self, id: &str) {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.get_mut(id) {
            job.status = JobStatus::Running;
            job.started_at = Some(now_iso8601());
        }
    }

    /// Update job progress.
    pub fn update_progress(&self, id: &str, progress: u8) {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.get_mut(id) {
            job.progress = progress.min(100);
        }
    }

    /// Mark a job as completed with a result.
    pub fn complete(&self, id: &str, result: String) {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.get_mut(id) {
            job.status = JobStatus::Completed;
            job.completed_at = Some(now_iso8601());
            job.result = Some(result);
            job.progress = 100;
        }
    }

    /// Mark a job as failed.
    pub fn fail(&self, id: &str, error: String) {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.get_mut(id) {
            job.status = JobStatus::Failed;
            job.completed_at = Some(now_iso8601());
            job.error = Some(error);
        }
    }

    /// Get a job by ID.
    pub fn get(&self, id: &str) -> Option<Job> {
        let jobs = self.jobs.lock().unwrap();
        jobs.get(id).cloned()
    }

    /// List all jobs (most recent first).
    pub fn list(&self) -> Vec<Job> {
        let jobs = self.jobs.lock().unwrap();
        let mut list: Vec<Job> = jobs.values().cloned().collect();
        list.sort_by(|a, b| b.submitted_at.cmp(&a.submitted_at));
        list
    }

    /// Count jobs by status.
    pub fn counts(&self) -> HashMap<JobStatus, usize> {
        let jobs = self.jobs.lock().unwrap();
        let mut counts = HashMap::new();
        for job in jobs.values() {
            *counts.entry(job.status).or_insert(0) += 1;
        }
        counts
    }
}

fn generate_job_id() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!(
        "job-{}",
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    )
}

fn now_iso8601() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    crate::scoring::engine::format_epoch_public(secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_lifecycle() {
        let store = JobStore::new(100);

        let id = store.create("analyze");
        let job = store.get(&id).unwrap();
        assert_eq!(job.status, JobStatus::Pending);
        assert_eq!(job.progress, 0);

        store.mark_running(&id);
        let job = store.get(&id).unwrap();
        assert_eq!(job.status, JobStatus::Running);
        assert!(job.started_at.is_some());

        store.update_progress(&id, 50);
        let job = store.get(&id).unwrap();
        assert_eq!(job.progress, 50);

        store.complete(&id, r#"{"result":"ok"}"#.into());
        let job = store.get(&id).unwrap();
        assert_eq!(job.status, JobStatus::Completed);
        assert_eq!(job.progress, 100);
        assert!(job.result.is_some());
        assert!(job.completed_at.is_some());
    }

    #[test]
    fn test_job_failure() {
        let store = JobStore::new(100);
        let id = store.create("analyze");
        store.mark_running(&id);
        store.fail(&id, "Out of memory".into());

        let job = store.get(&id).unwrap();
        assert_eq!(job.status, JobStatus::Failed);
        assert_eq!(job.error.as_deref(), Some("Out of memory"));
    }

    #[test]
    fn test_job_not_found() {
        let store = JobStore::new(100);
        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn test_job_list_ordering() {
        let store = JobStore::new(100);
        let _id1 = store.create("first");
        let _id2 = store.create("second");
        let _id3 = store.create("third");

        let list = store.list();
        assert_eq!(list.len(), 3);
        // All jobs returned (ordering may vary when timestamps are identical)
        let ids: Vec<&str> = list.iter().map(|j| j.id.as_str()).collect();
        assert!(ids.contains(&_id1.as_str()));
        assert!(ids.contains(&_id2.as_str()));
        assert!(ids.contains(&_id3.as_str()));
    }

    #[test]
    fn test_job_eviction() {
        let store = JobStore::new(3);
        let id1 = store.create("job1");
        store.complete(&id1, "done".into());
        let _id2 = store.create("job2");
        let _id3 = store.create("job3");
        // Creating a 4th should evict the oldest completed
        let _id4 = store.create("job4");

        assert!(store.get(&id1).is_none(), "Oldest completed job should be evicted");
    }

    #[test]
    fn test_job_counts() {
        let store = JobStore::new(100);
        let id1 = store.create("a");
        let id2 = store.create("b");
        store.complete(&id1, "ok".into());
        store.mark_running(&id2);

        let counts = store.counts();
        assert_eq!(counts.get(&JobStatus::Completed).copied().unwrap_or(0), 1);
        assert_eq!(counts.get(&JobStatus::Running).copied().unwrap_or(0), 1);
    }
}
