# ETL Manager Zero-Copy Refactoring Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refactor ETL Manager with zero-copy patterns, proper error types, and async channels to fix hanging tests and improve performance.

**Architecture:** Replace `Box<dyn Error>` with typed `ETLError` enum, use flume channels instead of std::sync::mpsc, remove unnecessary Arc wrapping, simplify config handling by using shared bucket config.

**Tech Stack:** Rust, thiserror, flume, tokio, async-trait

---

## Task 1: Create ETLError Type

**Files:**
- Modify: `src/etl/manager.rs:1-10`

**Step 1: Add ETLError enum to manager.rs**

Add this after the imports at the top of `src/etl/manager.rs`:

```rust
/// Errors that can occur during ETL manager operations
#[derive(Debug, thiserror::Error)]
pub enum ETLError {
    #[error("Pipeline '{0}' execution failed: {1}")]
    PipelineExecution(String, String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Cancellation requested")]
    Cancelled,

    #[error("Worker pool error: {0}")]
    WorkerPool(String),

    #[error("Bucket operation failed: {0}")]
    Bucket(#[from] crate::bucket::BucketError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Build error: {0}")]
    BuildError(String),
}
```

**Step 2: Verify it compiles**

Run: `cargo build --lib`
Expected: SUCCESS (thiserror already in dependencies)

**Step 3: Commit**

```bash
git add src/etl/manager.rs
git commit -m "feat: add ETLError type for manager operations"
```

---

## Task 2: Update ETLRunner Trait Signature

**Files:**
- Modify: `src/etl/manager.rs:32-40`

**Step 1: Update trait to use ETLError and borrow config**

Replace the ETLRunner trait (lines 32-40) with:

```rust
#[async_trait::async_trait]
pub trait ETLRunner: Send + Sync {
    fn name(&self) -> &str;
    async fn run(
        &self,
        config: &bucket::Config,  // Changed from Arc to borrow
        cancel: &CancellationToken,
    ) -> Result<(), ETLError>;  // Changed from Box<dyn Error + Send>
}
```

**Step 2: Verify it compiles (will fail - that's expected)**

Run: `cargo build --lib 2>&1 | head -20`
Expected: FAIL with errors about ETLPipelineAdapter::run signature mismatch

**Step 3: Commit**

```bash
git add src/etl/manager.rs
git commit -m "refactor: update ETLRunner trait to use ETLError and borrow config"
```

---

## Task 3: Update ETLPipelineManager Structure

**Files:**
- Modify: `src/etl/manager.rs:42-55`

**Step 1: Remove Arc from bucket_config field**

Replace the ETLPipelineManager struct (lines 42-46) with:

```rust
pub struct ETLPipelineManager {
    etl_runners: Vec<Arc<dyn ETLRunner + Send + Sync>>,
    cfg: Config,
    bucket_config: bucket::Config,  // Changed from Arc<bucket::Config>
}
```

**Step 2: Update constructor**

Replace the `new` method (lines 48-55) with:

```rust
impl ETLPipelineManager {
    pub fn new(cfg: &Config, bucket_config: bucket::Config) -> Self {
        ETLPipelineManager {
            etl_runners: Vec::new(),
            cfg: cfg.clone(),
            bucket_config,  // Move instead of clone
        }
    }
```

**Step 3: Verify it compiles (will fail - expected)**

Run: `cargo build --lib 2>&1 | head -20`
Expected: FAIL with errors about run_all method and tests

**Step 4: Commit**

```bash
git add src/etl/manager.rs
git commit -m "refactor: remove Arc from bucket_config in ETLPipelineManager"
```

---

## Task 4: Replace std::sync::mpsc with flume in run_all

**Files:**
- Modify: `src/etl/manager.rs:77-100`

**Step 1: Replace channel implementation**

Replace the `run_all` method (lines 77-100) with:

```rust
    pub async fn run_all(&self, cancel: &CancellationToken) -> Result<(), ETLError> {
        let semaphore = Arc::new(Semaphore::new(self.cfg.worker_num));
        let (tx, rx) = flume::bounded(self.etl_runners.len());

        for runner in &self.etl_runners {
            let runner = Arc::clone(runner);
            let config = &self.bucket_config;  // Borrow instead of clone
            let cancel = cancel.clone();
            let tx = tx.clone();
            let sem = Arc::clone(&semaphore);

            tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore poisoned");
                let result = runner.run(config, &cancel).await;
                let _ = tx.send_async(result).await;
            });
        }

        drop(tx);

        // Collect results
        while let Ok(result) = rx.recv_async().await {
            result?;
        }

        Ok(())
    }
}
```

**Step 2: Update imports at top of file**

Change line 4 from:
```rust
use std::sync::mpsc::channel;
```

To:
```rust
// Remove std::sync::mpsc::channel import - using flume instead
```

**Step 3: Verify it compiles (will still fail on adapter)**

Run: `cargo build --lib 2>&1 | head -30`
Expected: FAIL with errors about ETLPipelineAdapter

**Step 4: Commit**

```bash
git add src/etl/manager.rs
git commit -m "refactor: replace std::sync::mpsc with flume in run_all"
```

---

## Task 5: Simplify ETLPipelineAdapter

**Files:**
- Modify: `src/etl/manager.rs:103-127`
- Modify: `src/etl/manager.rs:128-188`

**Step 1: Remove config field and Arc from adapter struct**

Replace ETLPipelineAdapter struct (lines 104-108) with:

```rust
/// Adapter to make ETLPipeline<E, T> work with ETLRunner trait
pub struct ETLPipelineAdapter<E, T> {
    etl: ETL<E, T>,  // Direct storage, no Arc
    name: String,
    // Removed: config field (use shared config from manager)
}
```

**Step 2: Simplify adapter constructor**

Replace the ETLPipelineAdapter::new method (lines 115-125) with:

```rust
impl<E, T> ETLPipelineAdapter<E, T>
where
    E: Send + Sync + Clone + 'static,
    T: Send + 'static,
{
    pub fn new(
        pipeline: Box<dyn ETLPipeline<E, T> + Send + Sync>,
        name: String,
    ) -> Self {
        ETLPipelineAdapter {
            etl: ETL::from_box(pipeline),
            name,
        }
    }
}
```

**Step 3: Update adapter's run implementation**

Replace the ETLRunner impl for adapter (lines 128-188) with:

```rust
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
        config: &bucket::Config,  // Use shared config
        cancel: &CancellationToken,
    ) -> Result<(), ETLError> {
        // Run pre_process
        self.etl
            .pre_process(cancel)
            .await
            .map_err(|e| ETLError::PipelineExecution(self.name.clone(), e.to_string()))?;

        // Run main ETL with Arc config (ETL::run requires Arc)
        self.etl
            .run(Arc::new(config.clone()), cancel)
            .await
            .map_err(|e| ETLError::PipelineExecution(self.name.clone(), e.to_string()))?;

        // Run post_process
        self.etl
            .post_process(cancel)
            .await
            .map_err(|e| ETLError::PipelineExecution(self.name.clone(), e.to_string()))?;

        Ok(())
    }
}
```

**Step 4: Verify it compiles**

Run: `cargo build --lib 2>&1`
Expected: SUCCESS (main code now compiles, tests will fail)

**Step 5: Commit**

```bash
git add src/etl/manager.rs
git commit -m "refactor: simplify ETLPipelineAdapter removing Arc and config field"
```

---

## Task 6: Update add_pipeline Method

**Files:**
- Modify: `src/etl/manager.rs:57-70`

**Step 1: Simplify add_pipeline signature**

Replace the add_pipeline method (lines 57-70) with:

```rust
    /// Add an ETL pipeline with specific types E and T
    /// This is a convenience method that wraps the ETLPipeline in an adapter
    pub fn add_pipeline<E, T>(
        &mut self,
        pipeline: Box<dyn ETLPipeline<E, T> + Send + Sync>,
        name: String,
    ) where
        E: Send + Sync + Clone + 'static,
        T: Send + 'static,
    {
        let adapter = ETLPipelineAdapter::new(pipeline, name);
        self.etl_runners.push(Arc::new(adapter));
    }
```

**Step 2: Verify it compiles**

Run: `cargo build --lib`
Expected: SUCCESS

**Step 3: Commit**

```bash
git add src/etl/manager.rs
git commit -m "refactor: simplify add_pipeline removing config parameter"
```

---

## Task 7: Delete Old Test File

**Files:**
- Delete: `src/etl/manager_test.rs`

**Step 1: Delete the hanging test file**

Run: `git rm src/etl/manager_test.rs`

**Step 2: Verify it compiles**

Run: `cargo build --lib`
Expected: SUCCESS

**Step 3: Commit**

```bash
git commit -m "test: remove old hanging test suite"
```

---

## Task 8: Create New Test Suite - Part 1 (Config and Basic Structure)

**Files:**
- Create: `src/etl/manager_test.rs`

**Step 1: Create test file with imports and helpers**

Create `src/etl/manager_test.rs`:

```rust
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
```

**Step 2: Verify it compiles**

Run: `cargo test --lib manager_test::test_config_creation -- --exact`
Expected: PASS

**Step 3: Commit**

```bash
git add src/etl/manager_test.rs
git commit -m "test: add basic config tests for manager"
```

---

## Task 9: Create New Test Suite - Part 2 (Mock Runner)

**Files:**
- Modify: `src/etl/manager_test.rs`

**Step 1: Add mock ETLRunner implementation**

Add after the helper functions in `src/etl/manager_test.rs`:

```rust
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
```

**Step 2: Add test for mock runner**

Add at the end of `src/etl/manager_test.rs`:

```rust
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
```

**Step 3: Run tests**

Run: `cargo test --lib manager_test::test_mock_runner`
Expected: PASS (2 tests)

**Step 4: Commit**

```bash
git add src/etl/manager_test.rs
git commit -m "test: add MockETLRunner for testing manager"
```

---

## Task 10: Create New Test Suite - Part 3 (Manager Tests)

**Files:**
- Modify: `src/etl/manager_test.rs`

**Step 1: Add manager execution tests**

Add at the end of `src/etl/manager_test.rs`:

```rust
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
```

**Step 2: Run tests**

Run: `cargo test --lib manager_test`
Expected: PASS (all tests)

**Step 3: Commit**

```bash
git add src/etl/manager_test.rs
git commit -m "test: add manager execution tests for success cases"
```

---

## Task 11: Create New Test Suite - Part 4 (Failure and Cancellation)

**Files:**
- Modify: `src/etl/manager_test.rs`

**Step 1: Add failure and cancellation tests**

Add at the end of `src/etl/manager_test.rs`:

```rust
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
            MockETLRunner::new(format!("delayed{}", i)).with_delay(20)
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
```

**Step 2: Run all tests**

Run: `cargo test --lib manager_test`
Expected: PASS (all tests)

**Step 3: Commit**

```bash
git add src/etl/manager_test.rs
git commit -m "test: add failure and cancellation tests for manager"
```

---

## Task 12: Create New Test Suite - Part 5 (Pipeline Adapter)

**Files:**
- Modify: `src/etl/manager_test.rs`

**Step 1: Add mock ETLPipeline**

Add after MockETLRunner in `src/etl/manager_test.rs`:

```rust
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
```

**Step 2: Add pipeline adapter tests**

Add at the end of `src/etl/manager_test.rs`:

```rust
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
```

**Step 3: Run all tests**

Run: `cargo test --lib manager`
Expected: PASS (all tests)

**Step 4: Commit**

```bash
git add src/etl/manager_test.rs
git commit -m "test: add ETLPipeline adapter tests"
```

---

## Task 13: Final Verification

**Step 1: Run full test suite**

Run: `cargo test --lib`
Expected: PASS (all tests, no hanging)

**Step 2: Run specific manager tests with timeout**

Run: `timeout 10 cargo test --lib manager`
Expected: PASS within 10 seconds (no hanging)

**Step 3: Check for warnings**

Run: `cargo build --lib 2>&1 | grep warning`
Expected: Only dead_code warnings for test structs

**Step 4: Format code**

Run: `cargo fmt`

**Step 5: Final commit**

```bash
git add -A
git commit -m "refactor: complete ETL Manager zero-copy refactoring

- Replace Box<dyn Error> with ETLError enum
- Use flume instead of std::sync::mpsc (fixes hanging tests)
- Remove unnecessary Arc wrapping
- Simplify config handling with shared bucket config
- Complete new test suite with no hanging tests"
```

---

## Verification Commands

After completing all tasks:

```bash
# All tests should pass without hanging
timeout 30 cargo test --lib

# Build should succeed with minimal warnings
cargo build --lib

# Check test coverage
cargo test --lib -- --nocapture manager

# Verify no hanging with specific manager tests
timeout 10 cargo test --lib manager_test
```

## Success Criteria

- ✅ All tests pass without hanging
- ✅ No std::sync::mpsc usage in manager.rs
- ✅ ETLError type properly integrated
- ✅ Config borrowing instead of Arc cloning
- ✅ Simplified ETLPipelineAdapter
- ✅ Complete test coverage for success, failure, and cancellation cases
