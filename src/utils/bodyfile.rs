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
    match Utc.timestamp_opt(timestamp as i64, 0) {
        chrono::LocalResult::Single(dt) => dt.to_rfc3339(),
        _ => "0000-00-00T00:00:00Z".to_string() // Invalid timestamp
    }
}

/// Generate a bodyfile for the filesystem with advanced options
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
