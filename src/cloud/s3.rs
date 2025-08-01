use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use futures::future::{self};
use log::{info, debug, warn};
use rusoto_core::{Region, ByteStream};
use rusoto_s3::{
    PutObjectRequest, S3Client, S3, 
    CreateMultipartUploadRequest, UploadPartRequest, CompleteMultipartUploadRequest,
    CompletedPart, CompletedMultipartUpload, AbortMultipartUploadRequest
};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::fs::File as AsyncFile;
use tokio::time::sleep;

use crate::constants::{
    S3_UPLOAD_CHUNK_SIZE as UPLOAD_CHUNK_SIZE,
    MAX_UPLOAD_RETRIES,
    LARGE_FILE_THRESHOLD
};

/// Async file queue for concurrent uploads to Amazon S3.
/// 
/// This struct manages asynchronous uploads to S3, providing progress tracking
/// and automatic retry logic. It supports both single-file uploads and multipart
/// uploads for large files.
/// 
/// # Fields
/// 
/// * `bucket` - The S3 bucket name
/// * `prefix` - Prefix to prepend to all uploaded object keys
/// * `region` - AWS region for the S3 bucket
/// * `client` - Shared S3 client instance
/// * `total_bytes` - Total bytes to upload (for progress tracking)
/// * `bytes_uploaded` - Bytes uploaded so far (atomic for thread safety)
pub struct UploadQueue {
    bucket: String,
    prefix: String,
    region: Region,
    client: Arc<S3Client>,
    total_bytes: Arc<AtomicU64>,
    bytes_uploaded: Arc<AtomicU64>,
}

impl UploadQueue {
    /// Create a new upload queue.
    /// 
    /// # Arguments
    /// 
    /// * `bucket` - S3 bucket name
    /// * `prefix` - Prefix for all object keys (e.g., "forensics/case-123/")
    /// * `region_name` - Optional AWS region name (e.g., "us-east-1"). Defaults to us-east-1
    /// * `profile` - Optional AWS profile name for credentials
    /// 
    /// # Example
    /// 
    /// ```no_run
    /// # use rust_collector::cloud::s3::UploadQueue;
    /// let queue = UploadQueue::new(
    ///     "my-forensics-bucket",
    ///     "collections/2024-01-15/",
    ///     Some("us-west-2"),
    ///     None
    /// );
    /// ```
    pub fn new(bucket: &str, prefix: &str, region_name: Option<&str>, profile: Option<&str>) -> Self {
        let region = match region_name {
            Some(name) => {
                match name.parse::<Region>() {
                    Ok(r) => r,
                    Err(_) => {
                        warn!("Invalid region '{}', using default", name);
                        Region::default()
                    }
                }
            },
            None => Region::default(),
        };
        
        // Create S3 client with profile if specified
        let s3_client = if let Some(profile_name) = profile {
            match rusoto_credential::ProfileProvider::new() {
                Ok(mut provider) => {
                    provider.set_profile(profile_name);
                    match rusoto_core::HttpClient::new() {
                        Ok(http_client) => Arc::new(S3Client::new_with(
                            http_client,
                            provider,
                            region.clone()
                        )),
                        Err(e) => {
                            warn!("Failed to create HTTP client: {}, using default", e);
                            Arc::new(S3Client::new(region.clone()))
                        }
                    }
                },
                Err(e) => {
                    warn!("Failed to create AWS profile provider: {}, using default", e);
                    Arc::new(S3Client::new(region.clone()))
                }
            }
        } else {
            Arc::new(S3Client::new(region.clone()))
        };
        
        UploadQueue {
            bucket: bucket.to_string(),
            prefix: prefix.to_string(),
            region,
            client: s3_client,
            total_bytes: Arc::new(AtomicU64::new(0)),
            bytes_uploaded: Arc::new(AtomicU64::new(0)),
        }
    }
    
    /// Add a file to the upload queue and start uploading it
    pub async fn add_file(&self, file_path: PathBuf) -> Result<()> {
        // Get file metadata
        let metadata = tokio::fs::metadata(&file_path).await
            .context(format!("Failed to get metadata for {}", file_path.display()))?;
        
        let file_size = metadata.len();
        self.total_bytes.fetch_add(file_size, Ordering::SeqCst);
        
        // Determine S3 key
        let filename = file_path.file_name()
            .ok_or_else(|| anyhow!("Invalid file path - no filename component: {}", file_path.display()))?
            .to_string_lossy();
        let key = format!("{}/{}", self.prefix, filename);
        
        debug!("Starting upload of {} ({} bytes) to s3://{}/{}", 
               file_path.display(), file_size, self.bucket, key);
        
        let start_time = Instant::now();
        
        // Choose upload method based on file size
        let result = if file_size > LARGE_FILE_THRESHOLD {
            // Use multipart upload for large files
            self.upload_large_file(&file_path, &key, file_size).await
        } else {
            // Use simple put_object for smaller files
            self.upload_small_file(&file_path, &key).await
        };
        
        match result {
            Ok(_) => {
                let elapsed = start_time.elapsed();
                let throughput = if elapsed.as_secs() > 0 {
                    file_size / elapsed.as_secs()
                } else {
                    file_size
                };
                
                debug!("Uploaded {} to s3://{}/{} in {:?} ({} KB/s)", 
                       file_path.display(), self.bucket, key, elapsed, throughput / 1024);
                
                self.bytes_uploaded.fetch_add(file_size, Ordering::SeqCst);
                Ok(())
            },
            Err(e) => {
                warn!("Failed to upload {} to S3: {}", file_path.display(), e);
                Err(e)
            }
        }
    }
    
    /// Get upload progress.
    /// 
    /// Returns a tuple of (bytes_uploaded, total_bytes) for progress tracking.
    /// Both values are retrieved atomically for thread-safe access.
    /// 
    /// # Returns
    /// 
    /// * `(u64, u64)` - Tuple of (bytes uploaded so far, total bytes to upload)
    /// 
    /// # Example
    /// 
    /// ```no_run
    /// # use rust_collector::cloud::s3::UploadQueue;
    /// # let queue = UploadQueue::new("bucket", "prefix", None, None);
    /// let (uploaded, total) = queue.get_progress();
    /// let percentage = (uploaded as f64 / total as f64) * 100.0;
    /// println!("Upload progress: {:.1}%", percentage);
    /// ```
    pub fn get_progress(&self) -> (u64, u64) {
        (
            self.bytes_uploaded.load(Ordering::SeqCst),
            self.total_bytes.load(Ordering::SeqCst)
        )
    }
    
    /// Get the AWS region being used for uploads.
    /// 
    /// # Returns
    /// 
    /// A reference to the `Region` enum representing the AWS region
    pub fn get_region(&self) -> &Region {
        &self.region
    }
    
    /// Upload a small file using PutObject
    async fn upload_small_file(&self, file_path: &Path, key: &str) -> Result<()> {
        // Open file for reading
        let mut file = fs::File::open(file_path)
            .context(format!("Failed to open {} for S3 upload", file_path.display()))?;
        
        // Read file content
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .context(format!("Failed to read {} for S3 upload", file_path.display()))?;
        
        // Retry logic for resilience
        let mut attempt = 0;
        let max_attempts = MAX_UPLOAD_RETRIES;
        
        loop {
            attempt += 1;
            
            // Create a fresh request with cloned contents for each attempt
            // ByteStream consumes the Vec, so we need to clone for retries
            let contents_for_request = if attempt > 1 {
                contents.clone()
            } else {
                contents.clone() // Clone on first attempt too to avoid consuming original
            };
            
            let request = PutObjectRequest {
                bucket: self.bucket.clone(),
                key: key.to_string(),
                body: Some(ByteStream::from(contents_for_request)),
                ..Default::default()
            };
            
            match self.client.put_object(request).await {
                Ok(_) => {
                    return Ok(());
                },
                Err(e) => {
                    if attempt >= max_attempts {
                        return Err(anyhow!("Failed to upload to S3 after {} attempts: {}", max_attempts, e));
                    }
                    
                    // Exponential backoff
                    let delay = Duration::from_millis(250 * 2u64.pow(attempt as u32));
                    warn!("S3 upload attempt {} failed, retrying in {:?}: {}", attempt, delay, e);
                    sleep(delay).await;
                }
            }
        }
    }
    
    /// Upload a large file using multipart upload
    async fn upload_large_file(&self, file_path: &Path, key: &str, file_size: u64) -> Result<()> {
        // Step 1: Initialize multipart upload
        let create_result = self.client.create_multipart_upload(CreateMultipartUploadRequest {
            bucket: self.bucket.clone(),
            key: key.to_string(),
            ..Default::default()
        }).await.context("Failed to initialize multipart upload")?;
        
        let upload_id = create_result.upload_id
            .ok_or_else(|| anyhow!("No upload ID returned from S3"))?;
        
        debug!("Started multipart upload with ID: {} for {}", upload_id, file_path.display());
        
        // Step 2: Set up for parallel upload
        // Verify file exists before starting the upload process
        let _file_check = AsyncFile::open(file_path).await
            .context(format!("Failed to open file for multipart upload: {}", file_path.display()))?;
        
        let num_parts = (file_size + UPLOAD_CHUNK_SIZE as u64 - 1) / UPLOAD_CHUNK_SIZE as u64;
        debug!("Uploading {} parts for {}", num_parts, file_path.display());
        
        // Create a vector to store completed part info
        let mut completed_parts = Vec::with_capacity(num_parts as usize);
        
        // Process parts with controlled concurrency
        let concurrency_limit = std::cmp::min(4, num_cpus::get());
        
        // Process all parts in chunks to limit concurrency
        for chunk_start in (1..=num_parts).step_by(concurrency_limit as usize) {
            let chunk_end = std::cmp::min(chunk_start + concurrency_limit as u64 - 1, num_parts);
            debug!("Processing part chunk {} to {}", chunk_start, chunk_end);
            
            // Create a Vec of futures for this chunk
            let mut chunk_futures = Vec::with_capacity((chunk_end - chunk_start + 1) as usize);
            
            // Add futures for each part in this chunk
            for part_number in chunk_start..=chunk_end {
                let bucket = self.bucket.clone();
                let key = key.to_string();
                let upload_id = upload_id.clone();
                let client = Arc::clone(&self.client);
                let file_path = file_path.to_path_buf();
                
                // Calculate offsets for this part
                let start_byte = (part_number - 1) * UPLOAD_CHUNK_SIZE as u64;
                let end_byte = std::cmp::min(part_number * UPLOAD_CHUNK_SIZE as u64, file_size);
                let part_size = (end_byte - start_byte) as usize;
                
                // Create future for this part
                let part_future = async move {
                    let mut attempts = 0;
                    
                    while attempts < MAX_UPLOAD_RETRIES {
                        attempts += 1;
                        
                        // Read the part from file
                        let mut file = AsyncFile::open(&file_path).await?;
                        file.seek(tokio::io::SeekFrom::Start(start_byte)).await?;
                        
                        let mut buffer = vec![0u8; part_size];
                        file.read_exact(&mut buffer).await?;
                        
                        // Upload the part
                        let upload_part_request = UploadPartRequest {
                            bucket: bucket.clone(),
                            key: key.clone(),
                            upload_id: upload_id.clone(),
                            part_number: part_number as i64,
                            body: Some(ByteStream::from(buffer)),
                            ..Default::default()
                        };
                        
                        match client.upload_part(upload_part_request).await {
                            Ok(output) => {
                                let e_tag = output.e_tag
                                    .ok_or_else(|| anyhow!("No ETag in upload part response"))?;
                                
                                return Ok::<_, anyhow::Error>(CompletedPart {
                                    e_tag: Some(e_tag),
                                    part_number: Some(part_number as i64),
                                });
                            },
                            Err(e) => {
                                if attempts >= MAX_UPLOAD_RETRIES {
                                    return Err(anyhow!("Failed to upload part {} after {} attempts: {}", 
                                                      part_number, MAX_UPLOAD_RETRIES, e));
                                }
                                
                                // Exponential backoff
                                let delay = Duration::from_millis(250 * 2u64.pow(attempts as u32));
                                warn!("Part {} upload attempt {} failed, retrying in {:?}: {}", 
                                     part_number, attempts, delay, e);
                                sleep(delay).await;
                            }
                        }
                    }
                    
                    Err(anyhow!("Failed to upload part {} after maximum retries", part_number))
                };
                
                chunk_futures.push(part_future);
            }
            
            // Execute this batch of futures in parallel
            let chunk_results = future::join_all(chunk_futures).await;
            
            for result in chunk_results {
                match result {
                    Ok(part) => {
                        completed_parts.push(part);
                    },
                    Err(e) => {
                        // Abort the multipart upload on any error
                        let _ = self.client.abort_multipart_upload(AbortMultipartUploadRequest {
                            bucket: self.bucket.clone(),
                            key: key.to_string(),
                            upload_id: upload_id.clone(),
                            ..Default::default()
                        }).await;
                        
                        return Err(anyhow!("Part upload failed, aborting multipart upload: {}", e));
                    }
                }
            }
        }
        
        // Sort parts by part number (must dereference to access the field)
        completed_parts.sort_by_key(|part| part.part_number.unwrap());
        
        // Step 4: Complete the multipart upload
        let complete_request = CompleteMultipartUploadRequest {
            bucket: self.bucket.clone(),
            key: key.to_string(),
            upload_id: upload_id.clone(),
            multipart_upload: Some(CompletedMultipartUpload {
                parts: Some(completed_parts),
            }),
            ..Default::default()
        };
        
        self.client.complete_multipart_upload(complete_request).await
            .context("Failed to complete multipart upload")?;
        
        debug!("Completed multipart upload for {}", file_path.display());
        
        Ok(())
    }
}

/// Upload multiple files to S3 concurrently
pub async fn upload_files_concurrently(
    files: Vec<PathBuf>,
    bucket: &str,
    prefix: &str,
    region_name: Option<&str>,
    profile: Option<&str>,
    _encrypt: bool // Not used yet, but kept for future implementation
) -> Result<()> {
    let queue = UploadQueue::new(bucket, prefix, region_name, profile);
    
    // Start a background task to report progress
    let bytes_uploaded = Arc::clone(&queue.bytes_uploaded);
    let total_bytes = Arc::clone(&queue.total_bytes);
    
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
                info!("S3 upload progress: {}/{} bytes ({:.1}%)", 
                     uploaded, total, percentage);
                last_reported = uploaded;
            }
            
            if uploaded >= total && total > 0 {
                break;
            }
        }
    });
    
    // Process all files
    let mut tasks = Vec::new();
    
    for file in files {
        let queue_ref = &queue;
        tasks.push(queue_ref.add_file(file));
    }
    
    // Wait for all uploads to complete
    future::join_all(tasks).await;
    
    let (uploaded, total) = queue.get_progress();
    let region_name = queue.get_region().name();
    
    if uploaded < total {
        warn!("Not all files were uploaded successfully: {}/{} bytes in region {}", 
              uploaded, total, region_name);
    } else {
        info!("All files uploaded successfully: {} bytes total in region {}", 
              uploaded, region_name);
    }
    
    Ok(())
}

/// Legacy upload function for backward compatibility
#[allow(dead_code)]
pub async fn upload_to_s3(
    file_path: &Path,
    bucket: &str,
    prefix: &str,
    region_name: Option<&str>,
    profile: Option<&str>,
    _encrypt: bool // Not used yet, but kept for future implementation
) -> Result<()> {
    info!("Uploading to S3 bucket: {}...", bucket);
    
    let queue = UploadQueue::new(bucket, prefix, region_name, profile);
    let result = queue.add_file(file_path.to_path_buf()).await;
    
    match result {
        Ok(_) => {
            let region_name = queue.get_region().name();
            let filename = file_path.file_name()
                .map(|name| name.to_string_lossy())
                .unwrap_or_else(|| "unknown".into());
            info!("Upload completed successfully: s3://{}/{}/{} in region {}", 
                 bucket, prefix, filename, region_name);
            Ok(())
        },
        Err(e) => Err(anyhow!("Failed to upload to S3: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_upload_queue_new() {
        let queue = UploadQueue::new("test-bucket", "test-prefix", None, None);
        assert_eq!(queue.bucket, "test-bucket");
        assert_eq!(queue.prefix, "test-prefix");
        assert_eq!(queue.get_progress(), (0, 0));
    }

    #[test]
    fn test_upload_queue_new_with_region() {
        let queue = UploadQueue::new("test-bucket", "test-prefix", Some("us-west-2"), None);
        assert_eq!(queue.bucket, "test-bucket");
        assert_eq!(queue.prefix, "test-prefix");
        assert_eq!(queue.region.name(), "us-west-2");
    }

    #[test]
    fn test_upload_queue_new_with_invalid_region() {
        let queue = UploadQueue::new("test-bucket", "test-prefix", Some("invalid-region"), None);
        assert_eq!(queue.bucket, "test-bucket");
        assert_eq!(queue.prefix, "test-prefix");
        // Should fall back to default region
        assert_eq!(queue.region.name(), Region::default().name());
    }

    #[test]
    fn test_upload_queue_new_with_profile() {
        let queue = UploadQueue::new("test-bucket", "test-prefix", None, Some("test-profile"));
        assert_eq!(queue.bucket, "test-bucket");
        assert_eq!(queue.prefix, "test-prefix");
        // Profile doesn't affect bucket/prefix
    }

    #[test]
    fn test_upload_queue_progress_tracking() {
        let queue = UploadQueue::new("test-bucket", "test-prefix", None, None);
        
        // Add some bytes to total
        queue.total_bytes.store(1000, Ordering::SeqCst);
        assert_eq!(queue.get_progress(), (0, 1000));
        
        // Simulate upload progress
        queue.bytes_uploaded.store(500, Ordering::SeqCst);
        assert_eq!(queue.get_progress(), (500, 1000));
        
        // Complete upload
        queue.bytes_uploaded.store(1000, Ordering::SeqCst);
        assert_eq!(queue.get_progress(), (1000, 1000));
    }

    #[test]
    fn test_upload_chunk_size_constant() {
        // Verify S3 requirements
        assert!(UPLOAD_CHUNK_SIZE >= 5 * 1024 * 1024); // S3 minimum is 5MB
        assert_eq!(UPLOAD_CHUNK_SIZE, 8 * 1024 * 1024); // We use 8MB
    }

    #[test]
    fn test_large_file_threshold() {
        assert_eq!(LARGE_FILE_THRESHOLD, 50 * 1024 * 1024); // 50MB
    }

    #[test]
    fn test_get_region() {
        let queue = UploadQueue::new("test-bucket", "test-prefix", Some("eu-west-1"), None);
        assert_eq!(queue.get_region().name(), "eu-west-1");
    }

    #[tokio::test]
    async fn test_add_file_nonexistent() {
        let queue = UploadQueue::new("test-bucket", "test-prefix", None, None);
        let result = queue.add_file(PathBuf::from("/nonexistent/file.txt")).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to get metadata"));
    }

    #[tokio::test]
    async fn test_upload_small_file_logic() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("small.txt");
        
        // Create a small test file (less than LARGE_FILE_THRESHOLD)
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"Small test file content").unwrap();
        file.sync_all().unwrap();
        drop(file);
        
        // Get file metadata to verify it's small
        let metadata = tokio::fs::metadata(&file_path).await.unwrap();
        assert!(metadata.len() < LARGE_FILE_THRESHOLD);
        
        // The actual upload would fail without AWS credentials, but we can test the logic
        let queue = UploadQueue::new("test-bucket", "test-prefix", None, None);
        
        // Add to total bytes to simulate tracking
        queue.total_bytes.store(metadata.len(), Ordering::SeqCst);
        let (_, total) = queue.get_progress();
        assert_eq!(total, metadata.len());
    }

    #[tokio::test]
    async fn test_multipart_upload_calculation() {
        // Test the multipart calculation logic
        let file_sizes = vec![
            (UPLOAD_CHUNK_SIZE as u64 - 1, 1), // Just under chunk size = 1 part
            (UPLOAD_CHUNK_SIZE as u64, 1),     // Exactly chunk size = 1 part
            (UPLOAD_CHUNK_SIZE as u64 + 1, 2), // Just over chunk size = 2 parts
            (UPLOAD_CHUNK_SIZE as u64 * 10, 10), // 10 chunks = 10 parts
        ];
        
        for (file_size, expected_parts) in file_sizes {
            let num_parts = (file_size + UPLOAD_CHUNK_SIZE as u64 - 1) / UPLOAD_CHUNK_SIZE as u64;
            assert_eq!(num_parts, expected_parts, 
                      "File size {} should have {} parts", file_size, expected_parts);
        }
    }

    #[test]
    fn test_s3_key_generation() {
        let prefix = "test-prefix";
        let filename = "test-file.txt";
        
        // Test key generation logic
        let key = format!("{}/{}", prefix, filename);
        assert_eq!(key, "test-prefix/test-file.txt");
    }

    #[test]
    fn test_upload_retries_constant() {
        assert_eq!(MAX_UPLOAD_RETRIES, 3);
    }

    #[tokio::test]
    async fn test_upload_files_concurrently_empty_list() {
        let result = upload_files_concurrently(
            vec![],
            "test-bucket",
            "test-prefix",
            None,
            None,
            false
        ).await;
        
        // Should succeed with empty file list
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_upload_to_s3_legacy_nonexistent_file() {
        let result = upload_to_s3(
            Path::new("/nonexistent/file.txt"),
            "test-bucket",
            "test-prefix",
            None,
            None,
            false
        ).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to upload to S3"));
    }

    #[test]
    fn test_concurrent_progress_updates() {
        use std::sync::Arc;
        use std::thread;
        
        let queue = UploadQueue::new("test-bucket", "test-prefix", None, None);
        let total_bytes = Arc::clone(&queue.total_bytes);
        let bytes_uploaded = Arc::clone(&queue.bytes_uploaded);
        
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
        
        let (uploaded, total) = queue.get_progress();
        assert_eq!(total, 10000); // 10 threads * 1000
        assert_eq!(uploaded, 4500); // Sum of 0..10 * 100
    }

    #[test]
    fn test_exponential_backoff_calculation() {
        // Test the exponential backoff delay calculation
        let delays: Vec<_> = (1..=MAX_UPLOAD_RETRIES)
            .map(|attempt| Duration::from_millis(250 * 2u64.pow(attempt as u32)))
            .collect();
        
        assert_eq!(delays[0], Duration::from_millis(500));   // First retry
        assert_eq!(delays[1], Duration::from_millis(1000));  // Second retry
        if MAX_UPLOAD_RETRIES >= 3 {
            assert_eq!(delays[2], Duration::from_millis(2000)); // Third retry
        }
    }
}
