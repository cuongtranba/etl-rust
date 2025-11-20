//! Simple bucket example showing concurrent batch processing.
//!
//! Run with: cargo run --example simple_bucket

use async_trait::async_trait;
use etl_rust::bucket::{Bucket, BatchProcessor, BucketError, ConfigBuilder};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// Simple processor that prints batch information
struct PrintingProcessor;

#[async_trait]
impl BatchProcessor<i32> for PrintingProcessor {
    async fn process(&self, _ctx: &CancellationToken, items: &[i32]) -> Result<(), BucketError> {
        println!("Processing batch of {} items: {:?}", items.len(), items);
        // Simulate some work
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Create bucket configuration using builder
    let config = Arc::new(
        ConfigBuilder::default()
            .batch_size(10usize)
            .timeout(Duration::from_secs(1))
            .worker_num(2usize)
            .build()?
    );

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
    let processor = bucket.run(&cancel, PrintingProcessor);

    // Wait for both to complete
    let (prod_result, proc_result) = tokio::join!(producer, processor);

    prod_result?;
    proc_result?;

    println!("\nBucket processing completed!");
    Ok(())
}
