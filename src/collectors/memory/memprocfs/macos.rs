//! macOS-specific MemProcFS initialization
//!
//! This module provides macOS-specific initialization for the MemProcFS collector.

use anyhow::{Result, Context};
use log::{debug, warn, info};
#[cfg(feature = "memory_collection")]
use memprocfs::*;

use super::collector::MemProcFSCollector;
use super::helpers::get_library_path;

/// Create a MemProcFSCollector for macOS
#[cfg(feature = "memory_collection")]
pub fn create_collector() -> Result<MemProcFSCollector> {
    let lib_path = get_library_path()?;
    
    info!("Initializing macOS MemProcFSCollector with library: {}", lib_path);
    
    // Check if we're running as root, which is required for task_for_pid on macOS
    let is_root = unsafe { libc::geteuid() == 0 };
    if !is_root {
        warn!("Memory collection on macOS requires root privileges for full access");
    }
    
    // For live systems, we use the PMEM device
    let args = vec!["-printf", "-v", "-device", "PMEM"];
    let vmm = Vmm::new(&lib_path, &args)
        .context("Failed to initialize MemProcFS")?;
    
    Ok(MemProcFSCollector::with_vmm(vmm))
}

#[cfg(not(feature = "memory_collection"))]
pub fn create_collector() -> Result<MemProcFSCollector> {
    anyhow::bail!("Memory collection is not enabled. Recompile with the 'memory_collection' feature.");
}
