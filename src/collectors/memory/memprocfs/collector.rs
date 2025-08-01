//! Unified MemProcFS memory collector implementation
//!
//! This module provides a cross-platform implementation for memory collection
//! using the MemProcFS library.

#[cfg(feature = "memory_collection")]
use anyhow::{Result, bail, Context, anyhow};
#[cfg(feature = "memory_collection")]
use log::{debug, info, error, warn};
#[cfg(feature = "memory_collection")]
use std::collections::HashMap;
#[cfg(feature = "memory_collection")]
use std::path::PathBuf;
#[cfg(feature = "memory_collection")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "memory_collection")]
use memprocfs::*;

#[cfg(feature = "memory_collection")]
use crate::collectors::memory::models::{
    MemoryRegionInfo, MemoryRegionType, MemoryProtection, ModuleInfo,
};
#[cfg(feature = "memory_collection")]
use crate::collectors::memory::platforms::MemoryCollectorImpl;
#[cfg(feature = "memory_collection")]
use crate::collectors::volatile::models::ProcessInfo;
#[cfg(feature = "memory_collection")]
use crate::constants::MEMORY_CHUNK_SIZE;
#[cfg(feature = "memory_collection")]
use super::helpers::*;

/// MemProcFS-based memory collector implementation
pub struct MemProcFSCollector {
    #[cfg(feature = "memory_collection")]
    vmm: Arc<Mutex<Vmm>>,
    #[cfg(feature = "memory_collection")]
    proc_cache: HashMap<u32, VmmProcess>,
}

impl MemoryCollectorImpl for MemProcFSCollector {
    fn new() -> Result<Self> {
        // Use platform-specific initialization
        // This is resolved at compile time to the correct platform implementation
        #[cfg(feature = "memory_collection")]
        {
            super::create_collector()
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
            let proc = self.get_process(pid)?;
            
            // Get memory regions using VAD map
            let vad_map = proc.map_vad(true).context("Failed to retrieve VAD map")?;
            let mut regions = Vec::new();
            
            for vad in &*vad_map {
                let region_type = convert_region_type(&vad);
                
                let protection = MemoryProtection {
                    read: vad.protection & VM_PROT_READ != 0,
                    write: vad.protection & VM_PROT_WRITE != 0,
                    execute: vad.protection & VM_PROT_EXECUTE != 0,
                };
                
                let region = MemoryRegionInfo {
                    base_address: vad.va_start,
                    size: vad.va_end - vad.va_start,
                    region_type,
                    protection,
                    name: Some(vad.filename.clone()),
                    mapped_file: if !vad.filename.is_empty() { Some(vad.filename.clone()) } else { None },
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
            let proc = self.get_process(pid)?;
            
            // Handle large memory regions with chunking
            if size > 1024 * 1024 * 10 { // 10MB threshold
                self.read_large_memory(&proc, address, size)
            } else {
                // Standard memory read with flags for better error handling
                match proc.mem_read_ex(address, size, FLAG_NOCACHE | FLAG_ZEROPAD_ON_FAIL) {
                    Ok(data) => {
                        debug!("Read {} bytes from address {:x} for process {}", data.len(), address, pid);
                        Ok(data)
                    },
                    Err(e) => {
                        bail!("Failed to read memory at address {:x} for process {}: {}", address, pid, e)
                    }
                }
            }
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
            let proc = self.get_process(pid)?;
            
            // Get modules using module map
            let module_map = proc.map_module(true, true).context("Failed to retrieve module map")?;
            let mut modules = Vec::new();
            
            for module in &*module_map {
                let info = ModuleInfo {
                    base_address: module.va_base,
                    size: module.size as u64,
                    path: module.path.clone(),
                    name: module.name.clone(),
                    // Extract version from version info if available
                    version: module.version_info.as_ref().map(|v| v.file_version.clone()),
                };
                
                modules.push(info);
            }
            
            debug!("Found {} modules for process {}", modules.len(), pid);
            
            Ok(modules)
        }
        
        #[cfg(not(feature = "memory_collection"))]
        {
            bail!("Memory collection is not enabled. Recompile with the 'memory_collection' feature.");
        }
    }
    
    // NEW CAPABILITY: Memory searching
    fn search_memory(&self, pid: u32, pattern: &[u8], start_addr: u64, end_addr: Option<u64>) -> Result<Vec<u64>> {
        #[cfg(feature = "memory_collection")]
        {
            let proc = self.get_process(pid)?;
            let end_addr = end_addr.unwrap_or(u64::MAX);
            
            // Create a memory search object
            let mut search = proc.search(start_addr, end_addr, 0x10000, 0)?;
            
            // Add search pattern
            let search_term_id = search.add_search(pattern);
            
            // Execute search and get results
            let result = search.result();
            
            // Convert results to vector of addresses
            let mut addresses = Vec::new();
            for (addr, term_id) in &*result.result {
                if *term_id == search_term_id {
                    addresses.push(*addr);
                }
            }
            
            Ok(addresses)
        }
        
        #[cfg(not(feature = "memory_collection"))]
        {
            bail!("Memory searching is only available with the memory_collection feature enabled");
        }
    }
    
    // NEW CAPABILITY: YARA scanning
    #[cfg(feature = "yara")]
    fn scan_memory_yara(&self, pid: u32, rules: &[&str], start_addr: u64, end_addr: Option<u64>) -> Result<Vec<String>> {
        #[cfg(feature = "memory_collection")]
        {
            let proc = self.get_process(pid)?;
            let end_addr = end_addr.unwrap_or(u64::MAX);
            
            // Create yara search object
            let yara_rules = rules.iter().map(|&s| s).collect();
            let mut yara = proc.search_yara(yara_rules, start_addr, end_addr, 0x10000, 0)?;
            
            // Execute yara scan
            let result = yara.result();
            
            // Convert results to vector of matches
            let mut matches = Vec::new();
            for rule_match in &*result.result {
                matches.push(format!("Rule: {} at addresses: {:?}", 
                    rule_match.rule_name, 
                    rule_match.match_strings.iter().flat_map(|ms| &ms.addresses).collect::<Vec<_>>()));
            }
            
            Ok(matches)
        }
        
        #[cfg(not(feature = "memory_collection"))]
        {
            bail!("YARA scanning is only available with the memory_collection feature enabled");
        }
    }
}

#[cfg(feature = "memory_collection")]
impl MemProcFSCollector {
    /// Create a new MemProcFSCollector with an existing VMM instance
    pub fn with_vmm(vmm: Vmm) -> Self {
        Self {
            vmm: Arc::new(Mutex::new(vmm)),
            proc_cache: HashMap::new(),
        }
    }
    
    /// Get a VmmProcess object for a process ID, using cache when possible
    fn get_process(&self, pid: u32) -> Result<VmmProcess> {
        if let Some(proc) = self.proc_cache.get(&pid) {
            return Ok(proc.clone());
        }
        
        let vmm = self.vmm.lock()
            .map_err(|e| anyhow!("Failed to acquire VMM lock: {}", e))?;
        match vmm.process_from_pid(pid) {
            Ok(proc) => {
                // This would need to be mutable to update cache
                // For now we just return the process
                Ok(proc)
            },
            Err(e) => bail!("Failed to get process {}: {}", pid, e),
        }
    }
    
    /// Read large memory regions in chunks to avoid allocation issues
    fn read_large_memory(&self, proc: &VmmProcess, address: u64, size: usize) -> Result<Vec<u8>> {
        const CHUNK_SIZE: usize = MEMORY_CHUNK_SIZE;
        let mut result = Vec::with_capacity(size);
        let mut failures = 0;
        
        debug!("Reading large memory region of {} bytes in chunks for process {}", 
               size, proc.pid);
        
        for chunk_offset in (0..size).step_by(CHUNK_SIZE) {
            let chunk_size = std::cmp::min(CHUNK_SIZE, size - chunk_offset);
            let chunk_addr = address + chunk_offset as u64;
            
            match proc.mem_read_ex(chunk_addr, chunk_size, FLAG_NOCACHE | FLAG_ZEROPAD_ON_FAIL) {
                Ok(data) => {
                    result.extend(data);
                },
                Err(e) => {
                    // Log partial failure but continue
                    debug!("Failed to read memory chunk at {:x}: {}", chunk_addr, e);
                    failures += 1;
                    
                    // If too many failures, abort
                    if failures > 5 {
                        warn!("Too many failures reading memory for process {}, aborting", proc.pid);
                        break;
                    }
                }
            }
        }
        
        if result.is_empty() {
            bail!("Failed to read any memory from address {:x} for process {}", address, proc.pid);
        }
        
        debug!("Read {} bytes total from large memory region for process {}", result.len(), proc.pid);
        
        Ok(result)
    }
}
