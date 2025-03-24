//! Windows-specific memory collection implementation
//!
//! This module provides Windows-specific implementation for memory collection
//! using the MemProcFS library.

use anyhow::{Result, Context, bail};
use log::{debug, info, warn, error};
use std::collections::HashMap;

#[cfg(feature = "memory_collection")]
use memprocfs::{Vmm, VmmProcess};

use crate::collectors::memory::models::{
    MemoryRegionInfo, MemoryRegionType, MemoryProtection, ModuleInfo,
};
use crate::collectors::memory::platforms::MemoryCollectorImpl;
use crate::collectors::volatile::models::ProcessInfo;

/// Windows memory collector implementation
pub struct WindowsMemoryCollector {
    #[cfg(feature = "memory_collection")]
    vmm: Vmm,
}

impl MemoryCollectorImpl for WindowsMemoryCollector {
    fn new() -> Result<Self> {
        #[cfg(feature = "memory_collection")]
        {
            // Initialize MemProcFS
            let vmm = Vmm::initialize(None)
                .context("Failed to initialize MemProcFS")?;
            
            info!("Initialized MemProcFS for memory collection");
            
            Ok(Self { vmm })
        }
        
        #[cfg(not(feature = "memory_collection"))]
        {
            bail!("Memory collection is not enabled. Recompile with the 'memory_collection' feature.");
        }
    }
    
    fn get_memory_regions(&self, process: &ProcessInfo) -> Result<Vec<MemoryRegionInfo>> {
        #[cfg(feature = "memory_collection")]
        {
            let pid = process.pid;
            
            // Get process from MemProcFS
            let vmm_process = self.vmm.process_get(pid as u32)
                .context(format!("Failed to get process {} from MemProcFS", pid))?;
            
            // Get memory map
            let memory_map = vmm_process.map_vad()
                .context(format!("Failed to get memory map for process {}", pid))?;
            
            let mut regions = Vec::new();
            
            for entry in memory_map {
                // Determine region type
                let region_type = if entry.tag == "Heap" {
                    MemoryRegionType::Heap
                } else if entry.tag == "Stack" {
                    MemoryRegionType::Stack
                } else if entry.protection & 0x10 != 0 { // IMAGE flag
                    MemoryRegionType::Code
                } else if !entry.file_name.is_empty() {
                    MemoryRegionType::MappedFile
                } else {
                    MemoryRegionType::Other
                };
                
                // Parse protection flags
                let protection = MemoryProtection {
                    read: entry.protection & 0x1 != 0,    // PAGE_READONLY or PAGE_READWRITE
                    write: entry.protection & 0x2 != 0,   // PAGE_READWRITE
                    execute: entry.protection & 0x4 != 0, // PAGE_EXECUTE*
                };
                
                // Create region info
                let region = MemoryRegionInfo {
                    base_address: entry.va_start,
                    size: entry.va_end - entry.va_start,
                    region_type,
                    protection,
                    name: if !entry.tag.is_empty() {
                        Some(entry.tag)
                    } else {
                        None
                    },
                    mapped_file: if !entry.file_name.is_empty() {
                        Some(entry.file_name)
                    } else {
                        None
                    },
                    dumped: false,
                    dump_path: None,
                };
                
                regions.push(region);
            }
            
            debug!("Found {} memory regions for process {}", regions.len(), pid);
            
            Ok(regions)
        }
        
        #[cfg(not(feature = "memory_collection"))]
        {
            bail!("Memory collection is not enabled. Recompile with the 'memory_collection' feature.");
        }
    }
    
    fn read_memory(&self, pid: u32, address: u64, size: usize) -> Result<Vec<u8>> {
        #[cfg(feature = "memory_collection")]
        {
            // Get process from MemProcFS
            let vmm_process = self.vmm.process_get(pid)
                .context(format!("Failed to get process {} from MemProcFS", pid))?;
            
            // Read memory
            let mut buffer = vec![0u8; size];
            let bytes_read = vmm_process.mem_read(address, &mut buffer)
                .context(format!("Failed to read memory at address {:x} for process {}", address, pid))?;
            
            // Resize buffer to actual bytes read
            buffer.truncate(bytes_read);
            
            debug!("Read {} bytes from address {:x} for process {}", bytes_read, address, pid);
            
            Ok(buffer)
        }
        
        #[cfg(not(feature = "memory_collection"))]
        {
            bail!("Memory collection is not enabled. Recompile with the 'memory_collection' feature.");
        }
    }
    
    fn get_modules(&self, process: &ProcessInfo) -> Result<Vec<ModuleInfo>> {
        #[cfg(feature = "memory_collection")]
        {
            let pid = process.pid;
            
            // Get process from MemProcFS
            let vmm_process = self.vmm.process_get(pid as u32)
                .context(format!("Failed to get process {} from MemProcFS", pid))?;
            
            // Get modules
            let modules = vmm_process.map_module()
                .context(format!("Failed to get modules for process {}", pid))?;
            
            let mut module_infos = Vec::new();
            
            for module in modules {
                let module_info = ModuleInfo {
                    base_address: module.base,
                    size: module.size as u64,
                    path: module.path,
                    name: module.name,
                    version: None, // Could be populated from version info if needed
                };
                
                module_infos.push(module_info);
            }
            
            debug!("Found {} modules for process {}", module_infos.len(), pid);
            
            Ok(module_infos)
        }
        
        #[cfg(not(feature = "memory_collection"))]
        {
            bail!("Memory collection is not enabled. Recompile with the 'memory_collection' feature.");
        }
    }
}
