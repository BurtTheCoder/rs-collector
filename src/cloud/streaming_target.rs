use anyhow::Result;
use tokio::io::AsyncWrite;

/// A trait for streaming targets that can receive data and complete or abort uploads.
///
/// This trait abstracts over different streaming destinations like S3, SFTP, etc.,
/// allowing for generic implementations of streaming functionality.
pub trait StreamingTarget: AsyncWrite + Unpin + Send + 'static {
    /// Get the unique identifier for this target (for logs/errors)
    fn target_name(&self) -> String;
    
    /// Get the number of bytes uploaded so far
    fn bytes_uploaded(&self) -> u64;
    
    /// Complete the upload operation
    async fn complete(self) -> Result<()>;
    
    /// Abort the upload operation and clean up resources
    async fn abort(self) -> Result<()>;
}
