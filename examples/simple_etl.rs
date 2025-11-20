//! Simple ETL example demonstrating the etl-rust framework.
//!
//! This example extracts numbers 1-20, transforms them by squaring,
//! and loads them by printing to stdout.
//!
//! Run with: cargo run --example simple_etl

use async_trait::async_trait;
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
        let cancel_clone = cancel.clone();

        // Spawn producer task
        tokio::spawn(async move {
            for i in 1..=max {
                if cancel_clone.is_cancelled() {
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
    let etl = ETL::from_box(Box::new(pipeline));

    // Configure batch processing
    use etl_rust::bucket::ConfigBuilder;
    let config = Arc::new(
        ConfigBuilder::default()
            .batch_size(5usize)
            .timeout(std::time::Duration::from_secs(1))
            .worker_num(2usize)
            .build()?
    );

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
