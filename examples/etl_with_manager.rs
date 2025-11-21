//! Example demonstrating how to use ETLPipelineManager with multiple ETL pipelines
//!
//! This example shows how to:
//! 1. Create custom ETL pipelines with specific types
//! 2. Use the refactored ETLPipelineManager with zero-copy config sharing
//! 3. Run 4-5 pipelines concurrently with proper error handling
//! 4. Mix ETLPipeline and ETLRunner implementations

use async_trait::async_trait;
use etl_rust::bucket;
use etl_rust::etl::manager::{ETLError, ETLRunner};
use etl_rust::etl::{ETLPipeline, ETLPipelineManager};
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Pipeline 1: Processes integers to strings
struct NumberToStringPipeline {
    name: String,
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
        format!("Number: {} (squared: {})", item, item * item)
    }

    async fn load(
        &self,
        _cancel: &CancellationToken,
        items: Vec<String>,
    ) -> Result<(), Box<dyn Error>> {
        println!("[{}] Loading batch of {} items:", self.name, items.len());
        for item in &items {
            println!("  - {}", item);
        }
        Ok(())
    }

    async fn pre_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
        println!("[{}] Starting pipeline...", self.name);
        Ok(())
    }

    async fn post_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
        println!("[{}] Pipeline completed!", self.name);
        Ok(())
    }
}

/// Pipeline 2: Processes strings to uppercase
struct StringToUpperPipeline {
    name: String,
    items: Vec<String>,
}

#[async_trait]
impl ETLPipeline<String, String> for StringToUpperPipeline {
    async fn extract(
        &self,
        _cancel: &CancellationToken,
    ) -> Result<mpsc::Receiver<String>, Box<dyn Error>> {
        let (tx, rx) = mpsc::channel(50);
        let items = self.items.clone();

        tokio::spawn(async move {
            for item in items {
                if tx.send(item).await.is_err() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(15)).await;
            }
        });

        Ok(rx)
    }

    async fn transform(&self, _cancel: &CancellationToken, item: &String) -> String {
        item.to_uppercase()
    }

    async fn load(
        &self,
        _cancel: &CancellationToken,
        items: Vec<String>,
    ) -> Result<(), Box<dyn Error>> {
        println!(
            "[{}] Loaded {} uppercase items: {:?}",
            self.name,
            items.len(),
            items
        );
        Ok(())
    }

    async fn pre_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
        println!("[{}] Starting string processing...", self.name);
        Ok(())
    }

    async fn post_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
        println!("[{}] String processing completed!", self.name);
        Ok(())
    }
}

/// Pipeline 3: Processes data records
#[derive(Clone)]
struct DataRecord {
    id: u64,
    value: f64,
}

struct DataProcessingPipeline {
    name: String,
    record_count: usize,
}

#[async_trait]
impl ETLPipeline<DataRecord, String> for DataProcessingPipeline {
    async fn extract(
        &self,
        _cancel: &CancellationToken,
    ) -> Result<mpsc::Receiver<DataRecord>, Box<dyn Error>> {
        let (tx, rx) = mpsc::channel(100);
        let count = self.record_count;

        tokio::spawn(async move {
            for i in 0..count {
                let record = DataRecord {
                    id: i as u64,
                    value: (i as f64) * 1.5,
                };
                if tx.send(record).await.is_err() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        });

        Ok(rx)
    }

    async fn transform(&self, _cancel: &CancellationToken, item: &DataRecord) -> String {
        format!("Record[{}] = {:.2}", item.id, item.value * 2.0)
    }

    async fn load(
        &self,
        _cancel: &CancellationToken,
        items: Vec<String>,
    ) -> Result<(), Box<dyn Error>> {
        println!("[{}] Processed {} data records", self.name, items.len());
        Ok(())
    }

    async fn pre_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
        println!("[{}] Initializing data processing...", self.name);
        Ok(())
    }

    async fn post_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
        println!("[{}] Data processing finalized!", self.name);
        Ok(())
    }
}

/// Pipeline 4: Fast batch processor
struct FastBatchPipeline {
    name: String,
}

#[async_trait]
impl ETLPipeline<i32, i32> for FastBatchPipeline {
    async fn extract(
        &self,
        _cancel: &CancellationToken,
    ) -> Result<mpsc::Receiver<i32>, Box<dyn Error>> {
        let (tx, rx) = mpsc::channel(200);

        tokio::spawn(async move {
            for i in 0..20 {
                if tx.send(i).await.is_err() {
                    break;
                }
            }
        });

        Ok(rx)
    }

    async fn transform(&self, _cancel: &CancellationToken, item: &i32) -> i32 {
        item * 2
    }

    async fn load(
        &self,
        _cancel: &CancellationToken,
        items: Vec<i32>,
    ) -> Result<(), Box<dyn Error>> {
        println!("[{}] Batch processed: {:?}", self.name, items);
        Ok(())
    }

    async fn pre_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
        println!("[{}] Fast batch starting...", self.name);
        Ok(())
    }

    async fn post_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
        println!("[{}] Fast batch done!", self.name);
        Ok(())
    }
}

/// Custom ETLRunner that doesn't use ETLPipeline trait
struct CustomTaskRunner {
    name: String,
    task_count: usize,
}

#[async_trait]
impl ETLRunner for CustomTaskRunner {
    fn name(&self) -> &str {
        &self.name
    }

    async fn run(
        &self,
        config: &bucket::Config,
        _cancel: &CancellationToken,
    ) -> Result<(), ETLError> {
        println!(
            "[{}] Running custom task with batch_size={}",
            self.name,
            config.batch_size()
        );

        // Simulate work
        for i in 0..self.task_count {
            println!("[{}] Task {}/{}", self.name, i + 1, self.task_count);
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        println!("[{}] Custom task completed!", self.name);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("=== ETL Pipeline Manager Example (Refactored) ===\n");
    println!("Running 5 concurrent pipelines with zero-copy config sharing\n");

    // Configure the manager with shared bucket config
    let manager_config = etl_rust::etl::manager::Config::new(4); // 4 concurrent workers
    let bucket_config = bucket::ConfigBuilder::default()
        .batch_size(3usize)
        .timeout(Duration::from_millis(100))
        .build()
        .unwrap();

    let mut manager = ETLPipelineManager::new(&manager_config, bucket_config);

    // Add Pipeline 1: Number processing
    println!("✓ Adding NumberToStringPipeline (processes 1-10)");
    manager.add_pipeline(
        Box::new(NumberToStringPipeline {
            name: "NumberProcessor".to_string(),
            max_number: 10,
        }),
        "number_pipeline".to_string(),
    );

    // Add Pipeline 2: String uppercase conversion
    println!("✓ Adding StringToUpperPipeline (4 items)");
    manager.add_pipeline(
        Box::new(StringToUpperPipeline {
            name: "StringProcessor".to_string(),
            items: vec![
                "hello".to_string(),
                "world".to_string(),
                "rust".to_string(),
                "etl".to_string(),
            ],
        }),
        "string_pipeline".to_string(),
    );

    // Add Pipeline 3: Data record processing
    println!("✓ Adding DataProcessingPipeline (8 records)");
    manager.add_pipeline(
        Box::new(DataProcessingPipeline {
            name: "DataProcessor".to_string(),
            record_count: 8,
        }),
        "data_pipeline".to_string(),
    );

    // Add Pipeline 4: Fast batch processor
    println!("✓ Adding FastBatchPipeline (20 items)");
    manager.add_pipeline(
        Box::new(FastBatchPipeline {
            name: "FastBatch".to_string(),
        }),
        "fast_pipeline".to_string(),
    );

    // Add Pipeline 5: Custom task runner
    println!("✓ Adding CustomTaskRunner (5 tasks)");
    manager.add_runner(Arc::new(CustomTaskRunner {
        name: "CustomTasks".to_string(),
        task_count: 5,
    }));

    println!("\n--- Starting all 5 pipelines concurrently ---\n");

    let cancel = CancellationToken::new();

    // Set up graceful shutdown
    let cancel_clone = cancel.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        println!("\nShutdown signal received...");
        cancel_clone.cancel();
    });

    // Run all pipelines
    let start = std::time::Instant::now();
    match manager.run_all(&cancel).await {
        Ok(_) => {
            let duration = start.elapsed();
            println!(
                "\n=== All 5 pipelines completed successfully in {:.2}s ===",
                duration.as_secs_f64()
            );
            Ok(())
        }
        Err(e) => {
            eprintln!("\n=== Error running pipelines: {} ===", e);
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )) as Box<dyn Error>)
        }
    }
}
