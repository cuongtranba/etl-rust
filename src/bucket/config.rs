// src/bucket/config.rs

use derive_builder::Builder;
use std::time::Duration;

#[derive(Debug, Clone, Builder)]
#[builder(setter(into))]
pub struct Config {
    #[builder(default = "1")]
    pub batch_size: usize,

    #[builder(default = "Duration::from_secs(5)")]
    pub timeout: Duration,

    #[builder(default = "1")]
    pub worker_num: usize,
}
