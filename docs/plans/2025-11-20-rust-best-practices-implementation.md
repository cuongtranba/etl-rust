# Rust Best Practices Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refactor etl-rust codebase to follow Rust best practices for error handling, ownership patterns, code organization, and performance.

**Architecture:** Incremental refactoring starting with foundational changes (error types, trait names) then building up to performance optimizations. Each task is independent where possible to allow parallel work.

**Tech Stack:** Rust 1.91.0 (edition 2024), tokio, flume, thiserror, tracing

---

## Overview

This plan implements 16 recommendations from the code review (docs/plans/2025-11-20-rust-code-review-design.md):

**Priority 1 (P1):** High-impact, low-effort improvements
- Better error handling with thiserror
- Remove unnecessary Clone bounds
- Rename traits for clarity
- English-only comments

**Priority 2 (P2):** Quality and performance enhancements
- Structured logging with tracing
- Unify error types
- Optimize cloning patterns
- Private Config fields with getters
- Comprehensive documentation
- Better channel sizing

**Priority 3 (P3):** Polish and optimization
- MultipleErrors variant
- Improved Arc constructor
- Working examples
- Stream transformations
- Inline hints

---

## Task 1: Add thiserror Dependency (R1.1 - P1)

**Files:**
- Modify: `Cargo.toml:6-12`

**Step 1: Add thiserror to dependencies**

Edit `Cargo.toml`:
```toml
[dependencies]
async-trait = "0.1.89"
derive_builder = "0.20.2"
flume = "0.11.1"
futures = "0.3.31"
thiserror = "1.0"
tokio = {version = "1.48.0", features = ["full"]}
tokio-util = "0.7.17"
```

**Step 2: Verify dependency resolves**

Run: `cargo check`
Expected: Dependencies update successfully, no errors

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add thiserror dependency for better error handling"
```

---

## Task 2: Refactor BucketError with thiserror (R1.1 - P1)

**Files:**
- Modify: `src/bucket/types.rs:1-21`

**Step 1: Write test for error source preservation**

Create `src/bucket/types.rs` test module at the end:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_processor_error_preserves_source() {
        let source = std::io::Error::new(std::io::ErrorKind::Other, "test error");
        let bucket_err = BucketError::ProcessorError(Box::new(source));

        assert!(bucket_err.source().is_some());
        assert_eq!(bucket_err.to_string(), "processor failed");
    }

    #[test]
    fn test_error_display() {
        let err = BucketError::ChannelClosed;
        assert_eq!(err.to_string(), "channel closed");

        let err = BucketError::Cancelled;
        assert_eq!(err.to_string(), "operation cancelled");
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib bucket::types::tests`
Expected: Compilation errors - BucketError doesn't have source() method yet

**Step 3: Refactor BucketError with thiserror**

Replace `src/bucket/types.rs` content:
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BucketError {
    #[error("processor failed")]
    ProcessorError(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("channel closed")]
    ChannelClosed,

    #[error("operation cancelled")]
    Cancelled,

    #[error("consumer error")]
    ConsumerError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_processor_error_preserves_source() {
        let source = std::io::Error::new(std::io::ErrorKind::Other, "test error");
        let bucket_err = BucketError::ProcessorError(Box::new(source));

        assert!(bucket_err.source().is_some());
        assert_eq!(bucket_err.to_string(), "processor failed");
    }

    #[test]
    fn test_error_display() {
        let err = BucketError::ChannelClosed;
        assert_eq!(err.to_string(), "channel closed");

        let err = BucketError::Cancelled;
        assert_eq!(err.to_string(), "operation cancelled");
    }
}
```

**Step 4: Fix compilation errors from removing Clone**

BucketError can no longer derive Clone because Box<dyn Error> is not Clone.
This is expected and correct - errors shouldn't be cloned.

Run: `cargo check`
Expected: Errors about Clone trait not implemented

Identify usages:
Run: `rg "BucketError.*clone" --type rust`

**Step 5: Remove Clone from BucketError usages**

No code should be cloning errors. If any usages found, refactor to avoid cloning.
Errors should be returned, not cloned.

**Step 6: Run tests to verify**

Run: `cargo test`
Expected: All tests pass

**Step 7: Commit**

```bash
git add src/bucket/types.rs
git commit -m "refactor: use thiserror for BucketError with source preservation

- Add error source chaining
- Remove Clone derive (errors shouldn't be cloned)
- Add tests for error source preservation
- Improve error messages"
```

---

## Task 3: Add MultipleErrors Variant (R1.2 - P3)

**Files:**
- Modify: `src/bucket/types.rs`
- Modify: `src/bucket/bucket.rs:101-102`

**Step 1: Write test for multiple errors**

Add to `src/bucket/types.rs` test module:
```rust
#[test]
fn test_multiple_errors_display() {
    let errors = vec![
        BucketError::Cancelled,
        BucketError::ChannelClosed,
    ];
    let multi_err = BucketError::MultipleErrors(errors);

    let display = multi_err.to_string();
    assert!(display.contains("2 worker"));
    assert!(display.contains("failed"));
}

#[test]
fn test_multiple_errors_source() {
    let errors = vec![BucketError::Cancelled];
    let multi_err = BucketError::MultipleErrors(errors);

    // MultipleErrors doesn't have a single source
    assert!(multi_err.source().is_none());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib bucket::types::tests::test_multiple_errors`
Expected: Compilation error - MultipleErrors variant doesn't exist

**Step 3: Add MultipleErrors variant**

Modify `src/bucket/types.rs`:
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BucketError {
    #[error("processor failed")]
    ProcessorError(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("channel closed")]
    ChannelClosed,

    #[error("operation cancelled")]
    Cancelled,

    #[error("consumer error")]
    ConsumerError(String),

    #[error("{} worker(s) failed", .0.len())]
    MultipleErrors(Vec<BucketError>),
}
```

**Step 4: Update bucket.rs to return all errors**

Modify `src/bucket/bucket.rs:101-105`:
```rust
        if !errors.is_empty() {
            return Err(BucketError::MultipleErrors(errors));
        }

        Ok(())
```

**Step 5: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 6: Commit**

```bash
git add src/bucket/types.rs src/bucket/bucket.rs
git commit -m "feat: add MultipleErrors variant to preserve all worker failures

- Return all worker errors instead of just the first
- Improve error visibility for multi-worker scenarios"
```

---

## Task 4: Remove Unnecessary Clone Bound (R2.1 - P1)

**Files:**
- Modify: `src/bucket/bucket.rs:62, 119`

**Step 1: Write test with non-cloneable type**

Add to `src/bucket/bucket.rs` test module:
```rust
// Test that Bucket works with non-cloneable types
struct NonCloneable(i32);

struct NonCloneableProcessor;

#[async_trait]
impl Processor<NonCloneable> for NonCloneableProcessor {
    async fn process(
        &self,
        _ctx: &CancellationToken,
        items: &[NonCloneable],
    ) -> Result<(), BucketError> {
        // Just verify we can access items
        assert!(!items.is_empty());
        Ok(())
    }
}

#[tokio::test]
async fn test_bucket_with_non_cloneable_types() {
    let config = Arc::new(Config {
        batch_size: 2,
        timeout: Duration::from_millis(100),
        worker_num: 1,
    });

    let bucket: Bucket<NonCloneable> = Bucket::new(config);
    let bucket_clone = bucket.clone();
    let cancel = CancellationToken::new();

    let processor = NonCloneableProcessor;
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        bucket_clone.consume(&cancel_clone, NonCloneable(1)).await.unwrap();
        bucket_clone.consume(&cancel_clone, NonCloneable(2)).await.unwrap();
        bucket_clone.close();
    });

    let result = bucket.run(&cancel, processor).await;
    assert!(result.is_ok());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_bucket_with_non_cloneable_types`
Expected: Compilation error - NonCloneable doesn't implement Clone

**Step 3: Remove Clone bound from impl**

Modify `src/bucket/bucket.rs:29-32`:
```rust
impl<T> Bucket<T>
where
    T: Send + 'static,
{
```

Modify `src/bucket/bucket.rs:59-63`:
```rust
    pub async fn run<P>(&self, cancel: &CancellationToken, process: P) -> Result<(), BucketError>
    where
        P: Processor<T> + Send + Sync + 'static,
    {
```

Modify `src/bucket/bucket.rs:108-120`:
```rust
    async fn worker<P>(
        worker_id: usize,
        receiver: Receiver<T>,
        process: Arc<P>,
        cancel_token: &CancellationToken,
        done_token: &CancellationToken,
        batch_size: usize,
        timeout_duration: Duration,
    ) -> Result<(), BucketError>
    where
        P: Processor<T> + Send + Sync,
    {
```

Modify `src/bucket/bucket.rs:161-169`:
```rust
    async fn drain_and_process<P>(
        receiver: Receiver<T>,
        ctx: &CancellationToken,
        process: &P,
        queue: &mut Vec<T>,
        batch_size: usize,
    ) -> Result<(), BucketError>
    where
        P: Processor<T>,
    {
```

**Step 4: Run tests to verify**

Run: `cargo test`
Expected: All tests pass, including new non-cloneable test

**Step 5: Commit**

```bash
git add src/bucket/bucket.rs
git commit -m "refactor: remove unnecessary Clone bound from Bucket<T>

- T only needs Send + 'static, not Clone
- Allows using Bucket with non-cloneable types
- Simplifies API constraints"
```

---

## Task 5: Rename Processor Traits (R3.1 - P1)

**Files:**
- Modify: `src/bucket/processor.rs:9`
- Modify: `src/bucket/mod.rs:8`
- Modify: `src/bucket/bucket.rs` (multiple locations)
- Modify: `src/etl/processor.rs:12`
- Modify: `src/etl/mod.rs:4`

**Step 1: Rename bucket::Processor to BatchProcessor**

Modify `src/bucket/processor.rs`:
```rust
// src/bucket/processor.rs

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use super::types::BucketError;

#[async_trait]
pub trait BatchProcessor<T>: Send + Sync {
    async fn process(&self, cancel: &CancellationToken, items: &[T]) -> Result<(), BucketError>;
}

#[async_trait]
impl<T, F, Fut> BatchProcessor<T> for F
where
    F: Fn(&CancellationToken, &[T]) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<(), BucketError>> + Send,
    T: Send + Sync,
{
    async fn process(&self, ctx: &CancellationToken, items: &[T]) -> Result<(), BucketError> {
        self(ctx, items).await
    }
}
```

**Step 2: Update bucket/mod.rs exports**

Modify `src/bucket/mod.rs`:
```rust
pub mod bucket;
pub mod config;
pub mod processor;
pub mod types;

pub use bucket::Bucket;
pub use config::Config;
pub use processor::BatchProcessor;
pub use types::BucketError;
```

**Step 3: Update bucket.rs usages**

Find and replace in `src/bucket/bucket.rs`:
- Change `use super::processor::Processor;` to `use super::processor::BatchProcessor;`
- Change `P: Processor<T>` to `P: BatchProcessor<T>` (4 locations)
- Change `impl Processor<i32>` to `impl BatchProcessor<i32>` in tests (4 locations)

Modify `src/bucket/bucket.rs:8`:
```rust
use super::processor::BatchProcessor;
```

Modify trait bounds (lines 60, 109, 118, 162, 187):
```rust
where
    P: BatchProcessor<T> + Send + Sync + 'static,
```

Modify test impls (lines 210, 226, 244, 265, 293, 314):
```rust
#[async_trait]
impl BatchProcessor<i32> for CollectingProcessor {
```

**Step 4: Rename etl::Processor to ETLPipeline**

Modify `src/etl/processor.rs`:
```rust
use async_trait::async_trait;
use futures::future::join_all;
use mpsc::Receiver;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::bucket::{Bucket, BucketError, Config};

#[async_trait]
pub trait ETLPipeline<E, T>
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
    pub etl: Arc<dyn ETLPipeline<E, T> + Send + Sync>,
}

impl<E, T> ETL<E, T>
where
    E: Send + Sync + Clone + 'static,
    T: Send + 'static,
{
    pub fn new(etl: Box<dyn ETLPipeline<E, T> + Send + Sync>) -> Self {
        ETL {
            etl: Arc::from(etl),
        }
    }

    // Rest of implementation unchanged
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
```

**Step 5: Update etl/mod.rs exports**

Modify `src/etl/mod.rs`:
```rust
pub mod processor;

pub use processor::ETL;
pub use processor::ETLPipeline;
```

**Step 6: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 7: Commit**

```bash
git add src/bucket/processor.rs src/bucket/mod.rs src/bucket/bucket.rs src/etl/processor.rs src/etl/mod.rs
git commit -m "refactor: rename Processor traits for clarity

- bucket::Processor → BatchProcessor
- etl::Processor → ETLPipeline
- Eliminates naming ambiguity
- Improves API clarity"
```

---

## Task 6: Translate Vietnamese Comments to English (R3.3 - P1)

**Files:**
- Modify: `src/bucket/bucket.rs:91`
- Modify: `src/bucket/processor.rs:6`

**Step 1: Replace Vietnamese comments with English**

Modify `src/bucket/bucket.rs:91`:
```rust
        // Spawn workers to process batches
        let mut errors = Vec::new();
```

Modify `src/bucket/processor.rs:6`:
```rust
use super::types::BucketError;
```

Remove the comment entirely since it's obvious from the import.

**Step 2: Search for any other non-English comments**

Run: `rg "//.*[àáạảãâầấậẩẫăằắặẳẵèéẹẻẽêềếệểễìíịỉĩòóọỏõôồốộổỗơờớợởỡùúụủũưừứựửữỳýỵỷỹđ]" --type rust`
Expected: No matches (all Vietnamese comments translated)

**Step 3: Commit**

```bash
git add src/bucket/bucket.rs src/bucket/processor.rs
git commit -m "docs: translate Vietnamese comments to English

- Ensures international collaboration
- Maintains consistent code standards"
```

---

## Task 7: Add tracing Dependency (R4.4 - P2)

**Files:**
- Modify: `Cargo.toml:6-12`

**Step 1: Add tracing dependencies**

Edit `Cargo.toml`:
```toml
[dependencies]
async-trait = "0.1.89"
derive_builder = "0.20.2"
flume = "0.11.1"
futures = "0.3.31"
thiserror = "1.0"
tokio = {version = "1.48.0", features = ["full"]}
tokio-util = "0.7.17"
tracing = "0.1"

[dev-dependencies]
tracing-subscriber = "0.3"
```

**Step 2: Verify dependencies**

Run: `cargo check`
Expected: Dependencies resolve successfully

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add tracing dependencies for structured logging"
```

---

## Task 8: Replace println! with tracing (R4.4 - P2)

**Files:**
- Modify: `src/bucket/bucket.rs:128, 133, 154`

**Step 1: Add tracing import**

Modify `src/bucket/bucket.rs:1`:
```rust
use flume::{Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};

use super::config::Config;
use super::processor::BatchProcessor;
use super::types::BucketError;
```

**Step 2: Replace println! calls**

Modify `src/bucket/bucket.rs:128`:
```rust
                _ = cancel_token.cancelled() => {
                    info!(worker_id, "worker shutting down");
                    return Self::drain_and_process(receiver, cancel_token, &*process, &mut queue, batch_size).await;
                }
```

Modify `src/bucket/bucket.rs:133`:
```rust
                _ = done_token.cancelled() => {
                    debug!(worker_id, "worker done processing");
                    return Self::drain_and_process(receiver, cancel_token, &*process, &mut queue, batch_size).await;
                }
```

Modify `src/bucket/bucket.rs:154`:
```rust
                        Err(_) => {
                            debug!(worker_id, "worker channel closed");
                            return Self::process_queue(cancel_token, &*process, &mut queue).await;
                        }
```

**Step 3: Initialize tracing in tests**

Add helper at top of test module in `src/bucket/bucket.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::time::sleep;

    // Initialize tracing for tests
    fn init_tracing() {
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .try_init();
    }

    // Rest of tests...
```

Add `init_tracing();` as first line of each test:
```rust
    #[tokio::test]
    async fn test_should_process_full() {
        init_tracing();
        // rest of test...
    }
```

**Step 4: Run tests to verify logging works**

Run: `cargo test -- --nocapture`
Expected: Tests pass, tracing output visible

**Step 5: Commit**

```bash
git add src/bucket/bucket.rs
git commit -m "refactor: replace println! with structured tracing

- Use info! for important lifecycle events
- Use debug! for detailed processing events
- Initialize tracing in test module
- Better production observability"
```

---

## Task 9: Make Config Fields Private with Getters (R2.4 - P2)

**Files:**
- Modify: `src/bucket/config.rs:8-17`
- Modify: `src/bucket/bucket.rs` (update field access)

**Step 1: Write tests for Config getters**

Add to `src/bucket/config.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_getters() {
        let config = ConfigBuilder::default()
            .batch_size(10)
            .timeout(Duration::from_secs(30))
            .worker_num(4)
            .build()
            .unwrap();

        assert_eq!(config.batch_size(), 10);
        assert_eq!(config.timeout(), Duration::from_secs(30));
        assert_eq!(config.worker_num(), 4);
    }

    #[test]
    fn test_config_defaults() {
        let config = ConfigBuilder::default().build().unwrap();

        assert_eq!(config.batch_size(), 1);
        assert_eq!(config.timeout(), Duration::from_secs(5));
        assert_eq!(config.worker_num(), 1);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test config::tests`
Expected: Compilation error - getter methods don't exist

**Step 3: Make fields private and add getters**

Modify `src/bucket/config.rs`:
```rust
// src/bucket/config.rs

use derive_builder::Builder;
use std::time::Duration;

#[derive(Debug, Clone, Builder)]
#[builder(setter(into))]
pub struct Config {
    #[builder(default = "1")]
    batch_size: usize,

    #[builder(default = "Duration::from_secs(5)")]
    timeout: Duration,

    #[builder(default = "1")]
    worker_num: usize,
}

impl Config {
    /// Returns the batch size for processing
    #[inline]
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    /// Returns the timeout duration for batch collection
    #[inline]
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Returns the number of worker threads
    #[inline]
    pub fn worker_num(&self) -> usize {
        self.worker_num
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_getters() {
        let config = ConfigBuilder::default()
            .batch_size(10)
            .timeout(Duration::from_secs(30))
            .worker_num(4)
            .build()
            .unwrap();

        assert_eq!(config.batch_size(), 10);
        assert_eq!(config.timeout(), Duration::from_secs(30));
        assert_eq!(config.worker_num(), 4);
    }

    #[test]
    fn test_config_defaults() {
        let config = ConfigBuilder::default().build().unwrap();

        assert_eq!(config.batch_size(), 1);
        assert_eq!(config.timeout(), Duration::from_secs(5));
        assert_eq!(config.worker_num(), 1);
    }
}
```

**Step 4: Update bucket.rs to use getters**

Find all `config.batch_size`, `config.timeout`, `config.worker_num` and replace with method calls.

Modify `src/bucket/bucket.rs:34`:
```rust
    pub fn new(config: Arc<Config>) -> Self {
        let (sender, receiver) = flume::bounded(config.batch_size());
```

Modify `src/bucket/bucket.rs:67`:
```rust
        for worker_id in 0..self.config.worker_num() {
```

Modify `src/bucket/bucket.rs:70-71`:
```rust
            let batch_size = self.config.batch_size();
            let timeout = self.config.timeout();
```

**Step 5: Update tests to use ConfigBuilder or getters**

Update all test Config construction to use getters:
```rust
let config = Arc::new(Config {
    batch_size: 4,  // Won't compile - fields are private
    ...
});
```

Should use builder in tests, but for tests we can keep struct literal syntax by making fields pub(crate):

Actually, for tests in the same crate, change Config fields to `pub(crate)` instead of private:
```rust
#[derive(Debug, Clone, Builder)]
#[builder(setter(into))]
pub struct Config {
    #[builder(default = "1")]
    pub(crate) batch_size: usize,

    #[builder(default = "Duration::from_secs(5)")]
    pub(crate) timeout: Duration,

    #[builder(default = "1")]
    pub(crate) worker_num: usize,
}
```

This allows tests to use struct literals but prevents external crates from accessing fields directly.

**Step 6: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 7: Commit**

```bash
git add src/bucket/config.rs src/bucket/bucket.rs
git commit -m "refactor: make Config fields private with getter methods

- Add inline getter methods for all fields
- Fields are pub(crate) to allow internal tests
- Improves encapsulation
- Adds Config unit tests"
```

---

## Task 10: Improve Channel Capacity (R4.3 - P2)

**Files:**
- Modify: `src/bucket/bucket.rs:34`

**Step 1: Write test for high-throughput scenario**

Add to `src/bucket/bucket.rs` tests:
```rust
#[tokio::test]
async fn test_high_throughput_no_backpressure() {
    init_tracing();

    let config = Arc::new(Config {
        batch_size: 10,
        timeout: Duration::from_millis(100),
        worker_num: 4,
    });

    let bucket: Bucket<i32> = Bucket::new(config);
    let bucket_clone = bucket.clone();
    let cancel = CancellationToken::new();

    let counter = Arc::new(AtomicUsize::new(0));
    let processor = CountingProcessor {
        counter: Arc::clone(&counter),
    };

    let start = std::time::Instant::now();

    let cancel_clone = cancel.clone();
    let producer = tokio::spawn(async move {
        for i in 0..1000 {
            bucket_clone.consume(&cancel_clone, i).await.unwrap();
        }
        bucket_clone.close();
    });

    let processor_handle = bucket.run(&cancel, processor);

    let (_, _) = tokio::join!(producer, processor_handle);

    let elapsed = start.elapsed();
    assert_eq!(counter.load(Ordering::SeqCst), 1000);

    // Should complete quickly without backpressure delays
    assert!(elapsed < Duration::from_secs(2));
}
```

**Step 2: Run test with current implementation**

Run: `cargo test test_high_throughput_no_backpressure -- --nocapture`
Expected: Test might be slow due to backpressure (channel size = batch_size)

**Step 3: Increase channel capacity**

Modify `src/bucket/bucket.rs:33-35`:
```rust
    pub fn new(config: Arc<Config>) -> Self {
        // Channel capacity = batch_size * worker_num * 2
        // Allows buffering multiple batches per worker to reduce backpressure
        let capacity = config.batch_size() * config.worker_num() * 2;
        let (sender, receiver) = flume::bounded(capacity);

        Self {
            config,
            sender,
            receiver,
            done: CancellationToken::new(),
        }
    }
```

**Step 4: Run test again**

Run: `cargo test test_high_throughput_no_backpressure -- --nocapture`
Expected: Test completes faster with better throughput

**Step 5: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 6: Commit**

```bash
git add src/bucket/bucket.rs
git commit -m "perf: increase channel capacity to reduce backpressure

- Capacity = batch_size * worker_num * 2
- Prevents producers from blocking on full channel
- Improves throughput in multi-worker scenarios
- Add high-throughput test"
```

---

## Task 11: Fix ETL Cloning Issue (R2.2 - P2)

**Files:**
- Modify: `src/etl/processor.rs:59-74`

**Step 1: Analyze current cloning**

Current code in `src/etl/processor.rs:61`:
```rust
let items = items.to_vec();  // Clones entire batch
```

This is necessary to move items into the async block. However, we can avoid cloning by restructuring.

**Step 2: Refactor to avoid cloning the batch**

The issue is that `items: &[E]` needs to be moved into the async block returned by the closure.
We can't avoid the clone without changing the BatchProcessor signature.

Alternative: Accept that the clone is necessary for the closure-based API, but document it.
Better solution: Don't clone individual items during transform.

Modify `src/etl/processor.rs:59-74`:
```rust
                .run(
                    &cancel_clone,
                    move |ctx: &CancellationToken, items: &[E]| {
                        let etl = Arc::clone(&etl_clone);
                        // Clone items vector to move into async block
                        // Individual items aren't cloned during transform
                        let items = items.to_vec();
                        let ctx = ctx.clone();

                        async move {
                            // Transform items without cloning each one
                            let transform_futures: Vec<_> =
                                items.iter().map(|item| etl.transform(&ctx, item.clone())).collect();

                            let transforms: Vec<T> = join_all(transform_futures).await;

                            etl.load(&ctx, transforms)
                                .await
                                .map_err(|e| BucketError::ProcessorError(Box::new(e)))
                        }
                    },
                )
                .await
```

Wait - we're still cloning `item` in `etl.transform(&ctx, item.clone())`. This is because E is Clone.

**Step 3: Change transform to take reference**

The real fix is to change the ETLPipeline trait to accept &E instead of E:

Modify `src/etl/processor.rs:17`:
```rust
    async fn transform(&self, cancel: &CancellationToken, item: &E) -> T;
```

Then update the usage:
```rust
                        async move {
                            let transform_futures: Vec<_> =
                                items.iter().map(|item| etl.transform(&ctx, item)).collect();

                            let transforms: Vec<T> = join_all(transform_futures).await;

                            etl.load(&ctx, transforms)
                                .await
                                .map_err(|e| BucketError::ProcessorError(Box::new(e)))
                        }
```

**Step 4: Remove Clone bound from E**

Modify `src/etl/processor.rs:27-31`:
```rust
impl<E, T> ETL<E, T>
where
    E: Send + Sync + 'static,
    T: Send + 'static,
{
```

**Step 5: Run tests**

Run: `cargo test`
Expected: All tests pass (there are no ETL tests currently, so this should just compile)

**Step 6: Commit**

```bash
git add src/etl/processor.rs
git commit -m "perf: eliminate unnecessary cloning in ETL transform

- Change ETLPipeline::transform to take &E instead of E
- Remove Clone bound from E generic parameter
- Reduces memory allocations during batch processing"
```

---

## Task 12: Improve Arc Constructor (R2.3 - P3)

**Files:**
- Modify: `src/etl/processor.rs:32-36`

**Step 1: Change constructor to accept Arc directly**

Modify `src/etl/processor.rs:32-36`:
```rust
    pub fn new(etl: impl Into<Arc<dyn ETLPipeline<E, T> + Send + Sync>>) -> Self {
        ETL {
            etl: etl.into(),
        }
    }
```

This allows calling with either `Box` or `Arc`:
```rust
// Works with Box
ETL::new(Box::new(my_pipeline))

// Works with Arc
ETL::new(Arc::new(my_pipeline))

// Works with concrete Arc
let arc = Arc::new(my_pipeline);
ETL::new(arc)
```

**Step 2: Add test demonstrating usage**

Add to `src/etl/processor.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct TestPipeline;

    #[async_trait]
    impl ETLPipeline<i32, String> for TestPipeline {
        async fn extract(&self, _cancel: &CancellationToken) -> Result<Receiver<i32>, Box<dyn Error>> {
            let (tx, rx) = mpsc::channel(10);
            drop(tx);
            Ok(rx)
        }

        async fn transform(&self, _cancel: &CancellationToken, item: &i32) -> String {
            item.to_string()
        }

        async fn load(&self, _cancel: &CancellationToken, _items: Vec<String>) -> Result<(), Box<dyn Error>> {
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
    fn test_etl_new_with_box() {
        let _etl: ETL<i32, String> = ETL::new(Box::new(TestPipeline));
    }

    #[test]
    fn test_etl_new_with_arc() {
        let arc = Arc::new(TestPipeline) as Arc<dyn ETLPipeline<i32, String> + Send + Sync>;
        let _etl: ETL<i32, String> = ETL::new(arc);
    }
}
```

**Step 3: Run tests**

Run: `cargo test etl::processor::tests`
Expected: Tests pass

**Step 4: Commit**

```bash
git add src/etl/processor.rs
git commit -m "refactor: improve ETL constructor to accept Arc or Box

- Use Into<Arc<...>> for flexible construction
- Supports both Box and Arc inputs
- Add tests demonstrating usage"
```

---

## Task 13: Add Inline Hints (R4.5 - P3)

**Files:**
- Modify: `src/bucket/bucket.rs:55`
- Modify: `src/bucket/config.rs` (already done in Task 9)
- Modify: `src/bucket/types.rs` (error methods)

**Step 1: Add inline to Bucket::close**

Modify `src/bucket/bucket.rs:55-57`:
```rust
    #[inline]
    pub fn close(&self) {
        self.done.cancel();
    }
```

**Step 2: Verify inline hints on Config getters**

Already added in Task 9 - verify they're present in `src/bucket/config.rs:20,26,32`.

**Step 3: Add inline to small error methods if any**

BucketError uses thiserror which generates optimal implementations already.
No additional inline needed.

**Step 4: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 5: Check generated assembly (optional)**

Run: `cargo build --release`
Run: `cargo asm etl_rust::bucket::Bucket::close --rust`
Expected: Method is inlined in calling code

**Step 6: Commit**

```bash
git add src/bucket/bucket.rs
git commit -m "perf: add inline hints to small hot-path methods

- Inline Bucket::close for reduced call overhead
- Config getters already inlined from Task 9"
```

---

## Task 14: Add Comprehensive Documentation (R3.4 - P2)

**Files:**
- Modify: `src/lib.rs`
- Modify: `src/bucket/mod.rs`
- Modify: `src/bucket/bucket.rs`
- Modify: `src/bucket/processor.rs`
- Modify: `src/bucket/config.rs`
- Modify: `src/bucket/types.rs`
- Modify: `src/etl/mod.rs`
- Modify: `src/etl/processor.rs`

**Step 1: Add crate-level documentation**

Modify `src/lib.rs`:
```rust
//! # etl-rust
//!
//! A concurrent ETL (Extract-Transform-Load) framework built on Tokio and Rust.
//!
//! ## Features
//!
//! - **Concurrent batch processing** with configurable workers
//! - **Backpressure handling** via bounded channels
//! - **Graceful cancellation** support
//! - **Generic ETL pipeline** abstraction
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use etl_rust::bucket::{Bucket, BatchProcessor, Config};
//! use std::sync::Arc;
//! use tokio_util::sync::CancellationToken;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create bucket configuration
//! let config = Arc::new(Config {
//!     batch_size: 100,
//!     timeout: std::time::Duration::from_secs(5),
//!     worker_num: 4,
//! });
//!
//! // Create bucket for processing integers
//! let bucket: Bucket<i32> = Bucket::new(config);
//! let cancel = CancellationToken::new();
//!
//! // Process batches with a closure
//! bucket.run(&cancel, |_ctx, items: &[i32]| async move {
//!     println!("Processing batch of {} items", items.len());
//!     Ok(())
//! }).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Modules
//!
//! - [`bucket`] - Generic batching and concurrent processing
//! - [`etl`] - ETL pipeline abstraction for extract-transform-load workflows

pub mod bucket;
pub mod etl;
```

**Step 2: Add module-level documentation for bucket**

Modify `src/bucket/mod.rs`:
```rust
//! Concurrent batch processing with configurable workers.
//!
//! This module provides [`Bucket`], a generic batching processor that collects
//! items into batches and processes them concurrently using multiple workers.
//!
//! # Example
//!
//! ```rust,no_run
//! use etl_rust::bucket::{Bucket, Config};
//! use std::sync::Arc;
//! use tokio_util::sync::CancellationToken;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = Arc::new(Config {
//!     batch_size: 10,
//!     timeout: std::time::Duration::from_secs(1),
//!     worker_num: 2,
//! });
//!
//! let bucket: Bucket<String> = Bucket::new(config);
//! let cancel = CancellationToken::new();
//!
//! // Consume items
//! bucket.consume(&cancel, "item1".to_string()).await?;
//! bucket.consume(&cancel, "item2".to_string()).await?;
//! bucket.close();
//!
//! // Process batches
//! bucket.run(&cancel, |_ctx, items| async move {
//!     for item in items {
//!         println!("Processing: {}", item);
//!     }
//!     Ok(())
//! }).await?;
//! # Ok(())
//! # }
//! ```

pub mod bucket;
pub mod config;
pub mod processor;
pub mod types;

pub use bucket::Bucket;
pub use config::Config;
pub use processor::BatchProcessor;
pub use types::BucketError;
```

**Step 3: Document Bucket struct and methods**

Modify `src/bucket/bucket.rs:11-16`:
```rust
/// A concurrent batching processor that collects items into batches
/// and processes them with configurable workers.
///
/// `Bucket` uses a bounded channel for backpressure control and spawns
/// multiple worker tasks to process batches concurrently.
///
/// # Type Parameters
///
/// * `T` - The type of items to process. Must be `Send + 'static`.
///
/// # Example
///
/// ```rust,no_run
/// use etl_rust::bucket::{Bucket, Config};
/// use std::sync::Arc;
/// use tokio_util::sync::CancellationToken;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = Arc::new(Config {
///     batch_size: 50,
///     timeout: std::time::Duration::from_secs(5),
///     worker_num: 4,
/// });
///
/// let bucket: Bucket<i32> = Bucket::new(config);
/// let cancel = CancellationToken::new();
///
/// // Producer: send items
/// for i in 0..1000 {
///     bucket.consume(&cancel, i).await?;
/// }
/// bucket.close();
///
/// // Consumer: process batches
/// bucket.run(&cancel, |_ctx, items| async move {
///     println!("Batch size: {}", items.len());
///     Ok(())
/// }).await?;
/// # Ok(())
/// # }
/// ```
pub struct Bucket<T> {
    config: Arc<Config>,
    sender: Sender<T>,
    receiver: Receiver<T>,
    done: CancellationToken,
}
```

Add docs to methods:
```rust
    /// Creates a new `Bucket` with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Shared configuration specifying batch size, timeout, and worker count
    pub fn new(config: Arc<Config>) -> Self {
```

```rust
    /// Consumes an item, sending it to the processing queue.
    ///
    /// This method blocks if the channel is full (backpressure).
    ///
    /// # Arguments
    ///
    /// * `cancel` - Cancellation token to abort the operation
    /// * `item` - The item to process
    ///
    /// # Errors
    ///
    /// Returns `BucketError::Cancelled` if the cancellation token is triggered
    /// before the item is sent.
    pub async fn consume(&self, cancel: &CancellationToken, item: T) -> Result<(), BucketError> {
```

```rust
    /// Signals that no more items will be produced.
    ///
    /// Workers will finish processing queued items and then shut down.
    #[inline]
    pub fn close(&self) {
```

```rust
    /// Runs the bucket processor with the given batch processor.
    ///
    /// Spawns worker tasks to process batches concurrently. Blocks until all
    /// workers complete or a cancellation is triggered.
    ///
    /// # Arguments
    ///
    /// * `cancel` - Cancellation token to stop processing
    /// * `process` - Batch processor implementation or closure
    ///
    /// # Errors
    ///
    /// Returns `BucketError::MultipleErrors` if multiple workers fail.
    pub async fn run<P>(&self, cancel: &CancellationToken, process: P) -> Result<(), BucketError>
```

**Step 4: Document BatchProcessor trait**

Modify `src/bucket/processor.rs:8-11`:
```rust
/// Processes batches of items concurrently.
///
/// Implementations define how to process a slice of items, typically
/// performing I/O operations, transformations, or aggregations.
///
/// # Type Parameters
///
/// * `T` - The type of items in each batch
///
/// # Example
///
/// ```rust
/// use async_trait::async_trait;
/// use etl_rust::bucket::{BatchProcessor, BucketError};
/// use tokio_util::sync::CancellationToken;
///
/// struct MyProcessor;
///
/// #[async_trait]
/// impl BatchProcessor<String> for MyProcessor {
///     async fn process(&self, ctx: &CancellationToken, items: &[String]) -> Result<(), BucketError> {
///         for item in items {
///             if ctx.is_cancelled() {
///                 return Err(BucketError::Cancelled);
///             }
///             // Process item...
///         }
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait BatchProcessor<T>: Send + Sync {
    /// Processes a batch of items.
    ///
    /// # Arguments
    ///
    /// * `cancel` - Cancellation token to abort processing
    /// * `items` - Slice of items to process
    ///
    /// # Errors
    ///
    /// Returns `BucketError` if processing fails or is cancelled.
    async fn process(&self, cancel: &CancellationToken, items: &[T]) -> Result<(), BucketError>;
}
```

**Step 5: Document Config**

Modify `src/bucket/config.rs:6-17`:
```rust
/// Configuration for bucket batch processing.
///
/// # Examples
///
/// ```rust
/// use etl_rust::bucket::Config;
/// use std::time::Duration;
///
/// let config = Config {
///     batch_size: 100,
///     timeout: Duration::from_secs(5),
///     worker_num: 4,
/// };
///
/// assert_eq!(config.batch_size(), 100);
/// ```
#[derive(Debug, Clone, Builder)]
#[builder(setter(into))]
pub struct Config {
    /// Maximum number of items in a batch before processing
    #[builder(default = "1")]
    pub(crate) batch_size: usize,

    /// Maximum time to wait before processing a partial batch
    #[builder(default = "Duration::from_secs(5)")]
    pub(crate) timeout: Duration,

    /// Number of concurrent worker tasks
    #[builder(default = "1")]
    pub(crate) worker_num: usize,
}
```

**Step 6: Document BucketError**

Modify `src/bucket/types.rs:3-17`:
```rust
/// Errors that can occur during bucket processing.
#[derive(Debug, Error)]
pub enum BucketError {
    /// A processor failed with an error.
    ///
    /// Preserves the source error for debugging.
    #[error("processor failed")]
    ProcessorError(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// The channel was closed unexpectedly.
    #[error("channel closed")]
    ChannelClosed,

    /// Processing was cancelled via the cancellation token.
    #[error("operation cancelled")]
    Cancelled,

    /// Failed to send item to consumer channel.
    #[error("consumer error")]
    ConsumerError(String),

    /// Multiple workers failed during processing.
    ///
    /// Contains all worker errors for debugging.
    #[error("{} worker(s) failed", .0.len())]
    MultipleErrors(Vec<BucketError>),
}
```

**Step 7: Document ETL module**

Modify `src/etl/mod.rs`:
```rust
//! ETL (Extract-Transform-Load) pipeline abstraction.
//!
//! This module provides the [`ETLPipeline`] trait for defining data pipelines
//! and the [`ETL`] executor for running them with concurrent batch processing.
//!
//! # Example
//!
//! ```rust,no_run
//! use async_trait::async_trait;
//! use etl_rust::etl::{ETL, ETLPipeline};
//! use etl_rust::bucket::Config;
//! use std::sync::Arc;
//! use tokio::sync::mpsc;
//! use tokio_util::sync::CancellationToken;
//!
//! struct MyPipeline;
//!
//! #[async_trait]
//! impl ETLPipeline<i32, String> for MyPipeline {
//!     async fn extract(&self, _cancel: &CancellationToken) -> Result<mpsc::Receiver<i32>, Box<dyn std::error::Error>> {
//!         let (tx, rx) = mpsc::channel(100);
//!         // Send data...
//!         Ok(rx)
//!     }
//!
//!     async fn transform(&self, _cancel: &CancellationToken, item: &i32) -> String {
//!         item.to_string()
//!     }
//!
//!     async fn load(&self, _cancel: &CancellationToken, items: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
//!         // Load transformed data...
//!         Ok(())
//!     }
//!
//!     async fn pre_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn std::error::Error>> {
//!         Ok(())
//!     }
//!
//!     async fn post_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn std::error::Error>> {
//!         Ok(())
//!     }
//! }
//! ```

pub mod processor;

pub use processor::ETL;
pub use processor::ETLPipeline;
```

**Step 8: Document ETLPipeline trait**

Modify `src/etl/processor.rs:11-21`:
```rust
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
```

**Step 9: Document ETL struct**

Modify `src/etl/processor.rs:23-25`:
```rust
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
    pub etl: Arc<dyn ETLPipeline<E, T> + Send + Sync>,
}
```

**Step 10: Generate and review documentation**

Run: `cargo doc --no-deps --open`
Expected: Documentation opens in browser, review for completeness and correctness

**Step 11: Run doc tests**

Run: `cargo test --doc`
Expected: All doc examples compile (though they may not run due to no_run)

**Step 12: Commit**

```bash
git add src/lib.rs src/bucket/mod.rs src/bucket/bucket.rs src/bucket/processor.rs src/bucket/config.rs src/bucket/types.rs src/etl/mod.rs src/etl/processor.rs
git commit -m "docs: add comprehensive documentation to all public APIs

- Add crate-level docs with quick start
- Document all modules with examples
- Add doc comments to all public types and methods
- Include usage examples in doc tests
- Improve API discoverability"
```

---

## Task 15: Create Working Example (R3.2 - P3)

**Files:**
- Create: `examples/simple_etl.rs`

**Step 1: Create example directory if needed**

Run: `mkdir -p examples`

**Step 2: Write simple ETL example**

Create `examples/simple_etl.rs`:
```rust
//! Simple ETL example demonstrating the etl-rust framework.
//!
//! This example extracts numbers 1-100, transforms them by squaring,
//! and loads them by printing to stdout.
//!
//! Run with: cargo run --example simple_etl

use async_trait::async_trait;
use etl_rust::bucket::Config;
use etl_rust::etl::{ETLPipeline, ETL};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Simple pipeline that extracts numbers, squares them, and prints results
struct NumberSquaringPipeline {
    max_number: i32,
}

#[async_trait]
impl ETLPipeline<i32, i32> for NumberSquaringPipeline {
    async fn extract(&self, cancel: &CancellationToken) -> Result<mpsc::Receiver<i32>, Box<dyn Error>> {
        let (tx, rx) = mpsc::channel(100);
        let max = self.max_number;

        // Spawn producer task
        tokio::spawn(async move {
            for i in 1..=max {
                if cancel.is_cancelled() {
                    break;
                }
                if tx.send(i).await.is_err() {
                    break;
                }
            }
        });

        Ok(rx)
    }

    async fn transform(&self, _cancel: &CancellationToken, item: &i32) -> i32 {
        // Square the number
        item * item
    }

    async fn load(&self, _cancel: &CancellationToken, items: Vec<i32>) -> Result<(), Box<dyn Error>> {
        // Print batch of squared numbers
        println!("Loaded batch of {} items:", items.len());
        for item in items {
            println!("  {}", item);
        }
        Ok(())
    }

    async fn pre_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
        println!("Starting ETL pipeline...");
        Ok(())
    }

    async fn post_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
        println!("ETL pipeline completed!");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize tracing for logs
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Create pipeline
    let pipeline = NumberSquaringPipeline { max_number: 20 };
    let etl = ETL::new(Box::new(pipeline));

    // Configure batch processing
    let config = Arc::new(Config {
        batch_size: 5,
        timeout: std::time::Duration::from_secs(1),
        worker_num: 2,
    });

    // Run pipeline with cancellation support
    let cancel = CancellationToken::new();

    println!("\nRunning ETL pipeline:");
    println!("- Extracting numbers 1-20");
    println!("- Transforming by squaring");
    println!("- Loading in batches of 5\n");

    etl.pre_process(&cancel).await?;
    etl.run(config, &cancel).await?;

    Ok(())
}
```

**Step 3: Test the example**

Run: `cargo run --example simple_etl`
Expected: Output shows batches of squared numbers being processed

**Step 4: Create a simpler bucket-only example**

Create `examples/simple_bucket.rs`:
```rust
//! Simple bucket example showing concurrent batch processing.
//!
//! Run with: cargo run --example simple_bucket

use etl_rust::bucket::{Bucket, Config};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Create bucket configuration
    let config = Arc::new(Config {
        batch_size: 10,
        timeout: std::time::Duration::from_secs(1),
        worker_num: 2,
    });

    let bucket: Bucket<i32> = Bucket::new(config);
    let bucket_clone = bucket.clone();
    let cancel = CancellationToken::new();

    println!("Starting bucket processor...");
    println!("- Batch size: 10");
    println!("- Workers: 2");
    println!("- Processing 50 items\n");

    // Producer: send items
    let cancel_clone = cancel.clone();
    let producer = tokio::spawn(async move {
        for i in 0..50 {
            if let Err(e) = bucket_clone.consume(&cancel_clone, i).await {
                eprintln!("Failed to consume item {}: {}", i, e);
                break;
            }
        }
        bucket_clone.close();
    });

    // Consumer: process batches
    let processor = bucket.run(&cancel, |_ctx, items: &[i32]| async move {
        println!("Processing batch of {} items: {:?}", items.len(), items);
        // Simulate some work
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        Ok(())
    });

    // Wait for both to complete
    let (prod_result, proc_result) = tokio::join!(producer, processor);

    prod_result?;
    proc_result?;

    println!("\nBucket processing completed!");
    Ok(())
}
```

**Step 5: Test bucket example**

Run: `cargo run --example simple_bucket`
Expected: Shows batches being processed by workers

**Step 6: Add examples to README if it exists**

If `README.md` exists, add a section about examples:
```markdown
## Examples

Run the examples to see the framework in action:

```bash
# Simple bucket batch processing
cargo run --example simple_bucket

# Full ETL pipeline
cargo run --example simple_etl
```
```

**Step 7: Commit**

```bash
git add examples/simple_etl.rs examples/simple_bucket.rs README.md
git commit -m "docs: add working examples for bucket and ETL

- simple_bucket: Basic concurrent batch processing
- simple_etl: Full ETL pipeline with extract-transform-load
- Demonstrates real-world usage patterns"
```

---

## Task 16: Stream Transformations (R4.2 - P3)

**Files:**
- Modify: `src/etl/processor.rs:45-77`

**Step 1: Document current buffering behavior**

Current implementation uses `join_all` which waits for all transforms to complete.
This is simple but not optimal for long-running transforms.

**Step 2: Implement streaming with buffer_unordered**

Modify `src/etl/processor.rs:1`:
```rust
use async_trait::async_trait;
use futures::future::join_all;
use futures::StreamExt;  // Add this
use mpsc::Receiver;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::bucket::{Bucket, BucketError, Config};
```

Modify `src/etl/processor.rs:59-74`:
```rust
                .run(
                    &cancel_clone,
                    move |ctx: &CancellationToken, items: &[E]| {
                        let etl = Arc::clone(&etl_clone);
                        let items = items.to_vec();
                        let ctx = ctx.clone();

                        async move {
                            // Stream transforms with concurrency limit
                            // Process up to 10 transforms concurrently
                            let transforms: Vec<T> = futures::stream::iter(items.iter())
                                .map(|item| {
                                    let etl = Arc::clone(&etl);
                                    let ctx = ctx.clone();
                                    async move { etl.transform(&ctx, item).await }
                                })
                                .buffer_unordered(10)
                                .collect()
                                .await;

                            etl.load(&ctx, transforms)
                                .await
                                .map_err(|e| BucketError::ProcessorError(Box::new(e)))
                        }
                    },
                )
                .await
```

**Step 3: Add configuration for concurrency limit**

The concurrency limit (10) should be configurable. For now, use a reasonable default.

Alternative: Make it configurable via Config:

Add to `src/bucket/config.rs`:
```rust
pub(crate) transform_concurrency: usize,
```

With default:
```rust
#[builder(default = "10")]
pub(crate) transform_concurrency: usize,
```

And getter:
```rust
    /// Returns the maximum number of concurrent transforms
    #[inline]
    pub fn transform_concurrency(&self) -> usize {
        self.transform_concurrency
    }
```

But this adds complexity. For P3, keep it simple with hardcoded limit.

**Step 4: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 5: Benchmark if possible (optional)**

Create a benchmark to show performance improvement:

Run: `cargo build --release --example simple_etl`
Time the execution and compare.

**Step 6: Commit**

```bash
git add src/etl/processor.rs
git commit -m "perf: use streaming transforms with buffer_unordered

- Process up to 10 transforms concurrently
- Better latency than join_all waiting for all
- Reduces memory pressure for large batches"
```

---

## Verification

After completing all tasks, run full test suite and checks:

**Step 1: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 2: Run with release optimizations**

Run: `cargo test --release`
Expected: All tests pass faster

**Step 3: Check for warnings**

Run: `cargo clippy -- -D warnings`
Expected: No clippy warnings

**Step 4: Check formatting**

Run: `cargo fmt --check`
Expected: Code is properly formatted

**Step 5: Generate documentation**

Run: `cargo doc --no-deps --open`
Expected: Clean documentation with all examples

**Step 6: Run examples**

Run: `cargo run --example simple_bucket`
Run: `cargo run --example simple_etl`
Expected: Both examples run successfully

**Step 7: Final commit**

```bash
git add .
git commit -m "chore: verify all improvements complete

- All tests passing
- Documentation complete
- Examples working
- No clippy warnings"
```

---

## Summary

This plan implements all 16 recommendations from the code review:

**Completed:**
- ✅ R1.1: thiserror for better error handling (Task 1-2)
- ✅ R1.2: MultipleErrors variant (Task 3)
- ✅ R1.3: Unified error types via thiserror (Task 2)
- ✅ R2.1: Removed Clone bound (Task 4)
- ✅ R2.2: Fixed ETL cloning (Task 11)
- ✅ R2.3: Improved Arc constructor (Task 12)
- ✅ R2.4: Private Config fields (Task 9)
- ✅ R3.1: Renamed Processor traits (Task 5)
- ✅ R3.2: Working examples (Task 15)
- ✅ R3.3: English comments (Task 6)
- ✅ R3.4: Comprehensive docs (Task 14)
- ✅ R4.2: Stream transforms (Task 16)
- ✅ R4.3: Better channel capacity (Task 10)
- ✅ R4.4: Structured logging (Task 7-8)
- ✅ R4.5: Inline hints (Task 13)

**Total:** 16 tasks implementing all recommendations from P1-P3.

**Estimated time:** 3-5 hours total
- P1 tasks (1-6): ~1.5 hours
- P2 tasks (7-10, 14): ~2 hours
- P3 tasks (11-13, 15-16): ~1.5 hours
