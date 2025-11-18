use mpsc::Receiver;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::bucket::Bucket;
use crate::bucket::Config;

pub trait Processor<E, T>
where
    E: Send,
{
    fn extract(&self, cancel: &CancellationToken) -> Result<Receiver<E>, Box<dyn Error>>;
    fn transform(&self, cancel: &CancellationToken, item: E) -> T;
    fn load(&self, cancel: &CancellationToken, items: Vec<T>) -> Result<(), Box<dyn Error>>;
    fn pre_process(&self, cancel: &CancellationToken) -> Result<(), Box<dyn Error>>;
    fn post_process(&self, cancel: &CancellationToken) -> Result<(), Box<dyn Error>>;
}

pub struct ETL<E, T> {
    pub etl: Arc<dyn Processor<E, T> + Send + Sync>,
}

impl<E, T> ETL<E, T>
where
    E: Send + Sync + Clone + 'static,
    T: 'static,
{
    pub fn new(etl: Box<dyn Processor<E, T> + Send + Sync>) -> Self {
        ETL {
            etl: Arc::from(etl),
        }
    }

    pub fn pre_process(&self, cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
        self.etl.pre_process(cancel)
    }

    pub fn post_process(&self, cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
        self.etl.post_process(cancel)
    }
    pub async fn run(
        &self,
        config: Arc<Config>,
        cancel: &CancellationToken,
    ) -> Result<(), Box<dyn Error>> {
        let bucket = Bucket::new(config);
        let bucket_clone = bucket.clone();
        let cancel_clone = cancel.clone();
        let mut receiver = self.etl.extract(&cancel)?;

        let extract_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel_clone.cancelled() => {
                        break;
                    },
                    item = receiver.recv() => {
                        match item {
                            Some(item) => {
                                if let Err(e) = bucket_clone.consume(item).await {
                                    return Err(Box::new(e) as Box<dyn Error + Send + Sync>);
                                }
                            },
                            None => {
                                break;
                            },
                        }
                    }
                }
            }
            Ok::<(), Box<dyn Error + Send + Sync>>(())
        });
        let etl_clone = Arc::clone(&self.etl);
        let cancel_for_closure = cancel.clone();

        bucket
            .run(&cancel, move |_ctx: &CancellationToken, items: &[E]| {
                let transforms: Vec<T> = items
                    .iter()
                    .map(|item| etl_clone.transform(&cancel_for_closure, item.clone()))
                    .collect();
                etl_clone
                    .load(&cancel_for_closure, transforms)
                    .map_err(|e| {
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e.to_string(),
                        )) as Box<dyn std::error::Error + Send + Sync>
                    })
            })
            .await
            .map_err(|e| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Bucket run failed: {}", e),
                )) as Box<dyn Error>
            })?;

        let extract_result = extract_handle
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>)?;

        if let Err(e) = extract_result {
            return Err(format!("Extract task failed: {}", e).into());
        }

        self.post_process(cancel)?;
        Ok(())
    }
}
