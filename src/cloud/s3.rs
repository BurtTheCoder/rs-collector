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

// Constants for S3 uploads
const UPLOAD_CHUNK_SIZE: usize = 8 * 1024 * 1024; // 8MB (S3 minimum is 5MB)
const MAX_UPLOAD_RETRIES: usize = 3;
const LARGE_FILE_THRESHOLD: u64 = 50 * 1024 * 1024; // 50MB - use multipart for larger files

/// Async file queue for concurrent uploads
pub struct UploadQueue {
    bucket: String,
    prefix: String,
    region: Region,
    client: Arc<S3Client>,
    total_bytes: Arc<AtomicU64>,
    bytes_uploaded: Arc<AtomicU64>,
}

impl UploadQueue {
    /// Create a new upload queue
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
                    Arc::new(S3Client::new_with(
                        rusoto_core::HttpClient::new().expect("Failed to create HTTP client"),
                        provider,
                        region.clone()
                    ))
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
        let key = format!(
            "{}/{}",
            self.prefix,
            file_path.file_name().unwrap().to_string_lossy()
        );
        
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
    
    /// Get upload progress
    pub fn get_progress(&self) -> (u64, u64) {
        (
            self.bytes_uploaded.load(Ordering::SeqCst),
            self.total_bytes.load(Ordering::SeqCst)
        )
    }
    
    /// Get the region being used for uploads
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
            info!("Upload completed successfully: s3://{}/{}/{} in region {}", 
                 bucket, prefix, file_path.file_name().unwrap().to_string_lossy(), region_name);
            Ok(())
        },
        Err(e) => Err(anyhow!("Failed to upload to S3: {}", e))
    }
}
