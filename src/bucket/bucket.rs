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
    sender: Arc<mpsc::Sender<T>>,
    receiver: Arc<Mutex<mpsc::Receiver<T>>>,
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
    pub fn new(config: Arc<Config>) -> Self {
        let (sender, receiver) = mpsc::channel(config.batch_size);

        Self {
            config,
            sender: Arc::new(sender),
            receiver: Arc::new(Mutex::new(receiver)),
            done: CancellationToken::new(),
        }
    }

    pub async fn consume(&self, item: T) -> Result<(), mpsc::error::SendError<T>> {
        self.sender.send(item).await
    }

    pub fn close(&self) {
        self.done.cancel();
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
            let done_token = self.done.clone();

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
        done_token: &CancellationToken,
        batch_size: usize,
        timeout_duration: Duration,
    ) -> Result<(), BucketError>
    where
        P: Processor<T> + Send + Sync,
        T: Clone,
    {
        let mut queue: Vec<T> = Vec::with_capacity(batch_size);
        let mut ticker = interval(timeout_duration);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    println!("Worker {} shutting down", worker_id);
                    return Self::drain_and_process(receiver, cancel_token, &*process, &mut queue, batch_size).await;
                }

                _ = done_token.cancelled() => {
                    println!("done worker {}", worker_id);
                    return Self::drain_and_process(receiver, cancel_token, &*process, &mut queue, batch_size).await;
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
    async fn drain_and_process<P>(
        receiver: Arc<Mutex<mpsc::Receiver<T>>>,
        ctx: &CancellationToken,
        process: &P,
        queue: &mut Vec<T>,
        batch_size: usize,
    ) -> Result<(), BucketError>
    where
        P: Processor<T>,
    {
        Self::process_queue(ctx, process, queue).await?;
        loop {
            let item = {
                let mut rx = receiver.lock().await;
                rx.try_recv().ok()
            };
            match item {
                Some(item) => {
                    queue.push(item);
                    if queue.len() >= batch_size {
                        Self::process_queue(ctx, process, queue).await?;
                    }
                }
                None => break,
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

    // Processor that collects items into a shared vector
    struct CollectingProcessor {
        items: Arc<tokio::sync::Mutex<Vec<i32>>>,
    }

    #[async_trait]
    impl Processor<i32> for CollectingProcessor {
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

    // Processor that counts processed items
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

    // Processor that tracks batch sizes and can signal completion
    struct BatchTrackingProcessor {
        sizes: Arc<tokio::sync::Mutex<Vec<usize>>>,
        total_processed: Arc<AtomicUsize>,
        done_tx: Arc<tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
        target_count: usize,
    }

    #[async_trait]
    impl Processor<i32> for BatchTrackingProcessor {
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
    struct TimeoutProcessor {
        first_batch_size: Arc<tokio::sync::Mutex<Option<usize>>>,
    }

    #[async_trait]
    impl Processor<i32> for TimeoutProcessor {
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
    struct CancellationCheckProcessor {
        wait_for_signal: Arc<tokio::sync::Notify>,
        items_count: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Processor<i32> for CancellationCheckProcessor {
        async fn process(&self, ctx: &CancellationToken, items: &[i32]) -> Result<(), BucketError> {
            self.items_count.store(items.len(), Ordering::SeqCst);
            self.wait_for_signal.notified().await;

            if ctx.is_cancelled() {
                return Err(BucketError::ProcessorError("exit".to_string()));
            }
            Ok(())
        }
    }

    const ITEM_NUMS: usize = 100;

    #[tokio::test]
    async fn test_should_process_full() {
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

        tokio::spawn(async move {
            for i in 0..ITEM_NUMS {
                bucket_clone.consume(i as i32).await.unwrap();
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
}
