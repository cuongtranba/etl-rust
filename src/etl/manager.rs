use std::{error::Error, sync::Arc};

use derive_builder::Builder;
use std::sync::mpsc::channel;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use crate::bucket;
use crate::etl::{ETL, ETLPipeline};

/// Configuration for ETL pipeline execution
#[derive(Clone, Builder)]
pub struct Config {
    /// Maximum number of worker threads for parallel pipeline execution
    #[builder(default = "4")]
    pub worker_num: usize,
}

impl Config {
    /// Creates a new Config with specified number of workers
    pub fn new(worker_num: usize) -> Self {
        Config { worker_num }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config { worker_num: 4 }
    }
}

#[async_trait::async_trait]
pub trait ETLRunner: Send + Sync {
    fn name(&self) -> &str;
    async fn run(
        &self,
        config: Arc<bucket::Config>,
        cancel: &CancellationToken,
    ) -> Result<(), Box<dyn Error + Send>>;
}

pub struct ETLPipelineManager {
    etl_runners: Vec<Arc<dyn ETLRunner + Send + Sync>>,
    cfg: Config,
    bucket_config: Arc<bucket::Config>,
}

impl ETLPipelineManager {
    pub fn new(cfg: &Config, bucket_config: Arc<bucket::Config>) -> Self {
        ETLPipelineManager {
            etl_runners: Vec::new(),
            cfg: cfg.clone(),
            bucket_config,
        }
    }

    /// Add an ETL pipeline with specific types E and T
    /// This is a convenience method that wraps the ETLPipeline in an adapter
    pub fn add_pipeline<E, T>(
        &mut self,
        pipeline: Box<dyn ETLPipeline<E, T> + Send + Sync>,
        name: String,
        config: bucket::Config,
    ) where
        E: Send + Sync + Clone + 'static,
        T: Send + 'static,
    {
        let adapter = ETLPipelineAdapter::new(pipeline, name, config);
        self.etl_runners.push(Arc::new(adapter));
    }

    /// Add an ETLRunner directly to the manager
    pub fn add_runner(&mut self, runner: Arc<dyn ETLRunner>) {
        self.etl_runners.push(runner);
    }

    pub async fn run_all(&self, cancel: &CancellationToken) -> Result<(), Box<dyn Error + Send>> {
        let semaphore = Arc::new(Semaphore::new(self.cfg.worker_num));
        let (tx, rx) = channel();

        for runner in &self.etl_runners {
            let runner = Arc::clone(runner);
            let config = Arc::clone(&self.bucket_config);
            let cancel = cancel.clone();
            let tx = tx.clone();
            let sem = Arc::clone(&semaphore);

            tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let result = runner.run(config, &cancel).await;
                let _ = tx.send(result);
            });
        }

        drop(tx);
        for result in rx {
            result?;
        }
        Ok(())
    }
}

/// Adapter to make ETLPipeline<E, T> work with ETLRunner trait
pub struct ETLPipelineAdapter<E, T> {
    etl: Arc<ETL<E, T>>,
    name: String,
    config: bucket::Config,
}

impl<E, T> ETLPipelineAdapter<E, T>
where
    E: Send + Sync + Clone + 'static,
    T: Send + 'static,
{
    pub fn new(
        pipeline: Box<dyn ETLPipeline<E, T> + Send + Sync>,
        name: String,
        config: bucket::Config,
    ) -> Self {
        ETLPipelineAdapter {
            etl: Arc::new(ETL::from_box(pipeline)),
            name,
            config,
        }
    }
}

#[async_trait::async_trait]
impl<E, T> ETLRunner for ETLPipelineAdapter<E, T>
where
    E: Send + Sync + Clone + 'static,
    T: Send + 'static,
{
    fn name(&self) -> &str {
        &self.name
    }

    async fn run(
        &self,
        config: Arc<bucket::Config>,
        cancel: &CancellationToken,
    ) -> Result<(), Box<dyn Error + Send>> {
        let bucket_config = bucket::ConfigBuilder::default()
            .batch_size(self.config.batch_size())
            .timeout(self.config.timeout())
            .worker_num(config.worker_num)
            .build()
            .map_err(|e| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    e.to_string(),
                )) as Box<dyn Error + Send>
            })?;

        // Run pre_process
        self.etl
            .pre_process(cancel)
            .await
            .map_err(|e| -> Box<dyn Error + Send> {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        self.etl
            .run(Arc::new(bucket_config), cancel)
            .await
            .map_err(|e| -> Box<dyn Error + Send> {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        Ok(())
    }
}

#[cfg(test)]
#[path = "manager_test.rs"]
mod tests;
