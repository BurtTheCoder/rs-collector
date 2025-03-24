use std::path::Path;

use anyhow::{Result, anyhow};
use log::{error, warn};

use crate::cloud::sftp::SFTPConfig;
use crate::cloud::sftp_streaming::create_sftp_upload_stream;
use crate::collectors::streaming::core;

/// Stream artifacts directly to SFTP server.
///
/// This function:
/// 1. Creates a streaming SFTP upload with the specified buffer size
/// 2. Delegates to the core streaming implementation
/// 3. Handles SFTP-specific error cases
///
/// # Arguments
///
/// * `source_dir` - Path to the directory containing artifacts to stream
/// * `config` - SFTP configuration
/// * `remote_path` - Remote file path on the SFTP server
/// * `buffer_size_mb` - Buffer size in megabytes for streaming operations
///
/// # Returns
///
/// Ok(()) if the upload was successful, or an error with context
pub async fn stream_artifacts_to_sftp(
    source_dir: &Path,
    config: SFTPConfig,
    remote_path: &str,
    buffer_size_mb: usize,
) -> Result<()> {
    // Create SFTP upload stream
    let sftp_stream = match create_sftp_upload_stream(config.clone(), remote_path, buffer_size_mb).await {
        Ok(stream) => stream,
        Err(e) => {
            error!("Failed to create SFTP upload stream: {}", e);
            return Err(e);
        }
    };
    
    // Keep the remote path for potential cleanup
    let remote_path_for_cleanup = remote_path.to_string();
    let config_for_cleanup = config.clone();
    
    // Stream artifacts using the core implementation
    match core::stream_directory_to_target(source_dir, sftp_stream, buffer_size_mb).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Failed to stream artifacts to SFTP: {}", e);
            
            // Try to clean up the remote file manually
            warn!("Attempting to clean up the failed upload...");
            
            // Create a new SFTP connection to delete the file
            if let Ok(cleanup_stream) = create_sftp_upload_stream(config_for_cleanup, &remote_path_for_cleanup, buffer_size_mb).await {
                if let Err(abort_err) = cleanup_stream.abort().await {
                    warn!("Failed to clean up remote file: {}", abort_err);
                } else {
                    warn!("Successfully cleaned up the failed upload");
                }
            } else {
                warn!("Failed to create SFTP connection for cleanup");
            }
            
            Err(anyhow!("Failed to stream artifacts to SFTP: {}", e))
        }
    }
}

/// Stream a single file to SFTP server.
///
/// Similar to `stream_artifacts_to_sftp` but for a single file instead of a directory.
///
/// # Arguments
///
/// * `file_path` - Path to the file to stream
/// * `config` - SFTP configuration
/// * `remote_path` - Remote file path on the SFTP server
/// * `buffer_size_mb` - Buffer size in megabytes for streaming operations
///
/// # Returns
///
/// Ok(()) if the upload was successful, or an error with context
pub async fn stream_file_to_sftp(
    file_path: &Path,
    config: SFTPConfig,
    remote_path: &str,
    buffer_size_mb: usize,
) -> Result<()> {
    // Create SFTP upload stream
    let sftp_stream = match create_sftp_upload_stream(config.clone(), remote_path, buffer_size_mb).await {
        Ok(stream) => stream,
        Err(e) => {
            error!("Failed to create SFTP upload stream: {}", e);
            return Err(e);
        }
    };
    
    // Keep the remote path for potential cleanup
    let remote_path_for_cleanup = remote_path.to_string();
    let config_for_cleanup = config.clone();
    
    // Stream file using the core implementation
    match core::stream_file_to_target(file_path, sftp_stream, buffer_size_mb).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Failed to stream file to SFTP: {}", e);
            
            // Try to clean up the remote file manually
            warn!("Attempting to clean up the failed upload...");
            
            // Create a new SFTP connection to delete the file
            if let Ok(cleanup_stream) = create_sftp_upload_stream(config_for_cleanup, &remote_path_for_cleanup, buffer_size_mb).await {
                if let Err(abort_err) = cleanup_stream.abort().await {
                    warn!("Failed to clean up remote file: {}", abort_err);
                } else {
                    warn!("Successfully cleaned up the failed upload");
                }
            } else {
                warn!("Failed to create SFTP connection for cleanup");
            }
            
            Err(anyhow!("Failed to stream file to SFTP: {}", e))
        }
    }
}
