// src/bucket/config.rs

use derive_builder::Builder;
use std::time::Duration;

/// Configuration for bucket batch processing.
///
/// # Examples
///
/// ```rust,ignore
/// use etl_rust::bucket::Config;
/// use std::time::Duration;
///
/// let config = Config {
///     batch_size: 100,
///     timeout: Duration::from_secs(5),
///     worker_num: 4,
/// };
///
/// assert_eq!(config.batch_size(), 100);
/// assert_eq!(config.worker_num(), 4);
/// ```
#[derive(Debug, Clone, Builder)]
#[builder(setter(into))]
pub struct Config {
    /// Maximum number of items in a batch before processing
    #[builder(default = "1")]
    pub(crate) batch_size: usize,

    /// Maximum time to wait before processing a partial batch
    #[builder(default = "Duration::from_secs(5)")]
    pub(crate) timeout: Duration,

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
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Returns the number of worker threads
    #[inline]
    pub fn worker_num(&self) -> usize {
        self.worker_num
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_getters() {
        let config = ConfigBuilder::default()
            .batch_size(10usize)
            .timeout(Duration::from_secs(30))
            .worker_num(4usize)
            .build()
            .unwrap();

        assert_eq!(config.batch_size(), 10);
        assert_eq!(config.timeout(), Duration::from_secs(30));
        assert_eq!(config.worker_num(), 4);
    }

    #[test]
    fn test_config_defaults() {
        let config = ConfigBuilder::default().build().unwrap();

        assert_eq!(config.batch_size(), 1);
        assert_eq!(config.timeout(), Duration::from_secs(5));
        assert_eq!(config.worker_num(), 1);
    }
}
