use flume::{Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};

use super::config::Config;
use super::processor::BatchProcessor;
use super::types::BucketError;

/// A concurrent batching processor that collects items into batches
/// and processes them with configurable workers.
///
/// `Bucket` uses a bounded channel for backpressure control and spawns
/// multiple worker tasks to process batches concurrently.
///
/// # Type Parameters
///
/// * `T` - The type of items to process. Must be `Send + 'static`.
///
/// # Example
///
/// ```rust,ignore
/// use etl_rust::bucket::{Bucket, Config};
/// use std::sync::Arc;
/// use std::time::Duration;
///
/// let config = Arc::new(Config {
///     batch_size: 50,
///     timeout: Duration::from_secs(5),
///     worker_num: 4,
/// });
///
/// let bucket: Bucket<i32> = Bucket::new(config);
/// // Bucket collects items into batches and processes them with 4 workers
/// ```
pub struct Bucket<T> {
    config: Arc<Config>,
    sender: Sender<T>,
    receiver: Receiver<T>,
    done: CancellationToken,
}

impl<T> Clone for Bucket<T> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
            done: self.done.clone(),
        }
    }
}

impl<T> Bucket<T>
where
    T: Send + 'static,
{
    /// Creates a new `Bucket` with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Shared configuration specifying batch size, timeout, and worker count
    pub fn new(config: Arc<Config>) -> Self {
        // Channel capacity = batch_size * worker_num * 2
        // Allows buffering multiple batches per worker to reduce backpressure
        let capacity = config.batch_size() * config.worker_num() * 2;
        let (sender, receiver) = flume::bounded(capacity);

        Self {
            config,
            sender,
            receiver,
            done: CancellationToken::new(),
        }
    }

    /// Consumes an item, sending it to the processing queue.
    ///
    /// This method blocks if the channel is full (backpressure).
    ///
    /// # Arguments
    ///
    /// * `cancel` - Cancellation token to abort the operation
    /// * `item` - The item to process
    ///
    /// # Errors
    ///
    /// Returns `BucketError::Cancelled` if the cancellation token is triggered
    /// before the item is sent.
    pub async fn consume(&self, cancel: &CancellationToken, item: T) -> Result<(), BucketError> {
        if cancel.is_cancelled() {
            return Err(BucketError::Cancelled);
        }

        self.sender
            .send_async(item)
            .await
            .map_err(|err| BucketError::ConsumerError(err.to_string()))
    }

    /// Signals that no more items will be produced.
    ///
    /// Workers will finish processing queued items and then shut down.
    #[inline]
    pub fn close(&self) {
        self.done.cancel();
    }

    /// Runs the bucket processor with the given batch processor.
    ///
    /// Spawns worker tasks to process batches concurrently. Blocks until all
    /// workers complete or a cancellation is triggered.
    ///
    /// # Arguments
    ///
    /// * `cancel` - Cancellation token to stop processing
    /// * `process` - Batch processor implementation or closure
    ///
    /// # Errors
    ///
    /// Returns `BucketError::MultipleErrors` if multiple workers fail.
    pub async fn run<P>(&self, cancel: &CancellationToken, process: P) -> Result<(), BucketError>
    where
        P: BatchProcessor<T> + Send + Sync + 'static,
    {
        let process = Arc::new(process);
        let mut handles = Vec::new();

        for worker_id in 0..self.config.worker_num() {
            let process = process.clone();
            let cancel_token = cancel.clone();
            let batch_size = self.config.batch_size();
            let timeout = self.config.timeout();
            let done_token = self.done.clone();
            let receiver = self.receiver.clone();

            let handle = tokio::spawn(async move {
                Self::worker(
                    worker_id,
                    receiver,
                    process,
                    &cancel_token,
                    &done_token,
                    batch_size,
                    timeout,
                )
                .await
            });

            handles.push(handle);
        }

        // Spawn workers to process batches
        let mut errors = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => errors.push(e),
                Err(e) => errors.push(BucketError::ProcessorError(Box::new(e))),
            }
        }

        if !errors.is_empty() {
            return Err(BucketError::MultipleErrors(errors));
        }

        Ok(())
    }

    async fn worker<P>(
        worker_id: usize,
        receiver: Receiver<T>,
        process: Arc<P>,
        cancel_token: &CancellationToken,
        done_token: &CancellationToken,
        batch_size: usize,
        timeout_duration: Duration,
    ) -> Result<(), BucketError>
    where
        P: BatchProcessor<T> + Send + Sync,
    {
        let mut queue: Vec<T> = Vec::with_capacity(batch_size);
        let mut ticker = interval(timeout_duration);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    info!(worker_id, "worker shutting down");
                    return Self::drain_and_process(receiver, cancel_token, &*process, &mut queue, batch_size).await;
                }

                _ = done_token.cancelled() => {
                    debug!(worker_id, "worker done processing");
                    return Self::drain_and_process(receiver, cancel_token, &*process, &mut queue, batch_size).await;
                }

                _ = ticker.tick() => {
                    if !queue.is_empty() {
                        Self::process_queue(cancel_token, &*process, &mut queue).await?;
                    }
                }

                result = receiver.recv_async() => {
                    match result {
                        Ok(item) => {
                            queue.push(item);

                            if queue.len() >= batch_size {
                                Self::process_queue(cancel_token, &*process, &mut queue).await?;
                            }
                        }
                        Err(_) => {
                            debug!(worker_id, "worker channel closed");
                            return Self::process_queue(cancel_token, &*process, &mut queue).await;
                        }
                    }
                }
            }
        }
    }
    async fn drain_and_process<P>(
        receiver: Receiver<T>,
        ctx: &CancellationToken,
        process: &P,
        queue: &mut Vec<T>,
        batch_size: usize,
    ) -> Result<(), BucketError>
    where
        P: BatchProcessor<T>,
    {
        Self::process_queue(ctx, process, queue).await?;
        while let Ok(item) = receiver.try_recv() {
            queue.push(item);
            if queue.len() >= batch_size {
                Self::process_queue(ctx, process, queue).await?;
            }
        }
        Self::process_queue(ctx, process, queue).await
    }

    async fn process_queue<P>(
        ctx: &CancellationToken,
        process: &P,
        queue: &mut Vec<T>,
    ) -> Result<(), BucketError>
    where
        P: BatchProcessor<T>,
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

    // Initialize tracing for tests
    fn init_tracing() {
        let _ = tracing_subscriber::fmt().with_test_writer().try_init();
    }

    // Processor that collects items into a shared vector
    struct CollectingProcessor {
        items: Arc<tokio::sync::Mutex<Vec<i32>>>,
    }

    #[async_trait]
    impl BatchProcessor<i32> for CollectingProcessor {
        async fn process(
            &self,
            _ctx: &CancellationToken,
            items: &[i32],
        ) -> Result<(), BucketError> {
            let mut collected = self.items.lock().await;
            collected.extend_from_slice(items);
            Ok(())
        }
    }
    struct CollectingProcessorDelayed {
        items: Arc<tokio::sync::Mutex<Vec<i32>>>,
    }

    #[async_trait]
    impl BatchProcessor<i32> for CollectingProcessorDelayed {
        async fn process(
            &self,
            _ctx: &CancellationToken,
            items: &[i32],
        ) -> Result<(), BucketError> {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let mut collected = self.items.lock().await;
            collected.extend_from_slice(items);
            Ok(())
        }
    }

    // Processor that counts processed items
    struct CountingProcessor {
        counter: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl BatchProcessor<i32> for CountingProcessor {
        async fn process(
            &self,
            _ctx: &CancellationToken,
            items: &[i32],
        ) -> Result<(), BucketError> {
            self.counter.fetch_add(items.len(), Ordering::SeqCst);
            Ok(())
        }
    }

    // Processor that tracks batch sizes and can signal completion
    #[allow(dead_code)]
    struct BatchTrackingProcessor {
        sizes: Arc<tokio::sync::Mutex<Vec<usize>>>,
        total_processed: Arc<AtomicUsize>,
        done_tx: Arc<tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
        target_count: usize,
    }

    #[async_trait]
    impl BatchProcessor<i32> for BatchTrackingProcessor {
        async fn process(
            &self,
            _ctx: &CancellationToken,
            items: &[i32],
        ) -> Result<(), BucketError> {
            self.sizes.lock().await.push(items.len());
            let count = self
                .total_processed
                .fetch_add(items.len(), Ordering::SeqCst)
                + items.len();

            if count >= self.target_count {
                let mut tx = self.done_tx.lock().await;
                if let Some(sender) = tx.take() {
                    let _ = sender.send(());
                }
            }
            Ok(())
        }
    }

    // Processor for timeout testing
    #[allow(dead_code)]
    struct TimeoutProcessor {
        first_batch_size: Arc<tokio::sync::Mutex<Option<usize>>>,
    }

    #[async_trait]
    impl BatchProcessor<i32> for TimeoutProcessor {
        async fn process(
            &self,
            _ctx: &CancellationToken,
            items: &[i32],
        ) -> Result<(), BucketError> {
            let mut size = self.first_batch_size.lock().await;
            if size.is_none() {
                *size = Some(items.len());
            }
            Ok(())
        }
    }

    // Processor that checks context cancellation
    #[allow(dead_code)]
    struct CancellationCheckProcessor {
        wait_for_signal: Arc<tokio::sync::Notify>,
        items_count: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl BatchProcessor<i32> for CancellationCheckProcessor {
        async fn process(&self, ctx: &CancellationToken, items: &[i32]) -> Result<(), BucketError> {
            self.items_count.store(items.len(), Ordering::SeqCst);
            self.wait_for_signal.notified().await;

            if ctx.is_cancelled() {
                return Err(BucketError::ProcessorError(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "exit",
                ))));
            }
            Ok(())
        }
    }

    const ITEM_NUMS: usize = 100;

    #[tokio::test]
    async fn test_should_process_full() {
        init_tracing();
        let config = Arc::new(Config {
            batch_size: 4,
            timeout: Duration::from_millis(100),
            worker_num: 4,
        });

        let bucket: Bucket<i32> = Bucket::new(config);
        let bucket_clone = bucket.clone();
        let cancel = CancellationToken::new();

        // Collect all processed items
        let result = Arc::new(tokio::sync::Mutex::new(Vec::with_capacity(ITEM_NUMS)));
        let processor = CollectingProcessor {
            items: Arc::clone(&result),
        };
        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            for i in 0..ITEM_NUMS {
                bucket_clone.consume(&cancel_clone, i as i32).await.unwrap();
            }
            bucket_clone.close();
        });

        // Run bucket processor
        let run_result = bucket.run(&cancel, processor).await;
        assert!(run_result.is_ok());

        // Verify all items were processed
        let mut items = result.lock().await;
        items.sort();
        assert_eq!(items.len(), ITEM_NUMS);
        for i in 0..ITEM_NUMS {
            assert_eq!(items[i], i as i32);
        }
    }
    #[tokio::test]
    async fn test_should_process_full_with_cancel() {
        init_tracing();
        let config = Arc::new(Config {
            batch_size: 4,
            timeout: Duration::from_millis(100),
            worker_num: 4,
        });

        let bucket: Bucket<i32> = Bucket::new(config);
        let bucket_clone = bucket.clone();
        let cancel = CancellationToken::new();

        let result = Arc::new(tokio::sync::Mutex::new(Vec::with_capacity(ITEM_NUMS)));
        let processor = CollectingProcessorDelayed {
            items: Arc::clone(&result),
        };
        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            for i in 0..ITEM_NUMS {
                if bucket_clone.consume(&cancel_clone, i as i32).await.is_err() {
                    break;
                }
            }
            bucket_clone.close();
        });

        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            sleep(Duration::from_secs(3)).await;
            cancel_clone.cancel();
        });

        let run_result = bucket.run(&cancel, processor).await;
        assert!(run_result.is_ok());

        let items = result.lock().await;
        println!("Processed {} items before cancellation", items.len());
        assert!(items.len() > 0, "Should have processed at least some items");
        assert!(
            items.len() <= ITEM_NUMS,
            "Should not process more items than sent"
        );
    }

    // Test that Bucket works with non-cloneable types
    #[allow(dead_code)]
    struct NonCloneable(i32);

    struct NonCloneableProcessor;

    #[async_trait]
    impl BatchProcessor<NonCloneable> for NonCloneableProcessor {
        async fn process(
            &self,
            _ctx: &CancellationToken,
            items: &[NonCloneable],
        ) -> Result<(), BucketError> {
            // Just verify we can access items
            assert!(!items.is_empty());
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_bucket_with_non_cloneable_types() {
        init_tracing();
        let config = Arc::new(Config {
            batch_size: 2,
            timeout: Duration::from_millis(100),
            worker_num: 1,
        });

        let bucket: Bucket<NonCloneable> = Bucket::new(config);
        let bucket_clone = bucket.clone();
        let cancel = CancellationToken::new();

        let processor = NonCloneableProcessor;
        let cancel_clone = cancel.clone();

        tokio::spawn(async move {
            bucket_clone
                .consume(&cancel_clone, NonCloneable(1))
                .await
                .unwrap();
            bucket_clone
                .consume(&cancel_clone, NonCloneable(2))
                .await
                .unwrap();
            bucket_clone.close();
        });

        let result = bucket.run(&cancel, processor).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_high_throughput_no_backpressure() {
        init_tracing();

        let config = Arc::new(Config {
            batch_size: 10,
            timeout: Duration::from_millis(100),
            worker_num: 4,
        });

        let bucket: Bucket<i32> = Bucket::new(config);
        let bucket_clone = bucket.clone();
        let cancel = CancellationToken::new();

        let counter = Arc::new(AtomicUsize::new(0));
        let processor = CountingProcessor {
            counter: Arc::clone(&counter),
        };

        let start = std::time::Instant::now();

        let cancel_clone = cancel.clone();
        let producer = tokio::spawn(async move {
            for i in 0..1000 {
                bucket_clone.consume(&cancel_clone, i).await.unwrap();
            }
            bucket_clone.close();
        });

        let processor_handle = bucket.run(&cancel, processor);

        let (_, _) = tokio::join!(producer, processor_handle);

        let elapsed = start.elapsed();
        assert_eq!(counter.load(Ordering::SeqCst), 1000);

        // Should complete quickly without backpressure delays
        assert!(elapsed < Duration::from_secs(2));
    }
}
