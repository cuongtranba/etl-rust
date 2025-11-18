use crate::bucket::{Bucket, BucketError, Config, Processor};
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

#[cfg(test)]
mod tests {
    use super::*;

    // Helper struct for counting processed items
    struct CountingProcessor {
        counter: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Processor<i32> for CountingProcessor {
        async fn process(
            &self,
            _ctx: &CancellationToken,
            items: &[i32],
        ) -> Result<(), BucketError> {
            self.counter.fetch_add(items.len(), Ordering::SeqCst);
            Ok(())
        }
    }

    // Helper struct for tracking batch sizes
    struct BatchSizeTracker {
        sizes: Arc<tokio::sync::Mutex<Vec<usize>>>,
    }

    #[async_trait]
    impl Processor<i32> for BatchSizeTracker {
        async fn process(
            &self,
            _ctx: &CancellationToken,
            items: &[i32],
        ) -> Result<(), BucketError> {
            self.sizes.lock().await.push(items.len());
            Ok(())
        }
    }

    // Helper struct that returns errors
    struct ErrorProcessor;

    #[async_trait]
    impl Processor<i32> for ErrorProcessor {
        async fn process(
            &self,
            _ctx: &CancellationToken,
            _items: &[i32],
        ) -> Result<(), BucketError> {
            Err(BucketError::ProcessorError("Test error".to_string()))
        }
    }

    // Helper struct that does nothing
    struct NoOpProcessor;

    #[async_trait]
    impl Processor<i32> for NoOpProcessor {
        async fn process(
            &self,
            _ctx: &CancellationToken,
            _items: &[i32],
        ) -> Result<(), BucketError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_bucket_creation() {
        let config = Arc::new(Config {
            batch_size: 10,
            timeout: Duration::from_millis(100),
            worker_num: 2,
        });

        let bucket: Bucket<i32> = Bucket::new(config.clone());
        // Bucket should be created successfully
        assert_eq!(bucket.config.batch_size, 10);
        assert_eq!(bucket.config.worker_num, 2);
    }

    #[tokio::test]
    async fn test_bucket_consume_and_process() {
        let config = Arc::new(Config {
            batch_size: 5,
            timeout: Duration::from_millis(100),
            worker_num: 1,
        });

        let bucket: Bucket<i32> = Bucket::new(config);
        let bucket_clone = bucket.clone();
        let cancel = CancellationToken::new();

        // Track processed items
        let processed = Arc::new(AtomicUsize::new(0));
        let processor = CountingProcessor {
            counter: Arc::clone(&processed),
        };

        // Spawn consumer task
        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            for i in 0..10 {
                bucket_clone.consume(i).await.unwrap();
            }
            // Give time for processing
            sleep(Duration::from_millis(200)).await;
            cancel_clone.cancel();
        });

        // Run bucket processor
        let result = bucket.run(&cancel, processor).await;

        assert!(result.is_ok());
        assert_eq!(processed.load(Ordering::SeqCst), 10);
    }

    #[tokio::test]
    async fn test_bucket_batch_processing() {
        let config = Arc::new(Config {
            batch_size: 3,
            timeout: Duration::from_secs(10), // Long timeout to rely on batch size
            worker_num: 1,
        });

        let bucket: Bucket<i32> = Bucket::new(config);
        let bucket_clone = bucket.clone();
        let cancel = CancellationToken::new();

        // Track batch sizes
        let batch_sizes = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let processor = BatchSizeTracker {
            sizes: Arc::clone(&batch_sizes),
        };

        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            // Send exactly 6 items (2 batches of 3)
            for i in 0..6 {
                bucket_clone.consume(i).await.unwrap();
            }
            sleep(Duration::from_millis(100)).await;
            cancel_clone.cancel();
        });

        bucket.run(&cancel, processor).await.unwrap();

        let sizes = batch_sizes.lock().await;
        // Should have processed in batches of 3
        assert!(sizes.iter().any(|&s| s == 3));
    }

    #[tokio::test]
    async fn test_bucket_timeout_trigger() {
        let config = Arc::new(Config {
            batch_size: 100,                    // Large batch size
            timeout: Duration::from_millis(50), // Short timeout
            worker_num: 1,
        });

        let bucket: Bucket<i32> = Bucket::new(config);
        let bucket_clone = bucket.clone();
        let cancel = CancellationToken::new();

        let processed = Arc::new(AtomicUsize::new(0));
        let processor = CountingProcessor {
            counter: Arc::clone(&processed),
        };

        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            // Send only 2 items (less than batch size)
            bucket_clone.consume(1).await.unwrap();
            bucket_clone.consume(2).await.unwrap();

            // Wait for timeout to trigger
            sleep(Duration::from_millis(200)).await;
            cancel_clone.cancel();
        });

        bucket.run(&cancel, processor).await.unwrap();

        // Should process items via timeout, not batch size
        assert_eq!(processed.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_bucket_error_handling() {
        let config = Arc::new(Config {
            batch_size: 5,
            timeout: Duration::from_millis(100),
            worker_num: 1,
        });

        let bucket: Bucket<i32> = Bucket::new(config);
        let bucket_clone = bucket.clone();
        let cancel = CancellationToken::new();

        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            for i in 0..5 {
                bucket_clone.consume(i).await.unwrap();
            }
            sleep(Duration::from_millis(50)).await;
            cancel_clone.cancel();
        });

        // Processor that returns error
        let result = bucket.run(&cancel, ErrorProcessor).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            BucketError::ProcessorError(msg) => assert_eq!(msg, "Test error"),
            _ => panic!("Expected ProcessorError"),
        }
    }

    #[tokio::test]
    async fn test_bucket_multiple_workers() {
        let config = Arc::new(Config {
            batch_size: 2,
            timeout: Duration::from_millis(100),
            worker_num: 3, // Multiple workers
        });

        let bucket: Bucket<i32> = Bucket::new(config);
        let bucket_clone = bucket.clone();
        let cancel = CancellationToken::new();

        let processed = Arc::new(AtomicUsize::new(0));
        let processor = CountingProcessor {
            counter: Arc::clone(&processed),
        };

        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            for i in 0..20 {
                bucket_clone.consume(i).await.unwrap();
            }
            sleep(Duration::from_millis(200)).await;
            cancel_clone.cancel();
        });

        bucket.run(&cancel, processor).await.unwrap();

        assert_eq!(processed.load(Ordering::SeqCst), 20);
    }

    #[tokio::test]
    async fn test_bucket_cancellation() {
        let config = Arc::new(Config {
            batch_size: 10,
            timeout: Duration::from_secs(10),
            worker_num: 1,
        });

        let bucket: Bucket<i32> = Bucket::new(config);
        let cancel = CancellationToken::new();

        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            // Cancel immediately
            sleep(Duration::from_millis(10)).await;
            cancel_clone.cancel();
        });

        let result = bucket.run(&cancel, NoOpProcessor).await;

        // Should complete without error on cancellation
        assert!(result.is_ok());
    }

    #[test]
    fn test_bucket_error_display() {
        let err1 = BucketError::ProcessorError("test".to_string());
        assert_eq!(err1.to_string(), "ProcessorError: test");

        let err2 = BucketError::ChannelClosed;
        assert_eq!(err2.to_string(), "ChannelClosed");

        let err3 = BucketError::Cancelled;
        assert_eq!(err3.to_string(), "Cancelled");
    }
}
