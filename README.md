# ETL-Rust

A high-performance, async ETL (Extract, Transform, Load) library for Rust with batch processing and parallel worker support.

## Features

- **Async/Await Support**: Built on Tokio for efficient async I/O operations
- **Batch Processing**: Configurable batch sizes with automatic timeout-based flushing
- **Parallel Workers**: Multi-threaded processing with configurable worker count
- **Cancellation Support**: Graceful shutdown using `CancellationToken`
- **Flexible Architecture**: Trait-based design for custom ETL implementations
- **Error Handling**: Comprehensive error handling with custom error types
- **Type Safety**: Leverages Rust's type system for compile-time guarantees

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
etl-rust = "0.1.0"
```

## Quick Start

### Basic ETL Pipeline

```rust
use etl_rust::etl::{ETL, Processor};
use etl_rust::bucket::{Config, BucketError};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;

// Define your data types
#[derive(Clone)]
struct RawData {
    id: u32,
    value: String,
}

struct ProcessedData {
    id: u32,
    processed_value: String,
}

// Implement the ETL processor
struct MyETLProcessor;

#[async_trait]
impl Processor<RawData, ProcessedData> for MyETLProcessor {
    async fn extract(&self, cancel: &CancellationToken) -> Result<mpsc::Receiver<RawData>, Box<dyn Error>> {
        let (tx, rx) = mpsc::channel(100);

        // Spawn extraction task
        tokio::spawn(async move {
            for i in 0..100 {
                if cancel.is_cancelled() {
                    break;
                }
                tx.send(RawData {
                    id: i,
                    value: format!("data_{}", i),
                }).await.unwrap();
            }
        });

        Ok(rx)
    }

    async fn transform(&self, _cancel: &CancellationToken, item: RawData) -> ProcessedData {
        ProcessedData {
            id: item.id,
            processed_value: item.value.to_uppercase(),
        }
    }

    async fn load(&self, _cancel: &CancellationToken, items: Vec<ProcessedData>) -> Result<(), Box<dyn Error>> {
        println!("Loading batch of {} items", items.len());
        // Implement your loading logic here
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
async fn main() {
    let processor = Box::new(MyETLProcessor);
    let etl = ETL::new(processor);

    let config = Arc::new(Config {
        batch_size: 10,
        timeout: Duration::from_secs(5),
        worker_num: 4,
    });

    let cancel = CancellationToken::new();

    // Run the ETL pipeline
    match etl.run(config, &cancel).await {
        Ok(_) => println!("ETL completed successfully"),
        Err(e) => eprintln!("ETL failed: {}", e),
    }
}
```

### Using the Bucket System Directly

The bucket system can also be used independently for batch processing:

```rust
use etl_rust::bucket::{Bucket, Config, Processor, BucketError};
use tokio_util::sync::CancellationToken;
use async_trait::async_trait;

struct MyProcessor;

#[async_trait]
impl Processor<String> for MyProcessor {
    async fn process(
        &self,
        _ctx: &CancellationToken,
        items: &[String],
    ) -> Result<(), BucketError> {
        println!("Processing {} items", items.len());
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let config = Config {
        batch_size: 5,
        timeout: Duration::from_secs(1),
        worker_num: 2,
    };

    let bucket = Bucket::new(config);
    let cancel = CancellationToken::new();

    // Start processing in background
    let bucket_clone = bucket.clone();
    let handle = tokio::spawn(async move {
        bucket_clone.run(&cancel, MyProcessor).await
    });

    // Send items to bucket
    for i in 0..20 {
        bucket.consume(format!("item_{}", i)).await.unwrap();
    }

    // Signal completion
    bucket.close().await;
    handle.await.unwrap().unwrap();
}
```

## Configuration

### Bucket Configuration

The `Config` struct controls batch processing behavior:

```rust
pub struct Config {
    /// Maximum number of items in a batch
    pub batch_size: usize,

    /// Timeout duration for batch processing
    /// If a batch doesn't fill up within this time, it will be processed anyway
    pub timeout: Duration,

    /// Number of parallel worker threads
    pub worker_num: usize,
}
```

### Configuration Guidelines

- **batch_size**: Choose based on your processing requirements and memory constraints
  - Smaller batches = lower latency, more overhead
  - Larger batches = better throughput, higher memory usage

- **timeout**: Prevents small batches from waiting too long
  - Set lower for real-time processing
  - Set higher for batch-oriented workloads

- **worker_num**: Number of parallel processors
  - Set based on CPU cores and I/O characteristics
  - I/O-bound tasks can benefit from more workers than CPU cores

## Architecture

### Components

1. **ETL Module** (`src/etl/`)
   - Orchestrates the complete ETL pipeline
   - Manages extraction, transformation, and loading phases
   - Provides lifecycle hooks (pre_process, post_process)

2. **Bucket Module** (`src/bucket/`)
   - Handles batch processing and worker management
   - Implements timeout-based and size-based batch triggers
   - Manages parallel worker threads

3. **Processor Traits**
   - `etl::Processor<E, T>`: Define ETL pipeline behavior
   - `bucket::Processor<T>`: Define batch processing logic

### Data Flow

```
Extract → Channel → Transform (parallel) → Batch → Load
                         ↑                    ↑
                    Worker Pool          Timeout/Size
                                          Triggers
```

## Error Handling

The library provides custom error types:

```rust
pub enum BucketError {
    ProcessorError(String),
    ChannelClosed,
    Cancelled,
}
```

All errors are propagated through `Result` types for proper handling.

## Testing

Run the test suite:

```bash
cargo test
```

Run tests with output:

```bash
cargo test -- --nocapture
```

Run specific tests:

```bash
cargo test test_bucket_batch_processing
```

## Examples

Check the `tests` modules for more examples:
- `src/bucket/bucket.rs` - Bucket system tests
- `src/etl/tests.rs` - ETL pipeline tests

## Performance Considerations

1. **Memory Usage**: Batch size directly impacts memory consumption
2. **Throughput**: Increase workers and batch size for higher throughput
3. **Latency**: Decrease timeout and batch size for lower latency
4. **Backpressure**: Channel sizes provide natural backpressure

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Roadmap

- [ ] Add metrics and monitoring support
- [ ] Implement retry mechanisms
- [ ] Add more transformation operators
- [ ] Support for distributed processing
- [ ] Add compression and serialization options
- [ ] Implement checkpoint/resume functionality