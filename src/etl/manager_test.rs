use super::*;
use crate::etl::ETLPipeline;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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

// Mock ETLPipeline for testing adapter
struct MockPipeline {
    extract_count: Arc<AtomicUsize>,
    transform_count: Arc<AtomicUsize>,
    load_count: Arc<AtomicUsize>,
}

impl MockPipeline {
    fn new() -> Self {
        MockPipeline {
            extract_count: Arc::new(AtomicUsize::new(0)),
            transform_count: Arc::new(AtomicUsize::new(0)),
            load_count: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait::async_trait]
impl ETLPipeline<i32, String> for MockPipeline {
    async fn extract(
        &self,
        _cancel: &CancellationToken,
    ) -> Result<mpsc::Receiver<i32>, Box<dyn Error>> {
        self.extract_count.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel(10);

        tokio::spawn(async move {
            for i in 1..=3 {
                tx.send(i).await.ok();
            }
        });

        Ok(rx)
    }

    async fn transform(&self, _cancel: &CancellationToken, item: &i32) -> String {
        self.transform_count.fetch_add(1, Ordering::SeqCst);
        format!("item_{}", item)
    }

    async fn load(
        &self,
        _cancel: &CancellationToken,
        items: Vec<String>,
    ) -> Result<(), Box<dyn Error>> {
        self.load_count.fetch_add(items.len(), Ordering::SeqCst);
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

#[tokio::test]
async fn test_run_all_with_failure() {
    let config = test_config(2);
    let bucket_config = test_bucket_config();
    let mut manager = ETLPipelineManager::new(&config, bucket_config);

    manager.add_runner(Arc::new(MockETLRunner::new("success")));
    manager.add_runner(Arc::new(MockETLRunner::new("fail").with_failure()));

    let cancel = CancellationToken::new();
    let result = manager.run_all(&cancel).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ETLError::PipelineExecution(_, _)));
}

#[tokio::test]
async fn test_run_all_with_cancellation() {
    let config = test_config(1);
    let bucket_config = test_bucket_config();
    let mut manager = ETLPipelineManager::new(&config, bucket_config);

    // Add runner with delay to ensure cancellation happens during execution
    manager.add_runner(Arc::new(MockETLRunner::new("delayed").with_delay(100)));

    let cancel = CancellationToken::new();
    cancel.cancel();

    let result = manager.run_all(&cancel).await;

    assert!(result.is_err());
    if let Err(ETLError::Cancelled) = result {
        // Expected
    } else {
        panic!("Expected cancellation error");
    }
}

#[tokio::test]
async fn test_run_all_parallel_execution() {
    let config = test_config(3);
    let bucket_config = test_bucket_config();
    let mut manager = ETLPipelineManager::new(&config, bucket_config);

    // Add 3 runners with delays
    for i in 1..=3 {
        manager.add_runner(Arc::new(
            MockETLRunner::new(format!("delayed{}", i)).with_delay(20),
        ));
    }

    let cancel = CancellationToken::new();
    let start = std::time::Instant::now();
    let result = manager.run_all(&cancel).await;
    let duration = start.elapsed();

    assert!(result.is_ok());
    // With 3 workers running in parallel, should take ~20ms, not 60ms
    assert!(duration.as_millis() < 50);
}

#[tokio::test]
async fn test_add_pipeline() {
    let config = test_config(2);
    let bucket_config = test_bucket_config();
    let mut manager = ETLPipelineManager::new(&config, bucket_config);

    let pipeline = Box::new(MockPipeline::new());
    manager.add_pipeline(pipeline, "test_pipeline".to_string());

    assert_eq!(manager.etl_runners.len(), 1);
}

#[tokio::test]
async fn test_pipeline_execution() {
    let config = test_config(2);
    let bucket_config = test_bucket_config();
    let mut manager = ETLPipelineManager::new(&config, bucket_config);

    let pipeline = MockPipeline::new();
    let extract_check = Arc::clone(&pipeline.extract_count);
    let transform_check = Arc::clone(&pipeline.transform_count);
    let load_check = Arc::clone(&pipeline.load_count);

    manager.add_pipeline(Box::new(pipeline), "test_etl".to_string());

    let cancel = CancellationToken::new();
    let result = manager.run_all(&cancel).await;

    assert!(result.is_ok());
    assert_eq!(extract_check.load(Ordering::SeqCst), 1);
    assert_eq!(transform_check.load(Ordering::SeqCst), 3);
    assert_eq!(load_check.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_run_all_four_pipelines_concurrently() {
    let config = test_config(4);
    let bucket_config = test_bucket_config();
    let mut manager = ETLPipelineManager::new(&config, bucket_config);

    // Add 4 different mock pipelines
    let pipelines: Vec<_> = (1..=4)
        .map(|_i| {
            let pipeline = MockPipeline::new();
            let extract_count = Arc::clone(&pipeline.extract_count);
            let transform_count = Arc::clone(&pipeline.transform_count);
            let load_count = Arc::clone(&pipeline.load_count);
            (pipeline, extract_count, transform_count, load_count)
        })
        .collect();

    let counters: Vec<_> = pipelines
        .into_iter()
        .enumerate()
        .map(|(i, (pipeline, e, t, l))| {
            manager.add_pipeline(Box::new(pipeline), format!("pipeline_{}", i + 1));
            (e, t, l)
        })
        .collect();

    let cancel = CancellationToken::new();
    let result = manager.run_all(&cancel).await;

    assert!(result.is_ok());

    // Verify all 4 pipelines executed
    for (i, (extract, transform, load)) in counters.iter().enumerate() {
        assert_eq!(
            extract.load(Ordering::SeqCst),
            1,
            "Pipeline {} extract count",
            i + 1
        );
        assert_eq!(
            transform.load(Ordering::SeqCst),
            3,
            "Pipeline {} transform count",
            i + 1
        );
        assert_eq!(
            load.load(Ordering::SeqCst),
            3,
            "Pipeline {} load count",
            i + 1
        );
    }
}

#[tokio::test]
async fn test_run_all_five_mixed_runners() {
    let config = test_config(5);
    let bucket_config = test_bucket_config();
    let mut manager = ETLPipelineManager::new(&config, bucket_config);

    // Add 3 ETL pipelines
    let mut pipeline_counters = Vec::new();
    for i in 1..=3 {
        let pipeline = MockPipeline::new();
        let extract_count = Arc::clone(&pipeline.extract_count);
        let transform_count = Arc::clone(&pipeline.transform_count);
        let load_count = Arc::clone(&pipeline.load_count);
        pipeline_counters.push((extract_count, transform_count, load_count));
        manager.add_pipeline(Box::new(pipeline), format!("pipeline_{}", i));
    }

    // Add 2 mock runners
    let runner1 = Arc::new(MockETLRunner::new("runner_1"));
    let runner2 = Arc::new(MockETLRunner::new("runner_2"));
    let runner1_check = Arc::clone(&runner1);
    let runner2_check = Arc::clone(&runner2);

    manager.add_runner(runner1);
    manager.add_runner(runner2);

    let cancel = CancellationToken::new();
    let start = std::time::Instant::now();
    let result = manager.run_all(&cancel).await;
    let duration = start.elapsed();

    assert!(result.is_ok());

    // Verify all 3 pipelines executed
    for (i, (extract, transform, load)) in pipeline_counters.iter().enumerate() {
        assert_eq!(extract.load(Ordering::SeqCst), 1, "Pipeline {} extract", i + 1);
        assert_eq!(transform.load(Ordering::SeqCst), 3, "Pipeline {} transform", i + 1);
        assert_eq!(load.load(Ordering::SeqCst), 3, "Pipeline {} load", i + 1);
    }

    // Verify both runners executed
    assert!(runner1_check.was_executed(), "Runner 1 should execute");
    assert!(runner2_check.was_executed(), "Runner 2 should execute");

    // With 5 workers, should complete quickly (parallel execution)
    assert!(duration.as_millis() < 2000, "Should complete in under 2s");
}

#[tokio::test]
async fn test_run_all_five_pipelines_with_delays() {
    let config = test_config(5);
    let bucket_config = test_bucket_config();
    let mut manager = ETLPipelineManager::new(&config, bucket_config);

    // Add 5 runners with varying delays
    let delays = vec![50, 30, 40, 20, 10];
    let runners: Vec<_> = delays
        .iter()
        .enumerate()
        .map(|(i, &delay)| {
            Arc::new(
                MockETLRunner::new(format!("delayed_runner_{}", i + 1)).with_delay(delay)
            )
        })
        .collect();

    let runner_checks: Vec<_> = runners.iter().map(Arc::clone).collect();

    for runner in runners {
        manager.add_runner(runner);
    }

    let cancel = CancellationToken::new();
    let start = std::time::Instant::now();
    let result = manager.run_all(&cancel).await;
    let duration = start.elapsed();

    assert!(result.is_ok());

    // Verify all 5 runners executed
    for (i, runner) in runner_checks.iter().enumerate() {
        assert!(runner.was_executed(), "Runner {} should execute", i + 1);
    }

    // With 5 workers running in parallel, should take ~50ms (max delay), not 150ms (sum)
    assert!(
        duration.as_millis() < 100,
        "Should complete in under 100ms with parallel execution (was {}ms)",
        duration.as_millis()
    );
}

#[tokio::test]
async fn test_run_all_four_pipelines_one_fails() {
    let config = test_config(4);
    let bucket_config = test_bucket_config();
    let mut manager = ETLPipelineManager::new(&config, bucket_config);

    // Add 3 successful runners and 1 failing runner
    manager.add_runner(Arc::new(MockETLRunner::new("success_1")));
    manager.add_runner(Arc::new(MockETLRunner::new("success_2")));
    manager.add_runner(Arc::new(MockETLRunner::new("fail").with_failure()));
    manager.add_runner(Arc::new(MockETLRunner::new("success_3")));

    let cancel = CancellationToken::new();
    let result = manager.run_all(&cancel).await;

    // Should return error from the failing pipeline
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ETLError::PipelineExecution(_, _)));
}

#[tokio::test]
async fn test_run_all_mixed_four_pipelines_two_runners() {
    let config = test_config(6);
    let bucket_config = test_bucket_config();
    let mut manager = ETLPipelineManager::new(&config, bucket_config);

    // Add 2 pipelines
    let pipeline1 = MockPipeline::new();
    let p1_extract = Arc::clone(&pipeline1.extract_count);
    let p1_transform = Arc::clone(&pipeline1.transform_count);

    let pipeline2 = MockPipeline::new();
    let p2_extract = Arc::clone(&pipeline2.extract_count);
    let p2_transform = Arc::clone(&pipeline2.transform_count);

    manager.add_pipeline(Box::new(pipeline1), "etl_pipeline_1".to_string());
    manager.add_pipeline(Box::new(pipeline2), "etl_pipeline_2".to_string());

    // Add 2 mock runners
    let runner1 = Arc::new(MockETLRunner::new("runner_1").with_delay(10));
    let runner2 = Arc::new(MockETLRunner::new("runner_2").with_delay(15));
    let r1_check = Arc::clone(&runner1);
    let r2_check = Arc::clone(&runner2);

    manager.add_runner(runner1);
    manager.add_runner(runner2);

    let cancel = CancellationToken::new();
    let result = manager.run_all(&cancel).await;

    assert!(result.is_ok());

    // Verify pipelines executed
    assert_eq!(p1_extract.load(Ordering::SeqCst), 1);
    assert_eq!(p1_transform.load(Ordering::SeqCst), 3);
    assert_eq!(p2_extract.load(Ordering::SeqCst), 1);
    assert_eq!(p2_transform.load(Ordering::SeqCst), 3);

    // Verify runners executed
    assert!(r1_check.was_executed());
    assert!(r2_check.was_executed());
}
