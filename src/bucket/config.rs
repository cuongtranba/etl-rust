// src/bucket/config.rs

use derive_builder::Builder;
use std::time::Duration;

#[derive(Debug, Clone, Builder)]
#[builder(setter(into))]
pub struct Config {
    /// Maximum number of items in a batch before processing
    #[builder(default = "1")]
    pub(crate) batch_size: usize,

    /// Maximum time to wait before processing a partial batch
    #[builder(default = "Some(Duration::from_secs(5))")]
    pub(crate) timeout: Option<Duration>,

    /// Number of concurrent worker tasks
    #[builder(default = "1")]
    pub(crate) worker_num: usize,
}

impl Config {
    /// Returns the batch size for processing
    #[inline]
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    /// Returns the timeout duration for batch collection
    #[inline]
    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    /// Returns the number of worker threads
    #[inline]
    pub fn worker_num(&self) -> usize {
        self.worker_num
    }
}
