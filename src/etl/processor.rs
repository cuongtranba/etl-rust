use async_trait::async_trait;
use futures::future::join_all;
use mpsc::Receiver;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::bucket::{Bucket, BucketError, Config};

#[async_trait]
pub trait Processor<E, T>
where
    E: Send,
{
    async fn extract(&self, cancel: &CancellationToken) -> Result<Receiver<E>, Box<dyn Error>>;
    async fn transform(&self, cancel: &CancellationToken, item: E) -> T;
    async fn load(&self, cancel: &CancellationToken, items: Vec<T>) -> Result<(), Box<dyn Error>>;
    async fn pre_process(&self, cancel: &CancellationToken) -> Result<(), Box<dyn Error>>;
    async fn post_process(&self, cancel: &CancellationToken) -> Result<(), Box<dyn Error>>;
}

pub struct ETL<E, T> {
    pub etl: Arc<dyn Processor<E, T> + Send + Sync>,
}

impl<E, T> ETL<E, T>
where
    E: Send + Sync + Clone + 'static,
    T: Send + 'static,
{
    pub fn new(etl: Box<dyn Processor<E, T> + Send + Sync>) -> Self {
        ETL {
            etl: Arc::from(etl),
        }
    }

    pub async fn pre_process(&self, cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
        self.etl.pre_process(cancel).await
    }

    pub async fn post_process(&self, cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
        self.etl.post_process(cancel).await
    }
    pub async fn run(
        &self,
        config: Arc<Config>,
        cancel: &CancellationToken,
    ) -> Result<(), Box<dyn Error>> {
        let bucket = Bucket::new(config);
        let bucket_clone = bucket.clone();
        let etl_clone = Arc::clone(&self.etl);
        let cancel_clone = cancel.clone();

        let process_handle = tokio::spawn(async move {
            bucket_clone
                .run(
                    &cancel_clone,
                    move |ctx: &CancellationToken, items: &[E]| {
                        let etl = Arc::clone(&etl_clone);
                        let items = items.to_vec();
                        let ctx = ctx.clone();

                        async move {
                            let transform_futures =
                                items.iter().map(|item| etl.transform(&ctx, item.clone()));

                            let transforms: Vec<T> = join_all(transform_futures).await;

                            etl.load(&ctx, transforms)
                                .await
                                .map_err(|e| BucketError::ProcessorError(e.to_string()))
                        }
                    },
                )
                .await
        });

        let mut receiver = self.etl.extract(&cancel).await?;
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    break;
                },
                item = receiver.recv() => {
                    match item {
                        Some(item) => {
                            bucket.consume(item).await
                                .map_err(|e| Box::new(e) as Box<dyn Error>)?;
                        },
                        None => {
                            break;
                        },
                    }
                }
            }
        }

        let process_result = process_handle
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>)?;

        if let Err(e) = process_result {
            return Err(Box::new(e) as Box<dyn Error>);
        }

        self.post_process(cancel).await?;
        Ok(())
    }
}
