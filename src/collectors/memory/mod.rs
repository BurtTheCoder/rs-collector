//! Process memory collection module
//!
//! This module provides functionality for collecting memory from running processes.
//! It supports dumping memory regions from processes based on various filters and
//! organizing the output in a structured format.

pub mod models;
pub mod filters;
pub mod export;
pub mod platforms;
pub mod collector;

// New memprocfs implementation
#[cfg(feature = "memory_collection")]
pub mod memprocfs;

use anyhow::Result;
use log::{info, warn};
use std::path::Path;

use crate::collectors::volatile::models::ProcessInfo;
use crate::collectors::memory::collector::MemoryCollector;
use crate::collectors::memory::models::MemoryCollectionSummary;

/// Collect process memory based on command-line arguments
pub fn collect_process_memory(
    processes: &[ProcessInfo],
    output_dir: impl AsRef<Path>,
    process_names: Option<&str>,
    process_ids: Option<&str>,
    include_system_processes: bool,
    max_memory_size_mb: usize,
    memory_regions: &str,
) -> Result<MemoryCollectionSummary> {
    // Create memory collector from arguments
    let collector = MemoryCollector::from_args(
        process_names,
        process_ids,
        include_system_processes,
        max_memory_size_mb,
        memory_regions,
    )?;
    
    // Create memory directory
    let memory_dir = output_dir.as_ref().join("process_memory");
    
    // Try to use MemProcFS collector first, fall back to platform-specific if needed
    #[cfg(feature = "memory_collection")]
    {
        match memprocfs::MemProcFSCollector::new() {
            Ok(_) => {
                info!("Using MemProcFS for memory collection");
                // The collector will use MemProcFS internally
            },
            Err(e) => {
                warn!("MemProcFS not available, falling back to platform-specific implementation: {}", e);
            }
        }
    }
    
    // Collect memory
    collector.collect_all(processes, memory_dir)
}

/// Check if memory collection is available on this platform
pub fn is_memory_collection_available() -> bool {
    #[cfg(feature = "memory_collection")]
    {
        // Try to create a MemProcFS collector
        match memprocfs::MemProcFSCollector::new() {
            Ok(_) => true,
            Err(e) => {
                warn!("MemProcFS memory collection is not available: {}", e);
                
                // Fall back to platform-specific implementation
                match platforms::get_memory_collector() {
                    Ok(_) => {
                        info!("Using legacy platform-specific memory collection");
                        true
                    },
                    Err(e) => {
                        warn!("Legacy memory collection is not available: {}", e);
                        false
                    }
                }
            }
        }
    }
    
    #[cfg(not(feature = "memory_collection"))]
    {
        warn!("Memory collection is not available: feature not enabled");
        false
    }
}
