use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;
use sha2::{Sha256, Digest};

use crate::constants::DEFAULT_BUFFER_SIZE as BUFFER_SIZE;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::{TempDir, NamedTempFile};
    
    #[test]
    fn test_calculate_sha256_small_file() {
        // Create a temporary file with known content
        let content = b"Hello, World!";
        let mut temp_file = NamedTempFile::new().unwrap();
        use std::io::Write;
        temp_file.write_all(content).unwrap();
        temp_file.flush().unwrap();
        let path = temp_file.path();
        
        // Calculate hash
        let result = calculate_sha256(path, 100).unwrap();
        
        // Expected SHA-256 hash of "Hello, World!"
        let expected = "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f";
        
        assert_eq!(result, Some(expected.to_string()));
    }
    
    #[test]
    fn test_calculate_sha256_empty_file() {
        // Create an empty file
        let mut temp_file = NamedTempFile::new().unwrap();
        use std::io::Write;
        temp_file.write_all(b"").unwrap();
        temp_file.flush().unwrap();
        let path = temp_file.path();
        
        // Calculate hash
        let result = calculate_sha256(path, 100).unwrap();
        
        // Expected SHA-256 hash of empty string
        let expected = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        
        assert_eq!(result, Some(expected.to_string()));
    }
    
    #[test]
    fn test_calculate_sha256_large_file() {
        // Create a file with 2MB of data
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("large_file.bin");
        
        // Write 2MB of zeros
        let data = vec![0u8; 2 * 1024 * 1024];
        fs::write(&file_path, &data).unwrap();
        
        // Test with 1MB limit - should return None
        let result = calculate_sha256(&file_path, 1).unwrap();
        assert_eq!(result, None);
        
        // Test with 3MB limit - should calculate hash
        let result = calculate_sha256(&file_path, 3).unwrap();
        assert!(result.is_some());
    }
    
    #[test]
    fn test_calculate_sha256_directory() {
        // Create a temporary directory
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("subdir");
        fs::create_dir(&dir_path).unwrap();
        
        // Trying to hash a directory should return None
        let result = calculate_sha256(&dir_path, 100).unwrap();
        assert_eq!(result, None);
    }
    
    #[test]
    fn test_calculate_sha256_nonexistent_file() {
        let path = Path::new("/nonexistent/file.txt");
        let result = calculate_sha256(path, 100);
        
        // Should return an error
        assert!(result.is_err());
    }
    
    #[test]
    fn test_calculate_sha256_buffer_chunks() {
        // Create a file larger than the buffer size to test chunked reading
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("chunked_file.bin");
        
        // Create 2MB of repeating pattern
        let pattern = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let mut data = Vec::new();
        while data.len() < 2 * 1024 * 1024 {
            data.extend_from_slice(pattern);
        }
        data.truncate(2 * 1024 * 1024);
        
        fs::write(&file_path, &data).unwrap();
        
        // Calculate hash
        let result = calculate_sha256(&file_path, 10).unwrap();
        assert!(result.is_some());
        
        // Verify the hash is consistent
        let result2 = calculate_sha256(&file_path, 10).unwrap();
        assert_eq!(result, result2);
    }
    
    #[test]
    fn test_calculate_sha256_special_characters() {
        // Test with file containing special characters
        let content = b"\x00\x01\x02\x03\xFF\xFE\xFD\xFC";
        let mut temp_file = NamedTempFile::new().unwrap();
        use std::io::Write;
        temp_file.write_all(content).unwrap();
        temp_file.flush().unwrap();
        let path = temp_file.path();
        
        let result = calculate_sha256(path, 100).unwrap();
        assert!(result.is_some());
        
        // Verify hash format is lowercase hex
        let hash = result.unwrap();
        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex characters
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
    }
}
