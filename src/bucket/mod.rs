//! Concurrent batch processing with configurable workers.
//!
//! This module provides [`Bucket`], a generic batching processor that collects
//! items into batches and processes them concurrently using multiple workers.
//!
//! # Example
//!
//! ```rust,ignore
//! use etl_rust::bucket::{Bucket, Config};
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! let config = Arc::new(Config {
//!     batch_size: 10,
//!     timeout: Duration::from_secs(1),
//!     worker_num: 2,
//! });
//!
//! let bucket: Bucket<String> = Bucket::new(config);
//! // Use bucket to process items in batches with workers
//! ```

pub mod bucket;
pub mod config;
pub mod processor;
pub mod types;

pub use bucket::Bucket;
pub use config::{Config, ConfigBuilder};
pub use processor::BatchProcessor;
pub use types::BucketError;
