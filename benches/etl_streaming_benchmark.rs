use async_trait::async_trait;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use etl_rust::bucket;
use etl_rust::etl::manager::Config as ManagerConfig;
use etl_rust::etl::{ETLPipeline, ETLPipelineManager};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockUser {
    id: i64,
    username: String,
    email: String,
    first_name: String,
    last_name: String,
    age: i32,
    address: MockAddress,
    profile: MockProfile,
    preferences: MockPreferences,
    activity_log: Vec<MockLogEntry>,
    transactions: Vec<MockTransaction>,
    messages: Vec<MockMessage>,
    social_media: MockSocialMedia,
    large_data: MockLargeData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockAddress {
    street: String,
    city: String,
    state: String,
    zip_code: String,
    country: String,
    coordinates: MockCoordinates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockCoordinates {
    lat: f64,
    lng: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockProfile {
    bio: String,
    interests: Vec<String>,
    skills: Vec<String>,
    education: Vec<MockEducation>,
    experience: Vec<MockExperience>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockEducation {
    institution: String,
    degree: String,
    year: i32,
    description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockExperience {
    company: String,
    position: String,
    duration: String,
    description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockPreferences {
    language: String,
    timezone: String,
    notifications: Vec<String>,
    settings: Vec<MockSetting>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockSetting {
    key: String,
    value: String,
    metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockLogEntry {
    key: String,
    value: String,
    metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockTransaction {
    key: String,
    value: String,
    metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockMessage {
    id: String,
    from: String,
    to: String,
    subject: String,
    body: String,
    read: bool,
    attachments: Vec<MockAttachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockAttachment {
    name: String,
    size: i32,
    file_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockSocialMedia {
    connections: Vec<String>,
    posts: Vec<MockPost>,
    groups: Vec<MockGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockPost {
    key: String,
    value: String,
    metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockGroup {
    id: String,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockLargeData {
    blob1: String,
    blob2: String,
    blob3: String,
    blob4: String,
    blob5: String,
}

#[derive(Debug, Clone)]
struct TransformedUser {
    user_id: i64,
    full_name: String,
    email: String,
    location: String,
    profile_summary: String,
    activity_count: usize,
    transaction_count: usize,
    message_count: usize,
    social_connections: usize,
    data_size: usize,
}

fn generate_mock_user(id: i64) -> MockUser {
    let mut rng = rand::thread_rng();

    MockUser {
        id,
        username: format!("user_{}", id),
        email: format!("user{}@example.com", id),
        first_name: format!("FirstName{}", id),
        last_name: format!("LastName{}", id),
        age: rng.gen_range(18..80),
        address: MockAddress {
            street: format!("{} Main St", rng.gen_range(100..9999)),
            city: format!("City{}", rng.gen_range(1..100)),
            state: format!("State{}", rng.gen_range(1..50)),
            zip_code: format!("{:05}", rng.gen_range(10000..99999)),
            country: "USA".to_string(),
            coordinates: MockCoordinates {
                lat: rng.gen_range(-90.0..90.0),
                lng: rng.gen_range(-180.0..180.0),
            },
        },
        profile: MockProfile {
            bio: format!("Bio for user {}", id).repeat(10),
            interests: (0..5).map(|i| format!("Interest{}", i)).collect(),
            skills: (0..7).map(|i| format!("Skill{}", i)).collect(),
            education: (0..2)
                .map(|i| MockEducation {
                    institution: format!("University {}", i),
                    degree: format!("Degree {}", i),
                    year: 2020 - i,
                    description: format!("Education description {}", i).repeat(5),
                })
                .collect(),
            experience: (0..3)
                .map(|i| MockExperience {
                    company: format!("Company {}", i),
                    position: format!("Position {}", i),
                    duration: format!("{} years", i + 1),
                    description: format!("Experience description {}", i).repeat(5),
                })
                .collect(),
        },
        preferences: MockPreferences {
            language: "en".to_string(),
            timezone: "UTC".to_string(),
            notifications: vec!["email".to_string(), "sms".to_string()],
            settings: (0..10)
                .map(|i| MockSetting {
                    key: format!("setting_{}", i),
                    value: format!("value_{}", i),
                    metadata: Some(format!("metadata_{}", i)),
                })
                .collect(),
        },
        activity_log: (0..20)
            .map(|i| MockLogEntry {
                key: format!("activity_{}", i),
                value: format!("action_{}", i),
                metadata: Some(format!("meta_{}", i)),
            })
            .collect(),
        transactions: (0..15)
            .map(|i| MockTransaction {
                key: format!("transaction_{}", i),
                value: format!("amount_{}", rng.gen_range(10..1000)),
                metadata: Some(format!("tx_meta_{}", i)),
            })
            .collect(),
        messages: (0..10)
            .map(|i| MockMessage {
                id: format!("msg_{}_{}", id, i),
                from: format!("user_{}", rng.gen_range(1..1000)),
                to: format!("user_{}", id),
                subject: format!("Subject {}", i),
                body: format!("Message body {}", i).repeat(10),
                read: rng.gen_bool(0.5),
                attachments: (0..3)
                    .map(|j| MockAttachment {
                        name: format!("file_{}_{}.txt", i, j),
                        size: rng.gen_range(1000..100000),
                        file_type: "text/plain".to_string(),
                    })
                    .collect(),
            })
            .collect(),
        social_media: MockSocialMedia {
            connections: (0..50).map(|i| format!("connection_{}", i)).collect(),
            posts: (0..25)
                .map(|i| MockPost {
                    key: format!("post_{}", i),
                    value: format!("Post content {}", i).repeat(20),
                    metadata: Some(format!("post_meta_{}", i)),
                })
                .collect(),
            groups: (0..5)
                .map(|i| MockGroup {
                    id: format!("group_{}", i),
                    name: format!("Group Name {}", i),
                })
                .collect(),
        },
        large_data: MockLargeData {
            blob1: "x".repeat(10000),
            blob2: "y".repeat(10000),
            blob3: "z".repeat(10000),
            blob4: "a".repeat(10000),
            blob5: "b".repeat(10000),
        },
    }
}

struct StreamingETL {
    total_users: usize,
    processed_count: Arc<AtomicUsize>,
}

impl StreamingETL {
    fn new(total_users: usize) -> Self {
        Self {
            total_users,
            processed_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn get_processed_count(&self) -> usize {
        self.processed_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ETLPipeline<MockUser, TransformedUser> for StreamingETL {
    async fn extract(
        &self,
        cancel: &CancellationToken,
    ) -> Result<mpsc::Receiver<MockUser>, Box<dyn Error>> {
        let (tx, rx) = mpsc::channel(1000);
        let total = self.total_users;
        let cancel_clone = cancel.clone();

        tokio::spawn(async move {
            for i in 1..=total {
                if cancel_clone.is_cancelled() {
                    break;
                }

                let user = generate_mock_user(i as i64);
                if tx.send(user).await.is_err() {
                    break;
                }
            }
        });

        Ok(rx)
    }

    async fn transform(&self, _cancel: &CancellationToken, item: &MockUser) -> TransformedUser {
        let full_name = format!("{} {}", item.first_name, item.last_name);
        let location = format!(
            "{}, {}, {}",
            item.address.city, item.address.state, item.address.country
        );

        let profile_summary = format!(
            "Bio: {}... | Interests: {} | Skills: {} | Education: {} | Experience: {}",
            &item.profile.bio.chars().take(50).collect::<String>(),
            item.profile.interests.len(),
            item.profile.skills.len(),
            item.profile.education.len(),
            item.profile.experience.len()
        );

        let data_size = item.large_data.blob1.len()
            + item.large_data.blob2.len()
            + item.large_data.blob3.len()
            + item.large_data.blob4.len()
            + item.large_data.blob5.len();

        TransformedUser {
            user_id: item.id,
            full_name,
            email: item.email.clone(),
            location,
            profile_summary,
            activity_count: item.activity_log.len(),
            transaction_count: item.transactions.len(),
            message_count: item.messages.len(),
            social_connections: item.social_media.connections.len(),
            data_size,
        }
    }

    async fn load(
        &self,
        _cancel: &CancellationToken,
        items: Vec<TransformedUser>,
    ) -> Result<(), Box<dyn Error>> {
        self.processed_count
            .fetch_add(items.len(), Ordering::SeqCst);
        Ok(())
    }

    async fn pre_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    async fn post_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

fn bench_streaming_etl_batch_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_etl_batch_sizes");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let data_size = 1000;
    let batch_sizes = vec![10usize, 50, 100, 500];

    for batch_size in batch_sizes {
        group.throughput(Throughput::Elements(data_size as u64));
        group.bench_with_input(
            BenchmarkId::new("batch_size", batch_size),
            &batch_size,
            |b, &batch_size| {
                b.to_async(&runtime).iter(|| async move {
                    let etl = StreamingETL::new(data_size);
                    let bucket_config = bucket::ConfigBuilder::default()
                        .batch_size(batch_size)
                        .worker_num(4usize)
                        .timeout(Duration::from_secs(30))
                        .build()
                        .unwrap();

                    let manager_config = ManagerConfig { worker_num: 1 };
                    let mut manager = ETLPipelineManager::new(&manager_config, bucket_config);
                    manager.add_pipeline(Box::new(etl), "streaming_benchmark".to_string());

                    let cancel_token = CancellationToken::new();
                    manager.run_all(&cancel_token).await.unwrap();
                });
            },
        );
    }
    group.finish();
}

fn bench_streaming_etl_worker_counts(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_etl_worker_counts");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let data_size = 1000;
    let worker_counts = vec![1usize, 2, 4, 8];

    for worker_count in worker_counts {
        group.throughput(Throughput::Elements(data_size as u64));
        group.bench_with_input(
            BenchmarkId::new("workers", worker_count),
            &worker_count,
            |b, &worker_count| {
                b.to_async(&runtime).iter(|| async move {
                    let etl = StreamingETL::new(data_size);
                    let bucket_config = bucket::ConfigBuilder::default()
                        .batch_size(100usize)
                        .worker_num(worker_count)
                        .timeout(Duration::from_secs(30))
                        .build()
                        .unwrap();

                    let manager_config = ManagerConfig { worker_num: 1 };
                    let mut manager = ETLPipelineManager::new(&manager_config, bucket_config);
                    manager.add_pipeline(Box::new(etl), "streaming_benchmark".to_string());

                    let cancel_token = CancellationToken::new();
                    manager.run_all(&cancel_token).await.unwrap();
                });
            },
        );
    }
    group.finish();
}

fn bench_streaming_etl_data_volumes(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_etl_data_volumes");
    group.sample_size(10);
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let data_volumes = vec![100, 500, 1000, 2000];

    for data_volume in data_volumes {
        group.throughput(Throughput::Elements(data_volume as u64));
        group.bench_with_input(
            BenchmarkId::new("records", data_volume),
            &data_volume,
            |b, &data_volume| {
                b.to_async(&runtime).iter(|| async move {
                    let etl = StreamingETL::new(data_volume);
                    let bucket_config = bucket::ConfigBuilder::default()
                        .batch_size(100usize)
                        .worker_num(4usize)
                        .timeout(Duration::from_secs(60))
                        .build()
                        .unwrap();

                    let manager_config = ManagerConfig { worker_num: 1 };
                    let mut manager = ETLPipelineManager::new(&manager_config, bucket_config);
                    manager.add_pipeline(Box::new(etl), "streaming_benchmark".to_string());

                    let cancel_token = CancellationToken::new();
                    manager.run_all(&cancel_token).await.unwrap();
                });
            },
        );
    }
    group.finish();
}

fn bench_streaming_etl_parallel_pipelines(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_etl_parallel_pipelines");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let data_size = 500;
    let pipeline_counts = vec![1, 2, 4];

    for pipeline_count in pipeline_counts {
        group.throughput(Throughput::Elements((data_size * pipeline_count) as u64));
        group.bench_with_input(
            BenchmarkId::new("pipelines", pipeline_count),
            &pipeline_count,
            |b, &pipeline_count| {
                b.to_async(&runtime).iter(|| async move {
                    let bucket_config = bucket::ConfigBuilder::default()
                        .batch_size(100usize)
                        .worker_num(4usize)
                        .timeout(Duration::from_secs(30))
                        .build()
                        .unwrap();

                    let manager_config = ManagerConfig {
                        worker_num: pipeline_count,
                    };
                    let mut manager = ETLPipelineManager::new(&manager_config, bucket_config);

                    for i in 0..pipeline_count {
                        let etl = StreamingETL::new(data_size);
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

fn bench_streaming_etl_optimal_config(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_etl_optimal");
    group.sample_size(10);
    let runtime = tokio::runtime::Runtime::new().unwrap();

    group.throughput(Throughput::Elements(5000));
    group.bench_function("optimal_5000_records", |b| {
        b.to_async(&runtime).iter(|| async {
            let etl = StreamingETL::new(5000);
            let bucket_config = bucket::ConfigBuilder::default()
                .batch_size(500usize)
                .worker_num(8usize)
                .timeout(Duration::from_secs(120))
                .build()
                .unwrap();

            let manager_config = ManagerConfig { worker_num: 1 };
            let mut manager = ETLPipelineManager::new(&manager_config, bucket_config);
            manager.add_pipeline(Box::new(etl), "optimal_benchmark".to_string());

            let cancel_token = CancellationToken::new();
            manager.run_all(&cancel_token).await.unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_streaming_etl_batch_sizes,
    bench_streaming_etl_worker_counts,
    bench_streaming_etl_data_volumes,
    bench_streaming_etl_parallel_pipelines,
    bench_streaming_etl_optimal_config
);
criterion_main!(benches);
