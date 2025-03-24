//! Platform-specific memory collection implementations
//!
//! This module provides platform-specific implementations for memory collection.

use anyhow::Result;
use std::path::Path;

use crate::collectors::memory::models::{ProcessMemoryInfo, MemoryRegionInfo};
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
pub fn get_memory_collector() -> Result<Box<dyn MemoryCollectorImpl>> {
    #[cfg(target_os = "windows")]
    {
        Ok(Box::new(windows::WindowsMemoryCollector::new()?))
    }
    
    #[cfg(target_os = "linux")]
    {
        Ok(Box::new(linux::LinuxMemoryCollector::new()?))
    }
    
    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(macos::MacOSMemoryCollector::new()?))
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        anyhow::bail!("Unsupported platform for memory collection");
    }
}
