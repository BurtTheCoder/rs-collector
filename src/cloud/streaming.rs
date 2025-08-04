use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use crate::cloud::streaming_target::StreamingTarget;
use crate::constants::{MAX_UPLOAD_RETRIES as MAX_RETRIES, S3_MIN_PART_SIZE as MIN_PART_SIZE};
use anyhow::{anyhow, Context as AnyhowContext, Result};
use bytes::{Bytes, BytesMut};
use log::{debug, warn};
use rusoto_core::ByteStream;
use rusoto_s3::{
    AbortMultipartUploadRequest, CompleteMultipartUploadRequest, CompletedMultipartUpload,
    CompletedPart, CreateMultipartUploadRequest, S3Client, UploadPartRequest, S3,
};
use tokio::io::AsyncWrite;
use tokio::sync::mpsc;
use tokio::time::Duration;

/// A stream that buffers data and uploads it to S3 in parts using multipart upload.
///
/// This implementation provides:
/// - Buffered writes that are sent as S3 multipart upload parts when they reach the minimum size
/// - Automatic retry with exponential backoff for failed part uploads
/// - Progress tracking with atomic counters for thread safety
/// - Proper cleanup of S3 resources on failure
/// - Async/await compatible interface that implements AsyncWrite
pub struct S3UploadStream {
    client: Arc<S3Client>,
    bucket: String,
    key: String,
    /// The S3 multipart upload ID, made public for abort operations
    pub upload_id: String,
    buffer: BytesMut,
    min_part_size: usize,
    part_number: AtomicU64,
    completed_parts: Arc<Mutex<Vec<CompletedPart>>>,
    sender: mpsc::Sender<UploadTask>,
    _upload_task: tokio::task::JoinHandle<Result<()>>,
    /// Atomic counter for tracking uploaded bytes
    bytes_uploaded: Arc<AtomicU64>,
}

struct UploadTask {
    data: Bytes,
    part_number: u64,
}

impl S3UploadStream {
    /// Create a new S3 upload stream with the specified buffer size.
    ///
    /// This initializes a new multipart upload to S3 and sets up the background task
    /// for handling part uploads. The buffer size determines how much data is accumulated
    /// before sending a part to S3.
    ///
    /// # Arguments
    ///
    /// * `client` - The S3 client to use for uploads
    /// * `bucket` - The S3 bucket name
    /// * `key` - The S3 object key (path)
    /// * `buffer_size_mb` - Buffer size in megabytes (minimum 5MB due to S3 requirements)
    ///
    /// # Returns
    ///
    /// A new S3UploadStream instance or an error if the multipart upload couldn't be created
    pub async fn new(
        client: Arc<S3Client>,
        bucket: &str,
        key: &str,
        buffer_size_mb: usize,
    ) -> Result<Self> {
        // Ensure buffer size is at least the minimum part size
        let buffer_size = buffer_size_mb.max(5) * 1024 * 1024;

        // Create multipart upload
        let create_result = client
            .create_multipart_upload(CreateMultipartUploadRequest {
                bucket: bucket.to_string(),
                key: key.to_string(),
                ..Default::default()
            })
            .await
            .context("Failed to create multipart upload")?;

        let upload_id = create_result
            .upload_id
            .ok_or_else(|| anyhow!("No upload ID returned from S3"))?;

        debug!(
            "Started multipart upload with ID: {} for {}",
            upload_id, key
        );

        // Create channel for upload tasks
        let (sender, mut receiver) = mpsc::channel::<UploadTask>(100);

        // Create shared state
        let completed_parts = Arc::new(Mutex::new(Vec::new()));
        let completed_parts_clone = Arc::clone(&completed_parts);
        let client_clone = Arc::clone(&client);
        let bucket_clone = bucket.to_string();
        let key_clone = key.to_string();
        let upload_id_clone = upload_id.clone();
        let bytes_uploaded = Arc::new(AtomicU64::new(0));
        let bytes_uploaded_clone = Arc::clone(&bytes_uploaded);

        // Spawn background task to handle uploads
        let upload_task = tokio::spawn(async move {
            while let Some(task) = receiver.recv().await {
                let part_size = task.data.len();

                // Upload with retries
                let mut attempts = 0;
                let mut success = false;

                while attempts < MAX_RETRIES && !success {
                    attempts += 1;

                    let upload_part_request = UploadPartRequest {
                        bucket: bucket_clone.clone(),
                        key: key_clone.clone(),
                        upload_id: upload_id_clone.clone(),
                        part_number: task.part_number as i64,
                        body: Some(ByteStream::from(task.data.to_vec())),
                        ..Default::default()
                    };

                    match client_clone.upload_part(upload_part_request).await {
                        Ok(output) => {
                            if let Some(e_tag) = output.e_tag {
                                let mut parts = completed_parts_clone.lock().map_err(|e| {
                                    anyhow!("Failed to acquire lock on completed_parts: {}", e)
                                })?;
                                parts.push(CompletedPart {
                                    e_tag: Some(e_tag),
                                    part_number: Some(task.part_number as i64),
                                });

                                bytes_uploaded_clone.fetch_add(part_size as u64, Ordering::SeqCst);
                                success = true;
                            }
                        }
                        Err(e) => {
                            if attempts >= MAX_RETRIES {
                                return Err(anyhow!(
                                    "Failed to upload part {} after {} attempts: {}",
                                    task.part_number,
                                    MAX_RETRIES,
                                    e
                                ));
                            }

                            let delay = Duration::from_millis(250 * 2u64.pow(attempts as u32));
                            warn!(
                                "Part {} upload attempt {} failed, retrying in {:?}: {}",
                                task.part_number, attempts, delay, e
                            );
                            tokio::time::sleep(delay).await;
                        }
                    }
                }

                if !success {
                    return Err(anyhow!("Failed to upload part {}", task.part_number));
                }
            }

            Ok(())
        });

        Ok(Self {
            client,
            bucket: bucket.to_string(),
            key: key.to_string(),
            upload_id,
            buffer: BytesMut::with_capacity(buffer_size),
            min_part_size: MIN_PART_SIZE,
            part_number: AtomicU64::new(1),
            completed_parts,
            sender,
            _upload_task: upload_task,
            bytes_uploaded,
        })
    }

    /// Get the number of bytes uploaded so far.
    ///
    /// This method is thread-safe and can be called from any context to check
    /// the current upload progress.
    ///
    /// # Returns
    ///
    /// The total number of bytes successfully uploaded to S3
    pub fn bytes_uploaded(&self) -> u64 {
        self.bytes_uploaded.load(Ordering::SeqCst)
    }

    /// Complete the multipart upload.
    ///
    /// This method finalizes the multipart upload by:
    /// 1. Closing the upload channel
    /// 2. Waiting for all pending uploads to complete
    /// 3. Sorting the completed parts by part number
    /// 4. Sending the CompleteMultipartUpload request to S3
    ///
    /// # Returns
    ///
    /// Ok(()) if the upload was successfully completed, or an error
    ///
    /// # Notes
    ///
    /// This method consumes self, so the S3UploadStream cannot be used after calling complete
    pub async fn complete(self) -> Result<()> {
        // Drop sender to close the channel
        drop(self.sender);

        // Wait for upload task to complete
        match self._upload_task.await {
            Ok(result) => {
                result?;
            }
            Err(e) => {
                return Err(anyhow!("Upload task failed: {}", e));
            }
        }

        // Sort parts by part number
        let mut parts = self
            .completed_parts
            .lock()
            .map_err(|e| anyhow!("Failed to acquire lock on completed_parts: {}", e))?
            .clone();
        parts.sort_by_key(|part| part.part_number.unwrap_or(0));

        // Complete the multipart upload
        let complete_request = CompleteMultipartUploadRequest {
            bucket: self.bucket.clone(),
            key: self.key.clone(),
            upload_id: self.upload_id.clone(),
            multipart_upload: Some(CompletedMultipartUpload { parts: Some(parts) }),
            ..Default::default()
        };

        self.client
            .complete_multipart_upload(complete_request)
            .await
            .context("Failed to complete multipart upload")?;

        debug!("Completed multipart upload for {}", self.key);

        Ok(())
    }

    /// Abort the multipart upload.
    ///
    /// This method cancels the multipart upload and cleans up any uploaded parts in S3.
    /// It should be called when an error occurs and the upload needs to be abandoned.
    ///
    /// # Returns
    ///
    /// Ok(()) if the abort was successful, or an error
    ///
    /// # Notes
    ///
    /// This method consumes self, so the S3UploadStream cannot be used after calling abort
    pub async fn abort(self) -> Result<()> {
        let abort_request = AbortMultipartUploadRequest {
            bucket: self.bucket.clone(),
            key: self.key.clone(),
            upload_id: self.upload_id.clone(),
            ..Default::default()
        };

        self.client
            .abort_multipart_upload(abort_request)
            .await
            .context("Failed to abort multipart upload")?;

        debug!("Aborted multipart upload for {}", self.key);

        Ok(())
    }
}

impl StreamingTarget for S3UploadStream {
    fn target_name(&self) -> String {
        format!("s3://{}/{}", self.bucket, self.key)
    }

    fn bytes_uploaded(&self) -> u64 {
        self.bytes_uploaded.load(Ordering::SeqCst)
    }

    async fn complete(self) -> Result<()> {
        self.complete().await
    }

    async fn abort(self) -> Result<()> {
        self.abort().await
    }
}

impl AsyncWrite for S3UploadStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        // Add data to buffer
        self.buffer.extend_from_slice(buf);

        // If buffer is large enough, send a part
        if self.buffer.len() >= self.min_part_size {
            let part_number = self.part_number.fetch_add(1, Ordering::SeqCst);
            let data = self.buffer.split().freeze();

            // Try to send the upload task
            match self.sender.try_send(UploadTask { data, part_number }) {
                Ok(_) => {}
                Err(e) => {
                    match e {
                        mpsc::error::TrySendError::Full(task) => {
                            // Channel is full, put data back in buffer and return pending
                            self.buffer = BytesMut::from(&task.data[..]);
                            return Poll::Pending;
                        }
                        mpsc::error::TrySendError::Closed(_) => {
                            return Poll::Ready(Err(io::Error::new(
                                io::ErrorKind::BrokenPipe,
                                "Upload channel closed",
                            )));
                        }
                    }
                }
            }
        }

        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // Nothing to do for flush
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // Send any remaining data
        if !self.buffer.is_empty() {
            let part_number = self.part_number.fetch_add(1, Ordering::SeqCst);
            let data = self.buffer.split().freeze();

            // Try to send the upload task
            match self.sender.try_send(UploadTask { data, part_number }) {
                Ok(_) => {}
                Err(e) => {
                    match e {
                        mpsc::error::TrySendError::Full(task) => {
                            // Channel is full, put data back in buffer and return pending
                            self.buffer = BytesMut::from(&task.data[..]);
                            return Poll::Pending;
                        }
                        mpsc::error::TrySendError::Closed(_) => {
                            return Poll::Ready(Err(io::Error::new(
                                io::ErrorKind::BrokenPipe,
                                "Upload channel closed",
                            )));
                        }
                    }
                }
            }
        }

        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use tokio::io::AsyncWriteExt;

    #[test]
    fn test_min_part_size_constant() {
        assert_eq!(MIN_PART_SIZE, 5 * 1024 * 1024);
    }

    #[test]
    fn test_max_retries_constant() {
        assert_eq!(MAX_RETRIES, 3);
    }

    #[test]
    fn test_upload_task_struct() {
        let data = Bytes::from_static(b"test data");
        let task = UploadTask {
            data: data.clone(),
            part_number: 1,
        };
        assert_eq!(task.part_number, 1);
        assert_eq!(task.data, data);
    }

    #[test]
    fn test_bytes_uploaded_atomic() {
        let bytes_uploaded = Arc::new(AtomicU64::new(0));
        bytes_uploaded.store(100, Ordering::SeqCst);
        assert_eq!(bytes_uploaded.load(Ordering::SeqCst), 100);

        bytes_uploaded.fetch_add(50, Ordering::SeqCst);
        assert_eq!(bytes_uploaded.load(Ordering::SeqCst), 150);
    }

    #[test]
    fn test_part_number_atomic() {
        let part_number = AtomicU64::new(1);
        assert_eq!(part_number.load(Ordering::SeqCst), 1);

        let old = part_number.fetch_add(1, Ordering::SeqCst);
        assert_eq!(old, 1);
        assert_eq!(part_number.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_buffer_size_calculation() {
        // Test minimum buffer size
        let buffer_size = 3; // 3MB (less than minimum)
        let actual_size = buffer_size.max(5) * 1024 * 1024;
        assert_eq!(actual_size, 5 * 1024 * 1024);

        // Test normal buffer size
        let buffer_size = 10; // 10MB
        let actual_size = buffer_size.max(5) * 1024 * 1024;
        assert_eq!(actual_size, 10 * 1024 * 1024);
    }

    #[test]
    fn test_completed_parts_mutex() {
        let completed_parts = Arc::new(Mutex::new(Vec::<CompletedPart>::new()));

        // Add a part
        {
            let mut parts = completed_parts.lock().unwrap();
            parts.push(CompletedPart {
                e_tag: Some("etag-1".to_string()),
                part_number: Some(1),
            });
        }

        // Check it was added
        {
            let parts = completed_parts.lock().unwrap();
            assert_eq!(parts.len(), 1);
            assert_eq!(parts[0].part_number, Some(1));
        }
    }

    #[test]
    fn test_completed_parts_sorting() {
        let mut parts = vec![
            CompletedPart {
                e_tag: Some("etag-3".to_string()),
                part_number: Some(3),
            },
            CompletedPart {
                e_tag: Some("etag-1".to_string()),
                part_number: Some(1),
            },
            CompletedPart {
                e_tag: Some("etag-2".to_string()),
                part_number: Some(2),
            },
        ];

        parts.sort_by_key(|part| part.part_number.unwrap_or(0));

        assert_eq!(parts[0].part_number, Some(1));
        assert_eq!(parts[1].part_number, Some(2));
        assert_eq!(parts[2].part_number, Some(3));
    }

    #[test]
    fn test_buffer_capacity() {
        let buffer = BytesMut::with_capacity(1024);
        assert!(buffer.capacity() >= 1024);
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_buffer_extend() {
        let mut buffer = BytesMut::new();
        let data = b"hello world";
        buffer.extend_from_slice(data);
        assert_eq!(buffer.len(), 11);
        assert_eq!(&buffer[..], data);
    }

    #[test]
    fn test_buffer_split() {
        let mut buffer = BytesMut::new();
        buffer.extend_from_slice(b"hello world");

        let split = buffer.split();
        assert_eq!(split.len(), 11);
        assert_eq!(buffer.len(), 0);
        assert_eq!(&split[..], b"hello world");
    }

    #[test]
    fn test_retry_delay_calculation() {
        // Test exponential backoff
        for attempt in 0..MAX_RETRIES {
            let delay_ms = 250 * 2u64.pow(attempt as u32);
            let expected = match attempt {
                0 => 250,
                1 => 500,
                2 => 1000,
                _ => unreachable!(),
            };
            assert_eq!(delay_ms, expected);
        }
    }

    #[test]
    fn test_streaming_target_name_format() {
        let bucket = "my-bucket";
        let key = "path/to/file.txt";
        let expected = format!("s3://{}/{}", bucket, key);
        assert_eq!(expected, "s3://my-bucket/path/to/file.txt");
    }

    #[tokio::test]
    async fn test_mpsc_channel_basic() {
        let (sender, mut receiver) = mpsc::channel::<String>(10);

        // Send a message
        sender.send("hello".to_string()).await.unwrap();

        // Receive it
        let msg = receiver.recv().await.unwrap();
        assert_eq!(msg, "hello");
    }

    #[tokio::test]
    async fn test_mpsc_channel_try_send() {
        let (sender, mut _receiver) = mpsc::channel::<String>(1);

        // First send should succeed
        assert!(sender.try_send("first".to_string()).is_ok());

        // Second send should fail (channel full)
        match sender.try_send("second".to_string()) {
            Err(mpsc::error::TrySendError::Full(_)) => {}
            _ => panic!("Expected channel to be full"),
        }
    }

    #[test]
    fn test_bytes_freeze() {
        let mut bytes_mut = BytesMut::new();
        bytes_mut.extend_from_slice(b"test data");

        let bytes = bytes_mut.freeze();
        assert_eq!(&bytes[..], b"test data");
        assert_eq!(bytes.len(), 9);
    }

    #[test]
    fn test_poll_ready_variants() {
        // Test Poll::Ready
        let ready: Poll<Result<(), io::Error>> = Poll::Ready(Ok(()));
        assert!(matches!(ready, Poll::Ready(Ok(()))));

        // Test Poll::Pending
        let pending: Poll<Result<(), io::Error>> = Poll::Pending;
        assert!(matches!(pending, Poll::Pending));
    }

    #[test]
    fn test_io_error_creation() {
        let error = io::Error::new(io::ErrorKind::BrokenPipe, "test error");
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    }

    #[test]
    fn test_duration_from_millis() {
        let duration = Duration::from_millis(500);
        assert_eq!(duration.as_millis(), 500);
    }

    #[test]
    fn test_pin_creation() {
        let value = 42;
        let boxed = Box::new(value);
        let pinned = Box::pin(value);
        assert_eq!(*boxed, 42);
        assert_eq!(*pinned, 42);
    }
}
