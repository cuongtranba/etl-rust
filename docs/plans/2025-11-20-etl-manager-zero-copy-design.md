# ETL Manager Zero-Copy Refactoring Design

## Date: 2025-11-20

## Overview
Complete refactoring of the ETL Manager module to implement Rust best practices with a focus on zero-copy architecture for optimal performance.

## Goals
- Apply comprehensive Rust best practices
- Minimize allocations through zero-copy patterns
- Fix hanging test issues (by replacing std::sync::mpsc)
- Improve error handling with proper types
- Simplify architecture by removing unnecessary abstractions

## Architecture Design

### 1. Error Handling
Replace `Box<dyn Error + Send>` with a proper enum-based error type using `thiserror`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ETLError {
    #[error("Pipeline execution failed: {0}")]
    PipelineExecution(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Cancellation requested")]
    Cancelled,

    #[error("Worker pool exhausted")]
    WorkerPoolExhausted,

    #[error("Bucket operation failed: {0}")]
    Bucket(#[from] crate::bucket::BucketError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

### 2. ETLRunner Trait Improvements
Simplify the trait by using borrows instead of Arc:

```rust
#[async_trait::async_trait]
pub trait ETLRunner: Send + Sync {
    fn name(&self) -> &str;
    async fn run(
        &self,
        config: &bucket::Config,  // Borrow instead of Arc
        cancel: &CancellationToken,
    ) -> Result<(), ETLError>;
}
```

### 3. Manager Structure
Remove unnecessary Arc wrapping and use lifetimes appropriately:

```rust
pub struct ETLPipelineManager<'a> {
    etl_runners: Vec<Box<dyn ETLRunner + 'a>>,  // Box instead of Arc
    cfg: Config,
    bucket_config: bucket::Config,  // Store by value, not Arc
}
```

### 4. Concurrency with Flume
Replace std::sync::mpsc with flume (already in dependencies) to fix hanging tests:

```rust
pub async fn run_all(&self, cancel: &CancellationToken) -> Result<(), ETLError> {
    let semaphore = Arc::new(Semaphore::new(self.cfg.worker_num));
    let (tx, rx) = flume::bounded(self.etl_runners.len());

    // Spawn tasks with proper error handling
    let handles: Vec<_> = self.etl_runners
        .iter()
        .map(|runner| {
            // ... spawn logic
        })
        .collect();

    // Collect results without allocation
    drop(tx);
    while let Ok(result) = rx.recv_async().await {
        result?;
    }

    Ok(())
}
```

### 5. Simplified ETLPipelineAdapter
Since all pipelines share the same bucket config:

```rust
pub struct ETLPipelineAdapter<E, T> {
    etl: ETL<E, T>,  // Direct storage, no Arc
    name: String,
    // No config field - use shared config from manager
}

impl<E, T> ETLPipelineAdapter<E, T> {
    pub fn new(
        pipeline: impl ETLPipeline<E, T> + 'static,
        name: impl Into<String>,
    ) -> Self {
        ETLPipelineAdapter {
            etl: ETL::new(pipeline),
            name: name.into(),
        }
    }
}
```

### 6. Testing Strategy
Create new tests from scratch:
- Remove all std::sync::mpsc usage (source of hangs)
- Eliminate unnecessary Arc/Mutex in mocks
- Use const/static test fixtures for zero allocation
- Focus on unit tests without complex synchronization

## Implementation Benefits

1. **Performance**: Zero-copy patterns reduce memory allocations
2. **Reliability**: Flume channels eliminate test hanging issues
3. **Maintainability**: Cleaner error types and simpler architecture
4. **Type Safety**: Proper error enum instead of dynamic errors
5. **Simplicity**: Removed unnecessary Arc wrapping and config duplication

## Migration Path

1. Create new error type module
2. Update ETLRunner trait signature
3. Refactor ETLPipelineManager with flume
4. Simplify ETLPipelineAdapter
5. Remove old tests completely
6. Write new test suite from scratch

## Testing Approach

- Unit tests for each component
- Integration tests for pipeline execution
- Property-based tests for concurrent execution
- Benchmark tests to verify performance improvements

## Dependencies

- Keep: async-trait, tokio, flume, derive_builder
- Add: thiserror for error handling
- Remove: std::sync::mpsc usage

## Success Criteria

- All tests pass without hanging
- No unnecessary allocations in hot paths
- Clear error messages with context
- Simplified codebase following Rust idioms
- Improved performance in benchmarks