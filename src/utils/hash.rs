use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;
use sha2::{Sha256, Digest};

const BUFFER_SIZE: usize = 1024 * 1024; // 1MB buffer

/// Calculate SHA-256 hash of a file
/// 
/// Returns None if:
/// - The file is larger than max_size_mb
/// - The path is not a regular file
/// - There was an error reading the file
pub fn calculate_sha256(path: &Path, max_size_mb: u64) -> io::Result<Option<String>> {
    let metadata = std::fs::metadata(path)?;
    
    // Skip if file is too large
    if metadata.len() > max_size_mb * 1024 * 1024 {
        return Ok(None);
    }
    
    // Skip if not a regular file
    if !metadata.is_file() {
        return Ok(None);
    }
    
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0; BUFFER_SIZE];
    
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    
    let hash = hasher.finalize();
    let hash_string = format!("{:x}", hash);
    
    Ok(Some(hash_string))
}
