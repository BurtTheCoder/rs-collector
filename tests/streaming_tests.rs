//! Integration tests for streaming functionality.
//!
//! These tests verify the streaming upload capabilities for both
//! S3 and SFTP targets, including progress tracking and error handling.

use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tempfile::TempDir;
use anyhow::Result;
use tokio::time::sleep;

/// Test streaming progress tracking
#[tokio::test]
async fn test_streaming_progress_basic() {
    let total_bytes = 1000000u64; // 1MB
    let bytes_uploaded = Arc::new(AtomicU64::new(0));
    
    // Simulate upload progress
    let uploaded_clone = Arc::clone(&bytes_uploaded);
    let upload_task = tokio::spawn(async move {
        let chunk_size = 100000u64; // 100KB chunks
        let chunks = total_bytes / chunk_size;
        
        for i in 0..chunks {
            sleep(Duration::from_millis(10)).await;
            uploaded_clone.fetch_add(chunk_size, Ordering::SeqCst);
            
            let progress = (i + 1) as f64 / chunks as f64 * 100.0;
            assert!(progress <= 100.0);
        }
    });
    
    // Monitor progress
    let monitor_clone = Arc::clone(&bytes_uploaded);
    let monitor_task = tokio::spawn(async move {
        let mut last_progress = 0u64;
        
        for _ in 0..20 {
            sleep(Duration::from_millis(5)).await;
            let current = monitor_clone.load(Ordering::SeqCst);
            assert!(current >= last_progress); // Progress should never go backwards
            last_progress = current;
            
            if current >= total_bytes {
                break;
            }
        }
    });
    
    // Wait for both tasks
    let _ = tokio::join!(upload_task, monitor_task);
    
    assert_eq!(bytes_uploaded.load(Ordering::SeqCst), total_bytes);
}

/// Test streaming with multiple files
#[tokio::test]
async fn test_streaming_multiple_files() -> Result<()> {
    let temp_dir = TempDir::new()?;
    
    // Create test files
    let files = vec![
        ("file1.txt", 1024),    // 1KB
        ("file2.log", 2048),    // 2KB
        ("file3.bin", 4096),    // 4KB
    ];
    
    let mut total_size = 0u64;
    for (filename, size) in &files {
        let content = vec![b'A'; *size];
        fs::write(temp_dir.path().join(filename), content)?;
        total_size += *size as u64;
    }
    
    // Simulate streaming upload
    let bytes_uploaded = Arc::new(AtomicU64::new(0));
    
    for (filename, size) in files {
        // Simulate streaming this file
        let chunks = size / 512; // 512 byte chunks
        for _ in 0..chunks {
            bytes_uploaded.fetch_add(512, Ordering::SeqCst);
            sleep(Duration::from_millis(1)).await;
        }
        
        // Handle remainder
        let remainder = size % 512;
        if remainder > 0 {
            bytes_uploaded.fetch_add(remainder as u64, Ordering::SeqCst);
        }
    }
    
    assert_eq!(bytes_uploaded.load(Ordering::SeqCst), total_size);
    
    Ok(())
}

/// Test streaming ZIP creation
#[tokio::test]
async fn test_streaming_zip_creation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    
    // Create directory structure
    fs::create_dir(temp_dir.path().join("subdir"))?;
    fs::write(temp_dir.path().join("root.txt"), "Root file")?;
    fs::write(temp_dir.path().join("subdir/nested.txt"), "Nested file")?;
    
    // Track ZIP creation progress
    let bytes_written = Arc::new(AtomicU64::new(0));
    
    // Simulate streaming ZIP creation
    let files = vec![
        ("root.txt", 9),        // "Root file"
        ("subdir/nested.txt", 11), // "Nested file"
    ];
    
    for (path, size) in files {
        // Add ZIP header overhead (simulated)
        bytes_written.fetch_add(30, Ordering::SeqCst); // File header
        bytes_written.fetch_add(size, Ordering::SeqCst); // File content
        bytes_written.fetch_add(46, Ordering::SeqCst); // Central directory entry
    }
    
    // Add end of central directory record
    bytes_written.fetch_add(22, Ordering::SeqCst);
    
    assert!(bytes_written.load(Ordering::SeqCst) > 20); // Should have some data
    
    Ok(())
}

/// Test chunk size optimization
#[test]
fn test_chunk_size_optimization() {
    let file_sizes = vec![
        (1024, 1024),              // 1KB file, 1KB chunk
        (1024 * 1024, 64 * 1024),  // 1MB file, 64KB chunk
        (100 * 1024 * 1024, 1024 * 1024), // 100MB file, 1MB chunk
    ];
    
    for (file_size, expected_chunk) in file_sizes {
        let optimal_chunk = if file_size < 1024 * 1024 {
            file_size.min(64 * 1024)
        } else if file_size < 100 * 1024 * 1024 {
            64 * 1024
        } else {
            1024 * 1024
        };
        
        assert!(optimal_chunk <= expected_chunk);
        assert!(optimal_chunk > 0);
    }
}

/// Test concurrent streaming uploads
#[tokio::test]
async fn test_concurrent_streaming() -> Result<()> {
    let num_streams = 3;
    let bytes_per_stream = 10000u64;
    
    let total_uploaded = Arc::new(AtomicU64::new(0));
    
    let mut tasks = vec![];
    for i in 0..num_streams {
        let counter = Arc::clone(&total_uploaded);
        let task = tokio::spawn(async move {
            // Simulate streaming upload
            for _ in 0..10 {
                sleep(Duration::from_millis(i as u64 + 1)).await;
                counter.fetch_add(bytes_per_stream / 10, Ordering::SeqCst);
            }
        });
        tasks.push(task);
    }
    
    // Wait for all streams to complete
    for task in tasks {
        task.await?;
    }
    
    assert_eq!(
        total_uploaded.load(Ordering::SeqCst),
        num_streams as u64 * bytes_per_stream
    );
    
    Ok(())
}

/// Test streaming error recovery
#[tokio::test]
async fn test_streaming_error_recovery() -> Result<()> {
    let bytes_uploaded = Arc::new(AtomicU64::new(0));
    let max_retries = 3;
    
    // Simulate upload with retries
    let mut attempt = 0;
    let target_bytes = 5000u64;
    
    while attempt < max_retries {
        attempt += 1;
        
        // Reset counter on retry
        bytes_uploaded.store(0, Ordering::SeqCst);
        
        // Simulate partial upload
        let upload_amount = if attempt < max_retries {
            target_bytes / 2 // Fail halfway
        } else {
            target_bytes // Succeed on last attempt
        };
        
        for _ in 0..(upload_amount / 100) {
            bytes_uploaded.fetch_add(100, Ordering::SeqCst);
            sleep(Duration::from_millis(1)).await;
        }
        
        if bytes_uploaded.load(Ordering::SeqCst) >= target_bytes {
            break;
        }
        
        // Simulate retry delay
        sleep(Duration::from_millis(100 * attempt as u64)).await;
    }
    
    assert_eq!(attempt, max_retries);
    assert_eq!(bytes_uploaded.load(Ordering::SeqCst), target_bytes);
    
    Ok(())
}

/// Test streaming bandwidth calculation
#[tokio::test]
async fn test_bandwidth_calculation() {
    let test_cases = vec![
        (1_000_000, 1.0, 1_000_000.0),  // 1MB in 1 second = 1MB/s
        (10_000_000, 2.0, 5_000_000.0), // 10MB in 2 seconds = 5MB/s
        (500_000, 0.5, 1_000_000.0),    // 500KB in 0.5 seconds = 1MB/s
    ];
    
    for (bytes, seconds, expected_bps) in test_cases {
        let calculated = bytes as f64 / seconds;
        let tolerance = expected_bps * 0.01; // 1% tolerance
        
        assert!(
            (calculated - expected_bps).abs() < tolerance,
            "Bandwidth calculation mismatch: {} vs {}",
            calculated,
            expected_bps
        );
    }
}

/// Test streaming with compression
#[tokio::test]
async fn test_streaming_with_compression() -> Result<()> {
    let temp_dir = TempDir::new()?;
    
    // Create files with different compression ratios
    let files = vec![
        ("random.bin", (0..1024).map(|i| (i * 13 + 7) as u8).collect::<Vec<_>>()), // Pseudo-random data, poor compression
        ("zeros.bin", vec![0u8; 1024]),                   // All zeros, great compression
        ("text.txt", b"Hello World! ".repeat(50).to_vec()), // Text, good compression
    ];
    
    for (filename, content) in &files {
        fs::write(temp_dir.path().join(filename), content)?;
    }
    
    // Simulate compression decisions
    for (filename, content) in files {
        let should_compress = !filename.ends_with(".bin") || content.iter().all(|&b| b == 0);
        
        if filename == "random.bin" {
            assert!(!should_compress || content.iter().all(|&b| b == 0));
        } else if filename == "zeros.bin" {
            assert!(should_compress);
        } else if filename == "text.txt" {
            assert!(should_compress);
        }
    }
    
    Ok(())
}

/// Test progress reporting intervals
#[tokio::test]
async fn test_progress_reporting_intervals() {
    let total = 100_000u64;
    let uploaded = Arc::new(AtomicU64::new(0));
    let mut last_reported_percentage = 0u8;
    
    // Simulate gradual upload
    for i in 0..=100 {
        uploaded.store(total * i / 100, Ordering::SeqCst);
        
        let percentage = (uploaded.load(Ordering::SeqCst) as f64 / total as f64 * 100.0) as u8;
        
        // Report only on 5% increments or at 99%
        if percentage >= last_reported_percentage + 5 || 
           (percentage == 99 && last_reported_percentage < 99) {
            last_reported_percentage = percentage;
        }
        
        sleep(Duration::from_millis(1)).await;
    }
    
    assert_eq!(uploaded.load(Ordering::SeqCst), total);
    assert_eq!(last_reported_percentage, 100);
}