use thiserror::Error;

/// Errors that can occur during bucket processing.
#[derive(Debug, Error)]
pub enum BucketError {
    /// A processor failed with an error.
    ///
    /// Preserves the source error for debugging.
    #[error("processor failed")]
    ProcessorError(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// The channel was closed unexpectedly.
    #[error("channel closed")]
    ChannelClosed,

    /// Processing was cancelled via the cancellation token.
    #[error("operation cancelled")]
    Cancelled,

    // this can’t be annotated with ? because it has type Result<_, flume::SendError<T>>
    /// Failed to send item to consumer channel.
    #[error("consumer error")]
    ConsumerError(String),

    /// Multiple workers failed during processing.
    ///
    /// Contains all worker errors for debugging.
    #[error("{} worker(s) failed", .0.len())]
    MultipleErrors(Vec<BucketError>),
}

impl<T> From<flume::SendError<T>> for BucketError {
    fn from(err: flume::SendError<T>) -> Self {
        Self::ConsumerError(err.to_string())
    }
}
