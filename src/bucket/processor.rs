// src/bucket/processor.rs

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use super::types::BucketError; // Import BucketError từ file cùng module

#[async_trait]
pub trait Processor<T>: Send + Sync {
    async fn process(&self, cancel: &CancellationToken, items: &[T]) -> Result<(), BucketError>;
}

#[async_trait]
impl<T, F, Fut> Processor<T> for F
where
    F: Fn(&CancellationToken, &[T]) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<(), BucketError>> + Send,
    T: Send + Sync,
{
    async fn process(&self, ctx: &CancellationToken, items: &[T]) -> Result<(), BucketError> {
        self(ctx, items).await
    }
}
