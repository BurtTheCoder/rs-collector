use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use log::{info, debug, warn};
use ssh2::{Session, Sftp};
use tokio::time::sleep;

// Constants for SFTP uploads
const DEFAULT_PORT: u16 = 22;
const DEFAULT_CONCURRENT_CONNECTIONS: usize = 4;
const DEFAULT_BUFFER_SIZE: usize = 8 * 1024 * 1024; // 8MB
const DEFAULT_CONNECTION_TIMEOUT: u64 = 30; // seconds
const MAX_UPLOAD_RETRIES: usize = 3;
const LARGE_FILE_THRESHOLD: u64 = 50 * 1024 * 1024; // 50MB - use chunking for larger files

/// Configuration for SFTP uploads
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
            base_delay: Duration::from_millis(250),
            max_delay: Duration::from_secs(30),
        }
    }
}

/// SFTP client for uploading files
pub struct SFTPClient {
    config: SFTPConfig,
    retry_config: RetryConfig,
    total_bytes: Arc<AtomicU64>,
    bytes_uploaded: Arc<AtomicU64>,
}

impl SFTPClient {
    /// Create a new SFTP client with the specified configuration
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
        let sftp = Arc::new(Mutex::new(Self::create_sftp(&session.lock().unwrap())?));
        
        // Create remote file
        let remote_file = {
            let sftp_guard = sftp.lock().unwrap();
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
            let mut remote_file_guard = remote_file.lock().unwrap();
            remote_file_guard.write_all(&buffer[0..bytes_read])
                .context(format!("Failed to write chunk {} to {}", chunk_index, remote_path))?;
            
            file_offset += bytes_read as u64;
            
            // Update progress
            self.bytes_uploaded.fetch_add(bytes_read as u64, Ordering::SeqCst);
        }
        
        debug!("Completed chunked upload for {}", local_path.display());
        
        Ok(())
    }
    
    /// Get upload progress
    pub fn get_progress(&self) -> (u64, u64) {
        (
            self.bytes_uploaded.load(Ordering::SeqCst),
            self.total_bytes.load(Ordering::SeqCst)
        )
    }
}

/// Upload multiple files to SFTP server concurrently
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
            info!("Upload completed successfully: sftp://{}@{}:{}{}/{}", 
                 config.username, config.host, config.port, config.remote_path,
                 file_path.file_name().unwrap().to_string_lossy());
            Ok(())
        },
        Err(e) => Err(anyhow!("Failed to upload to SFTP: {}", e))
    }
}
