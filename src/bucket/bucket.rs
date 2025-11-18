use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, mpsc};
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

use super::config::Config;
use super::processor::Processor;
use super::types::BucketError;

pub struct Bucket<T> {
    config: Arc<Config>,
    sender: Arc<Mutex<Option<mpsc::Sender<T>>>>,
    receiver: Arc<Mutex<mpsc::Receiver<T>>>,
}

impl<T> Clone for Bucket<T> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
        }
    }
}

impl<T> Bucket<T>
where
    T: Send + 'static,
{
    pub fn new(config: Arc<Config>) -> Self {
        let (sender, receiver) = mpsc::channel(config.batch_size);

        Self {
            config,
            sender: Arc::new(Mutex::new(Some(sender))),
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    pub async fn consume(&self, item: T) -> Result<(), mpsc::error::SendError<T>> {
        let sender_guard = self.sender.lock().await;
        if let Some(sender) = sender_guard.as_ref() {
            sender.send(item).await
        } else {
            Err(mpsc::error::SendError(item))
        }
    }

    pub async fn close(&self) {
        let mut sender_guard = self.sender.lock().await;
        *sender_guard = None;
    }

    pub async fn run<P>(&self, cancel: &CancellationToken, process: P) -> Result<(), BucketError>
    where
        P: Processor<T> + Send + Sync + 'static,
        T: Clone,
    {
        let process = Arc::new(process);
        let mut handles = Vec::new();

        for worker_id in 0..self.config.worker_num {
            let receiver = self.receiver.clone();
            let process = process.clone();
            let cancel_token = cancel.clone();
            let batch_size = self.config.batch_size;
            let timeout = self.config.timeout;

            let handle = tokio::spawn(async move {
                Self::worker(
                    worker_id,
                    receiver,
                    process,
                    &cancel_token,
                    batch_size,
                    timeout,
                )
                .await
            });

            handles.push(handle);
        }

        // ... (phần còn lại của hàm run không đổi)
        let mut errors = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => errors.push(e),
                Err(e) => errors.push(BucketError::ProcessorError(e.to_string())),
            }
        }

        if !errors.is_empty() {
            return Err(errors.into_iter().next().unwrap());
        }

        Ok(())
    }

    async fn worker<P>(
        worker_id: usize,
        receiver: Arc<Mutex<mpsc::Receiver<T>>>,
        process: Arc<P>,
        cancel_token: &CancellationToken,
        batch_size: usize,
        timeout_duration: Duration,
    ) -> Result<(), BucketError>
    where
        P: Processor<T> + Send + Sync,
        T: Clone,
    {
        // ... (code của worker không đổi)
        let mut queue: Vec<T> = Vec::with_capacity(batch_size);
        let mut ticker = interval(timeout_duration);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    println!("Worker {} shutting down", worker_id);
                    return Self::process_queue(cancel_token, &*process, &mut queue).await;
                }

                _ = ticker.tick() => {
                    if !queue.is_empty() {
                        Self::process_queue(cancel_token, &*process, &mut queue).await?;
                    }
                }

                item = async {
                    let mut rx = receiver.lock().await;
                    rx.recv().await
                } => {
                    match item {
                        Some(item) => {
                            queue.push(item);

                            if queue.len() >= batch_size {
                                Self::process_queue(cancel_token, &*process, &mut queue).await?;
                            }
                        }
                        None => {
                            println!("Worker {} channel closed", worker_id);
                            return Self::process_queue(cancel_token, &*process, &mut queue).await;
                        }
                    }
                }
            }
        }
    }

    async fn process_queue<P>(
        ctx: &CancellationToken,
        process: &P,
        queue: &mut Vec<T>,
    ) -> Result<(), BucketError>
    where
        P: Processor<T>,
    {
        if !queue.is_empty() {
            process.process(ctx, queue).await?;
            queue.clear();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::time::sleep;

    struct CountingProcessor {
        counter: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Processor<i32> for CountingProcessor {
        async fn process(
            &self,
            _ctx: &CancellationToken,
            items: &[i32],
        ) -> Result<(), BucketError> {
            self.counter.fetch_add(items.len(), Ordering::SeqCst);
            Ok(())
        }
    }

    // Helper struct for tracking batch sizes
    struct BatchSizeTracker {
        sizes: Arc<tokio::sync::Mutex<Vec<usize>>>,
    }

    #[async_trait]
    impl Processor<i32> for BatchSizeTracker {
        async fn process(
            &self,
            _ctx: &CancellationToken,
            items: &[i32],
        ) -> Result<(), BucketError> {
            self.sizes.lock().await.push(items.len());
            Ok(())
        }
    }

    // Helper struct that returns errors
    struct ErrorProcessor;

    #[async_trait]
    impl Processor<i32> for ErrorProcessor {
        async fn process(
            &self,
            _ctx: &CancellationToken,
            _items: &[i32],
        ) -> Result<(), BucketError> {
            Err(BucketError::ProcessorError("Test error".to_string()))
        }
    }

    // Helper struct that does nothing
    struct NoOpProcessor;

    #[async_trait]
    impl Processor<i32> for NoOpProcessor {
        async fn process(
            &self,
            _ctx: &CancellationToken,
            _items: &[i32],
        ) -> Result<(), BucketError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_bucket_creation() {
        let config = Arc::new(Config {
            batch_size: 10,
            timeout: Duration::from_millis(100),
            worker_num: 2,
        });

        let bucket: Bucket<i32> = Bucket::new(config.clone());
        // Bucket should be created successfully
        assert_eq!(bucket.config.batch_size, 10);
        assert_eq!(bucket.config.worker_num, 2);
    }

    #[tokio::test]
    async fn test_bucket_consume_and_process() {
        let config = Arc::new(Config {
            batch_size: 5,
            timeout: Duration::from_millis(100),
            worker_num: 1,
        });

        let bucket: Bucket<i32> = Bucket::new(config);
        let bucket_clone = bucket.clone();
        let cancel = CancellationToken::new();

        // Track processed items
        let processed = Arc::new(AtomicUsize::new(0));
        let processor = CountingProcessor {
            counter: Arc::clone(&processed),
        };

        // Spawn consumer task
        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            for i in 0..10 {
                bucket_clone.consume(i).await.unwrap();
            }
            // Give time for processing
            sleep(Duration::from_millis(200)).await;
            cancel_clone.cancel();
        });

        // Run bucket processor
        let result = bucket.run(&cancel, processor).await;

        assert!(result.is_ok());
        assert_eq!(processed.load(Ordering::SeqCst), 10);
    }

    #[tokio::test]
    async fn test_bucket_batch_processing() {
        let config = Arc::new(Config {
            batch_size: 3,
            timeout: Duration::from_secs(10), // Long timeout to rely on batch size
            worker_num: 1,
        });

        let bucket: Bucket<i32> = Bucket::new(config);
        let bucket_clone = bucket.clone();
        let cancel = CancellationToken::new();

        // Track batch sizes
        let batch_sizes = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let processor = BatchSizeTracker {
            sizes: Arc::clone(&batch_sizes),
        };

        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            // Send exactly 6 items (2 batches of 3)
            for i in 0..6 {
                bucket_clone.consume(i).await.unwrap();
            }
            sleep(Duration::from_millis(100)).await;
            cancel_clone.cancel();
        });

        bucket.run(&cancel, processor).await.unwrap();

        let sizes = batch_sizes.lock().await;
        // Should have processed in batches of 3
        assert!(sizes.iter().any(|&s| s == 3));
    }

    #[tokio::test]
    async fn test_bucket_timeout_trigger() {
        let config = Arc::new(Config {
            batch_size: 100,                    // Large batch size
            timeout: Duration::from_millis(50), // Short timeout
            worker_num: 1,
        });

        let bucket: Bucket<i32> = Bucket::new(config);
        let bucket_clone = bucket.clone();
        let cancel = CancellationToken::new();

        let processed = Arc::new(AtomicUsize::new(0));
        let processor = CountingProcessor {
            counter: Arc::clone(&processed),
        };

        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            // Send only 2 items (less than batch size)
            bucket_clone.consume(1).await.unwrap();
            bucket_clone.consume(2).await.unwrap();

            // Wait for timeout to trigger
            sleep(Duration::from_millis(200)).await;
            cancel_clone.cancel();
        });

        bucket.run(&cancel, processor).await.unwrap();

        // Should process items via timeout, not batch size
        assert_eq!(processed.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_bucket_error_handling() {
        let config = Arc::new(Config {
            batch_size: 5,
            timeout: Duration::from_millis(100),
            worker_num: 1,
        });

        let bucket: Bucket<i32> = Bucket::new(config);
        let bucket_clone = bucket.clone();
        let cancel = CancellationToken::new();

        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            for i in 0..5 {
                bucket_clone.consume(i).await.unwrap();
            }
            sleep(Duration::from_millis(50)).await;
            cancel_clone.cancel();
        });

        // Processor that returns error
        let result = bucket.run(&cancel, ErrorProcessor).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            BucketError::ProcessorError(msg) => assert_eq!(msg, "Test error"),
            _ => panic!("Expected ProcessorError"),
        }
    }

    #[tokio::test]
    async fn test_bucket_multiple_workers() {
        let config = Arc::new(Config {
            batch_size: 2,
            timeout: Duration::from_millis(100),
            worker_num: 3, // Multiple workers
        });

        let bucket: Bucket<i32> = Bucket::new(config);
        let bucket_clone = bucket.clone();
        let cancel = CancellationToken::new();

        let processed = Arc::new(AtomicUsize::new(0));
        let processor = CountingProcessor {
            counter: Arc::clone(&processed),
        };

        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            for i in 0..20 {
                bucket_clone.consume(i).await.unwrap();
            }
            sleep(Duration::from_millis(200)).await;
            cancel_clone.cancel();
        });

        bucket.run(&cancel, processor).await.unwrap();

        assert_eq!(processed.load(Ordering::SeqCst), 20);
    }

    #[tokio::test]
    async fn test_bucket_cancellation() {
        let config = Arc::new(Config {
            batch_size: 10,
            timeout: Duration::from_secs(10),
            worker_num: 1,
        });

        let bucket: Bucket<i32> = Bucket::new(config);
        let cancel = CancellationToken::new();

        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            // Cancel immediately
            sleep(Duration::from_millis(10)).await;
            cancel_clone.cancel();
        });

        let result = bucket.run(&cancel, NoOpProcessor).await;

        // Should complete without error on cancellation
        assert!(result.is_ok());
    }

    #[test]
    fn test_bucket_error_display() {
        let err1 = BucketError::ProcessorError("test".to_string());
        assert_eq!(err1.to_string(), "ProcessorError: test");

        let err2 = BucketError::ChannelClosed;
        assert_eq!(err2.to_string(), "ChannelClosed");

        let err3 = BucketError::Cancelled;
        assert_eq!(err3.to_string(), "Cancelled");
    }
}
