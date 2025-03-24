use std::path::Path;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use log::{error, warn};
use rusoto_s3::{S3Client, S3, AbortMultipartUploadRequest};

use crate::cloud::streaming::S3UploadStream;
use crate::collectors::streaming::core;

/// Stream artifacts directly to S3 using multipart upload.
///
/// This function:
/// 1. Creates a streaming S3 upload with the specified buffer size
/// 2. Delegates to the core streaming implementation
/// 3. Handles S3-specific error cases
///
/// # Arguments
///
/// * `source_dir` - Path to the directory containing artifacts to stream
/// * `client` - S3 client for AWS operations
/// * `bucket` - S3 bucket name
/// * `key` - S3 object key (path)
/// * `buffer_size_mb` - Buffer size in megabytes for streaming operations
///
/// # Returns
///
/// Ok(()) if the upload was successful, or an error with context
pub async fn stream_artifacts_to_s3(
    source_dir: &Path,
    client: Arc<S3Client>,
    bucket: &str,
    key: &str,
    buffer_size_mb: usize,
) -> Result<()> {
    // Create S3 upload stream
    let s3_stream = match S3UploadStream::new(client.clone(), bucket, key, buffer_size_mb).await {
        Ok(stream) => stream,
        Err(e) => {
            error!("Failed to create S3 upload stream: {}", e);
            return Err(e);
        }
    };
    
    // Keep a clone of the client, bucket, and key for potential abort operations
    let client_for_abort = client.clone();
    let bucket_for_abort = bucket.to_string();
    let key_for_abort = key.to_string();
    let upload_id = s3_stream.upload_id.clone();
    
    // Stream artifacts using the core implementation
    match core::stream_directory_to_target(source_dir, s3_stream, buffer_size_mb).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Failed to stream artifacts to S3: {}", e);
            
            // Try to abort the upload to clean up
            warn!("Attempting to abort the failed upload...");
            
            // Create abort request
            let abort_request = AbortMultipartUploadRequest {
                bucket: bucket_for_abort,
                key: key_for_abort,
                upload_id,
                ..Default::default()
            };
            
            // Attempt to abort the upload
            if let Err(abort_err) = client_for_abort.abort_multipart_upload(abort_request).await {
                warn!("Failed to abort upload: {}", abort_err);
            } else {
                warn!("Successfully aborted the failed upload");
            }
            
            Err(anyhow!("Failed to stream artifacts to S3: {}", e))
        }
    }
}

/// Stream a single file to S3 using multipart upload.
///
/// Similar to `stream_artifacts_to_s3` but for a single file instead of a directory.
///
/// # Arguments
///
/// * `file_path` - Path to the file to stream
/// * `client` - S3 client for AWS operations
/// * `bucket` - S3 bucket name
/// * `key` - S3 object key (path)
/// * `buffer_size_mb` - Buffer size in megabytes for streaming operations
///
/// # Returns
///
/// Ok(()) if the upload was successful, or an error with context
pub async fn stream_file_to_s3(
    file_path: &Path,
    client: Arc<S3Client>,
    bucket: &str,
    key: &str,
    buffer_size_mb: usize,
) -> Result<()> {
    // Create S3 upload stream
    let s3_stream = match S3UploadStream::new(client.clone(), bucket, key, buffer_size_mb).await {
        Ok(stream) => stream,
        Err(e) => {
            error!("Failed to create S3 upload stream: {}", e);
            return Err(e);
        }
    };
    
    // Keep a clone of the client, bucket, and key for potential abort operations
    let client_for_abort = client.clone();
    let bucket_for_abort = bucket.to_string();
    let key_for_abort = key.to_string();
    let upload_id = s3_stream.upload_id.clone();
    
    // Stream file using the core implementation
    match core::stream_file_to_target(file_path, s3_stream, buffer_size_mb).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Failed to stream file to S3: {}", e);
            
            // Try to abort the upload to clean up
            warn!("Attempting to abort the failed upload...");
            
            // Create abort request
            let abort_request = AbortMultipartUploadRequest {
                bucket: bucket_for_abort,
                key: key_for_abort,
                upload_id,
                ..Default::default()
            };
            
            // Attempt to abort the upload
            if let Err(abort_err) = client_for_abort.abort_multipart_upload(abort_request).await {
                warn!("Failed to abort upload: {}", abort_err);
            } else {
                warn!("Successfully aborted the failed upload");
            }
            
            Err(anyhow!("Failed to stream file to S3: {}", e))
        }
    }
}
