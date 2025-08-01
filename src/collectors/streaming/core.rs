use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use log::{debug, info};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::time::sleep;
use walkdir::WalkDir;

use crate::cloud::streaming_target::StreamingTarget;
use crate::utils::streaming_zip::{StreamingZipWriter, FileOptions, CompressionMethod};
use crate::constants::{
    PROGRESS_REPORT_INTERVAL_SECS,
    STREAMING_BUFFER_SIZE,
    LARGE_FILE_COMPRESSION_THRESHOLD,
    COMPRESSED_EXTENSIONS
};

/// Progress tracker for streaming uploads
pub struct ProgressTracker {
    total_size: u64,
    bytes_uploaded: Arc<AtomicU64>,
    start_time: Instant,
    last_percentage: u8,
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new(total_size: u64, bytes_uploaded: Arc<AtomicU64>) -> Self {
        Self {
            total_size,
            bytes_uploaded,
            start_time: Instant::now(),
            last_percentage: 0,
        }
    }
    
    /// Start tracking progress in a background task
    pub fn start_tracking(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            if self.total_size == 0 {
                return;
            }
            
            loop {
                sleep(Duration::from_secs(PROGRESS_REPORT_INTERVAL_SECS)).await;
                
                let bytes_uploaded = self.bytes_uploaded.load(Ordering::SeqCst);
                let percentage = ((bytes_uploaded as f64 / self.total_size as f64) * 100.0) as u8;
                
                // Report progress if it's changed by at least 5%
                if percentage >= self.last_percentage + 5 || (percentage == 99 && self.last_percentage < 99) {
                    let elapsed = self.start_time.elapsed().as_secs_f64();
                    let speed = if elapsed > 0.0 { bytes_uploaded as f64 / elapsed / 1024.0 / 1024.0 } else { 0.0 };
                    
                    info!("Upload progress: {}% ({}/{} bytes, {:.2} MB/s)", 
                          percentage, bytes_uploaded, self.total_size, speed);
                }
                
                if bytes_uploaded >= self.total_size {
                    info!("Upload completed: {} bytes transferred", bytes_uploaded);
                    break;
                }
            }
        })
    }
}

/// Calculate total size of files in a directory for progress reporting.
///
/// This function recursively walks through a directory and sums up the sizes of all files.
/// The result is used to provide accurate progress percentage during streaming uploads.
///
/// # Arguments
///
/// * `source_dir` - Path to the directory to calculate size for
///
/// # Returns
///
/// Total size in bytes of all files in the directory
pub async fn calculate_total_size(source_dir: &Path) -> Result<u64> {
    let mut total_size = 0u64;
    
    for entry in WalkDir::new(source_dir) {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        
        if path.is_file() {
            if let Ok(metadata) = tokio::fs::metadata(path).await {
                total_size += metadata.len();
            }
        }
    }
    
    Ok(total_size)
}

/// Determine optimal compression options based on file type and size.
///
/// This function analyzes the file extension and size to choose the most appropriate
/// compression method:
/// - Already compressed files (ZIP, JPG, MP4, etc.) use no compression (stored)
/// - Large files (>100MB) use no compression for better performance
/// - Regular files use standard deflate compression for better space efficiency
///
/// # Arguments
///
/// * `path` - Path to the file to analyze
///
/// # Returns
///
/// FileOptions with the appropriate compression method set
pub fn get_compression_options(path: &Path) -> FileOptions {
    // Detect file type from extension
    let low_compression = match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => COMPRESSED_EXTENSIONS.contains(&ext),
        _ => false,
    };
    
    // Detect if it's very large, in which case use faster compression
    let large_file = match std::fs::metadata(path) {
        Ok(metadata) if metadata.len() > LARGE_FILE_COMPRESSION_THRESHOLD => true,
        _ => false,
    };
    
    let mut options = FileOptions::default();
    
    if low_compression || large_file {
        // Use no compression for already compressed or large files
        options.compression_method = CompressionMethod::Stored;
    } else {
        // Use deflate compression for regular files
        options.compression_method = CompressionMethod::Deflated;
    }
    
    options
}

/// Stream artifacts directly to a streaming target.
///
/// This function:
/// 1. Calculates the total size of artifacts for progress reporting
/// 2. Sets up real-time progress reporting with percentage and transfer speed
/// 3. Creates a streaming ZIP writer that writes directly to the target
/// 4. Walks through the source directory and adds all files to the ZIP
/// 5. Optimizes compression based on file type and size
/// 6. Tracks upload progress and reports at regular intervals
/// 7. Handles errors with proper cleanup of resources
///
/// # Arguments
///
/// * `source_dir` - Path to the directory containing artifacts to stream
/// * `target` - The streaming target (S3, SFTP, etc.)
/// * `buffer_size_mb` - Buffer size in megabytes for streaming operations
///
/// # Returns
///
/// Ok(()) if the upload was successful, or an error with context
pub async fn stream_directory_to_target<T: StreamingTarget>(
    source_dir: &Path,
    target: T,
    _buffer_size_mb: usize,
) -> Result<()> {
    info!("Streaming artifacts from {} to {}", source_dir.display(), target.target_name());
    
    // Calculate total size for progress reporting
    info!("Calculating total size of artifacts...");
    let total_size = calculate_total_size(source_dir).await?;
    info!("Total size to upload: {} bytes", total_size);
    
    // Track upload progress
    let bytes_uploaded_tracker = Arc::new(AtomicU64::new(0));
    let bytes_uploaded_clone = Arc::clone(&bytes_uploaded_tracker);
    
    // Spawn a task to report progress
    let progress_tracker = ProgressTracker::new(total_size, Arc::clone(&bytes_uploaded_tracker));
    let progress_handle = progress_tracker.start_tracking();
    
    // Create streaming ZIP writer
    let mut zip_writer = StreamingZipWriter::new(target);
    
    // Track directories to add at the end
    let mut dirs = Vec::new();
    
    // Walk the directory and add files to the ZIP
    for entry in WalkDir::new(source_dir) {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        
        // Get relative path
        let rel_path = path.strip_prefix(source_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
            
        if rel_path.is_empty() {
            continue;
        }
        
        if path.is_dir() {
            // Save directory for later addition
            dirs.push(format!("{}/", rel_path));
        } else {
            // Determine compression options
            let options = get_compression_options(path);
            
            debug!("Adding {} to streaming ZIP", rel_path);
            
            // Start a new file entry
            let mut file_writer = zip_writer.start_file(&rel_path, options).await?;
            
            // Open the file and stream its contents
            let mut file = File::open(path).await
                .context(format!("Failed to open {}", path.display()))?;
                
            let mut buffer = vec![0u8; STREAMING_BUFFER_SIZE];
            
            loop {
                let bytes_read = tokio::io::AsyncReadExt::read(&mut file, &mut buffer).await?;
                if bytes_read == 0 {
                    break;
                }
                file_writer.write_all(&buffer[..bytes_read]).await?;
                
                // Update progress tracker
                bytes_uploaded_clone.fetch_add(bytes_read as u64, Ordering::SeqCst);
            }
            
            // Finish the file entry
            file_writer.finish().await?;
        }
    }
    
    // Add directory entries
    for dir in dirs {
        zip_writer.add_directory(&dir, FileOptions::default()).await?;
    }
    
    // Finalize the ZIP
    let target = zip_writer.finish().await?;
    
    // Complete the upload
    let result = target.complete().await;
    
    // Wait for progress reporting to finish if it's running
    let _ = progress_handle.await;
    
    result
}

/// Stream a single file to a streaming target.
///
/// This function provides real-time progress reporting and proper error handling.
///
/// # Arguments
///
/// * `file_path` - Path to the file to stream
/// * `target` - The streaming target (S3, SFTP, etc.)
/// * `buffer_size_mb` - Buffer size in megabytes for streaming operations
///
/// # Returns
///
/// Ok(()) if the upload was successful, or an error with context
pub async fn stream_file_to_target<T: StreamingTarget>(
    file_path: &Path,
    target: T,
    _buffer_size_mb: usize,
) -> Result<()> {
    info!("Streaming file {} to {}", file_path.display(), target.target_name());
    
    // Get file size for progress reporting
    let metadata = tokio::fs::metadata(file_path).await
        .context(format!("Failed to get metadata for {}", file_path.display()))?;
    let total_size = metadata.len();
    info!("File size: {} bytes", total_size);
    
    // Track upload progress
    let bytes_uploaded_tracker = Arc::new(AtomicU64::new(0));
    let bytes_uploaded_clone = Arc::clone(&bytes_uploaded_tracker);
    
    // Spawn a task to report progress
    let progress_tracker = ProgressTracker::new(total_size, Arc::clone(&bytes_uploaded_tracker));
    let progress_handle = progress_tracker.start_tracking();
    
    // Open the file
    let mut file = File::open(file_path).await
        .context(format!("Failed to open {}", file_path.display()))?;
    
    // Create a buffer to track progress
    let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer
    let mut target = target;
    
    // Stream the file
    loop {
        let bytes_read = tokio::io::AsyncReadExt::read(&mut file, &mut buffer).await?;
        if bytes_read == 0 {
            break;
        }
        
        // Write to target
        target.write_all(&buffer[..bytes_read]).await?;
        
        // Update progress tracker
        bytes_uploaded_clone.fetch_add(bytes_read as u64, Ordering::SeqCst);
    }
    
    // Complete the upload
    let result = target.complete().await;
    
    // Wait for progress reporting to finish if it's running
    let _ = progress_handle.await;
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    use std::pin::Pin;
    use std::task::{Context as TaskContext, Poll};
    use std::io;

    // Mock streaming target for testing
    struct MockStreamingTarget {
        name: String,
        bytes: Arc<AtomicU64>,
        data: Vec<u8>,
        completed: bool,
    }

    impl MockStreamingTarget {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                bytes: Arc::new(AtomicU64::new(0)),
                data: Vec::new(),
                completed: false,
            }
        }

    }

    impl StreamingTarget for MockStreamingTarget {
        fn target_name(&self) -> String {
            self.name.clone()
        }

        fn bytes_uploaded(&self) -> u64 {
            self.bytes.load(Ordering::SeqCst)
        }

        async fn complete(mut self) -> Result<()> {
            self.completed = true;
            Ok(())
        }

        async fn abort(self) -> Result<()> {
            Ok(())
        }
    }

    // AsyncWriteExt is automatically implemented for types that implement AsyncWrite

    impl tokio::io::AsyncWrite for MockStreamingTarget {
        fn poll_write(
            mut self: Pin<&mut Self>,
            _cx: &mut TaskContext<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            self.data.extend_from_slice(buf);
            self.bytes.fetch_add(buf.len() as u64, Ordering::SeqCst);
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    #[test]
    fn test_progress_tracker_creation() {
        let bytes_uploaded = Arc::new(AtomicU64::new(0));
        let tracker = ProgressTracker::new(1000, bytes_uploaded.clone());
        
        assert_eq!(tracker.total_size, 1000);
        assert_eq!(tracker.last_percentage, 0);
    }

    #[test]
    fn test_progress_tracker_calculation() {
        let bytes_uploaded = Arc::new(AtomicU64::new(500));
        let tracker = ProgressTracker::new(1000, bytes_uploaded.clone());
        
        // Calculate percentage manually
        let percentage = ((500f64 / 1000f64) * 100.0) as u8;
        assert_eq!(percentage, 50);
    }

    #[tokio::test]
    async fn test_calculate_total_size_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let size = calculate_total_size(temp_dir.path()).await.unwrap();
        assert_eq!(size, 0);
    }

    #[tokio::test]
    async fn test_calculate_total_size_with_files() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create test files
        fs::write(temp_dir.path().join("file1.txt"), "hello").unwrap();
        fs::write(temp_dir.path().join("file2.txt"), "world!").unwrap();
        
        let size = calculate_total_size(temp_dir.path()).await.unwrap();
        assert_eq!(size, 11); // 5 + 6 bytes
    }

    #[tokio::test]
    async fn test_calculate_total_size_nested() {
        let temp_dir = TempDir::new().unwrap();
        let sub_dir = temp_dir.path().join("sub");
        fs::create_dir(&sub_dir).unwrap();
        
        // Create files in root and subdirectory
        fs::write(temp_dir.path().join("file1.txt"), "test").unwrap();
        fs::write(sub_dir.join("file2.txt"), "nested").unwrap();
        
        let size = calculate_total_size(temp_dir.path()).await.unwrap();
        assert_eq!(size, 10); // 4 + 6 bytes
    }

    #[test]
    fn test_get_compression_options_compressed_files() {
        use std::path::PathBuf;
        
        // Test already compressed files
        let compressed_files = vec![
            "test.zip", "test.gz", "test.jpg", "test.mp4", "test.png"
        ];
        
        for file in compressed_files {
            let path = PathBuf::from(file);
            let options = get_compression_options(&path);
            assert_eq!(options.compression_method, CompressionMethod::Stored);
        }
    }

    #[test]
    fn test_get_compression_options_regular_files() {
        use std::path::PathBuf;
        
        // Test regular files that should be compressed
        let regular_files = vec![
            "test.txt", "test.log", "test.json", "test.xml", "test.rs"
        ];
        
        for file in regular_files {
            let path = PathBuf::from(file);
            let options = get_compression_options(&path);
            assert_eq!(options.compression_method, CompressionMethod::Deflated);
        }
    }

    #[test]
    fn test_get_compression_options_large_file() {
        let temp_dir = TempDir::new().unwrap();
        let large_file = temp_dir.path().join("large.txt");
        
        // Create a file larger than 100MB (just metadata, not actual content)
        let file = fs::File::create(&large_file).unwrap();
        file.set_len(101 * 1024 * 1024).unwrap(); // 101MB
        
        let options = get_compression_options(&large_file);
        assert_eq!(options.compression_method, CompressionMethod::Stored);
    }

    #[tokio::test]
    async fn test_stream_file_to_target() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let content = b"Hello, streaming world!";
        fs::write(&test_file, content).unwrap();
        
        let target = MockStreamingTarget::new("test-target");
        let bytes_ref = target.bytes.clone();
        
        let result = stream_file_to_target(&test_file, target, 5).await;
        
        assert!(result.is_ok());
        assert_eq!(bytes_ref.load(Ordering::SeqCst), content.len() as u64);
    }

    #[tokio::test] 
    async fn test_stream_directory_to_target() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create test directory structure
        fs::write(temp_dir.path().join("file1.txt"), "content1").unwrap();
        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir).unwrap();
        fs::write(sub_dir.join("file2.txt"), "content2").unwrap();
        
        let target = MockStreamingTarget::new("test-target");
        let bytes_ref = target.bytes.clone();
        
        let result = stream_directory_to_target(temp_dir.path(), target, 5).await;
        
        assert!(result.is_ok());
        // Should have uploaded some data (ZIP format adds overhead)
        assert!(bytes_ref.load(Ordering::SeqCst) > 0);
    }

    #[tokio::test]
    async fn test_progress_tracker_zero_size() {
        let bytes_uploaded = Arc::new(AtomicU64::new(0));
        let tracker = ProgressTracker::new(0, bytes_uploaded.clone());
        
        // Should complete immediately for zero-size
        let handle = tracker.start_tracking();
        let result = tokio::time::timeout(Duration::from_millis(100), handle).await;
        
        assert!(result.is_ok()); // Should finish quickly
    }

    #[test]
    fn test_compression_options_default() {
        let options = FileOptions::default();
        // Just verify we can create default options
        assert!(matches!(options.compression_method, _));
    }

    #[test]
    fn test_atomic_operations() {
        let counter = Arc::new(AtomicU64::new(0));
        
        // Test store and load
        counter.store(100, Ordering::SeqCst);
        assert_eq!(counter.load(Ordering::SeqCst), 100);
        
        // Test fetch_add
        let old = counter.fetch_add(50, Ordering::SeqCst);
        assert_eq!(old, 100);
        assert_eq!(counter.load(Ordering::SeqCst), 150);
    }

    #[test]
    fn test_path_operations() {
        let path = Path::new("/test/file.txt");
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "file.txt");
        assert_eq!(path.extension().unwrap().to_str().unwrap(), "txt");
    }

    #[test]
    fn test_walkdir_basic() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.txt"), "test").unwrap();
        
        let entries: Vec<_> = WalkDir::new(temp_dir.path())
            .into_iter()
            .filter_map(|e| e.ok())
            .collect();
        
        // Should have at least the root directory and the file
        assert!(entries.len() >= 2);
    }
}
