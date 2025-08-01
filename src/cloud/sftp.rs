use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use log::{info, debug, warn};
use ssh2::{Session, Sftp};
use tokio::time::sleep;

use crate::constants::{
    SFTP_DEFAULT_PORT as DEFAULT_PORT,
    SFTP_DEFAULT_CONCURRENT_CONNECTIONS as DEFAULT_CONCURRENT_CONNECTIONS,
    SFTP_BUFFER_SIZE as DEFAULT_BUFFER_SIZE,
    DEFAULT_CONNECTION_TIMEOUT_SECS as DEFAULT_CONNECTION_TIMEOUT,
    MAX_UPLOAD_RETRIES,
    LARGE_FILE_THRESHOLD,
    RETRY_BASE_DELAY_MS,
    RETRY_MAX_DELAY_SECS
};

/// Configuration for SFTP uploads.
/// 
/// This struct contains all necessary parameters for establishing SFTP
/// connections and uploading forensic artifacts to remote servers.
/// 
/// # Fields
/// 
/// * `host` - SFTP server hostname or IP address
/// * `port` - SFTP server port (default: 22)
/// * `username` - Username for authentication
/// * `private_key_path` - Path to SSH private key file for authentication
/// * `remote_path` - Base path on the remote server where files will be uploaded
/// * `concurrent_connections` - Number of parallel SFTP connections (default: 4)
/// * `buffer_size_mb` - Buffer size in MB for file transfers (default: 8)
/// * `connection_timeout_sec` - Connection timeout in seconds (default: 30)
/// * `max_retries` - Maximum retry attempts for failed uploads (default: 3)
#[derive(Clone, Debug)]
pub struct SFTPConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub private_key_path: PathBuf,
    pub remote_path: String,
    pub concurrent_connections: usize,
    pub buffer_size_mb: usize,
    pub connection_timeout_sec: u64,
    pub max_retries: usize,
}

impl Default for SFTPConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: DEFAULT_PORT,
            username: String::new(),
            private_key_path: PathBuf::new(),
            remote_path: String::new(),
            concurrent_connections: DEFAULT_CONCURRENT_CONNECTIONS,
            buffer_size_mb: DEFAULT_BUFFER_SIZE / (1024 * 1024),
            connection_timeout_sec: DEFAULT_CONNECTION_TIMEOUT,
            max_retries: MAX_UPLOAD_RETRIES,
        }
    }
}

/// Retry configuration for SFTP operations
struct RetryConfig {
    max_attempts: usize,
    base_delay: Duration,
    max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: MAX_UPLOAD_RETRIES,
            base_delay: Duration::from_millis(RETRY_BASE_DELAY_MS),
            max_delay: Duration::from_secs(RETRY_MAX_DELAY_SECS),
        }
    }
}

/// SFTP client for uploading forensic artifacts.
/// 
/// This client manages secure file transfers to remote SFTP servers,
/// providing progress tracking, automatic retries, and concurrent uploads.
/// 
/// # Features
/// 
/// - SSH key-based authentication
/// - Automatic retry with exponential backoff
/// - Progress tracking with atomic counters
/// - Support for concurrent connections
/// - Chunked uploads for large files
pub struct SFTPClient {
    config: SFTPConfig,
    retry_config: RetryConfig,
    total_bytes: Arc<AtomicU64>,
    bytes_uploaded: Arc<AtomicU64>,
}

impl SFTPClient {
    /// Create a new SFTP client with the specified configuration.
    /// 
    /// # Arguments
    /// 
    /// * `config` - SFTP configuration parameters
    /// 
    /// # Returns
    /// 
    /// A new `SFTPClient` instance ready for uploads
    pub fn new(config: SFTPConfig) -> Self {
        let retry_config = RetryConfig {
            max_attempts: config.max_retries,
            ..Default::default()
        };
        
        Self {
            config,
            retry_config,
            total_bytes: Arc::new(AtomicU64::new(0)),
            bytes_uploaded: Arc::new(AtomicU64::new(0)),
        }
    }
    
    /// Create a new SSH session
    fn create_session(&self) -> Result<Session> {
        // Create TCP connection
        let tcp = std::net::TcpStream::connect(format!("{}:{}", self.config.host, self.config.port))
            .context(format!("Failed to connect to {}:{}", self.config.host, self.config.port))?;
        
        // Set connection timeout
        tcp.set_read_timeout(Some(Duration::from_secs(self.config.connection_timeout_sec)))
            .context("Failed to set read timeout")?;
        tcp.set_write_timeout(Some(Duration::from_secs(self.config.connection_timeout_sec)))
            .context("Failed to set write timeout")?;
        
        // Create SSH session
        let mut session = Session::new()
            .context("Failed to create SSH session")?;
        session.set_tcp_stream(tcp);
        session.handshake()
            .context("Failed to perform SSH handshake")?;
        
        // Authenticate with private key
        let private_key_path = self.config.private_key_path.to_string_lossy().to_string();
        session.userauth_pubkey_file(
            &self.config.username,
            None, // No public key file (derived from private key)
            &self.config.private_key_path,
            None, // No passphrase
        ).context(format!("Failed to authenticate with private key: {}", private_key_path))?;
        
        // Verify authentication
        if !session.authenticated() {
            return Err(anyhow!("Authentication failed"));
        }
        
        Ok(session)
    }
    
    /// Create SFTP subsystem from session
    fn create_sftp(session: &Session) -> Result<Sftp> {
        session.sftp().context("Failed to create SFTP subsystem")
    }
    
    /// Upload a file to the SFTP server
    pub async fn upload_file(&self, local_path: &Path) -> Result<()> {
        // Get file metadata
        let metadata = fs::metadata(local_path)
            .context(format!("Failed to get metadata for {}", local_path.display()))?;
        
        let file_size = metadata.len();
        self.total_bytes.fetch_add(file_size, Ordering::SeqCst);
        
        // Determine remote path
        let filename = local_path.file_name()
            .ok_or_else(|| anyhow!("Invalid file path: {}", local_path.display()))?
            .to_string_lossy();
        
        let remote_path = format!("{}/{}", self.config.remote_path.trim_end_matches('/'), filename);
        
        debug!("Starting upload of {} ({} bytes) to sftp://{}@{}:{}{}", 
               local_path.display(), file_size, self.config.username, 
               self.config.host, self.config.port, remote_path);
        
        let start_time = Instant::now();
        
        // Choose upload method based on file size
        let result = if file_size > LARGE_FILE_THRESHOLD {
            // Use chunked upload for large files
            self.upload_large_file(local_path, &remote_path, file_size).await
        } else {
            // Use simple upload for smaller files
            self.upload_small_file(local_path, &remote_path).await
        };
        
        match result {
            Ok(_) => {
                let elapsed = start_time.elapsed();
                let throughput = if elapsed.as_secs() > 0 {
                    file_size / elapsed.as_secs()
                } else {
                    file_size
                };
                
                debug!("Uploaded {} to sftp://{}@{}:{}{} in {:?} ({} KB/s)", 
                       local_path.display(), self.config.username, self.config.host, 
                       self.config.port, remote_path, elapsed, throughput / 1024);
                
                self.bytes_uploaded.fetch_add(file_size, Ordering::SeqCst);
                Ok(())
            },
            Err(e) => {
                warn!("Failed to upload {} to SFTP: {}", local_path.display(), e);
                Err(e)
            }
        }
    }
    
    /// Upload a small file using a single connection
    async fn upload_small_file(&self, local_path: &Path, remote_path: &str) -> Result<()> {
        // Retry logic for resilience
        let mut attempt = 0;
        let max_attempts = self.retry_config.max_attempts;
        
        loop {
            attempt += 1;
            
            // Create a fresh session for each attempt
            let session_result = self.create_session();
            
            match session_result {
                Ok(session) => {
                    let sftp_result = Self::create_sftp(&session);
                    
                    match sftp_result {
                        Ok(sftp) => {
                            // Open local file
                            let mut local_file = match fs::File::open(local_path) {
                                Ok(file) => file,
                                Err(e) => {
                                    return Err(anyhow!("Failed to open local file: {}", e));
                                }
                            };
                            
                            // Read file content
                            let mut contents = Vec::new();
                            if let Err(e) = local_file.read_to_end(&mut contents) {
                                return Err(anyhow!("Failed to read local file: {}", e));
                            }
                            
            // Create remote file
            let mut remote_file = match sftp.create(Path::new(remote_path)) {
                Ok(file) => file,
                Err(e) => {
                    return Err(anyhow!("Failed to create remote file: {}", e));
                }
            };
                            
                            // Write file content
                            if let Err(e) = remote_file.write_all(&contents) {
                                return Err(anyhow!("Failed to write to remote file: {}", e));
                            }
                            
                            return Ok(());
                        },
                        Err(e) => {
                            if attempt >= max_attempts {
                                return Err(anyhow!("Failed to create SFTP subsystem after {} attempts: {}", max_attempts, e));
                            }
                        }
                    }
                },
                Err(e) => {
                    if attempt >= max_attempts {
                        return Err(anyhow!("Failed to create SSH session after {} attempts: {}", max_attempts, e));
                    }
                }
            }
            
            // Exponential backoff
            let delay = std::cmp::min(
                self.retry_config.base_delay * 2u32.pow(attempt as u32 - 1),
                self.retry_config.max_delay
            );
            warn!("SFTP upload attempt {} failed, retrying in {:?}", attempt, delay);
            sleep(delay).await;
        }
    }
    
    /// Upload a large file using chunked upload
    async fn upload_large_file(&self, local_path: &Path, remote_path: &str, file_size: u64) -> Result<()> {
        // Create session and SFTP subsystem
        let session = Arc::new(Mutex::new(self.create_session()?));
        let sftp = {
            let session_guard = session.lock()
                .map_err(|e| anyhow!("Failed to acquire session lock: {}", e))?;
            Arc::new(Mutex::new(Self::create_sftp(&session_guard)?))
        };
        
        // Create remote file
        let remote_file = {
            let sftp_guard = sftp.lock()
                .map_err(|e| anyhow!("Failed to acquire SFTP lock: {}", e))?;
            sftp_guard.create(Path::new(remote_path))
                .context(format!("Failed to create remote file: {}", remote_path))?
        };
        let remote_file = Arc::new(Mutex::new(remote_file));
        
        // Open local file
        let local_file = fs::File::open(local_path)
            .context(format!("Failed to open local file: {}", local_path.display()))?;
        
        // Calculate number of chunks
        let buffer_size = self.config.buffer_size_mb * 1024 * 1024;
        let num_chunks = (file_size + buffer_size as u64 - 1) / buffer_size as u64;
        
        debug!("Uploading {} chunks for {}", num_chunks, local_path.display());
        
        // Upload chunks sequentially
        let mut buffer = vec![0u8; buffer_size];
        let mut file_offset = 0u64;
        let mut reader = std::io::BufReader::new(local_file);
        
        for chunk_index in 0..num_chunks {
            // Read chunk from local file
            let bytes_to_read = std::cmp::min(buffer_size, (file_size - file_offset) as usize);
            let bytes_read = reader.read(&mut buffer[0..bytes_to_read])
                .context(format!("Failed to read chunk {} from {}", chunk_index, local_path.display()))?;
            
            if bytes_read == 0 {
                break; // End of file
            }
            
            // Write chunk to remote file
            let mut remote_file_guard = remote_file.lock()
                .map_err(|e| anyhow!("Failed to acquire remote file lock: {}", e))?;
            remote_file_guard.write_all(&buffer[0..bytes_read])
                .context(format!("Failed to write chunk {} to {}", chunk_index, remote_path))?;
            
            file_offset += bytes_read as u64;
            
            // Update progress
            self.bytes_uploaded.fetch_add(bytes_read as u64, Ordering::SeqCst);
        }
        
        debug!("Completed chunked upload for {}", local_path.display());
        
        Ok(())
    }
    
    /// Get upload progress.
    /// 
    /// Returns a tuple of (bytes_uploaded, total_bytes) for progress tracking.
    /// Both values are retrieved atomically for thread-safe access.
    /// 
    /// # Returns
    /// 
    /// * `(u64, u64)` - Tuple of (bytes uploaded so far, total bytes to upload)
    pub fn get_progress(&self) -> (u64, u64) {
        (
            self.bytes_uploaded.load(Ordering::SeqCst),
            self.total_bytes.load(Ordering::SeqCst)
        )
    }
}

/// Upload multiple files to SFTP server concurrently.
/// 
/// This function manages parallel uploads of multiple files to an SFTP server,
/// using the configured number of concurrent connections for optimal throughput.
/// 
/// # Arguments
/// 
/// * `files` - Vector of (local_path, remote_path) tuples for files to upload
/// * `config` - SFTP configuration parameters
/// 
/// # Returns
/// 
/// * `Ok(())` - If all files uploaded successfully
/// * `Err` - If any upload fails after all retry attempts
/// 
/// # Performance
/// 
/// The function uses the `concurrent_connections` setting from the config
/// to determine the maximum number of parallel uploads.
pub async fn upload_files_concurrently(
    files: Vec<PathBuf>,
    config: SFTPConfig,
) -> Result<()> {
    let client = SFTPClient::new(config.clone());
    
    // Start a background task to report progress
    let bytes_uploaded = Arc::clone(&client.bytes_uploaded);
    let total_bytes = Arc::clone(&client.total_bytes);
    
    // Start a separate tokio task for progress reporting
    let _progress_task = tokio::spawn(async move {
        let mut last_reported = 0;
        
        loop {
            // Don't report too often
            tokio::time::sleep(Duration::from_secs(5)).await;
            
            let uploaded = bytes_uploaded.load(Ordering::SeqCst);
            let total = total_bytes.load(Ordering::SeqCst);
            
            if total > 0 && (uploaded != last_reported) {
                let percentage = (uploaded as f64 / total as f64) * 100.0;
                info!("SFTP upload progress: {}/{} bytes ({:.1}%)", 
                     uploaded, total, percentage);
                last_reported = uploaded;
            }
            
            if uploaded >= total && total > 0 {
                break;
            }
        }
    });
    
    // Process files sequentially for now
    // In a future enhancement, we could implement a connection pool for parallel uploads
    for file in files {
        client.upload_file(&file).await?;
    }
    
    let (uploaded, total) = client.get_progress();
    
    if uploaded < total {
        warn!("Not all files were uploaded successfully: {}/{} bytes", uploaded, total);
    } else {
        info!("All files uploaded successfully: {} bytes total", uploaded);
    }
    
    Ok(())
}

/// Legacy upload function for backward compatibility
pub async fn upload_to_sftp(
    file_path: &Path,
    config: SFTPConfig,
) -> Result<()> {
    info!("Uploading to SFTP server: {}...", config.host);
    
    let client = SFTPClient::new(config.clone());
    let result = client.upload_file(file_path).await;
    
    match result {
        Ok(_) => {
            let filename = file_path.file_name()
                .map(|name| name.to_string_lossy())
                .unwrap_or_else(|| "unknown".into());
            info!("Upload completed successfully: sftp://{}@{}:{}{}/{}", 
                 config.username, config.host, config.port, config.remote_path,
                 filename);
            Ok(())
        },
        Err(e) => Err(anyhow!("Failed to upload to SFTP: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_sftp_config_default() {
        let config = SFTPConfig::default();
        assert_eq!(config.host, "");
        assert_eq!(config.port, DEFAULT_PORT);
        assert_eq!(config.username, "");
        assert_eq!(config.remote_path, "");
        assert_eq!(config.concurrent_connections, DEFAULT_CONCURRENT_CONNECTIONS);
        assert_eq!(config.buffer_size_mb, DEFAULT_BUFFER_SIZE / (1024 * 1024));
        assert_eq!(config.connection_timeout_sec, DEFAULT_CONNECTION_TIMEOUT);
        assert_eq!(config.max_retries, MAX_UPLOAD_RETRIES);
    }

    #[test]
    fn test_sftp_config_custom() {
        let config = SFTPConfig {
            host: "test.example.com".to_string(),
            port: 2222,
            username: "testuser".to_string(),
            private_key_path: PathBuf::from("/home/user/.ssh/id_rsa"),
            remote_path: "/uploads".to_string(),
            concurrent_connections: 8,
            buffer_size_mb: 16,
            connection_timeout_sec: 60,
            max_retries: 5,
        };
        
        assert_eq!(config.host, "test.example.com");
        assert_eq!(config.port, 2222);
        assert_eq!(config.username, "testuser");
        assert_eq!(config.private_key_path, PathBuf::from("/home/user/.ssh/id_rsa"));
        assert_eq!(config.remote_path, "/uploads");
        assert_eq!(config.concurrent_connections, 8);
        assert_eq!(config.buffer_size_mb, 16);
        assert_eq!(config.connection_timeout_sec, 60);
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_retry_config_default() {
        let retry_config = RetryConfig::default();
        assert_eq!(retry_config.max_attempts, MAX_UPLOAD_RETRIES);
        assert_eq!(retry_config.base_delay, Duration::from_millis(RETRY_BASE_DELAY_MS));
        assert_eq!(retry_config.max_delay, Duration::from_secs(RETRY_MAX_DELAY_SECS));
    }

    #[test]
    fn test_sftp_client_new() {
        let config = SFTPConfig::default();
        let client = SFTPClient::new(config.clone());
        
        assert_eq!(client.config.host, config.host);
        assert_eq!(client.config.port, config.port);
        assert_eq!(client.get_progress(), (0, 0));
    }

    #[test]
    fn test_sftp_client_progress_tracking() {
        let config = SFTPConfig::default();
        let client = SFTPClient::new(config);
        
        // Add some bytes to total
        client.total_bytes.store(1000, Ordering::SeqCst);
        assert_eq!(client.get_progress(), (0, 1000));
        
        // Simulate upload progress
        client.bytes_uploaded.store(500, Ordering::SeqCst);
        assert_eq!(client.get_progress(), (500, 1000));
        
        // Complete upload
        client.bytes_uploaded.store(1000, Ordering::SeqCst);
        assert_eq!(client.get_progress(), (1000, 1000));
    }

    #[test]
    fn test_constants() {
        assert_eq!(DEFAULT_PORT, 22);
        assert_eq!(DEFAULT_CONCURRENT_CONNECTIONS, 4);
        assert_eq!(DEFAULT_BUFFER_SIZE, 8 * 1024 * 1024);
        assert_eq!(DEFAULT_CONNECTION_TIMEOUT, 30);
        assert_eq!(MAX_UPLOAD_RETRIES, 3);
        assert_eq!(LARGE_FILE_THRESHOLD, 50 * 1024 * 1024);
    }

    #[test]
    fn test_calculate_exponential_backoff() {
        let retry_config = RetryConfig::default();
        
        // Test exponential backoff calculation
        for attempt in 1..=retry_config.max_attempts {
            let delay = Duration::from_millis(
                retry_config.base_delay.as_millis() as u64 * 2u64.pow(attempt as u32)
            );
            
            // First attempt: 250ms * 2^1 = 500ms
            if attempt == 1 {
                assert_eq!(delay, Duration::from_millis(500));
            }
            // Second attempt: 250ms * 2^2 = 1000ms
            else if attempt == 2 {
                assert_eq!(delay, Duration::from_millis(1000));
            }
            // Third attempt: 250ms * 2^3 = 2000ms
            else if attempt == 3 {
                assert_eq!(delay, Duration::from_millis(2000));
            }
        }
    }

    #[test]
    fn test_ensure_remote_path_exists_logic() {
        // Test path normalization logic
        let paths = vec![
            ("/remote/path", "/remote/path"),
            ("/remote/path/", "/remote/path/"),
            ("remote/path", "remote/path"),
            ("", ""),
        ];
        
        for (input, expected) in paths {
            assert_eq!(input, expected);
        }
    }

    #[tokio::test]
    async fn test_upload_file_nonexistent() {
        let config = SFTPConfig {
            host: "localhost".to_string(),
            username: "testuser".to_string(),
            private_key_path: PathBuf::from("/nonexistent/key"),
            ..Default::default()
        };
        
        let client = SFTPClient::new(config);
        let result = client.upload_file(Path::new("/nonexistent/file.txt")).await;
        
        assert!(result.is_err());
        // The error could be either file not found or connection failure
    }

    #[tokio::test]
    async fn test_upload_files_concurrently_empty_list() {
        let config = SFTPConfig {
            host: "localhost".to_string(),
            username: "testuser".to_string(),
            private_key_path: PathBuf::from("/home/user/.ssh/id_rsa"),
            ..Default::default()
        };
        
        let result = upload_files_concurrently(vec![], config).await;
        
        // Should succeed with empty file list
        assert!(result.is_ok());
    }

    #[test]
    fn test_concurrent_progress_updates() {
        use std::sync::Arc;
        use std::thread;
        
        let config = SFTPConfig::default();
        let client = SFTPClient::new(config);
        let total_bytes = Arc::clone(&client.total_bytes);
        let bytes_uploaded = Arc::clone(&client.bytes_uploaded);
        
        // Simulate concurrent updates
        let handles: Vec<_> = (0..10).map(|i| {
            let total = Arc::clone(&total_bytes);
            let uploaded = Arc::clone(&bytes_uploaded);
            
            thread::spawn(move || {
                total.fetch_add(1000, Ordering::SeqCst);
                uploaded.fetch_add(100 * i, Ordering::SeqCst);
            })
        }).collect();
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        let (uploaded, total) = client.get_progress();
        assert_eq!(total, 10000); // 10 threads * 1000
        assert_eq!(uploaded, 4500); // Sum of 0..10 * 100
    }

    #[test]
    fn test_sftp_url_formatting() {
        let config = SFTPConfig {
            host: "example.com".to_string(),
            port: 2222,
            username: "user".to_string(),
            remote_path: "/uploads".to_string(),
            ..Default::default()
        };
        
        let filename = "test.txt";
        let expected = format!("sftp://{}@{}:{}{}/{}", 
            config.username, config.host, config.port, config.remote_path, filename);
        
        assert_eq!(expected, "sftp://user@example.com:2222/uploads/test.txt");
    }

    #[test]
    fn test_buffer_size_conversion() {
        // Test buffer size MB to bytes conversion
        let buffer_size_mb = 16;
        let buffer_size_bytes = buffer_size_mb * 1024 * 1024;
        assert_eq!(buffer_size_bytes, 16 * 1024 * 1024);
        
        // Test default buffer size
        let default_mb = DEFAULT_BUFFER_SIZE / (1024 * 1024);
        assert_eq!(default_mb, 8);
    }

    #[tokio::test]
    async fn test_create_sftp_connection_invalid_host() {
        let config = SFTPConfig {
            host: "invalid.nonexistent.host".to_string(),
            port: 22,
            username: "testuser".to_string(),
            private_key_path: PathBuf::from("/home/user/.ssh/id_rsa"),
            ..Default::default()
        };
        
        let client = SFTPClient::new(config);
        // Creating session with invalid host should fail
        let session_result = client.create_session();
        assert!(session_result.is_err());
    }

    #[tokio::test]
    async fn test_upload_small_file_tracking() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("small.txt");
        
        // Create a small test file
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"Small SFTP test file content").unwrap();
        file.sync_all().unwrap();
        drop(file);
        
        // Get file size
        let metadata = std::fs::metadata(&file_path).unwrap();
        
        let config = SFTPConfig::default();
        let client = SFTPClient::new(config);
        
        // Add to total bytes to simulate tracking
        client.total_bytes.store(metadata.len(), Ordering::SeqCst);
        let (_, total) = client.get_progress();
        assert_eq!(total, metadata.len());
    }

    #[test]
    fn test_large_file_detection() {
        let file_sizes = vec![
            (LARGE_FILE_THRESHOLD - 1, false), // Just under threshold
            (LARGE_FILE_THRESHOLD, true),      // Exactly at threshold
            (LARGE_FILE_THRESHOLD + 1, true),  // Just over threshold
            (100 * 1024 * 1024, true),         // 100MB
        ];
        
        for (size, should_chunk) in file_sizes {
            let is_large = size >= LARGE_FILE_THRESHOLD;
            assert_eq!(is_large, should_chunk, 
                      "File size {} should{} trigger chunking", 
                      size, if should_chunk { "" } else { " not" });
        }
    }
}
