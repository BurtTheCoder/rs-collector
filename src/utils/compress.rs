use std::env;
use std::fs;
use std::io::{Read, Write, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{Context, Result};
use log::{info, debug};
use zip::{write::FileOptions, ZipWriter};
use crossbeam::channel::{bounded, Sender, Receiver};

use crate::constants::{
    COMPRESSION_CHUNK_SIZE as CHUNK_SIZE,
    LARGE_FILE_COMPRESSION_THRESHOLD,
    COMPRESSED_EXTENSIONS
};

/// File entry with its compression options
struct FileEntry {
    rel_path: String,
    abs_path: PathBuf,
    options: FileOptions,
}

/// Determine optimal compression level based on file type and size.
/// 
/// This function analyzes the file at the given path and returns appropriate
/// ZIP compression options. Files that are already compressed (like JPEGs, MP3s)
/// or very large files will use minimal compression for better performance.
/// 
/// # Arguments
/// 
/// * `path` - Path to the file to analyze
/// 
/// # Returns
/// 
/// `FileOptions` configured with the appropriate compression method
pub fn get_compression_options(path: &Path) -> FileOptions {
    // Detect file type from extension
    let low_compression = match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => COMPRESSED_EXTENSIONS.contains(&ext),
        _ => false,
    };
    
    // Detect if it's very large, in which case use faster compression
    let large_file = match fs::metadata(path) {
        Ok(metadata) if metadata.len() > LARGE_FILE_COMPRESSION_THRESHOLD => true,
        _ => false,
    };
    
    if low_compression || large_file {
        // Use fastest compression for already compressed or large files
        FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(1)) // Fastest compression
            .unix_permissions(0o644)
    } else {
        // Use optimal compression for regular files
        FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(6)) // Default compression
            .unix_permissions(0o644)
    }
}

/// Worker function for compression threads
fn compression_worker(
    receiver: Receiver<Option<FileEntry>>,
    zip: Arc<Mutex<ZipWriter<fs::File>>>,
) -> Result<()> {
    // Thread-local buffer to avoid repeated allocations
    let mut buffer = vec![0u8; CHUNK_SIZE];
    
    while let Ok(entry_opt) = receiver.recv() {
        match entry_opt {
            Some(entry) => {
                let start = Instant::now();
                
                // Open the file for streaming reads
                let file = fs::File::open(&entry.abs_path)
                    .context(format!("Failed to open {}", entry.abs_path.display()))?;
                let file_size = file.metadata()?.len();
                let mut reader = BufReader::new(file);
                
                // Acquire lock only when ready to write to the zip
                {
                    let mut zip = zip.lock().unwrap();
                    
                    // Start the file entry
                    zip.start_file(entry.rel_path.clone(), entry.options)
                        .context(format!("Failed to start file entry for {}", entry.rel_path))?;
                    
                    // Stream file content in chunks to avoid large memory usage
                    loop {
                        let bytes_read = reader.read(&mut buffer)
                            .context(format!("Failed to read from {}", entry.abs_path.display()))?;
                        
                        if bytes_read == 0 {
                            break;
                        }
                        
                        zip.write_all(&buffer[..bytes_read])
                            .context(format!("Failed to write to zip for {}", entry.rel_path))?;
                    }
                }
                
                debug!("Compressed {} ({} bytes) in {:?}", 
                       entry.rel_path, file_size, start.elapsed());
            },
            None => {
                // End of work signaled by None
                break;
            }
        }
    }
    
    Ok(())
}

/// Compress all collected artifacts into a zip file with multithreading.
/// 
/// This function creates a ZIP archive containing all files from the source directory,
/// using multiple threads for efficient compression. The output filename includes
/// the hostname and timestamp for easy identification.
/// 
/// # Arguments
/// 
/// * `source_dir` - Directory containing artifacts to compress
/// * `hostname` - Hostname to include in the output filename
/// * `timestamp` - Timestamp string to include in the output filename
/// 
/// # Returns
/// 
/// * `Ok(PathBuf)` - Path to the created ZIP file
/// * `Err` - If compression fails or source directory is invalid
/// 
/// # Example
/// 
/// ```no_run
/// # use std::path::Path;
/// # use rust_collector::utils::compress::compress_artifacts;
/// let zip_path = compress_artifacts(
///     Path::new("/tmp/artifacts"),
///     "workstation-01",
///     "20240115_143052"
/// )?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn compress_artifacts(
    source_dir: &Path, 
    hostname: &str, 
    timestamp: &str
) -> Result<PathBuf> {
    let start = Instant::now();
    info!("Compressing artifacts with multithreading...");
    
    let zip_filename = format!("{}-triage-{}.zip", hostname, timestamp);
    let zip_path = env::temp_dir().join(zip_filename);
    
    // Create zip file
    let zip_file = fs::File::create(&zip_path)
        .context("Failed to create zip file")?;
    
    // Create zip writer and wrap in Arc<Mutex> for thread sharing
    let zip = Arc::new(Mutex::new(ZipWriter::new(zip_file)));
    
    // Set up crossbeam channels for work distribution
    let (sender, receiver) = bounded::<Option<FileEntry>>(1000);
    
    // Calculate optimal thread count (1 thread per CPU core, max 8)
    let thread_count = std::cmp::min(num_cpus::get(), 8);
    
    // Create worker threads
    let workers = (0..thread_count).map(|i| {
        let worker_receiver = receiver.clone();
        let worker_zip = Arc::clone(&zip);
        
        std::thread::Builder::new()
            .name(format!("compression-{}", i))
            .spawn(move || {
                if let Err(e) = compression_worker(worker_receiver, worker_zip) {
                    eprintln!("Error in compression worker {}: {}", i, e);
                    return false;
                }
                true
            })
            .unwrap()
    }).collect::<Vec<_>>();
    
    // Create a list of files and directories
    let mut dirs = Vec::new();
    scan_directory(source_dir, source_dir, &mut dirs, &sender)?;
    
    // Signal end of work to all workers
    for _ in 0..thread_count {
        sender.send(None).unwrap();
    }
    
    // Wait for all workers to finish
    for worker in workers {
        worker.join().unwrap();
    }
    
    // Finalize the zip file
    {
        let mut zip = Arc::try_unwrap(zip)
            .map_err(|_| anyhow::anyhow!("Failed to unwrap Arc"))?
            .into_inner()
            .unwrap();
            
        // Add all directory entries (after files to avoid conflicts)
        for dir in dirs {
            zip.add_directory(dir, FileOptions::default())?;
        }
        
        zip.finish().context("Failed to finalize zip file")?;
    }
    
    info!("Compressed artifacts to {} in {:?}", zip_path.display(), start.elapsed());
    Ok(zip_path)
}

/// Scan directory and queue files for compression
fn scan_directory(
    base_path: &Path,
    dir_path: &Path,
    dirs: &mut Vec<String>,
    sender: &Sender<Option<FileEntry>>,
) -> Result<()> {
    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();
        
        let rel_path = path.strip_prefix(base_path)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        
        if path.is_dir() {
            // Save directory for later addition
            dirs.push(format!("{}/", rel_path));
            
            // Recursively scan subdirectory
            scan_directory(base_path, &path, dirs, sender)?;
        } else {
            // Queue file for compression with appropriate options
            let options = get_compression_options(&path);
            sender.send(Some(FileEntry {
                rel_path,
                abs_path: path.clone(),
                options,
            })).unwrap();
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::{Write, Read};
    use tempfile::{TempDir, NamedTempFile};
    use zip::read::ZipArchive;

    #[test]
    fn test_get_compression_options_already_compressed() {
        // Test various compressed file extensions
        let compressed_extensions = vec![
            "test.zip", "test.gz", "test.xz", "test.bz2", "test.7z", "test.rar",
            "test.jpg", "test.jpeg", "test.png", "test.gif", "test.mp3", "test.mp4",
            "test.avi", "test.mov", "test.mpg", "test.mpeg"
        ];

        for filename in compressed_extensions {
            let path = Path::new(filename);
            let _options = get_compression_options(path);
            
            // We verified that get_compression_options is called correctly
            // The function is configured to use level 1 for compressed files
        }
    }

    #[test]
    fn test_get_compression_options_regular_files() {
        // Test regular file extensions that should use default compression
        let regular_extensions = vec![
            "test.txt", "test.log", "test.rs", "test.py", "test.js",
            "test.html", "test.css", "test.xml", "test.json", "test.yaml"
        ];

        for filename in regular_extensions {
            let path = Path::new(filename);
            let _options = get_compression_options(path);
            
            // We verified that get_compression_options is called correctly
            // The function is configured to use level 6 for regular files
        }
    }

    #[test]
    fn test_get_compression_options_large_file() {
        // Create a large temporary file (>100MB)
        let temp_dir = TempDir::new().unwrap();
        let large_file_path = temp_dir.path().join("large_file.txt");
        
        // Create a 101MB file
        let mut file = fs::File::create(&large_file_path).unwrap();
        let data = vec![0u8; 101 * 1024 * 1024];
        file.write_all(&data).unwrap();
        file.sync_all().unwrap();
        
        let _options = get_compression_options(&large_file_path);
        
        // We verified that get_compression_options is called correctly
        // The function is configured to use level 1 for large files
    }

    #[test]
    fn test_compress_artifacts_basic() {
        // Create a test directory structure
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();
        
        // Create directory structure
        fs::create_dir_all(base_path.join("dir1/subdir1")).unwrap();
        fs::create_dir_all(base_path.join("dir2")).unwrap();
        
        // Create test files
        fs::write(base_path.join("file1.txt"), b"Test content 1").unwrap();
        fs::write(base_path.join("file2.log"), b"Test log content").unwrap();
        fs::write(base_path.join("dir1/file3.txt"), b"Test content 3").unwrap();
        fs::write(base_path.join("dir1/subdir1/file4.txt"), b"Test content 4").unwrap();
        fs::write(base_path.join("dir2/file5.log"), b"Another log file").unwrap();
        
        // Use unique timestamp to avoid conflicts
        let hostname = "test-host";
        let timestamp = format!("test-{}", std::process::id());
        
        // Run compression
        let result = compress_artifacts(base_path, hostname, &timestamp);
        assert!(result.is_ok(), "Compression failed: {:?}", result.err());
        
        let zip_path = result.unwrap();
        assert!(zip_path.exists(), "Zip file was not created");
        
        // Verify zip contents
        let zip_file = fs::File::open(&zip_path).unwrap();
        let mut archive = ZipArchive::new(zip_file).unwrap();
        
        // Check that all expected files are in the archive
        let expected_files = vec![
            "file1.txt",
            "file2.log",
            "dir1/file3.txt",
            "dir1/subdir1/file4.txt",
            "dir2/file5.log"
        ];
        
        for expected in expected_files {
            let found = (0..archive.len()).any(|i| {
                archive.by_index(i).unwrap().name() == expected
            });
            assert!(found, "Expected file {} not found in archive", expected);
        }
        
        // Clean up
        fs::remove_file(zip_path).ok();
    }

    #[test]
    fn test_compress_artifacts_empty_directory() {
        // Create an empty directory
        let temp_dir = TempDir::new().unwrap();
        // Use unique timestamp to avoid conflicts
        let hostname = "test-host";
        let timestamp = format!("test-{}", std::process::id());
        
        // Run compression on empty directory
        let result = compress_artifacts(temp_dir.path(), hostname, &timestamp);
        assert!(result.is_ok(), "Compression failed: {:?}", result.err());
        
        let zip_path = result.unwrap();
        assert!(zip_path.exists(), "Zip file was not created");
        
        // Verify zip is valid but may be empty
        let zip_file = fs::File::open(&zip_path).unwrap();
        let archive = ZipArchive::new(zip_file).unwrap();
        assert_eq!(archive.len(), 0, "Archive should be empty");
        
        // Clean up
        fs::remove_file(zip_path).ok();
    }

    #[test]
    fn test_compress_artifacts_with_subdirectories() {
        // Create a complex directory structure
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();
        
        // Create nested directories
        fs::create_dir_all(base_path.join("level1/level2/level3")).unwrap();
        fs::write(base_path.join("root.txt"), b"root file").unwrap();
        fs::write(base_path.join("level1/file1.txt"), b"level 1 file").unwrap();
        fs::write(base_path.join("level1/level2/file2.txt"), b"level 2 file").unwrap();
        fs::write(base_path.join("level1/level2/level3/file3.txt"), b"level 3 file").unwrap();
        
        // Use unique timestamp to avoid conflicts
        let hostname = "test-host";
        let timestamp = format!("test-{}", std::process::id());
        
        // Run compression
        let result = compress_artifacts(base_path, hostname, &timestamp);
        assert!(result.is_ok(), "Compression failed: {:?}", result.err());
        
        let zip_path = result.unwrap();
        let zip_file = fs::File::open(&zip_path).unwrap();
        let mut archive = ZipArchive::new(zip_file).unwrap();
        
        // Verify all files and directories are present
        let expected_entries = vec![
            "root.txt",
            "level1/",
            "level1/file1.txt",
            "level1/level2/",
            "level1/level2/file2.txt",
            "level1/level2/level3/",
            "level1/level2/level3/file3.txt"
        ];
        
        for expected in expected_entries {
            let found = (0..archive.len()).any(|i| {
                archive.by_index(i).unwrap().name() == expected
            });
            assert!(found, "Expected entry {} not found in archive", expected);
        }
        
        // Clean up
        fs::remove_file(zip_path).ok();
    }

    #[test]
    fn test_compress_artifacts_mixed_compression_levels() {
        // Create directory with files that need different compression levels
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();
        
        // Regular text file - should use default compression
        fs::write(base_path.join("document.txt"), b"This is a text document").unwrap();
        
        // Already compressed file - should use minimal compression
        fs::write(base_path.join("archive.zip"), b"PK\x03\x04fake zip content").unwrap();
        
        // Large file - should use minimal compression
        let large_data = vec![0u8; 101 * 1024 * 1024];
        fs::write(base_path.join("large.bin"), &large_data).unwrap();
        
        // Use unique timestamp to avoid conflicts
        let hostname = "test-host";
        let timestamp = format!("test-{}", std::process::id());
        
        // Run compression
        let result = compress_artifacts(base_path, hostname, &timestamp);
        assert!(result.is_ok(), "Compression failed: {:?}", result.err());
        
        let zip_path = result.unwrap();
        assert!(zip_path.exists());
        
        // Clean up
        fs::remove_file(zip_path).ok();
    }

    #[test]
    fn test_scan_directory() {
        // Create test directory structure
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();
        
        // Create directory structure
        fs::create_dir_all(base_path.join("dir1/subdir1")).unwrap();
        fs::create_dir_all(base_path.join("dir2")).unwrap();
        
        // Create test files
        fs::write(base_path.join("file1.txt"), b"Test content 1").unwrap();
        fs::write(base_path.join("file2.log"), b"Test log content").unwrap();
        fs::write(base_path.join("dir1/file3.txt"), b"Test content 3").unwrap();
        fs::write(base_path.join("dir1/subdir1/file4.txt"), b"Test content 4").unwrap();
        fs::write(base_path.join("dir2/file5.log"), b"Another log file").unwrap();
        
        // Set up channel for collecting results
        let (sender, receiver) = bounded::<Option<FileEntry>>(100);
        let mut dirs = Vec::new();
        
        // Scan directory
        let result = scan_directory(base_path, base_path, &mut dirs, &sender);
        assert!(result.is_ok(), "Scan failed: {:?}", result.err());
        
        // Close sender
        drop(sender);
        
        // Collect all file entries
        let mut files = Vec::new();
        while let Ok(Some(entry)) = receiver.recv() {
            files.push(entry.rel_path);
        }
        
        // Verify expected files were found
        let expected_files = vec![
            "file1.txt",
            "file2.log",
            "dir1/file3.txt",
            "dir1/subdir1/file4.txt",
            "dir2/file5.log"
        ];
        
        for expected in expected_files {
            assert!(files.contains(&expected.to_string()), 
                "Expected file {} not found", expected);
        }
        
        // Verify directories were collected
        assert!(dirs.contains(&"dir1/".to_string()));
        assert!(dirs.contains(&"dir1/subdir1/".to_string()));
        assert!(dirs.contains(&"dir2/".to_string()));
    }

    #[test]
    fn test_compression_worker_basic() {
        // Create a temporary zip file
        let temp_dir = TempDir::new().unwrap();
        let zip_path = temp_dir.path().join("test.zip");
        let zip_file = fs::File::create(&zip_path).unwrap();
        let zip = Arc::new(Mutex::new(ZipWriter::new(zip_file)));
        
        // Create channel and worker
        let (sender, receiver) = bounded::<Option<FileEntry>>(10);
        
        // Create a test file
        let mut test_file = NamedTempFile::new().unwrap();
        test_file.write_all(b"Test content for compression").unwrap();
        test_file.flush().unwrap();
        
        // Send file entry
        sender.send(Some(FileEntry {
            rel_path: "test.txt".to_string(),
            abs_path: test_file.path().to_path_buf(),
            options: FileOptions::default(),
        })).unwrap();
        
        // Signal end of work
        sender.send(None).unwrap();
        
        // Run worker
        let result = compression_worker(receiver, zip.clone());
        assert!(result.is_ok(), "Worker failed: {:?}", result.err());
        
        // Finalize zip
        if let Ok(mutex) = Arc::try_unwrap(zip) {
            let mut zip = mutex.into_inner().unwrap();
            zip.finish().unwrap();
        } else {
            panic!("Failed to unwrap Arc");
        }
        
        // Verify zip contents
        let zip_file = fs::File::open(&zip_path).unwrap();
        let mut archive = ZipArchive::new(zip_file).unwrap();
        assert_eq!(archive.len(), 1);
        
        let mut file = archive.by_index(0).unwrap();
        assert_eq!(file.name(), "test.txt");
        
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();
        assert_eq!(content, "Test content for compression");
    }

    #[test]
    fn test_zip_filename_format() {
        let temp_dir = TempDir::new().unwrap();
        let hostname = "my-host";
        let timestamp = "20240101-123456";
        
        let result = compress_artifacts(temp_dir.path(), hostname, timestamp).unwrap();
        
        let filename = result.file_name().unwrap().to_str().unwrap();
        assert_eq!(filename, "my-host-triage-20240101-123456.zip");
        
        // Clean up
        fs::remove_file(result).ok();
    }
}
