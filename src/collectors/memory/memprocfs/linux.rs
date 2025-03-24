//! Linux-specific MemProcFS initialization
//!
//! This module provides Linux-specific initialization for the MemProcFS collector.

use anyhow::{Result, Context};
use log::{debug, warn, info};
use std::path::Path;
#[cfg(feature = "memory_collection")]
use memprocfs::*;

use super::collector::MemProcFSCollector;
use super::helpers::get_library_path;

/// Create a MemProcFSCollector for Linux
#[cfg(feature = "memory_collection")]
pub fn create_collector() -> Result<MemProcFSCollector> {
    let lib_path = get_library_path()?;
    
    info!("Initializing Linux MemProcFSCollector with library: {}", lib_path);
    
    // Check if we have access to /proc
    if !Path::new("/proc").exists() {
        warn!("Cannot access /proc filesystem. Memory collection may be limited.");
    }
    
    // For live systems, we use the PROC device
    let args = vec!["-printf", "-v", "-device", "PROC"];
    let vmm = Vmm::new(&lib_path, &args)
        .context("Failed to initialize MemProcFS")?;
    
    Ok(MemProcFSCollector::with_vmm(vmm))
}

#[cfg(not(feature = "memory_collection"))]
pub fn create_collector() -> Result<MemProcFSCollector> {
    anyhow::bail!("Memory collection is not enabled. Recompile with the 'memory_collection' feature.");
}
