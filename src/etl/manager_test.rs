use super::*;
use crate::etl::ETLPipeline;
use std::error::Error;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

// Helper function to create test config
fn test_config(worker_num: usize) -> Config {
    Config { worker_num }
}

// Helper function to create test bucket config
fn test_bucket_config() -> bucket::Config {
    bucket::ConfigBuilder::default()
        .batch_size(2usize)
        .timeout(Duration::from_millis(100))
        .build()
        .unwrap()
}

#[test]
fn test_config_creation() {
    let config = test_config(4);
    assert_eq!(config.worker_num, 4);

    let config2 = Config::new(5);
    assert_eq!(config2.worker_num, 5);

    let config3 = Config::default();
    assert_eq!(config3.worker_num, 4);
}

#[test]
fn test_manager_creation() {
    let config = test_config(2);
    let bucket_config = test_bucket_config();
    let manager = ETLPipelineManager::new(&config, bucket_config);

    assert_eq!(manager.etl_runners.len(), 0);
}
