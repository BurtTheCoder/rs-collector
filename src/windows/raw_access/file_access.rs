use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::ptr;
use std::cell::RefCell;

use anyhow::{Context, Result, anyhow};
use log::{debug, warn};
use widestring::U16CString;
use winapi::um::fileapi::{CreateFileW, ReadFile, OPEN_EXISTING};
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::winbase::{FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_SEQUENTIAL_SCAN};
use winapi::um::winnt::{FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_SHARE_DELETE, GENERIC_READ};
use winapi::shared::minwindef::{DWORD, FILETIME, LPVOID};

use crate::models::ArtifactMetadata;
use crate::windows::raw_access::directory::is_directory;
use crate::windows::raw_access::utils::filetime_to_iso8601;

/// Thread-local buffer for file operations to avoid repeated allocations
thread_local! {
    static FILE_BUFFER: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(8 * 1024 * 1024)); // 8MB max capacity
}

/// Check if the Windows Backup API is available
pub fn check_backup_api_available() -> bool {
    // Try to open a system file with backup semantics
    let test_path = "C:\\Windows\\System32\\ntdll.dll";
    let wide_path = match U16CString::from_str(test_path) {
        Ok(path) => path,
        Err(_) => return false,
    };
    
    // SAFETY: CreateFileW is safe to call with:
    // - Valid wide string pointer from to_wide_string
    // - GENERIC_READ for read-only access
    // - Full sharing mode to allow other processes to access the file
    // - null security attributes (default security)
    // - OPEN_EXISTING to only open existing files
    // - FILE_FLAG_BACKUP_SEMANTICS to open directories and use backup privileges
    // - null template file handle
    let handle = unsafe {
        CreateFileW(
            wide_path.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            ptr::null_mut(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            ptr::null_mut(),
        )
    };
    
    if handle == INVALID_HANDLE_VALUE {
        return false;
    }
    
    // Close the handle and return success
    // SAFETY: CloseHandle is safe to call with a valid handle from CreateFileW
    unsafe { CloseHandle(handle) };
    true
}

/// Determine optimal buffer size based on file type and size
fn get_optimal_buffer_size(file_path: &str) -> usize {
    // For large files like memory dumps, use larger buffers
    if file_path.contains("pagefile.sys") || 
       file_path.contains("hiberfil.sys") || 
       file_path.contains("memory") || 
       file_path.ends_with(".dmp") || 
       file_path.ends_with(".raw") {
        4 * 1024 * 1024 // 4MB for memory dumps
    } else if file_path.contains("$MFT") || file_path.contains("$LogFile") || file_path.contains("$UsnJrnl") {
        2 * 1024 * 1024 // 2MB for MFT and NTFS metadata
    } else if file_path.ends_with(".evt") || file_path.ends_with(".evtx") {
        1 * 1024 * 1024 // 1MB for event logs
    } else if file_path.contains("registry") || file_path.ends_with(".dat") || file_path.ends_with(".hive") {
        512 * 1024 // 512KB for registry hives
    } else {
        256 * 1024 // 256KB default for smaller files
    }
}

/// Collect a file using raw Windows file handle with backup semantics
pub fn collect_with_raw_handle(source_path: &str, dest_path: &Path) -> Result<ArtifactMetadata> {
    debug!("Collecting {} to {}", source_path, dest_path.display());
    
    // Check if the path is a directory
    if source_path.ends_with('\\') || is_directory(source_path)? {
        return crate::windows::raw_access::directory::collect_directory(source_path, dest_path);
    }
    
    // Convert the path to a wide string for Windows API
    let wide_path = match U16CString::from_str(source_path) {
        Ok(path) => path,
        Err(e) => return Err(anyhow!("Failed to convert path to wide string: {}", e)),
    };
    
    // Add file access hints for Windows to optimize IO
    #[cfg(target_os = "windows")]
    let file_flags = FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_SEQUENTIAL_SCAN;
    #[cfg(not(target_os = "windows"))]
    let file_flags = FILE_FLAG_BACKUP_SEMANTICS;

    // Open the source file with backup semantics
    let handle = unsafe {
        CreateFileW(
            wide_path.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            ptr::null_mut(),
            OPEN_EXISTING,
            file_flags,
            ptr::null_mut(),
        )
    };
    
    if handle == INVALID_HANDLE_VALUE {
        let err = io::Error::last_os_error();
        return Err(anyhow!("Failed to open file with backup semantics: {}", err));
    }
    
    // Initialize file time structures
    let mut creation_time = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let mut last_access_time = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let mut last_write_time = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    
    // Get file times
    let times_result = unsafe {
        winapi::um::fileapi::GetFileTime(
            handle,
            &mut creation_time,
            &mut last_access_time,
            &mut last_write_time,
        )
    };
    
    let created_time_str = if times_result != 0 {
        Some(filetime_to_iso8601(&creation_time))
    } else {
        warn!("Failed to get creation time: {}", io::Error::last_os_error());
        None
    };
    
    let accessed_time_str = if times_result != 0 {
        Some(filetime_to_iso8601(&last_access_time))
    } else {
        warn!("Failed to get access time: {}", io::Error::last_os_error());
        None
    };
    
    let modified_time_str = if times_result != 0 {
        Some(filetime_to_iso8601(&last_write_time))
    } else {
        warn!("Failed to get write time: {}", io::Error::last_os_error());
        None
    };
    
    // Create parent directories if they don't exist
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)
            .context(format!("Failed to create parent directories for {}", dest_path.display()))?;
    }
    
    // Create the destination file
    let mut dest_file = File::create(dest_path)
        .context(format!("Failed to create output file: {}", dest_path.display()))?;
    
    // Use thread-local buffer to avoid repeated allocations
    FILE_BUFFER.with(|buffer_cell| {
        let mut buffer = buffer_cell.borrow_mut();
        
        // Choose optimal buffer size based on file type
        let optimal_size = get_optimal_buffer_size(source_path);
        
        // Resize buffer if needed
        if buffer.len() < optimal_size {
            buffer.resize(optimal_size, 0);
        }
        
        let mut bytes_read: DWORD = 0;
        let mut total_bytes: u64 = 0;
        let mut is_locked = false;
        
        // Read from source and write to destination in chunks
        loop {
            let read_result = unsafe {
                ReadFile(
                    handle,
                    buffer.as_mut_ptr() as LPVOID,
                    optimal_size as DWORD,
                    &mut bytes_read,
                    ptr::null_mut(),
                )
            };
            
            if read_result == 0 {
                let err = io::Error::last_os_error();
                warn!("Error reading file {}: {}", source_path, err);
                is_locked = true;
                break;
            }
            
            if bytes_read == 0 {
                break; // End of file
            }
            
            if let Err(e) = dest_file.write_all(&buffer[0..bytes_read as usize]) {
                warn!("Error writing to {}: {}", dest_path.display(), e);
                is_locked = true;
                break;
            }
            
            total_bytes += bytes_read as u64;
        }
        
        // Close the handle before returning
        unsafe { CloseHandle(handle) };
        
        // Get current time for collection timestamp
        let collection_time = chrono::Utc::now().to_rfc3339();
        
        // Create metadata
        let metadata = ArtifactMetadata {
            original_path: source_path.to_string(),
            collection_time,
            file_size: total_bytes,
            created_time: created_time_str,
            accessed_time: accessed_time_str,
            modified_time: modified_time_str,
            is_locked,
        };
        
        Ok(metadata)
    })
}
