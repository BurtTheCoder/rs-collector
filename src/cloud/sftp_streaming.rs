use std::io::{self, Write};
use std::path::Path;
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use std::task::{Context, Poll};

use crate::cloud::streaming_target::StreamingTarget;
use anyhow::{anyhow, Context as AnyhowContext, Result};
use bytes::BytesMut;
use log::{debug, warn};
use ssh2::{Session, Sftp};
use tokio::io::AsyncWrite;
use tokio::sync::mpsc;
use tokio::time::Duration;

use crate::cloud::sftp::SFTPConfig;

// Constants
const MAX_RETRIES: usize = 3;

/// A stream that buffers data and uploads it to SFTP server.
///
/// This implementation provides:
/// - Buffered writes that are sent to the SFTP server when they reach the buffer size
/// - Automatic retry with exponential backoff for failed operations
/// - Progress tracking with atomic counters for thread safety
/// - Async/await compatible interface that implements AsyncWrite
pub struct SFTPUploadStream {
    _session: Arc<Mutex<Session>>,
    _sftp: Arc<Mutex<Sftp>>,
    _remote_file: Arc<Mutex<ssh2::File>>,
    remote_path: String,
    buffer: BytesMut,
    buffer_size: usize,
    sender: mpsc::Sender<UploadTask>,
    _upload_task: tokio::task::JoinHandle<Result<()>>,
    /// Atomic counter for tracking uploaded bytes
    bytes_uploaded: Arc<AtomicU64>,
}

struct UploadTask {
    data: BytesMut,
    _offset: u64,
}

impl SFTPUploadStream {
    /// Create a new SFTP upload stream with the specified buffer size.
    ///
    /// This initializes a new SFTP connection and creates the remote file.
    ///
    /// # Arguments
    ///
    /// * `config` - The SFTP configuration
    /// * `remote_path` - The remote file path
    /// * `buffer_size_mb` - Buffer size in megabytes
    ///
    /// # Returns
    ///
    /// A new SFTPUploadStream instance or an error if the connection couldn't be established
    pub async fn new(config: SFTPConfig, remote_path: &str, buffer_size_mb: usize) -> Result<Self> {
        // Create TCP connection
        let tcp =
            std::net::TcpStream::connect(format!("{}:{}", config.host, config.port)).context(
                format!("Failed to connect to {}:{}", config.host, config.port),
            )?;

        // Set connection timeout
        tcp.set_read_timeout(Some(Duration::from_secs(config.connection_timeout_sec)))
            .context("Failed to set read timeout")?;
        tcp.set_write_timeout(Some(Duration::from_secs(config.connection_timeout_sec)))
            .context("Failed to set write timeout")?;

        // Create SSH session
        let mut session = Session::new().context("Failed to create SSH session")?;
        session.set_tcp_stream(tcp);
        session
            .handshake()
            .context("Failed to perform SSH handshake")?;

        // Authenticate with private key
        let private_key_path = config.private_key_path.to_string_lossy().to_string();
        session
            .userauth_pubkey_file(
                &config.username,
                None, // No public key file (derived from private key)
                &config.private_key_path,
                None, // No passphrase
            )
            .context(format!(
                "Failed to authenticate with private key: {}",
                private_key_path
            ))?;

        // Verify authentication
        if !session.authenticated() {
            return Err(anyhow!("Authentication failed"));
        }

        // Create SFTP subsystem
        let sftp = session.sftp().context("Failed to create SFTP subsystem")?;

        // Create remote file
        let remote_file = sftp
            .create(Path::new(remote_path))
            .context(format!("Failed to create remote file: {}", remote_path))?;

        // Ensure buffer size is reasonable
        let buffer_size = buffer_size_mb.max(1) * 1024 * 1024;

        // Create shared state
        let session = Arc::new(Mutex::new(session));
        let sftp = Arc::new(Mutex::new(sftp));
        let remote_file = Arc::new(Mutex::new(remote_file));
        let bytes_uploaded = Arc::new(AtomicU64::new(0));

        // Create channel for upload tasks
        let (sender, mut receiver) = mpsc::channel::<UploadTask>(100);

        // Clone shared state for upload task
        let remote_file_clone = Arc::clone(&remote_file);
        let bytes_uploaded_clone = Arc::clone(&bytes_uploaded);

        // Spawn background task to handle uploads
        let upload_task = tokio::spawn(async move {
            let mut _file_offset = 0u64;

            while let Some(task) = receiver.recv().await {
                let data_size = task.data.len();

                // Upload with retries
                let mut attempts = 0;
                let mut success = false;

                while attempts < MAX_RETRIES && !success {
                    attempts += 1;

                    // Write to the remote file, but drop the guard before awaiting
                    let write_result = {
                        let mut remote_file_guard = match remote_file_clone.lock() {
                            Ok(guard) => guard,
                            Err(e) => {
                                return Err(anyhow!("Failed to lock remote file: {}", e));
                            }
                        };

                        remote_file_guard.write_all(&task.data)
                    };

                    match write_result {
                        Ok(_) => {
                            bytes_uploaded_clone.fetch_add(data_size as u64, Ordering::SeqCst);
                            _file_offset += data_size as u64;
                            success = true;
                        }
                        Err(e) => {
                            if attempts >= MAX_RETRIES {
                                return Err(anyhow!(
                                    "Failed to write to remote file after {} attempts: {}",
                                    MAX_RETRIES,
                                    e
                                ));
                            }

                            let delay = Duration::from_millis(250 * 2u64.pow(attempts as u32));
                            warn!(
                                "SFTP write attempt {} failed, retrying in {:?}: {}",
                                attempts, delay, e
                            );
                            tokio::time::sleep(delay).await;
                        }
                    }
                }

                if !success {
                    return Err(anyhow!("Failed to write to remote file"));
                }
            }

            Ok(())
        });

        Ok(Self {
            _session: session,
            _sftp: sftp,
            _remote_file: remote_file,
            remote_path: remote_path.to_string(),
            buffer: BytesMut::with_capacity(buffer_size),
            buffer_size,
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
    /// The total number of bytes successfully uploaded to the SFTP server
    pub fn bytes_uploaded(&self) -> u64 {
        self.bytes_uploaded.load(Ordering::SeqCst)
    }

    /// Complete the upload.
    ///
    /// This method finalizes the upload by:
    /// 1. Closing the upload channel
    /// 2. Waiting for all pending uploads to complete
    /// 3. Closing the remote file and SFTP session
    ///
    /// # Returns
    ///
    /// Ok(()) if the upload was successfully completed, or an error
    ///
    /// # Notes
    ///
    /// This method consumes self, so the SFTPUploadStream cannot be used after calling complete
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

        debug!("Completed streaming upload to {}", self.remote_path);

        Ok(())
    }

    /// Abort the upload.
    ///
    /// This method cancels the upload and attempts to remove the remote file.
    /// It should be called when an error occurs and the upload needs to be abandoned.
    ///
    /// # Returns
    ///
    /// Ok(()) if the abort was successful, or an error
    ///
    /// # Notes
    ///
    /// This method consumes self, so the SFTPUploadStream cannot be used after calling abort
    pub async fn abort(self) -> Result<()> {
        // Drop sender to close the channel
        drop(self.sender);

        // Wait for upload task to complete or fail
        let _ = self._upload_task.await;

        // Try to remove the remote file
        let sftp_guard = match self._sftp.lock() {
            Ok(guard) => guard,
            Err(e) => {
                return Err(anyhow!("Failed to lock SFTP: {}", e));
            }
        };

        match sftp_guard.unlink(Path::new(&self.remote_path)) {
            Ok(_) => {
                debug!(
                    "Aborted upload and removed remote file: {}",
                    self.remote_path
                );
            }
            Err(e) => {
                warn!("Failed to remove remote file after abort: {}", e);
            }
        }

        Ok(())
    }
}

impl StreamingTarget for SFTPUploadStream {
    fn target_name(&self) -> String {
        format!("sftp://{}", self.remote_path)
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

impl AsyncWrite for SFTPUploadStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        // Add data to buffer
        self.buffer.extend_from_slice(buf);

        // If buffer is large enough, send a part
        if self.buffer.len() >= self.buffer_size {
            let data = self.buffer.split();
            let offset = self.bytes_uploaded.load(Ordering::SeqCst);

            // Try to send the upload task
            match self.sender.try_send(UploadTask {
                data,
                _offset: offset,
            }) {
                Ok(_) => {}
                Err(e) => {
                    match e {
                        mpsc::error::TrySendError::Full(task) => {
                            // Channel is full, put data back in buffer and return pending
                            self.buffer = task.data;
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
            let data = self.buffer.split();
            let offset = self.bytes_uploaded.load(Ordering::SeqCst);

            // Try to send the upload task
            match self.sender.try_send(UploadTask {
                data,
                _offset: offset,
            }) {
                Ok(_) => {}
                Err(e) => {
                    match e {
                        mpsc::error::TrySendError::Full(task) => {
                            // Channel is full, put data back in buffer and return pending
                            self.buffer = task.data;
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

/// Create a new SFTP upload stream with the given configuration.
///
/// This is a convenience function that creates a new SFTPUploadStream with the
/// specified configuration and remote path.
///
/// # Arguments
///
/// * `config` - The SFTP configuration
/// * `remote_path` - The remote file path
/// * `buffer_size_mb` - Buffer size in megabytes
///
/// # Returns
///
/// A new SFTPUploadStream instance or an error if the connection couldn't be established
pub async fn create_sftp_upload_stream(
    config: SFTPConfig,
    remote_path: &str,
    buffer_size_mb: usize,
) -> Result<SFTPUploadStream> {
    SFTPUploadStream::new(config, remote_path, buffer_size_mb).await
}
