//! Linux-specific memory collection implementation
//!
//! This module provides Linux-specific implementation for memory collection
//! using the /proc filesystem.

use anyhow::{anyhow, Context, Result};
use log::{debug, info, warn};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use crate::collectors::memory::models::{
    MemoryProtection, MemoryRegionInfo, MemoryRegionType, ModuleInfo,
};
use crate::collectors::memory::platforms::MemoryCollectorImpl;
use crate::collectors::volatile::models::ProcessInfo;
use crate::constants::MEMORY_CHUNK_SIZE;

/// Linux memory collector implementation
pub struct LinuxMemoryCollector {
    // No state needed for Linux implementation
}

// Regular methods outside of the trait implementation
impl LinuxMemoryCollector {
    /// Internal implementation of memory reading
    fn read_memory_internal(&self, pid: u32, address: u64, size: usize) -> Result<Vec<u8>> {
        let mem_path = format!("/proc/{}/mem", pid);

        // Open the process memory file with improved error handling
        let mut file = match File::open(&mem_path) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                return Err(anyhow!("Permission denied when accessing process memory for pid {}. Run as root or adjust ptrace_scope.", pid));
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Err(anyhow!(
                    "Process {} no longer exists or is not accessible",
                    pid
                ));
            }
            Err(e) => {
                return Err(anyhow!(
                    "Failed to open memory file for process {}: {}",
                    pid,
                    e
                ));
            }
        };

        // Seek to the specified address
        if let Err(e) = file.seek(SeekFrom::Start(address)) {
            return Err(anyhow!(
                "Failed to seek to address {:x} for process {}: {}",
                address,
                pid,
                e
            ));
        }

        // Read the memory
        let mut buffer = vec![0u8; size];
        let bytes_read = match file.read(&mut buffer) {
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::InvalidInput => {
                // This can happen for special memory regions like vsyscall
                debug!(
                    "Invalid memory region at {:x} for process {}: {}",
                    address, pid, e
                );
                return Ok(Vec::new());
            }
            Err(e) => {
                return Err(anyhow!(
                    "Failed to read memory at address {:x} for process {}: {}",
                    address,
                    pid,
                    e
                ));
            }
        };

        // Resize buffer to actual bytes read
        buffer.truncate(bytes_read);

        debug!(
            "Read {} bytes from address {:x} for process {}",
            bytes_read, address, pid
        );

        Ok(buffer)
    }

    /// Read large memory regions in chunks to avoid allocation issues
    fn read_large_memory(&self, pid: u32, address: u64, size: usize) -> Result<Vec<u8>> {
        const CHUNK_SIZE: usize = MEMORY_CHUNK_SIZE;
        let mut result = Vec::with_capacity(size);
        let mut failures = 0;

        debug!(
            "Reading large memory region of {} bytes in chunks for process {}",
            size, pid
        );

        for chunk_offset in (0..size).step_by(CHUNK_SIZE) {
            let chunk_size = std::cmp::min(CHUNK_SIZE, size - chunk_offset);
            let chunk_addr = address + chunk_offset as u64;

            match self.read_memory_internal(pid, chunk_addr, chunk_size) {
                Ok(data) => {
                    result.extend(data);
                }
                Err(e) => {
                    // Log partial failure but continue
                    debug!("Failed to read memory chunk at {:x}: {}", chunk_addr, e);
                    failures += 1;

                    // If too many failures, abort
                    if failures > 5 {
                        warn!(
                            "Too many failures reading memory for process {}, aborting",
                            pid
                        );
                        break;
                    }
                }
            }
        }

        if result.is_empty() {
            return Err(anyhow!(
                "Failed to read any memory from address {:x} for process {}",
                address,
                pid
            ));
        }

        debug!(
            "Read {} bytes total from large memory region for process {}",
            result.len(),
            pid
        );

        Ok(result)
    }
}

impl MemoryCollectorImpl for LinuxMemoryCollector {
    fn new() -> Result<Self> {
        // Check if we have access to /proc
        if !Path::new("/proc").exists() {
            return Err(anyhow!(
                "Cannot access /proc filesystem. Memory collection requires /proc to be mounted."
            ));
        }

        info!("Initialized Linux memory collector");

        Ok(Self {})
    }

    fn get_memory_regions(&self, process: &ProcessInfo) -> Result<Vec<MemoryRegionInfo>> {
        let pid = process.pid;
        let maps_path = format!("/proc/{}/maps", pid);

        // Read the memory maps file
        let maps_content = fs::read_to_string(&maps_path)
            .context(format!("Failed to read memory maps for process {}", pid))?;

        let mut regions = Vec::new();

        // Parse each line of the maps file
        for line in maps_content.lines() {
            // Example line:
            // 55d3195fc000-55d319619000 r--p 00000000 08:05 1048602 /usr/bin/bash

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            // Parse address range
            let addr_range = parts[0];
            let addrs: Vec<&str> = addr_range.split('-').collect();
            if addrs.len() != 2 {
                warn!(
                    "Invalid address range format in /proc/{}/maps: {}",
                    pid, addr_range
                );
                continue;
            }

            let start_addr = u64::from_str_radix(addrs[0], 16)
                .context(format!("Failed to parse start address: {}", addrs[0]))?;

            let end_addr = u64::from_str_radix(addrs[1], 16)
                .context(format!("Failed to parse end address: {}", addrs[1]))?;

            // Parse permissions
            let perms = if parts.len() > 1 { parts[1] } else { "" };
            let protection = MemoryProtection {
                read: perms.contains('r'),
                write: perms.contains('w'),
                execute: perms.contains('x'),
            };

            // Get mapped file path if available
            let mapped_file = if parts.len() >= 6 {
                Some(parts[5..].join(" "))
            } else {
                None
            };

            // Determine region type
            let region_type = if let Some(path) = &mapped_file {
                if path.ends_with(".so") || path.contains(".so.") || path.contains("/lib/") {
                    MemoryRegionType::Code
                } else if path.contains("[heap]") {
                    MemoryRegionType::Heap
                } else if path.contains("[stack]") {
                    MemoryRegionType::Stack
                } else if !path.is_empty() {
                    MemoryRegionType::MappedFile
                } else {
                    MemoryRegionType::Other
                }
            } else {
                // Heuristics for anonymous memory
                if perms.contains('x') {
                    MemoryRegionType::Code
                } else {
                    MemoryRegionType::Other
                }
            };

            // Create region info
            let region = MemoryRegionInfo {
                base_address: start_addr,
                size: end_addr - start_addr,
                region_type,
                protection,
                name: mapped_file.clone(),
                mapped_file,
                dumped: false,
                dump_path: None,
            };

            regions.push(region);
        }

        debug!("Found {} memory regions for process {}", regions.len(), pid);

        Ok(regions)
    }

    fn read_memory(&self, pid: u32, address: u64, size: usize) -> Result<Vec<u8>> {
        // For large memory regions, use chunked reading
        if size > 1024 * 1024 * 10 {
            // 10MB threshold
            return self.read_large_memory(pid, address, size);
        }

        self.read_memory_internal(pid, address, size)
    }

    fn get_modules(&self, process: &ProcessInfo) -> Result<Vec<ModuleInfo>> {
        let pid = process.pid;
        let maps_path = format!("/proc/{}/maps", pid);

        // Read the memory maps file
        let maps_content = fs::read_to_string(&maps_path)
            .context(format!("Failed to read memory maps for process {}", pid))?;

        let mut modules: HashMap<String, ModuleInfo> = HashMap::new();

        // Parse each line of the maps file to find modules
        for line in maps_content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 6 {
                continue;
            }

            // Get the mapped file path
            let path = parts[5..].join(" ");

            // Skip non-library and non-executable files
            if !path.ends_with(".so")
                && !path.contains(".so.")
                && !path.contains("/bin/")
                && !path.contains("/lib/")
            {
                continue;
            }

            // Parse address range
            let addr_range = parts[0];
            let addrs: Vec<&str> = addr_range.split('-').collect();
            if addrs.len() != 2 {
                continue;
            }

            let start_addr = u64::from_str_radix(addrs[0], 16)?;
            let end_addr = u64::from_str_radix(addrs[1], 16)?;

            // Extract module name from path
            let name = Path::new(&path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Skip if we already have this module with a smaller base address
            if let Some(existing) = modules.get(&name) {
                if existing.base_address <= start_addr {
                    continue;
                }
            }

            // Create module info
            let module = ModuleInfo {
                base_address: start_addr,
                size: end_addr - start_addr,
                path: path.clone(),
                name,
                version: None,
            };

            modules.insert(module.name.clone(), module);
        }

        let module_list: Vec<ModuleInfo> = modules.into_values().collect();
        debug!("Found {} modules for process {}", module_list.len(), pid);

        Ok(module_list)
    }
}
