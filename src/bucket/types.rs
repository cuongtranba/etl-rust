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

    /// Failed to send item to consumer channel.
    #[error("consumer error")]
    ConsumerError(String),

    /// Multiple workers failed during processing.
    ///
    /// Contains all worker errors for debugging.
    #[error("{} worker(s) failed", .0.len())]
    MultipleErrors(Vec<BucketError>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_processor_error_preserves_source() {
        let source = std::io::Error::new(std::io::ErrorKind::Other, "test error");
        let bucket_err = BucketError::ProcessorError(Box::new(source));

        assert!(bucket_err.source().is_some());
        assert_eq!(bucket_err.to_string(), "processor failed");
    }

    #[test]
    fn test_error_display() {
        let err = BucketError::ChannelClosed;
        assert_eq!(err.to_string(), "channel closed");

        let err = BucketError::Cancelled;
        assert_eq!(err.to_string(), "operation cancelled");
    }

    #[test]
    fn test_multiple_errors_display() {
        let errors = vec![
            BucketError::Cancelled,
            BucketError::ChannelClosed,
        ];
        let multi_err = BucketError::MultipleErrors(errors);

        let display = multi_err.to_string();
        assert!(display.contains("2 worker"));
        assert!(display.contains("failed"));
    }

    #[test]
    fn test_multiple_errors_source() {
        let errors = vec![BucketError::Cancelled];
        let multi_err = BucketError::MultipleErrors(errors);

        // MultipleErrors doesn't have a single source
        assert!(multi_err.source().is_none());
    }
}
