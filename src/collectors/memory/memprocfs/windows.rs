//! Windows-specific MemProcFS initialization
//!
//! This module provides Windows-specific initialization for the MemProcFS collector.

use anyhow::{Context, Result};
use log::{debug, info, warn};
#[cfg(feature = "memory_collection")]
use memprocfs::*;

use super::collector::MemProcFSCollector;
use super::helpers::get_library_path;

/// Create a MemProcFSCollector for Windows
#[cfg(feature = "memory_collection")]
pub fn create_collector() -> Result<MemProcFSCollector> {
    let lib_path = get_library_path()?;

    info!(
        "Initializing Windows MemProcFSCollector with library: {}",
        lib_path
    );

    // Initialize MemProcFS with FPGA support for live systems
    let args = vec!["-printf", "-v", "-device", "FPGA"];
    let vmm = Vmm::new(&lib_path, &args).context("Failed to initialize MemProcFS")?;

    Ok(MemProcFSCollector::with_vmm(vmm))
}

#[cfg(not(feature = "memory_collection"))]
pub fn create_collector() -> Result<MemProcFSCollector> {
    anyhow::bail!(
        "Memory collection is not enabled. Recompile with the 'memory_collection' feature."
    );
}
