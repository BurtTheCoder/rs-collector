use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::UNIX_EPOCH;
use std::os::unix::fs::MetadataExt;
use std::collections::HashMap;
use std::sync::{Mutex, atomic::{AtomicUsize, Ordering}};

use anyhow::{Context, Result};
use bodyfile::Bodyfile3Line;
use chrono::{TimeZone, Utc};
use log::{debug, info};
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::utils::hash::calculate_sha256;

/// Convert Unix timestamp to ISO 8601 format
fn unix_to_iso8601(timestamp: u64) -> String {
    // Check if timestamp is too large to fit in i64
    if timestamp > i64::MAX as u64 {
        return "0000-00-00T00:00:00Z".to_string(); // Invalid timestamp
    }
    
    match Utc.timestamp_opt(timestamp as i64, 0) {
        chrono::LocalResult::Single(dt) => dt.to_rfc3339(),
        _ => "0000-00-00T00:00:00Z".to_string() // Invalid timestamp
    }
}

/// Generate a bodyfile for the filesystem with advanced options.
/// 
/// Creates a bodyfile containing metadata for all files in the filesystem,
/// formatted according to the Sleuth Kit bodyfile format. This format is
/// commonly used in digital forensics for timeline analysis.
/// 
/// # Arguments
/// 
/// * `output_path` - Path where the bodyfile will be written
/// * `options` - HashMap of options controlling bodyfile generation:
///   - `"bodyfile_calculate_hash"` - Calculate SHA256 hashes ("true"/"false")
///   - `"max_file_size_mb"` - Maximum file size for hashing (in MB)
/// 
/// # Returns
/// 
/// * `Ok(())` - If bodyfile generation succeeds
/// * `Err` - If file creation or filesystem traversal fails
/// 
/// # Bodyfile Format
/// 
/// Each line follows the format:
/// `MD5|name|inode|mode|UID|GID|size|atime|mtime|ctime|crtime`
pub fn generate_bodyfile(output_path: &Path, options: &HashMap<String, String>) -> Result<()> {
    info!("Generating bodyfile at {}", output_path.display());
    
    // Parse configuration options
    let calculate_hash = options.get("bodyfile_calculate_hash")
        .map(|v| v == "true")
        .unwrap_or(false);
        
    let max_hash_size_mb = options.get("bodyfile_hash_max_size_mb")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(100);
        
    let use_iso8601 = options.get("bodyfile_use_iso8601")
        .map(|v| v == "true")
        .unwrap_or(true);
        
    let skip_paths: Vec<&str> = options.get("bodyfile_skip_paths")
        .map(|v| v.split(',').collect())
        .unwrap_or_else(|| vec!["/proc", "/sys", "/dev"]);
    
    info!("Bodyfile options: calculate_hash={}, max_hash_size={}MB, use_iso8601={}", 
          calculate_hash, max_hash_size_mb, use_iso8601);
    
    // Create output file
    let file = File::create(output_path)
        .context(format!("Failed to create bodyfile at {}", output_path.display()))?;
    let writer = BufWriter::new(file);
    let writer = Mutex::new(writer);
    
    // Write header
    let header = if use_iso8601 {
        "# SHA256|name|inode|mode_as_string|UID|GID|size|atime_iso|mtime_iso|ctime_iso|crtime_iso"
    } else {
        "# SHA256|name|inode|mode_as_string|UID|GID|size|atime|mtime|ctime|crtime"
    };
    writeln!(writer.lock().unwrap(), "{}", header)?;
    
    // Use walkdir to traverse the filesystem
    let root = Path::new("/");
    let walker = WalkDir::new(root)
        .follow_links(false)
        .same_file_system(true); // Don't cross filesystem boundaries
    
    // Process files in parallel
    let count = AtomicUsize::new(0);
    walker.into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            let path = entry.path();
            !skip_paths.iter().any(|skip| path.starts_with(skip))
        })
        .par_bridge()
        .for_each(|entry| {
            if let Some(line) = create_bodyfile_line_advanced(
                entry.path(), 
                calculate_hash, 
                max_hash_size_mb,
                use_iso8601
            ) {
                let mut writer = writer.lock().unwrap();
                if writeln!(writer, "{}", line).is_ok() {
                    let current = count.fetch_add(1, Ordering::SeqCst);
                    if current % 10000 == 0 {
                        info!("Processed {} files for bodyfile", current);
                    }
                }
            }
        });
    
    info!("Bodyfile generation complete: {} entries", count.load(Ordering::SeqCst));
    Ok(())
}

/// Create a bodyfile line for a single file with advanced options
fn create_bodyfile_line_advanced(
    path: &Path, 
    calculate_hash: bool, 
    max_hash_size_mb: u64,
    use_iso8601: bool
) -> Option<String> {
    // Skip files we can't access
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            debug!("Cannot access metadata for {}: {}", path.display(), e);
            return None;
        }
    };
    
    // Get file times
    let atime = metadata.accessed().ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    
    let mtime = metadata.modified().ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    
    // ctime and crtime are platform-specific
    let (ctime, crtime) = get_platform_specific_times(&metadata);
    
    // Calculate hash if requested
    let hash = if calculate_hash && metadata.is_file() {
        match calculate_sha256(path, max_hash_size_mb) {
            Ok(Some(h)) => h,
            Ok(None) => "0".to_string(), // Skip due to size or not a file
            Err(_) => "0".to_string(),   // Error calculating hash
        }
    } else {
        "0".to_string() // Default when not calculating hash
    };
    
    // Convert timestamps if requested
    let (atime_str, mtime_str, ctime_str, crtime_str) = if use_iso8601 {
        (
            unix_to_iso8601(atime),
            unix_to_iso8601(mtime),
            unix_to_iso8601(ctime),
            unix_to_iso8601(crtime)
        )
    } else {
        (
            atime.to_string(),
            mtime.to_string(),
            ctime.to_string(),
            crtime.to_string()
        )
    };
    
    // Create the bodyfile line
    let line = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        hash,
        path.to_string_lossy(),
        metadata.ino(),
        get_mode_string(&metadata),
        metadata.uid(),
        metadata.gid(),
        metadata.len(),
        atime_str,
        mtime_str,
        ctime_str,
        crtime_str
    );
    
    Some(line)
}

/// Create a bodyfile line for a single file
#[allow(dead_code)]
fn create_bodyfile_line(path: &Path) -> Option<Bodyfile3Line> {
    // Skip files we can't access
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            debug!("Cannot access metadata for {}: {}", path.display(), e);
            return None;
        }
    };
    
    // Get file times
    let atime = metadata.accessed().ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    
    let mtime = metadata.modified().ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    
    // ctime and crtime are platform-specific
    let (ctime, crtime) = get_platform_specific_times(&metadata);
    
    // Create the bodyfile line using string format
    let line_str = format!(
        "0|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        path.to_string_lossy(),
        metadata.ino(),
        get_mode_string(&metadata),
        metadata.uid(),
        metadata.gid(),
        metadata.len(),
        atime,
        mtime,
        ctime,
        crtime
    );
    
    // Convert the string to a Bodyfile3Line
    match Bodyfile3Line::try_from(line_str.as_str()) {
        Ok(line) => Some(line),
        Err(e) => {
            debug!("Failed to create bodyfile line for {}: {}", path.display(), e);
            None
        }
    }
}

// Platform-specific implementations for getting ctime and crtime
#[cfg(target_os = "macos")]
fn get_platform_specific_times(metadata: &fs::Metadata) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt;
    
    // On macOS, we can get ctime but not crtime directly
    let ctime = metadata.ctime() as u64;
    
    // For crtime (creation time), we need to use platform-specific APIs
    // This is a simplified version; in practice we'd use more robust methods
    let crtime = metadata.created().ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    
    (ctime, crtime)
}

#[cfg(target_os = "linux")]
fn get_platform_specific_times(metadata: &fs::Metadata) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt;
    
    // On Linux, we can get ctime but not crtime directly
    let ctime = metadata.ctime() as u64;
    
    // Linux doesn't track creation time in most filesystems
    let crtime = 0;
    
    (ctime, crtime)
}

// Default implementation for other platforms
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn get_platform_specific_times(_metadata: &fs::Metadata) -> (u64, u64) {
    (0, 0)
}

/// Get mode string (e.g., "d/drwxr-xr-x")
fn get_mode_string(metadata: &fs::Metadata) -> String {
    use std::os::unix::fs::PermissionsExt;
    
    let mode = metadata.permissions().mode();
    let file_type = if metadata.is_dir() { "d/" } else if metadata.is_symlink() { "l/" } else { "-/" };
    
    // Format the mode string
    let user_r = if mode & 0o400 != 0 { "r" } else { "-" };
    let user_w = if mode & 0o200 != 0 { "w" } else { "-" };
    let user_x = if mode & 0o100 != 0 { "x" } else { "-" };
    
    let group_r = if mode & 0o040 != 0 { "r" } else { "-" };
    let group_w = if mode & 0o020 != 0 { "w" } else { "-" };
    let group_x = if mode & 0o010 != 0 { "x" } else { "-" };
    
    let other_r = if mode & 0o004 != 0 { "r" } else { "-" };
    let other_w = if mode & 0o002 != 0 { "w" } else { "-" };
    let other_x = if mode & 0o001 != 0 { "x" } else { "-" };
    
    format!("{}{}{}{}{}{}{}{}{}{}", file_type, user_r, user_w, user_x, group_r, group_w, group_x, other_r, other_w, other_x)
}

/// Generate a bodyfile with a limited scope for testing or specific directories
#[allow(dead_code)]
pub fn generate_limited_bodyfile(output_path: &Path, root_path: &Path) -> Result<()> {
    // Use default options
    let mut options = HashMap::new();
    options.insert("bodyfile_calculate_hash".to_string(), "false".to_string());
    options.insert("bodyfile_hash_max_size_mb".to_string(), "100".to_string());
    options.insert("bodyfile_use_iso8601".to_string(), "true".to_string());
    options.insert("bodyfile_skip_paths".to_string(), "/proc,/sys,/dev".to_string());
    
    generate_limited_bodyfile_with_options(output_path, root_path, &options)
}

/// Generate a bodyfile with a limited scope and custom options
#[allow(dead_code)]
pub fn generate_limited_bodyfile_with_options(
    output_path: &Path, 
    root_path: &Path,
    options: &HashMap<String, String>
) -> Result<()> {
    info!("Generating limited bodyfile at {} for path {}", output_path.display(), root_path.display());
    
    // Parse configuration options
    let calculate_hash = options.get("bodyfile_calculate_hash")
        .map(|v| v == "true")
        .unwrap_or(false);
        
    let max_hash_size_mb = options.get("bodyfile_hash_max_size_mb")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(100);
        
    let use_iso8601 = options.get("bodyfile_use_iso8601")
        .map(|v| v == "true")
        .unwrap_or(true);
    
    info!("Bodyfile options: calculate_hash={}, max_hash_size={}MB, use_iso8601={}", 
          calculate_hash, max_hash_size_mb, use_iso8601);
    
    // Create output file
    let file = File::create(output_path)
        .context(format!("Failed to create bodyfile at {}", output_path.display()))?;
    let writer = BufWriter::new(file);
    let writer = Mutex::new(writer);
    
    // Write header
    let header = if use_iso8601 {
        "# SHA256|name|inode|mode_as_string|UID|GID|size|atime_iso|mtime_iso|ctime_iso|crtime_iso"
    } else {
        "# SHA256|name|inode|mode_as_string|UID|GID|size|atime|mtime|ctime|crtime"
    };
    writeln!(writer.lock().unwrap(), "{}", header)?;
    
    // Use walkdir to traverse the specified directory
    let walker = WalkDir::new(root_path)
        .follow_links(false)
        .same_file_system(true);
    
    // Process files in parallel
    let count = AtomicUsize::new(0);
    walker.into_iter()
        .filter_map(Result::ok)
        .par_bridge()
        .for_each(|entry| {
            if let Some(line) = create_bodyfile_line_advanced(
                entry.path(), 
                calculate_hash, 
                max_hash_size_mb,
                use_iso8601
            ) {
                let mut writer = writer.lock().unwrap();
                if writeln!(writer, "{}", line).is_ok() {
                    let current = count.fetch_add(1, Ordering::SeqCst);
                    if current % 1000 == 0 {
                        info!("Processed {} files for limited bodyfile", current);
                    }
                }
            }
        });
    
    info!("Limited bodyfile generation complete: {} entries", count.load(Ordering::SeqCst));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::{TempDir, NamedTempFile};

    #[test]
    fn test_unix_to_iso8601() {
        // Test valid timestamp
        let timestamp = 1609459200; // 2021-01-01 00:00:00 UTC
        let result = unix_to_iso8601(timestamp);
        assert!(result.starts_with("2021-01-01T00:00:00"));
        
        // Test zero timestamp
        let result = unix_to_iso8601(0);
        assert!(result.starts_with("1970-01-01T00:00:00"));
        
        // Test invalid timestamp (too large)
        let result = unix_to_iso8601(u64::MAX);
        assert_eq!(result, "0000-00-00T00:00:00Z");
    }

    #[test]
    fn test_get_mode_string() {
        let temp_dir = TempDir::new().unwrap();
        
        // Test regular file with 644 permissions
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, b"test").unwrap();
        fs::set_permissions(&file_path, fs::Permissions::from_mode(0o644)).unwrap();
        
        let metadata = fs::metadata(&file_path).unwrap();
        let mode_str = get_mode_string(&metadata);
        assert_eq!(mode_str, "-/rw-r--r--");
        
        // Test directory with 755 permissions
        let dir_path = temp_dir.path().join("testdir");
        fs::create_dir(&dir_path).unwrap();
        fs::set_permissions(&dir_path, fs::Permissions::from_mode(0o755)).unwrap();
        
        let metadata = fs::metadata(&dir_path).unwrap();
        let mode_str = get_mode_string(&metadata);
        assert_eq!(mode_str, "d/rwxr-xr-x");
        
        // Test executable file with 755 permissions
        let exec_path = temp_dir.path().join("test.sh");
        fs::write(&exec_path, b"#!/bin/bash").unwrap();
        fs::set_permissions(&exec_path, fs::Permissions::from_mode(0o755)).unwrap();
        
        let metadata = fs::metadata(&exec_path).unwrap();
        let mode_str = get_mode_string(&metadata);
        assert_eq!(mode_str, "-/rwxr-xr-x");
    }

    #[test]
    fn test_create_bodyfile_line_advanced() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), b"test content").unwrap();
        
        // Test without hash calculation
        let line = create_bodyfile_line_advanced(
            temp_file.path(),
            false,
            100,
            false
        );
        
        assert!(line.is_some());
        let line = line.unwrap();
        
        // Verify line format
        let parts: Vec<&str> = line.split('|').collect();
        assert_eq!(parts.len(), 11);
        assert_eq!(parts[0], "0"); // No hash calculated
        assert!(parts[1].contains(temp_file.path().file_name().unwrap().to_str().unwrap()));
        assert_eq!(parts[6], "12"); // File size
        
        // Test with hash calculation
        let line = create_bodyfile_line_advanced(
            temp_file.path(),
            true,
            100,
            false
        );
        
        assert!(line.is_some());
        let line = line.unwrap();
        let parts: Vec<&str> = line.split('|').collect();
        assert_ne!(parts[0], "0"); // Hash should be calculated
        assert_eq!(parts[0].len(), 64); // SHA256 hash length
        
        // Test with ISO8601 timestamps
        let line = create_bodyfile_line_advanced(
            temp_file.path(),
            false,
            100,
            true
        );
        
        assert!(line.is_some());
        let line = line.unwrap();
        let parts: Vec<&str> = line.split('|').collect();
        assert!(parts[7].contains("T")); // ISO8601 format contains 'T'
        assert!(parts[8].contains("T"));
        assert!(parts[9].contains("T"));
    }

    #[test]
    fn test_create_bodyfile_line_nonexistent_file() {
        let path = Path::new("/nonexistent/file.txt");
        let line = create_bodyfile_line_advanced(path, false, 100, false);
        assert!(line.is_none());
    }

    #[test]
    fn test_generate_limited_bodyfile() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("bodyfile.txt");
        
        // Create a test directory structure
        let test_dir = temp_dir.path().join("test");
        fs::create_dir(&test_dir).unwrap();
        fs::write(test_dir.join("file1.txt"), b"content1").unwrap();
        fs::write(test_dir.join("file2.txt"), b"content2").unwrap();
        
        let subdir = test_dir.join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("file3.txt"), b"content3").unwrap();
        
        // Generate bodyfile
        let result = generate_limited_bodyfile(&output_path, &test_dir);
        assert!(result.is_ok());
        
        // Verify output file exists and contains data
        assert!(output_path.exists());
        let content = fs::read_to_string(&output_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        
        // Should have header + 4 entries (1 dir + 3 files + 1 subdir)
        assert!(lines.len() >= 5);
        assert!(lines[0].starts_with("# SHA256"));
        
        // Check that file entries are present
        assert!(content.contains("file1.txt"));
        assert!(content.contains("file2.txt"));
        assert!(content.contains("file3.txt"));
        assert!(content.contains("subdir"));
    }

    #[test]
    fn test_generate_limited_bodyfile_with_options() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("bodyfile_with_hash.txt");
        
        // Create test files
        let test_dir = temp_dir.path().join("test");
        fs::create_dir(&test_dir).unwrap();
        fs::write(test_dir.join("small.txt"), b"small content").unwrap();
        
        // Create options with hash calculation enabled
        let mut options = HashMap::new();
        options.insert("bodyfile_calculate_hash".to_string(), "true".to_string());
        options.insert("bodyfile_hash_max_size_mb".to_string(), "1".to_string());
        options.insert("bodyfile_use_iso8601".to_string(), "true".to_string());
        
        // Generate bodyfile
        let result = generate_limited_bodyfile_with_options(&output_path, &test_dir, &options);
        assert!(result.is_ok());
        
        // Verify output
        let content = fs::read_to_string(&output_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        
        // Find the line for small.txt
        let small_line = lines.iter()
            .find(|line| line.contains("small.txt"))
            .expect("small.txt not found in bodyfile");
        
        let parts: Vec<&str> = small_line.split('|').collect();
        
        // Should have hash calculated (not "0")
        assert_ne!(parts[0], "0");
        assert_eq!(parts[0].len(), 64); // SHA256 hash
        
        // Should use ISO8601 timestamps
        assert!(parts[7].contains("T"));
    }

    #[test]
    fn test_generate_bodyfile_large_file_hash_skip() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("bodyfile_large.txt");
        
        // Create a large file (2MB)
        let test_dir = temp_dir.path().join("test");
        fs::create_dir(&test_dir).unwrap();
        let large_file = test_dir.join("large.bin");
        let large_data = vec![0u8; 2 * 1024 * 1024];
        fs::write(&large_file, &large_data).unwrap();
        
        // Create options with 1MB hash limit
        let mut options = HashMap::new();
        options.insert("bodyfile_calculate_hash".to_string(), "true".to_string());
        options.insert("bodyfile_hash_max_size_mb".to_string(), "1".to_string());
        
        // Generate bodyfile
        let result = generate_limited_bodyfile_with_options(&output_path, &test_dir, &options);
        assert!(result.is_ok());
        
        // Verify that large file has hash "0" (skipped)
        let content = fs::read_to_string(&output_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        
        let large_line = lines.iter()
            .find(|line| line.contains("large.bin"))
            .expect("large.bin not found in bodyfile");
        
        let parts: Vec<&str> = large_line.split('|').collect();
        assert_eq!(parts[0], "0"); // Hash should be skipped
    }

    #[test]
    fn test_platform_specific_times() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), b"test").unwrap();
        
        let metadata = fs::metadata(temp_file.path()).unwrap();
        let (ctime, crtime) = get_platform_specific_times(&metadata);
        
        // ctime should be non-zero on Unix platforms
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        assert!(ctime > 0);
        
        // crtime behavior varies by platform
        #[cfg(target_os = "linux")]
        assert_eq!(crtime, 0); // Linux doesn't track creation time
        
        #[cfg(target_os = "macos")]
        assert!(crtime >= 0); // macOS may have creation time
    }

    #[test]
    fn test_bodyfile_permissions() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("bodyfile_perms.txt");
        
        // Create files with different permissions
        let test_dir = temp_dir.path().join("test");
        fs::create_dir(&test_dir).unwrap();
        
        let file_400 = test_dir.join("readonly.txt");
        fs::write(&file_400, b"readonly").unwrap();
        fs::set_permissions(&file_400, fs::Permissions::from_mode(0o400)).unwrap();
        
        let file_666 = test_dir.join("readwrite.txt");
        fs::write(&file_666, b"readwrite").unwrap();
        fs::set_permissions(&file_666, fs::Permissions::from_mode(0o666)).unwrap();
        
        // Generate bodyfile
        let result = generate_limited_bodyfile(&output_path, &test_dir);
        assert!(result.is_ok());
        
        // Verify permissions in output
        let content = fs::read_to_string(&output_path).unwrap();
        
        // Check readonly file
        let readonly_line = content.lines()
            .find(|line| line.contains("readonly.txt"))
            .expect("readonly.txt not found");
        assert!(readonly_line.contains("-/r--------"));
        
        // Check readwrite file
        let readwrite_line = content.lines()
            .find(|line| line.contains("readwrite.txt"))
            .expect("readwrite.txt not found");
        assert!(readwrite_line.contains("-/rw-rw-rw-"));
    }
}
