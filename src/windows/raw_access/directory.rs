use std::path::Path;
use std::ptr;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use log::{debug, warn};
use widestring::U16CString;
use winapi::um::fileapi::CreateFileW;
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::winbase::FILE_FLAG_BACKUP_SEMANTICS;
use winapi::um::winnt::{FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_SHARE_DELETE};
use winapi::shared::minwindef::FILETIME;

use crate::models::ArtifactMetadata;
use crate::windows::raw_access::file_access::collect_with_raw_handle;

/// Check if a path is a directory
pub fn is_directory(path: &str) -> Result<bool> {
    // Convert the path to a wide string for Windows API
    let wide_path = match U16CString::from_str(path) {
        Ok(path) => path,
        Err(e) => return Err(anyhow!("Failed to convert path to wide string: {}", e)),
    };
    
    // Open the file to check its attributes
    let handle = unsafe {
        CreateFileW(
            wide_path.as_ptr(),
            0, // No access needed, just checking attributes
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            ptr::null_mut(),
            winapi::um::fileapi::OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS, // Need this to open directories
            ptr::null_mut(),
        )
    };
    
    if handle == INVALID_HANDLE_VALUE {
        let err = std::io::Error::last_os_error();
        return Err(anyhow!("Failed to open path to check attributes: {}", err));
    }
    
    // Get file information
    let mut file_info = winapi::um::minwinbase::BY_HANDLE_FILE_INFORMATION {
        dwFileAttributes: 0,
        ftCreationTime: FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 },
        ftLastAccessTime: FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 },
        ftLastWriteTime: FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 },
        dwVolumeSerialNumber: 0,
        nFileSizeHigh: 0,
        nFileSizeLow: 0,
        nNumberOfLinks: 0,
        nFileIndexHigh: 0,
        nFileIndexLow: 0,
    };
    
    let get_info_result = unsafe {
        winapi::um::fileapi::GetFileInformationByHandle(
            handle,
            &mut file_info,
        )
    };
    
    // Close the handle
    unsafe { CloseHandle(handle) };
    
    if get_info_result == 0 {
        let err = std::io::Error::last_os_error();
        return Err(anyhow!("Failed to get file information: {}", err));
    }
    
    // Check directory attribute
    Ok((file_info.dwFileAttributes & winapi::um::winnt::FILE_ATTRIBUTE_DIRECTORY) != 0)
}

/// Parallel collector for directory traversal
/// Uses a thread pool to collect files in parallel
pub fn collect_directory(source_path: &str, dest_path: &Path) -> Result<ArtifactMetadata> {
    debug!("Collecting directory {} to {}", source_path, dest_path.display());
    
    // Create the destination directory
    std::fs::create_dir_all(dest_path)
        .map_err(|e| anyhow!("Failed to create destination directory: {}: {}", dest_path.display(), e))?;
    
    // Prepare for directory scanning
    let mut pattern = source_path.to_string();
    if !pattern.ends_with('\\') {
        pattern.push('\\');
    }
    pattern.push_str("*");
    
    // Convert the path to a wide string for Windows API
    let wide_pattern = match U16CString::from_str(&pattern) {
        Ok(path) => path,
        Err(e) => return Err(anyhow!("Failed to convert pattern to wide string: {}", e)),
    };
    
    // Find first file
    let mut find_data = winapi::um::minwinbase::WIN32_FIND_DATAW {
        dwFileAttributes: 0,
        ftCreationTime: FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 },
        ftLastAccessTime: FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 },
        ftLastWriteTime: FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 },
        nFileSizeHigh: 0,
        nFileSizeLow: 0,
        dwReserved0: 0,
        dwReserved1: 0,
        cFileName: [0; 260],
        cAlternateFileName: [0; 14],
    };
    
    let find_handle = unsafe {
        winapi::um::fileapi::FindFirstFileW(
            wide_pattern.as_ptr(),
            &mut find_data,
        )
    };
    
    if find_handle == winapi::um::handleapi::INVALID_HANDLE_VALUE {
        let err = std::io::Error::last_os_error();
        return Err(anyhow!("Failed to start directory enumeration: {}", err));
    }
    
    // Collect all files and directories to process
    // We'll process directories first, then files
    let mut directories = Vec::new();
    let mut files = Vec::new();
    
    // Process files
    loop {
        // Convert filename to Rust string
        let filename = unsafe {
            let len = (0..260).position(|i| find_data.cFileName[i] == 0).unwrap_or(260);
            let slice = &find_data.cFileName[0..len];
            let os_str = OsString::from_wide(slice);
            os_str.to_string_lossy().into_owned()
        };
        
        // Skip . and ..
        if filename != "." && filename != ".." {
            let child_source = format!("{}\\{}", source_path.trim_end_matches('\\'), filename);
            let child_dest = dest_path.join(&filename);
            
            // Sort entries into directories and files
            if (find_data.dwFileAttributes & winapi::um::winnt::FILE_ATTRIBUTE_DIRECTORY) != 0 {
                directories.push((child_source, child_dest));
            } else {
                files.push((child_source, child_dest));
            }
        }
        
        // Find next file
        let find_next_result = unsafe {
            winapi::um::fileapi::FindNextFileW(
                find_handle,
                &mut find_data,
            )
        };
        
        if find_next_result == 0 {
            let err_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
            if err_code == winapi::shared::winerror::ERROR_NO_MORE_FILES {
                break; // No more files, we're done
            } else {
                let err = std::io::Error::from_raw_os_error(err_code as i32);
                warn!("Error during directory enumeration: {}", err);
                break;
            }
        }
    }
    
    // Close the find handle
    unsafe { winapi::um::findfiles::FindClose(find_handle) };
    
    // Create atomic counters for parallel processing
    let total_files = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let total_bytes = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let is_locked = Arc::new(std::sync::atomic::AtomicBool::new(false));
    
    // Process directories sequentially to create structure first
    for (dir_src, dir_dest) in directories {
        match collect_directory(&dir_src, &dir_dest) {
            Ok(metadata) => {
                total_files.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                total_bytes.fetch_add(metadata.file_size, std::sync::atomic::Ordering::SeqCst);
                if metadata.is_locked {
                    is_locked.store(true, std::sync::atomic::Ordering::SeqCst);
                }
            }
            Err(e) => {
                warn!("Failed to collect directory {}: {}", dir_src, e);
                is_locked.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        }
    }
    
    // Process files in parallel using crossbeam
    // Only use multithreading if we have enough files to make it worthwhile
    if files.len() > 8 {
        // Use crossbeam scoped threads for parallel file collection
        let scope_result = crossbeam::scope(|scope| {
            // Determine optimal thread count based on CPU cores and file count
            let thread_count = std::cmp::min(
                files.len(),
                std::cmp::min(num_cpus::get() * 2, 16) // Max 16 threads or 2x CPU cores
            );
            
            // Split files among threads
            let chunks = files.chunks((files.len() + thread_count - 1) / thread_count);
            
            // Spawn threads to process file chunks
            let handles: Vec<_> = chunks.map(|chunk| {
                let chunk_files = chunk.to_vec();
                let total_files = total_files.clone();
                let total_bytes = total_bytes.clone();
                let is_locked_flag = is_locked.clone();
                
                scope.spawn(move |_| {
                    for (file_src, file_dest) in chunk_files {
                        match collect_with_raw_handle(&file_src, &file_dest) {
                            Ok(metadata) => {
                                total_files.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                total_bytes.fetch_add(metadata.file_size, std::sync::atomic::Ordering::SeqCst);
                                if metadata.is_locked {
                                    is_locked_flag.store(true, std::sync::atomic::Ordering::SeqCst);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to collect file {}: {}", file_src, e);
                                is_locked_flag.store(true, std::sync::atomic::Ordering::SeqCst);
                            }
                        }
                    }
                })
            }).collect();
            
            // Wait for all threads to complete
            for handle in handles {
                let _ = handle.join();
            }
        });
        
        if let Err(e) = scope_result {
            warn!("Error in parallel file collection: {:?}", e);
            is_locked.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    } else {
        // Process files sequentially for small sets
        for (file_src, file_dest) in files {
            match collect_with_raw_handle(&file_src, &file_dest) {
                Ok(metadata) => {
                    total_files.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    total_bytes.fetch_add(metadata.file_size, std::sync::atomic::Ordering::SeqCst);
                    if metadata.is_locked {
                        is_locked.store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                }
                Err(e) => {
                    warn!("Failed to collect file {}: {}", file_src, e);
                    is_locked.store(true, std::sync::atomic::Ordering::SeqCst);
                }
            }
        }
    }
    
    // Get the final counts
    let files_count = total_files.load(std::sync::atomic::Ordering::SeqCst);
    let bytes_count = total_bytes.load(std::sync::atomic::Ordering::SeqCst);
    let locked_status = is_locked.load(std::sync::atomic::Ordering::SeqCst);
    
    // Get current time for collection timestamp
    let collection_time = chrono::Utc::now().to_rfc3339();
    
    // Get the collection time for the directory itself (we don't have actual creation time)
    let now = chrono::Utc::now().to_rfc3339();
    
    // Create metadata
    let metadata = ArtifactMetadata {
        original_path: source_path.to_string(),
        collection_time,
        file_size: bytes_count,
        created_time: Some(now.clone()),
        accessed_time: Some(now.clone()),
        modified_time: Some(now),
        is_locked: locked_status,
    };
    
    debug!("Collected directory with {} files, {} bytes", files_count, bytes_count);
    
    Ok(metadata)
}
