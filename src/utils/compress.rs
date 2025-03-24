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

/// File entry with its compression options
struct FileEntry {
    rel_path: String,
    abs_path: PathBuf,
    options: FileOptions,
}

// Static chunk size for file reading
const CHUNK_SIZE: usize = 512 * 1024; // 512KB

/// Determine optimal compression level based on file type
fn get_compression_options(path: &Path) -> FileOptions {
    // Detect file type from extension
    let low_compression = match path.extension().and_then(|e| e.to_str()) {
        // Files that are already compressed - use minimal compression
        Some("zip" | "gz" | "xz" | "bz2" | "7z" | "rar" | "jpg" | "jpeg" | 
             "png" | "gif" | "mp3" | "mp4" | "avi" | "mov" | "mpg" | "mpeg") => true,
        _ => false,
    };
    
    // Detect if it's very large, in which case use faster compression
    let large_file = match fs::metadata(path) {
        Ok(metadata) if metadata.len() > 100 * 1024 * 1024 => true, // > 100MB
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

/// Compress all collected artifacts into a zip file with multithreading
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
