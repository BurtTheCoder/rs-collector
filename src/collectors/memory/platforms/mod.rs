//! Platform-specific memory collection implementations
//!
//! This module provides platform-specific implementations for memory collection.

use anyhow::Result;

use crate::collectors::memory::models::MemoryRegionInfo;
use crate::collectors::volatile::models::ProcessInfo;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

/// Memory collector trait for platform-specific implementations
pub trait MemoryCollectorImpl: Send + Sync {
    /// Initialize the memory collector
    fn new() -> Result<Self> where Self: Sized;
    
    /// Get memory regions for a process
    fn get_memory_regions(&self, process: &ProcessInfo) -> Result<Vec<MemoryRegionInfo>>;
    
    /// Read memory from a process
    fn read_memory(&self, pid: u32, address: u64, size: usize) -> Result<Vec<u8>>;
    
    /// Get loaded modules for a process
    fn get_modules(&self, process: &ProcessInfo) -> Result<Vec<crate::collectors::memory::models::ModuleInfo>>;
    
    /// Search for a pattern in process memory (default implementation)
    fn search_memory(&self, _pid: u32, _pattern: &[u8], _start_addr: u64, 
                    _end_addr: Option<u64>) -> Result<Vec<u64>> {
        anyhow::bail!("Memory searching not implemented for this platform")
    }
    
    /// YARA scan of process memory (default implementation)
    #[cfg(feature = "yara")]
    fn scan_memory_yara(&self, _pid: u32, _rules: &[&str], _start_addr: u64,
                      _end_addr: Option<u64>) -> Result<Vec<String>> {
        anyhow::bail!("YARA scanning not implemented for this platform")
    }
}

/// Get the appropriate memory collector implementation for the current platform
/// 
/// This function attempts to use MemProcFS first if available, then falls back
/// to platform-specific implementations.
pub fn get_memory_collector() -> Result<Box<dyn MemoryCollectorImpl>> {
    // Try MemProcFS first if the feature is enabled
    #[cfg(feature = "memory_collection")]
    {
        use crate::collectors::memory::memprocfs::MemProcFSCollector;
        
        match MemProcFSCollector::new() {
            Ok(collector) => {
                log::info!("Using MemProcFS for memory collection");
                return Ok(Box::new(collector));
            }
            Err(e) => {
                log::debug!("MemProcFS initialization failed, falling back to platform-specific: {}", e);
            }
        }
    }
    
    // Fall back to platform-specific implementations
    #[cfg(target_os = "windows")]
    {
        log::info!("Using Windows native memory collection");
        Ok(Box::new(windows::WindowsMemoryCollector::new()?))
    }
    
    #[cfg(target_os = "linux")]
    {
        log::info!("Using Linux /proc-based memory collection");
        Ok(Box::new(linux::LinuxMemoryCollector::new()?))
    }
    
    #[cfg(target_os = "macos")]
    {
        log::info!("Using macOS mach_vm memory collection");
        Ok(Box::new(macos::MacOSMemoryCollector::new()?))
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        anyhow::bail!("Unsupported platform for memory collection");
    }
}
