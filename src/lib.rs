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
//! ```rust,ignore
//! use etl_rust::bucket::{Bucket, Config};
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! // Create bucket configuration
//! let config = Arc::new(Config {
//!     batch_size: 100,
//!     timeout: Duration::from_secs(5),
//!     worker_num: 4,
//! });
//!
//! // Create bucket for processing integers
//! let bucket: Bucket<i32> = Bucket::new(config);
//! // Bucket is ready to process items concurrently
//! ```
//!
//! ## Modules
//!
//! - [`bucket`] - Generic batching and concurrent processing
//! - [`etl`] - ETL pipeline abstraction for extract-transform-load workflows

pub mod bucket;
pub mod etl;
