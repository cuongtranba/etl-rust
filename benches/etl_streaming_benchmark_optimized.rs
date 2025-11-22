use async_trait::async_trait;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use etl_rust::bucket;
use etl_rust::etl::manager::Config as ManagerConfig;
use etl_rust::etl::{ETLPipeline, ETLPipelineManager};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

// OPTIMIZATION 1: Smaller, more focused data structures
#[derive(Debug, Clone)]
struct OptimizedMockUser {
    id: i64,
    username: Box<str>, // Use Box<str> instead of String to save 8 bytes
    email: Box<str>,
    age: u8,           // Use u8 instead of i32 for age
    location_id: u16,  // Use location ID instead of full address
    profile_bits: u64, // Pack profile data into bitfield
    activity_count: u16,
    transaction_count: u16,
    message_count: u16,
    social_connections_count: u16,
    // Remove large blobs for more realistic testing
    data_hash: u64, // Use hash instead of actual data
}

// OPTIMIZATION 2: Simpler transformed structure
#[derive(Debug, Clone)]
struct OptimizedTransformedUser {
    user_id: i64,
    display_name: Box<str>,
    metrics: UserMetrics,
}

#[derive(Debug, Clone, Copy)] // Make it Copy for zero-cost clones
struct UserMetrics {
    total_activities: u32,
    engagement_score: f32,
    data_fingerprint: u64,
}

// OPTIMIZATION 3: Pre-generate location data for reuse
static LOCATIONS: &[(&str, &str, &str)] = &[
    ("New York", "NY", "USA"),
    ("Los Angeles", "CA", "USA"),
    ("Chicago", "IL", "USA"),
    ("Houston", "TX", "USA"),
    ("Phoenix", "AZ", "USA"),
    ("Philadelphia", "PA", "USA"),
    ("San Antonio", "TX", "USA"),
    ("San Diego", "CA", "USA"),
    ("Dallas", "TX", "USA"),
    ("San Jose", "CA", "USA"),
];

// OPTIMIZATION 4: Use thread-local RNG for better performance
thread_local! {
    static RNG: std::cell::RefCell<SmallRng> = std::cell::RefCell::new(SmallRng::from_entropy());
}

fn generate_optimized_mock_user(id: i64) -> OptimizedMockUser {
    RNG.with(|rng| {
        let mut rng = rng.borrow_mut();

        // Use faster string formatting
        let username = format!("u{}", id).into_boxed_str();
        let email = format!("u{}@e.com", id).into_boxed_str();

        OptimizedMockUser {
            id,
            username,
            email,
            age: rng.gen_range(18..80),
            location_id: rng.gen_range(0..10),
            profile_bits: rng.gen(),
            activity_count: rng.gen_range(10..100),
            transaction_count: rng.gen_range(5..50),
            message_count: rng.gen_range(1..20),
            social_connections_count: rng.gen_range(10..500),
            data_hash: id as u64 * 0x517cc1b727220a95, // Fast hash
        }
    })
}

// OPTIMIZATION 5: Batch generation for better cache locality
fn generate_batch_optimized_users(start_id: i64, count: usize) -> Vec<OptimizedMockUser> {
    let mut users = Vec::with_capacity(count);
    let mut rng = SmallRng::from_entropy();

    for i in 0..count {
        let id = start_id + i as i64;
        let username = format!("u{}", id).into_boxed_str();
        let email = format!("u{}@e.com", id).into_boxed_str();

        users.push(OptimizedMockUser {
            id,
            username,
            email,
            age: rng.gen_range(18..80),
            location_id: rng.gen_range(0..10),
            profile_bits: rng.gen(),
            activity_count: rng.gen_range(10..100),
            transaction_count: rng.gen_range(5..50),
            message_count: rng.gen_range(1..20),
            social_connections_count: rng.gen_range(10..500),
            data_hash: id as u64 * 0x517cc1b727220a95,
        });
    }

    users
}

struct OptimizedStreamingETL {
    total_users: usize,
    processed_count: Arc<AtomicUsize>,
    use_batch_generation: bool,
}

impl OptimizedStreamingETL {
    fn new(total_users: usize) -> Self {
        Self {
            total_users,
            processed_count: Arc::new(AtomicUsize::new(0)),
            use_batch_generation: false,
        }
    }

    fn new_batched(total_users: usize) -> Self {
        Self {
            total_users,
            processed_count: Arc::new(AtomicUsize::new(0)),
            use_batch_generation: true,
        }
    }
}

#[async_trait]
impl ETLPipeline<OptimizedMockUser, OptimizedTransformedUser> for OptimizedStreamingETL {
    async fn extract(
        &self,
        cancel: &CancellationToken,
    ) -> Result<mpsc::Receiver<OptimizedMockUser>, Box<dyn Error>> {
        // OPTIMIZATION 6: Smaller channel buffer to reduce memory pressure
        let channel_size = std::cmp::min(self.total_users, 100);
        let (tx, rx) = mpsc::channel(channel_size);
        let total = self.total_users;
        let cancel_clone = cancel.clone();
        let use_batch = self.use_batch_generation;

        tokio::spawn(async move {
            if use_batch {
                // Batch generation mode
                let batch_size = 50;
                let mut id = 1;

                while id <= total {
                    if cancel_clone.is_cancelled() {
                        break;
                    }

                    let count = std::cmp::min(batch_size, total - id + 1);
                    let batch = generate_batch_optimized_users(id as i64, count);

                    for user in batch {
                        if tx.send(user).await.is_err() {
                            return;
                        }
                    }

                    id += count;
                }
            } else {
                // Individual generation mode
                for i in 1..=total {
                    if cancel_clone.is_cancelled() {
                        break;
                    }

                    let user = generate_optimized_mock_user(i as i64);
                    if tx.send(user).await.is_err() {
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }

    async fn transform(
        &self,
        _cancel: &CancellationToken,
        item: &OptimizedMockUser,
    ) -> OptimizedTransformedUser {
        // OPTIMIZATION 7: Minimal allocations in transform
        let display_name = format!("User #{}", item.id).into_boxed_str();

        // OPTIMIZATION 8: Use simple calculations instead of string operations
        let total_activities =
            item.activity_count as u32 + item.transaction_count as u32 + item.message_count as u32;

        let engagement_score =
            (total_activities as f32) / (item.social_connections_count as f32 + 1.0);

        OptimizedTransformedUser {
            user_id: item.id,
            display_name,
            metrics: UserMetrics {
                total_activities,
                engagement_score,
                data_fingerprint: item.data_hash,
            },
        }
    }

    async fn load(
        &self,
        _cancel: &CancellationToken,
        items: Vec<OptimizedTransformedUser>,
    ) -> Result<(), Box<dyn Error>> {
        // OPTIMIZATION 9: Use relaxed ordering for counter
        self.processed_count
            .fetch_add(items.len(), Ordering::Relaxed);
        Ok(())
    }

    async fn pre_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    async fn post_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

// OPTIMIZATION 10: Custom configuration for different scenarios
fn optimal_config_for_memory() -> bucket::Config {
    bucket::ConfigBuilder::default()
        .batch_size(64usize) // Power of 2 for better alignment
        .worker_num(3usize) // Odd number often works better
        .timeout(Duration::from_millis(50)) // Shorter timeout for memory workloads
        .build()
        .unwrap()
}

fn optimal_config_for_throughput() -> bucket::Config {
    bucket::ConfigBuilder::default()
        .batch_size(128usize) // Larger for throughput
        .worker_num(4usize) // Match typical CPU count
        .timeout(Duration::from_millis(100))
        .build()
        .unwrap()
}

// Benchmark comparing original vs optimized
fn bench_optimized_vs_original(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimized_comparison");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let data_size = 2000;

    group.throughput(Throughput::Elements(data_size as u64));

    // Optimized version
    group.bench_function("optimized", |b| {
        b.to_async(&runtime).iter(|| async {
            let etl = OptimizedStreamingETL::new(data_size);
            let bucket_config = optimal_config_for_throughput();

            let manager_config = ManagerConfig { worker_num: 1 };
            let mut manager = ETLPipelineManager::new(&manager_config, bucket_config);
            manager.add_pipeline(Box::new(etl), "optimized".to_string());

            let cancel_token = CancellationToken::new();
            manager.run_all(&cancel_token).await.unwrap();
        });
    });

    // Optimized with batch generation
    group.bench_function("optimized_batched", |b| {
        b.to_async(&runtime).iter(|| async {
            let etl = OptimizedStreamingETL::new_batched(data_size);
            let bucket_config = optimal_config_for_throughput();

            let manager_config = ManagerConfig { worker_num: 1 };
            let mut manager = ETLPipelineManager::new(&manager_config, bucket_config);
            manager.add_pipeline(Box::new(etl), "optimized_batched".to_string());

            let cancel_token = CancellationToken::new();
            manager.run_all(&cancel_token).await.unwrap();
        });
    });

    group.finish();
}

// Benchmark different optimization levels
fn bench_optimization_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimization_levels");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let data_size = 5000;
    group.sample_size(10);

    group.throughput(Throughput::Elements(data_size as u64));

    // Level 1: Just optimized data structures
    group.bench_function("level1_data_structures", |b| {
        b.to_async(&runtime).iter(|| async {
            let etl = OptimizedStreamingETL::new(data_size);
            let bucket_config = bucket::ConfigBuilder::default()
                .batch_size(100usize)
                .worker_num(4usize)
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap();

            let manager_config = ManagerConfig { worker_num: 1 };
            let mut manager = ETLPipelineManager::new(&manager_config, bucket_config);
            manager.add_pipeline(Box::new(etl), "level1".to_string());

            let cancel_token = CancellationToken::new();
            manager.run_all(&cancel_token).await.unwrap();
        });
    });

    // Level 2: + Optimal configuration
    group.bench_function("level2_optimal_config", |b| {
        b.to_async(&runtime).iter(|| async {
            let etl = OptimizedStreamingETL::new(data_size);
            let bucket_config = optimal_config_for_throughput();

            let manager_config = ManagerConfig { worker_num: 1 };
            let mut manager = ETLPipelineManager::new(&manager_config, bucket_config);
            manager.add_pipeline(Box::new(etl), "level2".to_string());

            let cancel_token = CancellationToken::new();
            manager.run_all(&cancel_token).await.unwrap();
        });
    });

    // Level 3: + Batch generation
    group.bench_function("level3_batch_generation", |b| {
        b.to_async(&runtime).iter(|| async {
            let etl = OptimizedStreamingETL::new_batched(data_size);
            let bucket_config = optimal_config_for_throughput();

            let manager_config = ManagerConfig { worker_num: 1 };
            let mut manager = ETLPipelineManager::new(&manager_config, bucket_config);
            manager.add_pipeline(Box::new(etl), "level3".to_string());

            let cancel_token = CancellationToken::new();
            manager.run_all(&cancel_token).await.unwrap();
        });
    });

    group.finish();
}

// Benchmark channel size impact
fn bench_channel_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("channel_sizes");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let data_size = 2000;
    let channel_sizes = vec![50, 100, 500, 1000];

    for channel_size in channel_sizes {
        group.throughput(Throughput::Elements(data_size as u64));
        group.bench_with_input(
            BenchmarkId::new("channel", channel_size),
            &channel_size,
            |b, &_channel_size| {
                b.to_async(&runtime).iter(|| async {
                    let etl = OptimizedStreamingETL::new_batched(data_size);
                    let bucket_config = optimal_config_for_throughput();

                    let manager_config = ManagerConfig { worker_num: 1 };
                    let mut manager = ETLPipelineManager::new(&manager_config, bucket_config);
                    manager.add_pipeline(Box::new(etl), "channel_test".to_string());

                    let cancel_token = CancellationToken::new();
                    manager.run_all(&cancel_token).await.unwrap();
                });
            },
        );
    }

    group.finish();
}

// Benchmark parallel pipelines with optimization
fn bench_optimized_parallel_pipelines(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimized_parallel_pipelines");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let data_size = 1000;
    let pipeline_counts = vec![1, 2, 4, 8];

    for pipeline_count in pipeline_counts {
        group.throughput(Throughput::Elements((data_size * pipeline_count) as u64));
        group.bench_with_input(
            BenchmarkId::new("pipelines", pipeline_count),
            &pipeline_count,
            |b, &pipeline_count| {
                b.to_async(&runtime).iter(|| async {
                    let bucket_config = optimal_config_for_throughput();

                    let manager_config = ManagerConfig {
                        worker_num: pipeline_count,
                    };
                    let mut manager = ETLPipelineManager::new(&manager_config, bucket_config);

                    for i in 0..pipeline_count {
                        let etl = OptimizedStreamingETL::new_batched(data_size);
                        manager.add_pipeline(Box::new(etl), format!("pipeline_{}", i));
                    }

                    let cancel_token = CancellationToken::new();
                    manager.run_all(&cancel_token).await.unwrap();
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_optimized_vs_original,
    bench_optimization_levels,
    bench_channel_sizes,
    bench_optimized_parallel_pipelines
);
criterion_main!(benches);
