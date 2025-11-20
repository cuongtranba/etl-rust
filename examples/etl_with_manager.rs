//! Example demonstrating how to use ETLPipelineManager with ETLPipeline
//!
//! This example shows how to:
//! 1. Create custom ETL pipelines with specific types
//! 2. Use ETLPipelineAdapter to integrate with ETLPipelineManager
//! 3. Run multiple pipelines concurrently
//! 4. Mix ETLPipeline and ETLRunner implementations

use async_trait::async_trait;
use etl_rust::bucket;
use etl_rust::etl::manager::ETLRunner;
use etl_rust::etl::{ETLPipeline, ETLPipelineManager};
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Example ETL pipeline that processes integers to strings
struct NumberToStringPipeline {
    max_number: i32,
}

#[async_trait]
impl ETLPipeline<i32, String> for NumberToStringPipeline {
    async fn extract(
        &self,
        _cancel: &CancellationToken,
    ) -> Result<mpsc::Receiver<i32>, Box<dyn Error>> {
        let (tx, rx) = mpsc::channel(100);
        let max = self.max_number;

        tokio::spawn(async move {
            for i in 1..=max {
                if tx.send(i).await.is_err() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });

        Ok(rx)
    }

    async fn transform(&self, _cancel: &CancellationToken, item: &i32) -> String {
        // Simulate some processing
        format!("Number: {} (squared: {})", item, item * item)
    }

    async fn load(
        &self,
        _cancel: &CancellationToken,
        items: Vec<String>,
    ) -> Result<(), Box<dyn Error>> {
        println!("Loading batch of {} items:", items.len());
        for item in &items {
            println!("  - {}", item);
        }
        Ok(())
    }

    async fn pre_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
        println!("Starting NumberToStringPipeline...");
        Ok(())
    }

    async fn post_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
        println!("NumberToStringPipeline completed!");
        Ok(())
    }
}

/// Example of a simple ETLRunner that doesn't use ETLPipeline
struct SimpleRunner {
    name: String,
}

#[async_trait]
impl ETLRunner for SimpleRunner {
    fn name(&self) -> &str {
        &self.name
    }

    async fn run(
        &self,
        config: Arc<bucket::Config>,
        _cancel: &CancellationToken,
    ) -> Result<(), Box<dyn Error + Send>> {
        println!(
            "Running SimpleRunner: {} with {} workers",
            self.name,
            config.worker_num()
        );

        // Simulate some work
        tokio::time::sleep(Duration::from_millis(500)).await;

        println!("SimpleRunner {} completed!", self.name);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("=== ETL Pipeline Manager Example ===\n");

    // Configure the manager
    let config = etl_rust::etl::manager::Config::new(4); // 4 concurrent workers
    let bucket_config = Arc::new(
        bucket::ConfigBuilder::default()
            .batch_size(2usize)
            .timeout(Duration::from_millis(100))
            .build()
            .unwrap(),
    );
    let mut manager = ETLPipelineManager::new(&config, bucket_config);

    // Add an ETL pipeline using the add_pipeline method
    println!("Adding NumberToStringPipeline...");
    let bucket_config1 = bucket::ConfigBuilder::default()
        .batch_size(2usize) // batch_size - process 2 items at a time
        .timeout(Duration::from_millis(100)) // timeout for batching
        .build()
        .unwrap();
    manager.add_pipeline(
        Box::new(NumberToStringPipeline { max_number: 5 }),
        "numbers_pipeline".to_string(),
        bucket_config1,
    );

    // Add another ETL pipeline with different parameters
    println!("Adding second NumberToStringPipeline...");
    let bucket_config2 = bucket::ConfigBuilder::default()
        .batch_size(1usize) // smaller batch size
        .timeout(Duration::from_millis(50))
        .build()
        .unwrap();
    manager.add_pipeline(
        Box::new(NumberToStringPipeline { max_number: 3 }),
        "small_numbers_pipeline".to_string(),
        bucket_config2,
    );

    // Add a simple runner (non-ETL pipeline)
    println!("Adding SimpleRunner...");
    manager.add_runner(Arc::new(SimpleRunner {
        name: "simple_task".to_string(),
    }));

    println!("\n--- Starting all pipelines ---\n");

    let cancel = CancellationToken::new();

    // Set up graceful shutdown
    let cancel_clone = cancel.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        println!("\nShutdown signal received...");
        cancel_clone.cancel();
    });

    // Run all pipelines
    match manager.run_all(&cancel).await {
        Ok(_) => println!("\n=== All pipelines completed successfully! ==="),
        Err(e) => eprintln!("\n=== Error running pipelines: {} ===", e),
    }

    Ok(())
}
