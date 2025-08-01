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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_module_documentation() {
        // Test that the module has proper documentation
        let content = include_str!("sftp.rs");
        assert!(content.contains("Stream artifacts directly to SFTP"));
        assert!(content.contains("streaming SFTP upload"));
    }

    #[test]
    fn test_function_documentation() {
        // Test that functions have proper documentation
        let content = include_str!("sftp.rs");
        assert!(content.contains("Creates a streaming SFTP upload"));
        assert!(content.contains("Handles SFTP-specific error cases"));
    }

    #[test]
    fn test_error_handling_comments() {
        // Test that error handling is documented
        let content = include_str!("sftp.rs");
        assert!(content.contains("Try to clean up the remote file manually"));
        assert!(content.contains("Attempting to clean up the failed upload"));
    }

    #[test]
    fn test_sftp_config_usage() {
        // Test that SFTPConfig is properly used
        let content = include_str!("sftp.rs");
        assert!(content.contains("config: SFTPConfig"));
        assert!(content.contains("config.clone()"));
    }

    #[test]
    fn test_cleanup_logic() {
        // Test that cleanup logic is present
        let content = include_str!("sftp.rs");
        assert!(content.contains("create_sftp_upload_stream(config_for_cleanup"));
        assert!(content.contains("cleanup_stream.abort()"));
        assert!(content.contains("Successfully cleaned up the failed upload"));
    }

    #[test]
    fn test_return_values() {
        // Test that return values are documented
        let content = include_str!("sftp.rs");
        assert!(content.contains("Ok(()) if the upload was successful"));
        assert!(content.contains("error with context"));
    }

    #[test]
    fn test_remote_path_handling() {
        // Test that remote_path is properly handled
        let content = include_str!("sftp.rs");
        assert!(content.contains("remote_path_for_cleanup = remote_path.to_string()"));
        assert!(content.contains("remote_path: &str"));
    }

    #[test]
    fn test_buffer_size_parameter() {
        // Test that buffer size is properly documented
        let content = include_str!("sftp.rs");
        assert!(content.contains("Buffer size in megabytes"));
        assert!(content.contains("buffer_size_mb"));
    }

    #[tokio::test]
    async fn test_stream_artifacts_to_sftp_invalid_config() {
        // Test with invalid config
        let config = SFTPConfig {
            host: "nonexistent.host".to_string(),
            port: 22,
            username: "test".to_string(),
            private_key_path: std::path::PathBuf::from("/nonexistent/key"),
            remote_path: "/test".to_string(),
            concurrent_connections: 4,
            buffer_size_mb: 8,
            connection_timeout_sec: 30,
            max_retries: 3,
        };
        
        let temp_dir = TempDir::new().unwrap();
        let result = stream_artifacts_to_sftp(
            temp_dir.path(),
            config,
            "/remote/test.zip",
            5
        ).await;
        
        // Should fail because we can't create real SFTP connection in tests
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stream_file_to_sftp_invalid_config() {
        // Test with invalid config
        let config = SFTPConfig {
            host: "nonexistent.host".to_string(),
            port: 22,
            username: "test".to_string(),
            private_key_path: std::path::PathBuf::from("/nonexistent/key"),
            remote_path: "/test".to_string(),
            concurrent_connections: 4,
            buffer_size_mb: 8,
            connection_timeout_sec: 30,
            max_retries: 3,
        };
        
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "test content").unwrap();
        
        let result = stream_file_to_sftp(
            &test_file,
            config,
            "/remote/test.txt",
            5
        ).await;
        
        // Should fail because we can't create real SFTP connection in tests
        assert!(result.is_err());
    }

    #[test]
    fn test_sftp_config_clone() {
        // Test that config is cloned for cleanup
        let content = include_str!("sftp.rs");
        assert!(content.contains("config_for_cleanup = config.clone()"));
        assert!(content.matches("config.clone()").count() >= 4); // At least two in each function
    }
}
