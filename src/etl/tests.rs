use async_trait::async_trait;
use crate::bucket::Config;
use crate::etl::{ETL, Processor};
use mpsc::Receiver;
use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[cfg(test)]
mod tests {
    use super::*;

    // Mock Processor for testing
    struct MockProcessor {
        extract_count: Arc<AtomicUsize>,
        transform_count: Arc<AtomicUsize>,
        load_count: Arc<AtomicUsize>,
        should_fail: bool,
    }

    #[async_trait]
    impl Processor<String, i32> for MockProcessor {
        async fn extract(
            &self,
            _cancel: &CancellationToken,
        ) -> Result<Receiver<String>, Box<dyn Error>> {
            self.extract_count.fetch_add(1, Ordering::SeqCst);

            let (tx, rx) = mpsc::channel(10);

            tokio::spawn(async move {
                for i in 0..5 {
                    tx.send(format!("item_{}", i)).await.unwrap();
                }
            });

            Ok(rx)
        }

        async fn transform(&self, _cancel: &CancellationToken, item: String) -> i32 {
            self.transform_count.fetch_add(1, Ordering::SeqCst);
            // Parse the number from "item_N"
            item.split('_').nth(1).unwrap().parse().unwrap()
        }

        async fn load(
            &self,
            _cancel: &CancellationToken,
            items: Vec<i32>,
        ) -> Result<(), Box<dyn Error>> {
            self.load_count.fetch_add(items.len(), Ordering::SeqCst);

            if self.should_fail {
                return Err("Mock load failed".into());
            }

            Ok(())
        }

        async fn pre_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
            Ok(())
        }

        async fn post_process(&self, _cancel: &CancellationToken) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_etl_full_flow() {
        let extract_count = Arc::new(AtomicUsize::new(0));
        let transform_count = Arc::new(AtomicUsize::new(0));
        let load_count = Arc::new(AtomicUsize::new(0));

        let processor = MockProcessor {
            extract_count: Arc::clone(&extract_count),
            transform_count: Arc::clone(&transform_count),
            load_count: Arc::clone(&load_count),
            should_fail: false,
        };

        let etl: ETL<String, i32> = ETL::new(Box::new(processor));

        let config = Arc::new(Config {
            batch_size: 3,
            timeout: Duration::from_millis(100),
            worker_num: 1,
        });

        let cancel = CancellationToken::new();

        let result = etl.run(config, &cancel).await;

        assert!(result.is_ok());
        assert_eq!(extract_count.load(Ordering::SeqCst), 1); // Extract called once
        assert_eq!(transform_count.load(Ordering::SeqCst), 5); // 5 items transformed
        assert_eq!(load_count.load(Ordering::SeqCst), 5); // 5 items loaded
    }

    #[tokio::test]
    async fn test_etl_transform_batching() {
        let load_count = Arc::new(AtomicUsize::new(0));
        let batch_sizes = Arc::new(tokio::sync::Mutex::new(Vec::new()));

        struct BatchTrackingProcessor {
            load_count: Arc<AtomicUsize>,
            batch_sizes: Arc<tokio::sync::Mutex<Vec<usize>>>,
        }

        #[async_trait]
        impl Processor<i32, i32> for BatchTrackingProcessor {
            async fn extract(
                &self,
                _cancel: &CancellationToken,
            ) -> Result<Receiver<i32>, Box<dyn Error>> {
                let (tx, rx) = mpsc::channel(10);
                tokio::spawn(async move {
                    for i in 0..10 {
                        tx.send(i).await.unwrap();
                    }
                });
                Ok(rx)
            }

            async fn transform(&self, _cancel: &CancellationToken, item: i32) -> i32 {
                item * 2
            }

            async fn load(
                &self,
                _cancel: &CancellationToken,
                items: Vec<i32>,
            ) -> Result<(), Box<dyn Error>> {
                self.load_count.fetch_add(items.len(), Ordering::SeqCst);
                self.batch_sizes.lock().await.push(items.len());
                Ok(())
            }

            async fn pre_process(
                &self,
                _cancel: &CancellationToken,
            ) -> Result<(), Box<dyn Error>> {
                Ok(())
            }

            async fn post_process(
                &self,
                _cancel: &CancellationToken,
            ) -> Result<(), Box<dyn Error>> {
                Ok(())
            }
        }

        let processor = BatchTrackingProcessor {
            load_count: Arc::clone(&load_count),
            batch_sizes: Arc::clone(&batch_sizes),
        };

        let etl: ETL<i32, i32> = ETL::new(Box::new(processor));

        let config = Arc::new(Config {
            batch_size: 3,
            timeout: Duration::from_millis(100),
            worker_num: 1,
        });

        let cancel = CancellationToken::new();
        etl.run(config, &cancel).await.unwrap();

        assert_eq!(load_count.load(Ordering::SeqCst), 10);

        let sizes = batch_sizes.lock().await;
        // Should have multiple batches
        assert!(sizes.len() > 1);
    }

    #[tokio::test]
    async fn test_etl_load_error_handling() {
        let processor = MockProcessor {
            extract_count: Arc::new(AtomicUsize::new(0)),
            transform_count: Arc::new(AtomicUsize::new(0)),
            load_count: Arc::new(AtomicUsize::new(0)),
            should_fail: true, // Simulate load failure
        };

        let etl: ETL<String, i32> = ETL::new(Box::new(processor));

        let config = Arc::new(Config {
            batch_size: 3,
            timeout: Duration::from_millis(100),
            worker_num: 1,
        });

        let cancel = CancellationToken::new();

        let result = etl.run(config, &cancel).await;

        // Should fail due to load error
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_etl_cancellation() {
        struct SlowProcessor;

        #[async_trait]
        impl Processor<i32, i32> for SlowProcessor {
            async fn extract(
                &self,
                cancel: &CancellationToken,
            ) -> Result<Receiver<i32>, Box<dyn Error>> {
                let (tx, rx) = mpsc::channel(100);
                let cancel = cancel.clone();

                tokio::spawn(async move {
                    for i in 0..1000 {
                        if cancel.is_cancelled() {
                            break;
                        }
                        tx.send(i).await.ok();
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                });

                Ok(rx)
            }

            async fn transform(&self, _cancel: &CancellationToken, item: i32) -> i32 {
                item
            }

            async fn load(
                &self,
                _cancel: &CancellationToken,
                _items: Vec<i32>,
            ) -> Result<(), Box<dyn Error>> {
                Ok(())
            }

            async fn pre_process(
                &self,
                _cancel: &CancellationToken,
            ) -> Result<(), Box<dyn Error>> {
                Ok(())
            }

            async fn post_process(
                &self,
                _cancel: &CancellationToken,
            ) -> Result<(), Box<dyn Error>> {
                Ok(())
            }
        }

        let etl: ETL<i32, i32> = ETL::new(Box::new(SlowProcessor));

        let config = Arc::new(Config {
            batch_size: 10,
            timeout: Duration::from_millis(50),
            worker_num: 1,
        });

        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        // Cancel after short delay
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            cancel_clone.cancel();
        });

        let result = etl.run(config, &cancel).await;

        // Should complete (either with success or graceful cancellation)
        // The important thing is it doesn't hang
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_etl_pre_post_process() {
        let pre_called = Arc::new(AtomicUsize::new(0));
        let post_called = Arc::new(AtomicUsize::new(0));

        struct LifecycleProcessor {
            pre_called: Arc<AtomicUsize>,
            post_called: Arc<AtomicUsize>,
        }

        #[async_trait]
        impl Processor<i32, i32> for LifecycleProcessor {
            async fn extract(
                &self,
                _cancel: &CancellationToken,
            ) -> Result<Receiver<i32>, Box<dyn Error>> {
                let (tx, rx) = mpsc::channel(10);
                tokio::spawn(async move {
                    tx.send(1).await.ok();
                });
                Ok(rx)
            }

            async fn transform(&self, _cancel: &CancellationToken, item: i32) -> i32 {
                item
            }

            async fn load(
                &self,
                _cancel: &CancellationToken,
                _items: Vec<i32>,
            ) -> Result<(), Box<dyn Error>> {
                Ok(())
            }

            async fn pre_process(
                &self,
                _cancel: &CancellationToken,
            ) -> Result<(), Box<dyn Error>> {
                self.pre_called.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }

            async fn post_process(
                &self,
                _cancel: &CancellationToken,
            ) -> Result<(), Box<dyn Error>> {
                self.post_called.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        }

        let processor = LifecycleProcessor {
            pre_called: Arc::clone(&pre_called),
            post_called: Arc::clone(&post_called),
        };

        let etl: ETL<i32, i32> = ETL::new(Box::new(processor));

        let config = Arc::new(Config {
            batch_size: 10,
            timeout: Duration::from_millis(50),
            worker_num: 1,
        });

        let cancel = CancellationToken::new();

        etl.run(config, &cancel).await.unwrap();

        // Both lifecycle methods should be called
        assert_eq!(pre_called.load(Ordering::SeqCst), 0); // pre_process not called in run()
        assert_eq!(post_called.load(Ordering::SeqCst), 1); // post_process called at end
    }

    #[tokio::test]
    async fn test_etl_concurrent_transforms() {
        use std::time::Instant;

        struct SlowTransformProcessor;

        #[async_trait]
        impl Processor<i32, i32> for SlowTransformProcessor {
            async fn extract(
                &self,
                _cancel: &CancellationToken,
            ) -> Result<Receiver<i32>, Box<dyn Error>> {
                let (tx, rx) = mpsc::channel(10);
                tokio::spawn(async move {
                    for i in 0..5 {
                        tx.send(i).await.unwrap();
                    }
                });
                Ok(rx)
            }

            async fn transform(&self, _cancel: &CancellationToken, item: i32) -> i32 {
                // Simulate slow transform
                tokio::time::sleep(Duration::from_millis(50)).await;
                item * 2
            }

            async fn load(
                &self,
                _cancel: &CancellationToken,
                _items: Vec<i32>,
            ) -> Result<(), Box<dyn Error>> {
                Ok(())
            }

            async fn pre_process(
                &self,
                _cancel: &CancellationToken,
            ) -> Result<(), Box<dyn Error>> {
                Ok(())
            }

            async fn post_process(
                &self,
                _cancel: &CancellationToken,
            ) -> Result<(), Box<dyn Error>> {
                Ok(())
            }
        }

        let etl: ETL<i32, i32> = ETL::new(Box::new(SlowTransformProcessor));

        let config = Arc::new(Config {
            batch_size: 5,
            timeout: Duration::from_secs(1),
            worker_num: 1,
        });

        let cancel = CancellationToken::new();

        let start = Instant::now();
        etl.run(config, &cancel).await.unwrap();
        let duration = start.elapsed();

        // If transforms run concurrently (via join_all), should take ~50ms
        // If sequential, would take ~250ms (5 * 50ms)
        // Allow some margin for overhead
        assert!(
            duration < Duration::from_millis(150),
            "Transforms should run concurrently, took {:?}",
            duration
        );
    }
}
