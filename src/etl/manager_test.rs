use super::*;
use crate::etl::ETLPipeline;
use std::error::Error;
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

// Mock implementation of ETLRunner for testing
struct MockETLRunner {
    name: String,
    should_fail: bool,
    execution_order: Arc<Mutex<Vec<String>>>,
    execution_count: Arc<AtomicUsize>,
    delay_ms: u64,
}

#[async_trait::async_trait]
impl ETLRunner for MockETLRunner {
    fn name(&self) -> &str {
        &self.name
    }

    async fn run(
        &self,
        _config: Arc<bucket::Config>,
        cancel: &CancellationToken,
    ) -> Result<(), Box<dyn Error + Send>> {
        // Track execution
        self.execution_count.fetch_add(1, Ordering::SeqCst);
        self.execution_order.lock().unwrap().push(self.name.clone());

        // Simulate work with delay
        if self.delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
        }

        // Check for cancellation
        if cancel.is_cancelled() {
            return Err(Box::new(io::Error::new(
                io::ErrorKind::Interrupted,
                "Pipeline cancelled",
            )));
        }

        // Return success or failure based on configuration
        if self.should_fail {
            Err(Box::new(io::Error::new(
                io::ErrorKind::Other,
                format!("Pipeline {} failed", self.name),
            )))
        } else {
            Ok(())
        }
    }
}

// Helper function to create a test config
fn test_config(worker_num: usize) -> Config {
    Config { worker_num }
}

// Helper function to create a test bucket config
fn test_bucket_config() -> Arc<bucket::Config> {
    Arc::new(
        bucket::ConfigBuilder::default()
            .batch_size(2usize)
            .timeout(Duration::from_millis(100))
            .build()
            .unwrap(),
    )
}

#[tokio::test]
async fn test_run_all_success() {
    let execution_order = Arc::new(Mutex::new(Vec::new()));
    let execution_count = Arc::new(AtomicUsize::new(0));

    let config = test_config(2);
    let bucket_config = test_bucket_config();
    let mut manager = ETLPipelineManager::new(&config, bucket_config);

    // Add multiple successful pipelines
    for i in 1..=3 {
        manager.add_runner(Arc::new(MockETLRunner {
            name: format!("pipeline_{}", i),
            should_fail: false,
            execution_order: execution_order.clone(),
            execution_count: execution_count.clone(),
            delay_ms: 0,
        }));
    }

    let cancel = CancellationToken::new();

    let result = manager.run_all(&cancel).await;
    assert!(result.is_ok());

    // All pipelines should have executed
    assert_eq!(execution_count.load(Ordering::SeqCst), 3);
    assert_eq!(execution_order.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn test_run_all_with_failure() {
    let execution_order = Arc::new(Mutex::new(Vec::new()));
    let execution_count = Arc::new(AtomicUsize::new(0));

    let config = test_config(2);
    let bucket_config = test_bucket_config();
    let mut manager = ETLPipelineManager::new(&config, bucket_config);

    // Add mix of successful and failing pipelines
    manager.add_runner(Arc::new(MockETLRunner {
        name: "pipeline_success".to_string(),
        should_fail: false,
        execution_order: execution_order.clone(),
        execution_count: execution_count.clone(),
        delay_ms: 0,
    }));

    manager.add_runner(Arc::new(MockETLRunner {
        name: "pipeline_fail".to_string(),
        should_fail: true,
        execution_order: execution_order.clone(),
        execution_count: execution_count.clone(),
        delay_ms: 0,
    }));

    let cancel = CancellationToken::new();

    let result = manager.run_all(&cancel).await;

    // Should return error
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("pipeline_fail"));

    // Both pipelines should still execute (parallel execution)
    assert_eq!(execution_count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_run_all_with_cancellation() {
    let execution_order = Arc::new(Mutex::new(Vec::new()));
    let execution_count = Arc::new(AtomicUsize::new(0));

    let config = test_config(1);
    let bucket_config = test_bucket_config();
    let mut manager = ETLPipelineManager::new(&config, bucket_config);
    let cancel = CancellationToken::new();

    // Add pipeline that checks cancellation
    manager.add_runner(Arc::new(MockETLRunner {
        name: "pipeline_cancelled".to_string(),
        should_fail: false,
        execution_order: execution_order.clone(),
        execution_count: execution_count.clone(),
        delay_ms: 0,
    }));

    // Cancel before running
    cancel.cancel();

    let result = manager.run_all(&cancel).await;

    // Should return cancellation error
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cancelled"));
}

#[tokio::test]
async fn test_run_all_parallel_execution() {
    let execution_order = Arc::new(Mutex::new(Vec::new()));
    let execution_count = Arc::new(AtomicUsize::new(0));

    let config = test_config(3); // 3 workers for true parallel execution
    let bucket_config = test_bucket_config();
    let mut manager = ETLPipelineManager::new(&config, bucket_config);

    // Add pipelines with different delays to test parallelism
    for i in 1..=3 {
        manager.add_runner(Arc::new(MockETLRunner {
            name: format!("pipeline_{}", i),
            should_fail: false,
            execution_order: execution_order.clone(),
            execution_count: execution_count.clone(),
            delay_ms: (4 - i) * 10, // Reverse delays: 30ms, 20ms, 10ms
        }));
    }

    let cancel = CancellationToken::new();

    let start = std::time::Instant::now();
    let result = manager.run_all(&cancel).await;
    let duration = start.elapsed();

    assert!(result.is_ok());

    // All should execute
    assert_eq!(execution_count.load(Ordering::SeqCst), 3);

    // With parallel execution, should take ~30ms (max delay), not 60ms (sum)
    // Adding some buffer for execution overhead
    assert!(duration.as_millis() < 50);
}

#[tokio::test]
async fn test_run_all_empty_manager() {
    let config = test_config(1);
    let bucket_config = test_bucket_config();
    let manager = ETLPipelineManager::new(&config, bucket_config);
    let cancel = CancellationToken::new();

    let result = manager.run_all(&cancel).await;

    // Should succeed with no pipelines
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_run_all_single_worker() {
    let execution_order = Arc::new(Mutex::new(Vec::new()));
    let execution_count = Arc::new(AtomicUsize::new(0));

    let config = test_config(1); // Only 1 worker
    let bucket_config = test_bucket_config();
    let mut manager = ETLPipelineManager::new(&config, bucket_config);

    // Add multiple pipelines
    for i in 1..=3 {
        manager.add_runner(Arc::new(MockETLRunner {
            name: format!("pipeline_{}", i),
            should_fail: false,
            execution_order: execution_order.clone(),
            execution_count: execution_count.clone(),
            delay_ms: 10,
        }));
    }

    let cancel = CancellationToken::new();

    let start = std::time::Instant::now();
    let result = manager.run_all(&cancel).await;
    let duration = start.elapsed();

    assert!(result.is_ok());
    assert_eq!(execution_count.load(Ordering::SeqCst), 3);

    // With 1 worker, should take ~30ms (sequential execution)
    assert!(duration.as_millis() >= 30);
}

#[tokio::test]
async fn test_multiple_failures_reports_first() {
    let execution_order = Arc::new(Mutex::new(Vec::new()));
    let execution_count = Arc::new(AtomicUsize::new(0));

    let config = test_config(3);
    let bucket_config = test_bucket_config();
    let mut manager = ETLPipelineManager::new(&config, bucket_config);

    // Add multiple failing pipelines
    for i in 1..=3 {
        manager.add_runner(Arc::new(MockETLRunner {
            name: format!("failing_pipeline_{}", i),
            should_fail: true,
            execution_order: execution_order.clone(),
            execution_count: execution_count.clone(),
            delay_ms: 0,
        }));
    }

    let cancel = CancellationToken::new();

    let result = manager.run_all(&cancel).await;

    assert!(result.is_err());
    // All pipelines should still execute
    assert_eq!(execution_count.load(Ordering::SeqCst), 3);
}

#[test]
fn test_config_creation() {
    let config = test_config(4);
    assert_eq!(config.worker_num, 4);

    // Test Config::new()
    let config2 = Config::new(5);
    assert_eq!(config2.worker_num, 5);

    // Test default
    let config3 = Config::default();
    assert_eq!(config3.worker_num, 4);
}

#[test]
fn test_manager_add_runner() {
    let config = test_config(1);
    let bucket_config = test_bucket_config();
    let mut manager = ETLPipelineManager::new(&config, bucket_config);
    let execution_order = Arc::new(Mutex::new(Vec::new()));
    let execution_count = Arc::new(AtomicUsize::new(0));

    manager.add_runner(Arc::new(MockETLRunner {
        name: "test".to_string(),
        should_fail: false,
        execution_order,
        execution_count,
        delay_ms: 0,
    }));

    assert_eq!(manager.etl_runners.len(), 1);
}

// Mock implementation of ETLPipeline for testing
struct MockETLPipeline {
    name: String,
    extract_count: Arc<AtomicUsize>,
    transform_count: Arc<AtomicUsize>,
    load_count: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl ETLPipeline<i32, String> for MockETLPipeline {
    async fn extract(
        &self,
        _cancel: &CancellationToken,
    ) -> Result<mpsc::Receiver<i32>, Box<dyn Error>> {
        self.extract_count.fetch_add(1, Ordering::SeqCst);

        let (tx, rx) = mpsc::channel(10);

        // Send some test data
        tokio::spawn(async move {
            for i in 1..=3 {
                tx.send(i).await.ok();
            }
        });

        Ok(rx)
    }

    async fn transform(&self, _cancel: &CancellationToken, item: &i32) -> String {
        self.transform_count.fetch_add(1, Ordering::SeqCst);
        format!("transformed_{}", item)
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

#[tokio::test]
async fn test_etl_pipeline_adapter() {
    let config = test_config(2);
    let bucket_config = test_bucket_config();
    let mut manager = ETLPipelineManager::new(&config, bucket_config);

    let extract_count = Arc::new(AtomicUsize::new(0));
    let transform_count = Arc::new(AtomicUsize::new(0));
    let load_count = Arc::new(AtomicUsize::new(0));

    let pipeline = MockETLPipeline {
        name: "test_etl".to_string(),
        extract_count: extract_count.clone(),
        transform_count: transform_count.clone(),
        load_count: load_count.clone(),
    };

    // Add ETL pipeline using the add_pipeline method
    let bucket_config = bucket::ConfigBuilder::default()
        .batch_size(2usize)
        .timeout(Duration::from_secs(1))
        .build()
        .unwrap();

    manager.add_pipeline(Box::new(pipeline), "test_etl".to_string(), bucket_config);

    assert_eq!(manager.etl_runners.len(), 1);

    let cancel = CancellationToken::new();

    let result = manager.run_all(&cancel).await;
    assert!(result.is_ok());

    // Verify that the pipeline was executed
    assert_eq!(extract_count.load(Ordering::SeqCst), 1);
    assert_eq!(transform_count.load(Ordering::SeqCst), 3); // 3 items transformed
    assert_eq!(load_count.load(Ordering::SeqCst), 3); // 3 items loaded
}

#[tokio::test]
async fn test_multiple_etl_pipelines() {
    let config = test_config(3);
    let bucket_config = test_bucket_config();
    let mut manager = ETLPipelineManager::new(&config, bucket_config);

    // Add multiple ETL pipelines with different types
    for i in 1..=2 {
        let extract_count = Arc::new(AtomicUsize::new(0));
        let transform_count = Arc::new(AtomicUsize::new(0));
        let load_count = Arc::new(AtomicUsize::new(0));

        let pipeline = MockETLPipeline {
            name: format!("etl_pipeline_{}", i),
            extract_count,
            transform_count,
            load_count,
        };

        let bucket_config = bucket::ConfigBuilder::default()
            .batch_size(3usize)
            .timeout(Duration::from_millis(500))
            .build()
            .unwrap();

        manager.add_pipeline(
            Box::new(pipeline),
            format!("etl_pipeline_{}", i),
            bucket_config,
        );
    }

    // Also add a regular ETLRunner
    let execution_order = Arc::new(Mutex::new(Vec::new()));
    let execution_count = Arc::new(AtomicUsize::new(0));

    manager.add_runner(Arc::new(MockETLRunner {
        name: "regular_runner".to_string(),
        should_fail: false,
        execution_order,
        execution_count: execution_count.clone(),
        delay_ms: 0,
    }));

    assert_eq!(manager.etl_runners.len(), 3);

    let cancel = CancellationToken::new();

    let result = manager.run_all(&cancel).await;
    assert!(result.is_ok());

    // Verify regular runner was also executed
    assert_eq!(execution_count.load(Ordering::SeqCst), 1);
}

#[test]
fn test_etl_pipeline_adapter_creation() {
    use std::time::Duration;

    let pipeline = MockETLPipeline {
        name: "test".to_string(),
        extract_count: Arc::new(AtomicUsize::new(0)),
        transform_count: Arc::new(AtomicUsize::new(0)),
        load_count: Arc::new(AtomicUsize::new(0)),
    };

    let bucket_config = bucket::ConfigBuilder::default()
        .batch_size(10usize)
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    let adapter = ETLPipelineAdapter::new(
        Box::new(pipeline),
        "test_adapter".to_string(),
        bucket_config.clone(),
    );

    assert_eq!(adapter.name(), "test_adapter");
    assert_eq!(adapter.config.batch_size(), 10);
    assert_eq!(adapter.config.timeout(), Duration::from_secs(5));
}
