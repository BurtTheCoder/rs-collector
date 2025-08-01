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

#[cfg(test)]
mod tests {
    use super::*;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use std::io;
    use std::sync::atomic::{AtomicU64, Ordering};

    // Mock implementation for testing
    struct MockStreamingTarget {
        name: String,
        bytes: AtomicU64,
        completed: std::sync::Arc<std::sync::Mutex<bool>>,
        aborted: std::sync::Arc<std::sync::Mutex<bool>>,
    }

    impl MockStreamingTarget {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                bytes: AtomicU64::new(0),
                completed: std::sync::Arc::new(std::sync::Mutex::new(false)),
                aborted: std::sync::Arc::new(std::sync::Mutex::new(false)),
            }
        }

        fn is_completed(&self) -> bool {
            *self.completed.lock().unwrap()
        }

        fn is_aborted(&self) -> bool {
            *self.aborted.lock().unwrap()
        }
    }

    impl StreamingTarget for MockStreamingTarget {
        fn target_name(&self) -> String {
            self.name.clone()
        }

        fn bytes_uploaded(&self) -> u64 {
            self.bytes.load(Ordering::SeqCst)
        }

        async fn complete(self) -> Result<()> {
            *self.completed.lock().unwrap() = true;
            Ok(())
        }

        async fn abort(self) -> Result<()> {
            *self.aborted.lock().unwrap() = true;
            Ok(())
        }
    }

    impl AsyncWrite for MockStreamingTarget {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            self.bytes.fetch_add(buf.len() as u64, Ordering::SeqCst);
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    #[test]
    fn test_mock_streaming_target_name() {
        let target = MockStreamingTarget::new("test-target");
        assert_eq!(target.target_name(), "test-target");
    }

    #[test]
    fn test_mock_streaming_target_bytes() {
        let target = MockStreamingTarget::new("test-target");
        assert_eq!(target.bytes_uploaded(), 0);
        
        target.bytes.store(100, Ordering::SeqCst);
        assert_eq!(target.bytes_uploaded(), 100);
    }

    #[tokio::test]
    async fn test_mock_streaming_target_complete() {
        let target = MockStreamingTarget::new("test-target");
        let completed_ref = target.completed.clone();
        
        assert!(!*completed_ref.lock().unwrap());
        
        target.complete().await.unwrap();
        
        assert!(*completed_ref.lock().unwrap());
    }

    #[tokio::test]
    async fn test_mock_streaming_target_abort() {
        let target = MockStreamingTarget::new("test-target");
        let aborted_ref = target.aborted.clone();
        
        assert!(!*aborted_ref.lock().unwrap());
        
        target.abort().await.unwrap();
        
        assert!(*aborted_ref.lock().unwrap());
    }

    #[tokio::test]
    async fn test_mock_streaming_target_write() {
        use tokio::io::AsyncWriteExt;
        
        let mut target = MockStreamingTarget::new("test-target");
        
        assert_eq!(target.bytes_uploaded(), 0);
        
        let data = b"hello world";
        target.write_all(data).await.unwrap();
        
        assert_eq!(target.bytes_uploaded(), 11);
    }

    #[tokio::test]
    async fn test_mock_streaming_target_multiple_writes() {
        use tokio::io::AsyncWriteExt;
        
        let mut target = MockStreamingTarget::new("test-target");
        
        target.write_all(b"hello").await.unwrap();
        assert_eq!(target.bytes_uploaded(), 5);
        
        target.write_all(b" world").await.unwrap();
        assert_eq!(target.bytes_uploaded(), 11);
    }

    #[tokio::test]
    async fn test_direct_usage() {
        use tokio::io::AsyncWriteExt;
        
        let mut target = MockStreamingTarget::new("test");
        
        assert_eq!(target.target_name(), "test");
        assert_eq!(target.bytes_uploaded(), 0);
        
        target.write_all(b"test data").await.unwrap();
        assert_eq!(target.bytes_uploaded(), 9);
    }

    #[test]
    fn test_trait_requirements() {
        // This test verifies that our mock implements all required traits
        fn assert_streaming_target<T: StreamingTarget>() {}
        assert_streaming_target::<MockStreamingTarget>();
        
        // Verify it's Send + 'static
        fn assert_send_static<T: Send + 'static>() {}
        assert_send_static::<MockStreamingTarget>();
        
        // Verify it's Unpin
        fn assert_unpin<T: Unpin>() {}
        assert_unpin::<MockStreamingTarget>();
    }

    #[test]
    fn test_poll_variants() {
        // Test Poll::Ready variants
        let ready_ok: Poll<io::Result<()>> = Poll::Ready(Ok(()));
        assert!(matches!(ready_ok, Poll::Ready(Ok(()))));
        
        let ready_err: Poll<io::Result<()>> = Poll::Ready(Err(io::Error::new(
            io::ErrorKind::Other,
            "test error"
        )));
        assert!(matches!(ready_err, Poll::Ready(Err(_))));
        
        // Test Poll::Pending
        let pending: Poll<io::Result<()>> = Poll::Pending;
        assert!(matches!(pending, Poll::Pending));
    }
}
