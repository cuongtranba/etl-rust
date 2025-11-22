mod migration;
mod mongodb_model;
mod postgres_model;

use async_trait::async_trait;
use etl_rust::bucket;
use etl_rust::etl::manager::Config as ManagerConfig;
use etl_rust::etl::{ETLPipeline, ETLPipelineManager};
use futures::stream::TryStreamExt;
use mongodb::bson::doc;
use mongodb_model::User as MongoUser;
use postgres_model::*;
use sea_orm::{Database, DatabaseConnection, EntityTrait, Set};
use std::env;
use std::fs::File;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

struct DbConfig {
    mongodb_url: String,
    postgres_url: String,
}

impl DbConfig {
    fn load() -> Result<Self, env::VarError> {
        dotenvy::dotenv().ok();
        let mongodb_url = env::var("MONGODB_URL")?;
        let postgres_url = env::var("POSTGRES_URL")?;

        Ok(DbConfig {
            mongodb_url,
            postgres_url,
        })
    }
}

#[derive(Clone)]
struct TransformedUser {
    user: users::Model,
    address: addresses::Model,
    profile: profiles::Model,
    education: Vec<education::Model>,
    experience: Vec<experience::Model>,
    preferences: preferences::Model,
    settings: Vec<settings::Model>,
    activity_log: Vec<activity_log::Model>,
    transactions: Vec<transactions::Model>,
    messages: Vec<(messages::Model, Vec<attachments::Model>)>,
    social_media: social_media::Model,
    posts: Vec<posts::Model>,
    groups: Vec<groups::Model>,
    large_data: large_data::Model,
}

struct UserETL {
    mongodb_client: mongodb::Client,
    postgres_client: DatabaseConnection,
}

#[async_trait]
impl ETLPipeline<MongoUser, TransformedUser> for UserETL {
    async fn extract(
        &self,
        cancel: &CancellationToken,
    ) -> Result<mpsc::Receiver<MongoUser>, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::channel(100);

        let collection = self
            .mongodb_client
            .database("sample_db")
            .collection::<MongoUser>("users");

        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            let mut cursor = match collection.find(doc! {}).await {
                Ok(cursor) => cursor,
                Err(e) => {
                    eprintln!("Failed to create cursor: {}", e);
                    return;
                }
            };

            while let Ok(Some(user)) = cursor.try_next().await {
                if cancel_clone.is_cancelled() {
                    break;
                }

                if tx.send(user).await.is_err() {
                    break;
                }
            }
        });

        Ok(rx)
    }

    async fn transform(&self, _cancel: &CancellationToken, item: &MongoUser) -> TransformedUser {
        let user = users::Model {
            id: item.id,
            username: item.username.clone(),
            email: item.email.clone(),
            first_name: item.first_name.clone(),
            last_name: item.last_name.clone(),
            age: item.age,
            created_at: item.created_at.to_chrono().into(),
            updated_at: item.updated_at.to_chrono().into(),
        };

        let address = addresses::Model {
            id: item.id,
            user_id: item.id,
            street: item.address.street.clone(),
            city: item.address.city.clone(),
            state: item.address.state.clone(),
            zip_code: item.address.zip_code.clone(),
            country: item.address.country.clone(),
            coordinates: serde_json::json!({
                "lat": item.address.coordinates.lat,
                "lng": item.address.coordinates.lng
            }),
        };

        let profile = profiles::Model {
            id: item.id,
            user_id: item.id,
            bio: item.profile.bio.clone(),
            interests: serde_json::to_value(&item.profile.interests).unwrap(),
            skills: serde_json::to_value(&item.profile.skills).unwrap(),
        };

        let education: Vec<education::Model> = item
            .profile
            .education
            .iter()
            .enumerate()
            .map(|(idx, edu)| education::Model {
                id: (item.id * 10000 + idx as i64),
                profile_id: item.id,
                institution: edu.institution.clone(),
                degree: edu.degree.clone(),
                year: edu.year,
                description: edu.description.clone(),
            })
            .collect();

        let experience: Vec<experience::Model> = item
            .profile
            .experience
            .iter()
            .enumerate()
            .map(|(idx, exp)| experience::Model {
                id: (item.id * 10000 + idx as i64),
                profile_id: item.id,
                company: exp.company.clone(),
                position: exp.position.clone(),
                duration: exp.duration.clone(),
                description: exp.description.clone(),
            })
            .collect();

        let preferences = preferences::Model {
            id: item.id,
            user_id: item.id,
            language: item.preferences.language.clone(),
            timezone: item.preferences.timezone.clone(),
            notifications: serde_json::to_value(&item.preferences.notifications).unwrap(),
        };

        let settings: Vec<settings::Model> = item
            .preferences
            .settings
            .iter()
            .enumerate()
            .map(|(idx, setting)| settings::Model {
                id: (item.id * 10000 + idx as i64),
                preference_id: item.id,
                key: setting.key.clone(),
                value: setting.value.clone(),
                timestamp: setting.timestamp.to_chrono().into(),
                metadata: setting.metadata.clone(),
            })
            .collect();

        let activity_log: Vec<activity_log::Model> = item
            .activity_log
            .iter()
            .enumerate()
            .map(|(idx, log)| activity_log::Model {
                id: (item.id * 10000 + idx as i64),
                user_id: item.id,
                key: log.key.clone(),
                value: log.value.clone(),
                timestamp: log.timestamp.to_chrono().into(),
                metadata: log.metadata.clone(),
            })
            .collect();

        let transactions: Vec<transactions::Model> = item
            .transactions
            .iter()
            .enumerate()
            .map(|(idx, tx)| transactions::Model {
                id: (item.id * 10000 + idx as i64),
                user_id: item.id,
                key: tx.key.clone(),
                value: tx.value.clone(),
                timestamp: tx.timestamp.to_chrono().into(),
                metadata: tx.metadata.clone(),
            })
            .collect();

        let messages: Vec<(messages::Model, Vec<attachments::Model>)> = item
            .messages
            .iter()
            .map(|msg| {
                let message = messages::Model {
                    id: msg.id.clone(),
                    user_id: item.id,
                    from: msg.from.clone(),
                    to: msg.to.clone(),
                    subject: msg.subject.clone(),
                    body: msg.body.clone(),
                    timestamp: msg.timestamp.to_chrono().into(),
                    read: msg.read,
                };

                let attachments: Vec<attachments::Model> = msg
                    .attachments
                    .iter()
                    .enumerate()
                    .map(|(idx, att)| attachments::Model {
                        id: (item.id * 10000 + idx as i64),
                        message_id: msg.id.clone(),
                        name: att.name.clone(),
                        size: att.size,
                        file_type: att.file_type.clone(),
                    })
                    .collect();

                (message, attachments)
            })
            .collect();

        let social_media = social_media::Model {
            id: item.id,
            user_id: item.id,
            connections: serde_json::to_value(&item.social_media.connections).unwrap(),
        };

        let posts: Vec<posts::Model> = item
            .social_media
            .posts
            .iter()
            .enumerate()
            .map(|(idx, post)| posts::Model {
                id: (item.id * 10000 + idx as i64),
                social_media_id: item.id,
                key: post.key.clone(),
                value: post.value.clone(),
                timestamp: post.timestamp.to_chrono().into(),
                metadata: post.metadata.clone(),
            })
            .collect();

        let groups: Vec<groups::Model> = item
            .social_media
            .groups
            .iter()
            .map(|group| groups::Model {
                id: group.id.clone(),
                social_media_id: item.id,
                name: group.name.clone(),
                joined: group.joined.to_chrono().into(),
            })
            .collect();

        let large_data = large_data::Model {
            id: item.id,
            user_id: item.id,
            blob1: item.large_data.blob1.clone(),
            blob2: item.large_data.blob2.clone(),
            blob3: item.large_data.blob3.clone(),
            blob4: item.large_data.blob4.clone(),
            blob5: item.large_data.blob5.clone(),
        };

        TransformedUser {
            user,
            address,
            profile,
            education,
            experience,
            preferences,
            settings,
            activity_log,
            transactions,
            messages,
            social_media,
            posts,
            groups,
            large_data,
        }
    }

    async fn load(
        &self,
        _cancel: &CancellationToken,
        items: Vec<TransformedUser>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if items.is_empty() {
            return Ok(());
        }

        let items_count = items.len();

        // Collect all entities for batch inserts
        let mut all_users = Vec::new();
        let mut all_addresses = Vec::new();
        let mut all_profiles = Vec::new();
        let mut all_education = Vec::new();
        let mut all_experience = Vec::new();
        let mut all_preferences = Vec::new();
        let mut all_settings = Vec::new();
        let mut all_activity_log = Vec::new();
        let mut all_transactions = Vec::new();
        let mut all_messages = Vec::new();
        let mut all_attachments = Vec::new();
        let mut all_social_media = Vec::new();
        let mut all_posts = Vec::new();
        let mut all_groups = Vec::new();
        let mut all_large_data = Vec::new();

        // Collect all data from items
        for item in items {
            all_users.push(users::ActiveModel {
                id: Set(item.user.id),
                username: Set(item.user.username),
                email: Set(item.user.email),
                first_name: Set(item.user.first_name),
                last_name: Set(item.user.last_name),
                age: Set(item.user.age),
                created_at: Set(item.user.created_at),
                updated_at: Set(item.user.updated_at),
            });

            all_addresses.push(addresses::ActiveModel {
                id: Set(item.address.id),
                user_id: Set(item.address.user_id),
                street: Set(item.address.street),
                city: Set(item.address.city),
                state: Set(item.address.state),
                zip_code: Set(item.address.zip_code),
                country: Set(item.address.country),
                coordinates: Set(item.address.coordinates),
            });

            all_profiles.push(profiles::ActiveModel {
                id: Set(item.profile.id),
                user_id: Set(item.profile.user_id),
                bio: Set(item.profile.bio),
                interests: Set(item.profile.interests),
                skills: Set(item.profile.skills),
            });

            for edu in item.education {
                all_education.push(education::ActiveModel {
                    id: Set(edu.id),
                    profile_id: Set(edu.profile_id),
                    institution: Set(edu.institution),
                    degree: Set(edu.degree),
                    year: Set(edu.year),
                    description: Set(edu.description),
                });
            }

            for exp in item.experience {
                all_experience.push(experience::ActiveModel {
                    id: Set(exp.id),
                    profile_id: Set(exp.profile_id),
                    company: Set(exp.company),
                    position: Set(exp.position),
                    duration: Set(exp.duration),
                    description: Set(exp.description),
                });
            }

            all_preferences.push(preferences::ActiveModel {
                id: Set(item.preferences.id),
                user_id: Set(item.preferences.user_id),
                language: Set(item.preferences.language),
                timezone: Set(item.preferences.timezone),
                notifications: Set(item.preferences.notifications),
            });

            for setting in item.settings {
                all_settings.push(settings::ActiveModel {
                    id: Set(setting.id),
                    preference_id: Set(setting.preference_id),
                    key: Set(setting.key),
                    value: Set(setting.value),
                    timestamp: Set(setting.timestamp),
                    metadata: Set(setting.metadata),
                });
            }

            for log in item.activity_log {
                all_activity_log.push(activity_log::ActiveModel {
                    id: Set(log.id),
                    user_id: Set(log.user_id),
                    key: Set(log.key),
                    value: Set(log.value),
                    timestamp: Set(log.timestamp),
                    metadata: Set(log.metadata),
                });
            }

            for tx in item.transactions {
                all_transactions.push(transactions::ActiveModel {
                    id: Set(tx.id),
                    user_id: Set(tx.user_id),
                    key: Set(tx.key),
                    value: Set(tx.value),
                    timestamp: Set(tx.timestamp),
                    metadata: Set(tx.metadata),
                });
            }

            for (msg, attachments_list) in item.messages {
                all_messages.push(messages::ActiveModel {
                    id: Set(msg.id),
                    user_id: Set(msg.user_id),
                    from: Set(msg.from),
                    to: Set(msg.to),
                    subject: Set(msg.subject),
                    body: Set(msg.body),
                    timestamp: Set(msg.timestamp),
                    read: Set(msg.read),
                });

                for att in attachments_list {
                    all_attachments.push(attachments::ActiveModel {
                        id: Set(att.id),
                        message_id: Set(att.message_id),
                        name: Set(att.name),
                        size: Set(att.size),
                        file_type: Set(att.file_type),
                    });
                }
            }

            all_social_media.push(social_media::ActiveModel {
                id: Set(item.social_media.id),
                user_id: Set(item.social_media.user_id),
                connections: Set(item.social_media.connections),
            });

            for post in item.posts {
                all_posts.push(posts::ActiveModel {
                    id: Set(post.id),
                    social_media_id: Set(post.social_media_id),
                    key: Set(post.key),
                    value: Set(post.value),
                    timestamp: Set(post.timestamp),
                    metadata: Set(post.metadata),
                });
            }

            for group in item.groups {
                all_groups.push(groups::ActiveModel {
                    id: Set(group.id),
                    social_media_id: Set(group.social_media_id),
                    name: Set(group.name),
                    joined: Set(group.joined),
                });
            }

            all_large_data.push(large_data::ActiveModel {
                id: Set(item.large_data.id),
                user_id: Set(item.large_data.user_id),
                blob1: Set(item.large_data.blob1),
                blob2: Set(item.large_data.blob2),
                blob3: Set(item.large_data.blob3),
                blob4: Set(item.large_data.blob4),
                blob5: Set(item.large_data.blob5),
            });
        }

        // Perform batch inserts with error context
        println!("Batch inserting {} users...", all_users.len());
        users::Entity::insert_many(all_users)
            .exec(&self.postgres_client)
            .await
            .map_err(|e| format!("Failed to insert {} users: {}", items_count, e))?;

        println!("Batch inserting {} addresses...", all_addresses.len());
        addresses::Entity::insert_many(all_addresses)
            .exec(&self.postgres_client)
            .await
            .map_err(|e| format!("Failed to insert {} addresses: {}", items_count, e))?;

        println!("Batch inserting {} profiles...", all_profiles.len());
        profiles::Entity::insert_many(all_profiles)
            .exec(&self.postgres_client)
            .await
            .map_err(|e| format!("Failed to insert {} profiles: {}", items_count, e))?;

        if !all_education.is_empty() {
            let count = all_education.len();
            println!("Batch inserting {} education records...", count);
            education::Entity::insert_many(all_education)
                .exec(&self.postgres_client)
                .await
                .map_err(|e| format!("Failed to insert {} education records: {}", count, e))?;
        }

        if !all_experience.is_empty() {
            let count = all_experience.len();
            println!("Batch inserting {} experience records...", count);
            experience::Entity::insert_many(all_experience)
                .exec(&self.postgres_client)
                .await
                .map_err(|e| format!("Failed to insert {} experience records: {}", count, e))?;
        }

        println!("Batch inserting {} preferences...", all_preferences.len());
        preferences::Entity::insert_many(all_preferences)
            .exec(&self.postgres_client)
            .await
            .map_err(|e| format!("Failed to insert {} preferences: {}", items_count, e))?;

        if !all_settings.is_empty() {
            let count = all_settings.len();
            println!("Batch inserting {} settings...", count);
            settings::Entity::insert_many(all_settings)
                .exec(&self.postgres_client)
                .await
                .map_err(|e| format!("Failed to insert {} settings: {}", count, e))?;
        }

        if !all_activity_log.is_empty() {
            let count = all_activity_log.len();
            println!("Batch inserting {} activity logs...", count);
            activity_log::Entity::insert_many(all_activity_log)
                .exec(&self.postgres_client)
                .await
                .map_err(|e| format!("Failed to insert {} activity logs: {}", count, e))?;
        }

        if !all_transactions.is_empty() {
            let count = all_transactions.len();
            println!("Batch inserting {} transactions...", count);
            transactions::Entity::insert_many(all_transactions)
                .exec(&self.postgres_client)
                .await
                .map_err(|e| format!("Failed to insert {} transactions: {}", count, e))?;
        }

        if !all_messages.is_empty() {
            let count = all_messages.len();
            println!("Batch inserting {} messages...", count);
            messages::Entity::insert_many(all_messages)
                .exec(&self.postgres_client)
                .await
                .map_err(|e| format!("Failed to insert {} messages: {}", count, e))?;
        }

        if !all_attachments.is_empty() {
            let count = all_attachments.len();
            println!("Batch inserting {} attachments...", count);
            attachments::Entity::insert_many(all_attachments)
                .exec(&self.postgres_client)
                .await
                .map_err(|e| format!("Failed to insert {} attachments: {}", count, e))?;
        }

        println!(
            "Batch inserting {} social media records...",
            all_social_media.len()
        );
        social_media::Entity::insert_many(all_social_media)
            .exec(&self.postgres_client)
            .await
            .map_err(|e| {
                format!(
                    "Failed to insert {} social media records: {}",
                    items_count, e
                )
            })?;

        if !all_posts.is_empty() {
            let count = all_posts.len();
            println!("Batch inserting {} posts...", count);
            posts::Entity::insert_many(all_posts)
                .exec(&self.postgres_client)
                .await
                .map_err(|e| format!("Failed to insert {} posts: {}", count, e))?;
        }

        if !all_groups.is_empty() {
            let count = all_groups.len();
            println!("Batch inserting {} groups...", count);
            groups::Entity::insert_many(all_groups)
                .exec(&self.postgres_client)
                .await
                .map_err(|e| format!("Failed to insert {} groups: {}", count, e))?;
        }

        println!(
            "Batch inserting {} large data records...",
            all_large_data.len()
        );
        large_data::Entity::insert_many(all_large_data)
            .exec(&self.postgres_client)
            .await
            .map_err(|e| format!("Failed to insert {} large data records: {}", items_count, e))?;

        println!(
            "✓ Batch inserted {} users with all related data!",
            items_count
        );
        Ok(())
    }

    async fn pre_process(
        &self,
        _cancel: &CancellationToken,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting ETL pipeline...");
        Ok(())
    }

    async fn post_process(
        &self,
        _cancel: &CancellationToken,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("ETL pipeline completed successfully!");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("ETL with Database Example");
    println!("=========================\n");

    let config = DbConfig::load().map_err(|e| {
        format!(
            "Failed to load database configuration: {}. Check your .env file.",
            e
        )
    })?;

    println!("Connecting to databases...");
    let mongodb_client = mongodb::Client::with_uri_str(&config.mongodb_url)
        .await
        .map_err(|e| {
            format!(
                "Failed to connect to MongoDB at {}: {}",
                config.mongodb_url, e
            )
        })?;

    let postgres_client = Database::connect(&config.postgres_url).await.map_err(|e| {
        format!(
            "Failed to connect to PostgreSQL at {}: {}",
            config.postgres_url, e
        )
    })?;
    println!("Connected successfully!\n");

    println!("Running PostgreSQL migrations...");
    migration::run_migrations(&postgres_client)
        .await
        .map_err(|e| format!("Failed to run database migrations: {}", e))?;
    println!("✓ Migrations completed successfully!\n");

    let user_etl = UserETL {
        mongodb_client,
        postgres_client,
    };
    let num_cpus = num_cpus::get();
    let bucket_config = bucket::ConfigBuilder::default()
        .batch_size(500usize)
        .worker_num(num_cpus * 2)
        .build()
        .unwrap();

    let manager_config = ManagerConfig { worker_num: 1 };

    let mut manager = ETLPipelineManager::new(&manager_config, bucket_config);

    println!("✓ Adding User ETL Pipeline (MongoDB -> PostgreSQL)");
    manager.add_pipeline(Box::new(user_etl), "user_migration_pipeline".to_string());

    println!("\n--- Starting ETL pipeline with profiling ---\n");
    let cancel_token = CancellationToken::new();

    let cancel_clone = cancel_token.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        println!("\nShutdown signal received...");
        cancel_clone.cancel();
    });

    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(100)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .map_err(|e| format!("Failed to start profiler: {}", e))?;

    let start = std::time::Instant::now();
    let result = manager.run_all(&cancel_token).await;
    let duration = start.elapsed();

    let is_success = result.is_ok();

    match &result {
        Ok(_) => {
            println!(
                "\n=== ETL process completed successfully in {:.2}s ===",
                duration.as_secs_f64()
            );
        }
        Err(e) => {
            eprintln!("\n=== Error running pipeline: {} ===", e);
        }
    }

    println!("\nGenerating profiling reports...");
    if let Ok(report) = guard.report().build() {
        let file = File::create("flamegraph.svg")
            .map_err(|e| format!("Failed to create flamegraph file: {}", e))?;
        report
            .flamegraph(file)
            .map_err(|e| format!("Failed to generate flamegraph: {}", e))?;
        println!("✓ Flamegraph saved to: flamegraph.svg");

        println!("\n=== Performance Metrics ===");
        println!("Duration: {:.2}s", duration.as_secs_f64());
        println!(
            "Status: {}",
            if is_success {
                "✓ Success"
            } else {
                "✗ Failed"
            }
        );
        println!("CPU Profiling: Enabled (100Hz sampling rate)");
        println!("📊 Open flamegraph.svg in your browser for visual CPU profiling");
        println!("   - Shows function call stacks and CPU time distribution");
        println!("   - Wider bars = more CPU time spent");
        println!("===========================\n");
    }

    result.map_err(|e| {
        Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string(),
        )) as Box<dyn std::error::Error>
    })
}
