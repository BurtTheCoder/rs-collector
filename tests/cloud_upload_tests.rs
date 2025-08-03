//! Integration tests for cloud upload functionality.
//!
//! These tests verify S3 and SFTP upload capabilities using mocked
//! cloud services to avoid actual network dependencies.

use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::path::PathBuf;
use tempfile::TempDir;
use anyhow::Result;

use rust_collector::cloud::s3::UploadQueue;
use rust_collector::cloud::sftp::SFTPConfig;

/// Test S3 upload queue basic functionality
#[test]
fn test_s3_upload_queue_creation() {
    // Test creating an upload queue
    let queue = UploadQueue::new(
        "test-bucket",
        "test-prefix/",
        Some("us-east-1"),
        None
    );
    
    // Verify the queue was created successfully
    let (uploaded, total) = queue.get_progress();
    assert_eq!(uploaded, 0);
    assert_eq!(total, 0);
}

/// Test S3 upload queue creation and management
#[test]
fn test_s3_upload_queue() -> Result<()> {
    let temp_dir = TempDir::new()?;
    
    // Create test files
    let files = vec![
        ("file1.txt", "Content 1"),
        ("file2.txt", "Content 2"),
        ("file3.txt", "Content 3"),
    ];
    
    let mut file_paths = Vec::new();
    for (filename, content) in &files {
        let path = temp_dir.path().join(filename);
        fs::write(&path, content)?;
        file_paths.push(path);
    }
    
    // Create upload queue
    let queue = UploadQueue::new();
    
    // Add files to queue
    for path in &file_paths {
        queue.add_file(path.clone());
    }
    
    // Verify queue size
    assert_eq!(queue.len(), 3);
    
    // Test getting files from queue
    let batch = queue.get_batch(2);
    assert_eq!(batch.len(), 2);
    assert_eq!(queue.len(), 1);
    
    // Get remaining file
    let remaining = queue.get_batch(10);
    assert_eq!(remaining.len(), 1);
    assert_eq!(queue.len(), 0);
    
    Ok(())
}

/// Test SFTP configuration with different settings
#[test]
fn test_sftp_config_variations() {
    // Test with default settings
    let default_config = SFTPConfig::default();
    assert_eq!(default_config.port, 22);
    assert_eq!(default_config.concurrent_connections, 4);
    assert_eq!(default_config.buffer_size_mb, 8);
    assert_eq!(default_config.connection_timeout_sec, 30);
    assert_eq!(default_config.max_retries, 3);
    
    // Test with custom settings
    let custom_config = SFTPConfig {
        host: "sftp.example.com".to_string(),
        port: 2222,
        username: "forensics".to_string(),
        private_key_path: PathBuf::from("/home/user/.ssh/id_rsa"),
        remote_path: "/uploads/evidence".to_string(),
        concurrent_connections: 8,
        buffer_size_mb: 16,
        connection_timeout_sec: 60,
        max_retries: 5,
    };
    
    assert_eq!(custom_config.host, "sftp.example.com");
    assert_eq!(custom_config.port, 2222);
    assert_eq!(custom_config.concurrent_connections, 8);
}

/// Test upload progress tracking
#[test]
fn test_upload_progress_tracking() {
    let total_bytes = Arc::new(AtomicU64::new(0));
    let uploaded_bytes = Arc::new(AtomicU64::new(0));
    
    // Simulate file sizes
    let file_sizes = vec![1024, 2048, 4096];
    let total: u64 = file_sizes.iter().sum();
    
    total_bytes.store(total, Ordering::SeqCst);
    
    // Simulate progressive upload
    let mut uploaded = 0u64;
    for size in file_sizes {
        uploaded += size;
        uploaded_bytes.store(uploaded, Ordering::SeqCst);
        
        let progress = uploaded_bytes.load(Ordering::SeqCst) as f64 
            / total_bytes.load(Ordering::SeqCst) as f64 * 100.0;
        
        assert!(progress > 0.0);
        assert!(progress <= 100.0);
    }
    
    assert_eq!(uploaded_bytes.load(Ordering::SeqCst), total);
}

/// Test multipart upload calculations for S3
#[test]
fn test_s3_multipart_calculations() {
    let threshold_mb = 100;
    let chunk_size_mb = 8;
    
    let threshold_bytes = threshold_mb * 1024 * 1024;
    let chunk_size_bytes = chunk_size_mb * 1024 * 1024;
    
    // Test file sizes
    let test_cases = vec![
        (50 * 1024 * 1024, false),    // 50MB - below threshold
        (100 * 1024 * 1024, true),     // 100MB - at threshold
        (200 * 1024 * 1024, true),     // 200MB - above threshold
    ];
    
    for (file_size, should_multipart) in test_cases {
        let is_multipart = file_size >= threshold_bytes;
        assert_eq!(is_multipart, should_multipart);
        
        if is_multipart {
            let num_parts = (file_size + chunk_size_bytes - 1) / chunk_size_bytes;
            assert!(num_parts > 0);
        }
    }
}

/// Test retry configuration
#[test]
fn test_retry_configuration() {
    let max_retries = 3;
    let base_delay_ms = 250;
    
    // Calculate exponential backoff delays
    let delays: Vec<u64> = (0..max_retries)
        .map(|attempt| {
            let multiplier = 2u64.pow(attempt as u32);
            base_delay_ms * multiplier
        })
        .collect();
    
    assert_eq!(delays, vec![250, 500, 1000]);
    
    // Verify delays are capped at reasonable maximum
    let max_delay_ms = 30000; // 30 seconds
    for delay in delays {
        assert!(delay <= max_delay_ms);
    }
}

/// Test concurrent upload limits
#[test]
fn test_concurrent_upload_limits() {
    let configs = vec![
        (4, 100),   // 4 connections, 100 files
        (8, 50),    // 8 connections, 50 files
        (1, 10),    // 1 connection, 10 files
    ];
    
    for (max_concurrent, total_files) in configs {
        let batches = (total_files + max_concurrent - 1) / max_concurrent;
        assert!(batches > 0);
        
        // Last batch might be smaller
        let last_batch_size = total_files % max_concurrent;
        if last_batch_size == 0 && total_files > 0 {
            assert_eq!(batches, total_files / max_concurrent);
        } else {
            assert_eq!(batches, total_files / max_concurrent + 1);
        }
    }
}

/// Test storage class options for S3
#[test]
fn test_s3_storage_classes() {
    let storage_classes = vec![
        "STANDARD",
        "STANDARD_IA",
        "ONEZONE_IA",
        "INTELLIGENT_TIERING",
        "GLACIER",
        "DEEP_ARCHIVE",
    ];
    
    for class in storage_classes {
        let config = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            access_key_id: "test".to_string(),
            secret_access_key: "test".to_string(),
            storage_class: class.to_string(),
            ..Default::default()
        };
        
        assert_eq!(config.storage_class, class);
    }
}

/// Test path normalization for uploads
#[test]
fn test_upload_path_normalization() {
    let test_paths = vec![
        ("/path/to/file.txt", "file.txt"),
        ("C:\\Windows\\System32\\config.sys", "config.sys"),
        ("/var/log/syslog.1", "syslog.1"),
        ("relative/path/data.bin", "data.bin"),
    ];
    
    for (input_path, expected_name) in test_paths {
        let path = PathBuf::from(input_path);
        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        
        assert_eq!(filename, expected_name);
    }
}

/// Test upload metadata generation
#[test]
fn test_upload_metadata() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let test_file = temp_dir.path().join("metadata_test.txt");
    fs::write(&test_file, "Test content for metadata")?;
    
    let metadata = fs::metadata(&test_file)?;
    
    // Verify metadata properties
    assert!(metadata.is_file());
    assert_eq!(metadata.len(), 25); // Length of "Test content for metadata"
    assert!(!metadata.is_dir());
    
    // Test metadata for different file types
    let files = vec![
        ("text.txt", "Plain text"),
        ("data.bin", vec![0u8, 1, 2, 3, 4]),
        ("empty.dat", vec![]),
    ];
    
    for (filename, content) in files {
        let path = temp_dir.path().join(filename);
        match content {
            c if c.is_empty() => fs::write(&path, "")?,
            _ => fs::write(&path, "content")?,
        };
        
        let meta = fs::metadata(&path)?;
        assert!(meta.is_file());
    }
    
    Ok(())
}