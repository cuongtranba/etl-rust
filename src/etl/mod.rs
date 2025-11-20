//! ETL (Extract-Transform-Load) pipeline abstraction.
//!
//! This module provides the [`ETLPipeline`] trait for defining data pipelines
//! and the [`ETL`] executor for running them with concurrent batch processing.
//!
//! # Example
//!
//! ```rust,no_run
//! use async_trait::async_trait;
//! use etl_rust::etl::{ETL, ETLPipeline};
//! use etl_rust::bucket::Config;
//! use std::sync::Arc;
//! use tokio::sync::mpsc;
//! use tokio_util::sync::CancellationToken;
//!
//! struct MyPipeline;
//!
//! #[async_trait]
//! impl ETLPipeline<i32, String> for MyPipeline {
//!     async fn extract(&self, _cancel: &CancellationToken) -> Result<mpsc::Receiver<i32>, Box<dyn std::error::Error>> {
//!         let (tx, rx) = mpsc::channel(100);
//!         // Send data...
//!         Ok(rx)
//!     }
//!
//!     async fn transform(&self, _cancel: &CancellationToken, item: &i32) -> String {
//!         item.to_string()
//!     }
//!
//!     async fn load(&self, _cancel: &CancellationToken, items: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
//!         // Load transformed data...
//!         Ok(())
//!     }
//!
//!     async fn pre_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn std::error::Error>> {
//!         Ok(())
//!     }
//!
//!     async fn post_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn std::error::Error>> {
//!         Ok(())
//!     }
//! }
//! ```

pub mod manager;
pub mod processor;

pub use manager::ETLPipelineManager;
pub use processor::ETL;
pub use processor::ETLPipeline;
