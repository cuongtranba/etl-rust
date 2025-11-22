use async_trait::async_trait;
use futures::StreamExt;
use mpsc::Receiver;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::bucket::{Bucket, BucketError, Config};

/// Defines an ETL (Extract-Transform-Load) pipeline.
///
/// # Type Parameters
///
/// * `E` - Type of items extracted from the source
/// * `T` - Type of items after transformation
///
/// # Lifecycle
///
/// 1. `pre_process()` - Setup before extraction
/// 2. `extract()` - Produce items from source
/// 3. `transform()` - Convert each extracted item
/// 4. `load()` - Persist batches of transformed items
/// 5. `post_process()` - Cleanup after processing
#[async_trait]
pub trait ETLPipeline<E, T>
where
    E: Send,
{
    /// Extracts items from the data source.
    ///
    /// Returns a receiver channel that yields items to process.
    async fn extract(&self, cancel: &CancellationToken) -> Result<Receiver<E>, Box<dyn Error>>;

    /// Transforms a single extracted item.
    ///
    /// Called concurrently for each item in a batch.
    async fn transform(&self, cancel: &CancellationToken, item: &E) -> T;

    /// Loads a batch of transformed items to the destination.
    ///
    /// Called once per batch after all transforms complete.
    async fn load(&self, cancel: &CancellationToken, items: Vec<T>) -> Result<(), Box<dyn Error>>;

    /// Pre-processing hook called before extraction starts.
    async fn pre_process(&self, cancel: &CancellationToken) -> Result<(), Box<dyn Error>>;

    /// Post-processing hook called after all items are loaded.
    async fn post_process(&self, cancel: &CancellationToken) -> Result<(), Box<dyn Error>>;
}

/// Executor for ETL pipelines with concurrent batch processing.
///
/// Wraps an [`ETLPipeline`] implementation and runs it using a [`Bucket`]
/// for efficient batch processing.
///
/// # Type Parameters
///
/// * `E` - Type of extracted items
/// * `T` - Type of transformed items
pub struct ETL<E, T> {
    etl: Arc<dyn ETLPipeline<E, T> + Send + Sync>,
}

impl<E, T> ETL<E, T>
where
    E: Send + Sync + Clone + 'static,
    T: Send + 'static,
{
    /// Creates a new ETL from a pipeline implementation.
    ///
    /// Accepts an Arc of a pipeline directly or can be extended
    /// to work with other types that can be converted to Arc.
    pub fn new(etl: Arc<dyn ETLPipeline<E, T> + Send + Sync>) -> Self {
        ETL { etl }
    }

    /// Creates a new ETL from a boxed pipeline implementation.
    ///
    /// Convenience constructor for when you have a `Box<dyn ETLPipeline>`.
    pub fn from_box(etl: Box<dyn ETLPipeline<E, T> + Send + Sync>) -> Self {
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
                        // Use Arc to avoid cloning items - zero-copy optimization
                        let items: Arc<[E]> = Arc::from(items);
                        let ctx = ctx.clone();

                        async move {
                            let concurrency =
                                std::cmp::min(items.len(), std::cmp::max(10, num_cpus::get() * 2));

                            let mut transform_futures = Vec::with_capacity(items.len());
                            for item in items.iter() {
                                let etl = Arc::clone(&etl);
                                let ctx = ctx.clone();
                                transform_futures
                                    .push(async move { etl.transform(&ctx, item).await });
                            }

                            let transforms: Vec<T> = futures::stream::iter(transform_futures)
                                .buffer_unordered(concurrency)
                                .collect()
                                .await;

                            etl.load(&ctx, transforms).await.map_err(|e| {
                                let error_msg = e.to_string();
                                BucketError::ProcessorError(Box::new(std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    error_msg,
                                )))
                            })
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
                            bucket.consume(&cancel, item).await
                                .map_err(|e| Box::new(e) as Box<dyn Error>)?;
                        },
                        None => {
                            break;
                        },
                    }
                }
            }
        }

        // Signal bucket that no more items will be produced
        bucket.close();

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

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPipeline;

    #[async_trait]
    impl ETLPipeline<i32, String> for TestPipeline {
        async fn extract(
            &self,
            _cancel: &CancellationToken,
        ) -> Result<Receiver<i32>, Box<dyn Error>> {
            let (tx, rx) = mpsc::channel(10);
            drop(tx);
            Ok(rx)
        }

        async fn transform(&self, _cancel: &CancellationToken, item: &i32) -> String {
            item.to_string()
        }

        async fn load(
            &self,
            _cancel: &CancellationToken,
            _items: Vec<String>,
        ) -> Result<(), Box<dyn Error>> {
            Ok(())
        }

        async fn pre_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
            Ok(())
        }

        async fn post_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
    }

    #[test]
    fn test_etl_new_with_arc() {
        let arc = Arc::new(TestPipeline) as Arc<dyn ETLPipeline<i32, String> + Send + Sync>;
        let _etl: ETL<i32, String> = ETL::new(arc);
    }

    #[test]
    fn test_etl_from_box() {
        let _etl: ETL<i32, String> = ETL::from_box(Box::new(TestPipeline));
    }
}
