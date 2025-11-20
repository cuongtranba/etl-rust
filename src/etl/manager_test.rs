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

// Simple mock ETLRunner for testing
struct MockETLRunner {
    name: String,
    should_fail: bool,
    delay_ms: u64,
    executed: Arc<AtomicBool>,
}

impl MockETLRunner {
    fn new(name: impl Into<String>) -> Self {
        MockETLRunner {
            name: name.into(),
            should_fail: false,
            delay_ms: 0,
            executed: Arc::new(AtomicBool::new(false)),
        }
    }

    fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }

    fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = delay_ms;
        self
    }

    fn was_executed(&self) -> bool {
        self.executed.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl ETLRunner for MockETLRunner {
    fn name(&self) -> &str {
        &self.name
    }

    async fn run(
        &self,
        _config: &bucket::Config,
        cancel: &CancellationToken,
    ) -> Result<(), ETLError> {
        self.executed.store(true, Ordering::SeqCst);

        if self.delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
        }

        if cancel.is_cancelled() {
            return Err(ETLError::Cancelled);
        }

        if self.should_fail {
            Err(ETLError::PipelineExecution(
                self.name.clone(),
                "mock failure".to_string(),
            ))
        } else {
            Ok(())
        }
    }
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

#[tokio::test]
async fn test_mock_runner_success() {
    let runner = MockETLRunner::new("test");
    let config = test_bucket_config();
    let cancel = CancellationToken::new();

    let result = runner.run(&config, &cancel).await;

    assert!(result.is_ok());
    assert!(runner.was_executed());
}

#[tokio::test]
async fn test_mock_runner_failure() {
    let runner = MockETLRunner::new("test").with_failure();
    let config = test_bucket_config();
    let cancel = CancellationToken::new();

    let result = runner.run(&config, &cancel).await;

    assert!(result.is_err());
    assert!(runner.was_executed());
}

#[tokio::test]
async fn test_run_all_empty_manager() {
    let config = test_config(2);
    let bucket_config = test_bucket_config();
    let manager = ETLPipelineManager::new(&config, bucket_config);
    let cancel = CancellationToken::new();

    let result = manager.run_all(&cancel).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_run_all_single_success() {
    let config = test_config(2);
    let bucket_config = test_bucket_config();
    let mut manager = ETLPipelineManager::new(&config, bucket_config);

    let runner = Arc::new(MockETLRunner::new("test1"));
    let runner_check = Arc::clone(&runner);
    manager.add_runner(runner);

    let cancel = CancellationToken::new();
    let result = manager.run_all(&cancel).await;

    assert!(result.is_ok());
    assert!(runner_check.was_executed());
}

#[tokio::test]
async fn test_run_all_multiple_success() {
    let config = test_config(3);
    let bucket_config = test_bucket_config();
    let mut manager = ETLPipelineManager::new(&config, bucket_config);

    let runners: Vec<_> = (1..=3)
        .map(|i| Arc::new(MockETLRunner::new(format!("test{}", i))))
        .collect();

    let runner_checks: Vec<_> = runners.iter().map(Arc::clone).collect();

    for runner in runners {
        manager.add_runner(runner);
    }

    let cancel = CancellationToken::new();
    let result = manager.run_all(&cancel).await;

    assert!(result.is_ok());
    for runner in runner_checks {
        assert!(runner.was_executed());
    }
}
