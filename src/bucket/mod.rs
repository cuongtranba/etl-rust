use async_trait::async_trait;
use derive_builder::Builder;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub enum BucketError {
    ProcessorError(String),
    ChannelClosed,
    Cancelled,
}

// BucketError is automatically Send + Sync because:
// - String is Send + Sync
// - All variants contain only Send + Sync types

impl std::fmt::Display for BucketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BucketError::ProcessorError(msg) => write!(f, "ProcessorError: {}", msg),
            BucketError::ChannelClosed => write!(f, "ChannelClosed"),
            BucketError::Cancelled => write!(f, "Cancelled"),
        }
    }
}

impl std::error::Error for BucketError {}

// Note: Rust automatically implements From<BucketError> for Box<dyn Error>
// because BucketError implements std::error::Error trait.
// No manual implementation needed!

#[async_trait]
pub trait Processor<T>: Send + Sync {
    async fn process(&self, cancel: &CancellationToken, items: &[T]) -> Result<(), BucketError>;
}

#[async_trait]
impl<T, F, Fut> Processor<T> for F
where
    F: Fn(&CancellationToken, &[T]) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<(), BucketError>> + Send,
    T: Send + Sync,
{
    async fn process(&self, ctx: &CancellationToken, items: &[T]) -> Result<(), BucketError> {
        self(ctx, items).await
    }
}

#[derive(Debug, Clone, Builder)]
#[builder(setter(into))]
pub struct Config {
    #[builder(default = "1")]
    pub batch_size: usize,

    #[builder(default = "Duration::from_secs(5)")]
    pub timeout: Duration,

    #[builder(default = "1")]
    pub worker_num: usize,
}

pub struct Bucket<T> {
    config: Arc<Config>,
    sender: mpsc::Sender<T>,
    receiver: Arc<tokio::sync::Mutex<mpsc::Receiver<T>>>,
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
            sender,
            receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
        }
    }

    pub async fn consume(&self, item: T) -> Result<(), mpsc::error::SendError<T>> {
        self.sender.send(item).await
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
        receiver: Arc<tokio::sync::Mutex<mpsc::Receiver<T>>>,
        process: Arc<P>,
        cancel_token: &CancellationToken,
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
mod tests;
