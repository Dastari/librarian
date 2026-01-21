//! Background job queue with bounded concurrency
//!
//! Provides a job queue system that limits concurrent operations to prevent
//! overwhelming system resources and external APIs.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Semaphore, mpsc};
use tracing::{debug, error, info};
use uuid::Uuid;

/// Job priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobPriority {
    Low = 0,
    Normal = 1,
    High = 2,
}

impl Default for JobPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Configuration for a job queue
#[derive(Debug, Clone)]
pub struct JobQueueConfig {
    /// Maximum concurrent jobs
    pub max_concurrent: usize,
    /// Queue capacity (pending jobs)
    pub queue_capacity: usize,
    /// Delay between processing jobs (for rate limiting)
    pub job_delay: Duration,
}

impl Default for JobQueueConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 5,
            queue_capacity: 1000,
            job_delay: Duration::from_millis(100),
        }
    }
}

/// A generic job that can be queued
pub struct Job<T> {
    pub id: Uuid,
    pub priority: JobPriority,
    pub payload: T,
}

impl<T> Job<T> {
    pub fn new(payload: T) -> Self {
        Self {
            id: Uuid::new_v4(),
            priority: JobPriority::Normal,
            payload,
        }
    }

    pub fn with_priority(mut self, priority: JobPriority) -> Self {
        self.priority = priority;
        self
    }
}

/// A bounded work queue that processes jobs with limited concurrency
pub struct WorkQueue<T> {
    sender: mpsc::Sender<Job<T>>,
    semaphore: Arc<Semaphore>,
    config: JobQueueConfig,
    name: String,
}

impl<T: Send + 'static> WorkQueue<T> {
    /// Create a new work queue with a processor function
    pub fn new<F, Fut>(name: &str, config: JobQueueConfig, processor: F) -> Self
    where
        F: Fn(T) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let (sender, mut receiver) = mpsc::channel::<Job<T>>(config.queue_capacity);
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent));
        let job_delay = config.job_delay;
        let queue_name = name.to_string();

        let sem_clone = semaphore.clone();
        let processor = Arc::new(processor);

        tokio::spawn(async move {
            info!(queue = %queue_name, "Work queue '{}' started", queue_name);

            while let Some(job) = receiver.recv().await {
                let sem = sem_clone.clone();
                let proc = processor.clone();
                let name = queue_name.clone();

                tokio::spawn(async move {
                    let _permit = sem.acquire().await.expect("Semaphore closed");
                    debug!(queue = %name, job_id = %job.id, "Processing job");

                    proc(job.payload).await;

                    debug!(queue = %name, job_id = %job.id, "Job completed");
                });

                // Small delay between spawning jobs to prevent overwhelming
                if job_delay > Duration::ZERO {
                    tokio::time::sleep(job_delay).await;
                }
            }

            info!(queue = %queue_name, "Work queue stopped");
        });

        Self {
            sender,
            semaphore,
            config,
            name: name.to_string(),
        }
    }

    /// Submit a job to the queue
    pub async fn submit(&self, payload: T) -> Result<Uuid, mpsc::error::SendError<Job<T>>> {
        let job = Job::new(payload);
        let id = job.id;
        self.sender.send(job).await?;
        debug!(queue = %self.name, job_id = %id, "Job submitted");
        Ok(id)
    }

    /// Submit a job with priority
    pub async fn submit_with_priority(
        &self,
        payload: T,
        priority: JobPriority,
    ) -> Result<Uuid, mpsc::error::SendError<Job<T>>> {
        let job = Job::new(payload).with_priority(priority);
        let id = job.id;
        self.sender.send(job).await?;
        debug!(queue = %self.name, job_id = %id, priority = ?priority, "Priority job submitted");
        Ok(id)
    }

    /// Get current queue statistics
    pub fn stats(&self) -> QueueStats {
        QueueStats {
            max_concurrent: self.config.max_concurrent,
            available_permits: self.semaphore.available_permits(),
            queue_capacity: self.config.queue_capacity,
        }
    }
}

/// Queue statistics
#[derive(Debug, Clone)]
pub struct QueueStats {
    pub max_concurrent: usize,
    pub available_permits: usize,
    pub queue_capacity: usize,
}

/// Concurrency limiter for ad-hoc bounded parallel operations
///
/// Use this when you need to limit concurrency for a batch of operations
/// without setting up a full work queue.
#[derive(Clone)]
pub struct ConcurrencyLimiter {
    semaphore: Arc<Semaphore>,
    name: String,
}

impl ConcurrencyLimiter {
    /// Create a new concurrency limiter
    pub fn new(name: &str, max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            name: name.to_string(),
        }
    }

    /// Acquire a permit and run the operation
    pub async fn run<F, Fut, T>(&self, operation: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let _permit = self.semaphore.acquire().await.expect("Semaphore closed");
        debug!(limiter = %self.name, "Acquired concurrency permit");
        let result = operation().await;
        debug!(limiter = %self.name, "Released concurrency permit");
        result
    }

    /// Get available permits
    pub fn available(&self) -> usize {
        self.semaphore.available_permits()
    }
}

/// Process items in chunks with bounded concurrency
///
/// This is useful for batch operations where you want to:
/// 1. Process multiple items in parallel (up to max_concurrent)
/// 2. Wait for a chunk to complete before starting the next
/// 3. Add a delay between chunks
pub async fn process_in_chunks<T, F, Fut, R>(
    items: Vec<T>,
    chunk_size: usize,
    max_concurrent: usize,
    chunk_delay: Duration,
    processor: F,
) -> Vec<R>
where
    T: Send + Clone + 'static,
    F: Fn(T) -> Fut + Send + Sync + Clone + 'static,
    Fut: std::future::Future<Output = R> + Send,
    R: Send + 'static,
{
    let mut all_results = Vec::with_capacity(items.len());
    let semaphore = Arc::new(Semaphore::new(max_concurrent));

    for chunk in items
        .chunks(chunk_size)
        .map(|c| c.to_vec())
        .collect::<Vec<_>>()
    {
        let mut handles = Vec::with_capacity(chunk.len());

        for item in chunk {
            let sem = semaphore.clone();
            let proc = processor.clone();

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("Semaphore closed");
                proc(item).await
            });

            handles.push(handle);
        }

        // Wait for all items in this chunk to complete
        for handle in handles {
            match handle.await {
                Ok(result) => all_results.push(result),
                Err(e) => {
                    error!(error = %e, "Task panicked during chunk processing");
                }
            }
        }

        // Delay between chunks
        if chunk_delay > Duration::ZERO {
            tokio::time::sleep(chunk_delay).await;
        }
    }

    all_results
}

/// Process items with bounded concurrency using streams
///
/// More memory-efficient than chunks for large datasets
pub async fn process_concurrent<T, F, Fut, R>(
    items: impl IntoIterator<Item = T>,
    max_concurrent: usize,
    processor: F,
) -> Vec<R>
where
    T: Send + 'static,
    F: Fn(T) -> Fut + Send + Sync + Clone + 'static,
    Fut: std::future::Future<Output = R> + Send,
    R: Send + 'static,
{
    use futures::stream::{self, StreamExt};

    let semaphore = Arc::new(Semaphore::new(max_concurrent));

    stream::iter(items)
        .map(|item| {
            let sem = semaphore.clone();
            let proc = processor.clone();
            async move {
                let _permit = sem.acquire().await.expect("Semaphore closed");
                proc(item).await
            }
        })
        .buffer_unordered(max_concurrent)
        .collect()
        .await
}

/// A specialized metadata fetch queue with rate limiting built-in
pub struct MetadataQueue {
    limiter: ConcurrencyLimiter,
    fetch_delay: Duration,
}

impl MetadataQueue {
    /// Create a new metadata queue
    ///
    /// Default: 3 concurrent fetches with 200ms delay between operations
    pub fn new() -> Self {
        Self {
            limiter: ConcurrencyLimiter::new("metadata", 3),
            fetch_delay: Duration::from_millis(200),
        }
    }

    /// Create with custom settings
    pub fn with_config(max_concurrent: usize, fetch_delay_ms: u64) -> Self {
        Self {
            limiter: ConcurrencyLimiter::new("metadata", max_concurrent),
            fetch_delay: Duration::from_millis(fetch_delay_ms),
        }
    }

    /// Execute a metadata fetch operation with rate limiting
    pub async fn fetch<F, Fut, T>(&self, operation: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let result = self.limiter.run(operation).await;
        tokio::time::sleep(self.fetch_delay).await;
        result
    }
}

impl Default for MetadataQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_concurrency_limiter() {
        let counter = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));
        let limiter = ConcurrencyLimiter::new("test", 3);

        let mut handles = vec![];

        for _ in 0..10 {
            let c = counter.clone();
            let m = max_seen.clone();
            let l = limiter.clone();

            handles.push(tokio::spawn(async move {
                l.run(|| async {
                    let current = c.fetch_add(1, Ordering::SeqCst) + 1;
                    m.fetch_max(current, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    c.fetch_sub(1, Ordering::SeqCst);
                })
                .await;
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        assert!(max_seen.load(Ordering::SeqCst) <= 3);
    }

    #[tokio::test]
    async fn test_process_concurrent() {
        let items: Vec<i32> = (1..=10).collect();

        let results = process_concurrent(items, 3, |x| async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            x * 2
        })
        .await;

        assert_eq!(results.len(), 10);
        // Results may be in any order due to concurrency
        let mut sorted = results.clone();
        sorted.sort();
        assert_eq!(sorted, vec![2, 4, 6, 8, 10, 12, 14, 16, 18, 20]);
    }
}
