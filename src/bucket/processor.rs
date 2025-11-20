use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use super::types::BucketError;

/// Processes batches of items concurrently.
///
/// Implementations define how to process a slice of items, typically
/// performing I/O operations, transformations, or aggregations.
///
/// # Type Parameters
///
/// * `T` - The type of items in each batch
///
/// # Example
///
/// ```rust
/// use async_trait::async_trait;
/// use etl_rust::bucket::{BatchProcessor, BucketError};
/// use tokio_util::sync::CancellationToken;
///
/// struct MyProcessor;
///
/// #[async_trait]
/// impl BatchProcessor<String> for MyProcessor {
///     async fn process(&self, ctx: &CancellationToken, items: &[String]) -> Result<(), BucketError> {
///         for item in items {
///             if ctx.is_cancelled() {
///                 return Err(BucketError::Cancelled);
///             }
///             // Process item...
///         }
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait BatchProcessor<T>: Send + Sync {
    /// Processes a batch of items.
    ///
    /// # Arguments
    ///
    /// * `cancel` - Cancellation token to abort processing
    /// * `items` - Slice of items to process
    ///
    /// # Errors
    ///
    /// Returns `BucketError` if processing fails or is cancelled.
    async fn process(&self, cancel: &CancellationToken, items: &[T]) -> Result<(), BucketError>;
}

#[async_trait]
impl<T, F, Fut> BatchProcessor<T> for F
where
    F: Fn(&CancellationToken, &[T]) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<(), BucketError>> + Send,
    T: Send + Sync,
{
    async fn process(&self, ctx: &CancellationToken, items: &[T]) -> Result<(), BucketError> {
        self(ctx, items).await
    }
}
