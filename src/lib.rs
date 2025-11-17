use derive_builder::Builder;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct Context {
    cancel_token: CancellationToken,
}

impl Context {
    pub fn new() -> Self {
        Self {
            cancel_token: CancellationToken::new(),
        }
    }

    pub fn with_cancel_token(token: CancellationToken) -> Self {
        Self {
            cancel_token: token,
        }
    }

    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }
}

pub trait Processor<T> {
    fn process(&self, ctx: &Context, items: &[T]) -> Result<(), Box<dyn Error + Send + Sync>>;
}

impl<T, F> Processor<T> for F
where
    F: Fn(&Context, &[T]) -> Result<(), Box<dyn Error + Send + Sync>>,
{
    fn process(&self, ctx: &Context, items: &[T]) -> Result<(), Box<dyn Error + Send + Sync>> {
        self(ctx, items)
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
    config: Config,
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
    pub fn new(config: Config) -> Self {
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

    pub async fn run<P>(
        &self,
        ctx: &Context,
        process: P,
    ) -> Result<(), Box<dyn Error + Send + Sync>>
    where
        P: Processor<T> + Send + Sync + 'static,
        T: Clone,
    {
        let process = Arc::new(process);
        let cancel_token = ctx.cancel_token.clone();

        let mut handles = Vec::new();

        for worker_id in 0..self.config.worker_num {
            let receiver = self.receiver.clone();
            let process = process.clone();
            let cancel_token = cancel_token.clone();
            let batch_size = self.config.batch_size;
            let timeout = self.config.timeout;

            let handle = tokio::spawn(async move {
                Self::worker(
                    worker_id,
                    receiver,
                    process,
                    cancel_token,
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
                Err(e) => errors.push(Box::new(e) as Box<dyn Error + Send + Sync>),
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
        cancel_token: CancellationToken,
        batch_size: usize,
        timeout_duration: Duration,
    ) -> Result<(), Box<dyn Error + Send + Sync>>
    where
        P: Processor<T> + Send + Sync,
        T: Clone,
    {
        let mut queue: Vec<T> = Vec::with_capacity(batch_size);
        let mut ticker = interval(timeout_duration);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let ctx = Context::with_cancel_token(cancel_token.clone());

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    println!("Worker {} shutting down", worker_id);
                    return Self::process_queue(&ctx, &*process, &mut queue);
                }

                _ = ticker.tick() => {
                    if !queue.is_empty() {
                        Self::process_queue(&ctx, &*process, &mut queue)?;
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
                                Self::process_queue(&ctx, &*process, &mut queue)?;
                            }
                        }
                        None => {
                            println!("Worker {} channel closed", worker_id);
                            return Self::process_queue(&ctx, &*process, &mut queue);
                        }
                    }
                }
            }
        }
    }

    fn process_queue<P>(
        ctx: &Context,
        process: &P,
        queue: &mut Vec<T>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>
    where
        P: Processor<T>,
    {
        if !queue.is_empty() {
            process.process(ctx, queue)?; // Call trait method
            queue.clear();
        }
        Ok(())
    }
}
