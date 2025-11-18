#[derive(Debug, Clone)]
pub enum BucketError {
    ProcessorError(String),
    ChannelClosed,
    Cancelled,
}

impl std::fmt::Display for BucketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BucketError::ProcessorError(msg) => write!(f, "ProcessorError: {}", msg),
            BucketError::ChannelClosed => write!(f, "ChannelClosed"),
            BucketError::Cancelled => write!(f, "Cancelled"),
        }
    }
}

impl std::error::Error for BucketError {}
