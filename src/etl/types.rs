/// Errors that can occur during ETL manager operations
#[derive(Debug, thiserror::Error)]
pub enum ETLError {
    #[error("Pipeline '{0}' execution failed: {1}")]
    PipelineExecution(String, String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Cancellation requested")]
    Cancelled,

    #[error("Worker pool error: {0}")]
    WorkerPool(String),

    #[error("Bucket operation failed: {0}")]
    Bucket(#[from] crate::bucket::BucketError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Build error: {0}")]
    BuildError(String),
}
