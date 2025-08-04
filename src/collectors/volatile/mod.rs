//! Volatile data collection module
//!
//! This module handles the collection of volatile system data such as:
//! - System information
//! - Running processes
//! - Network connections
//! - Memory usage
//! - Disk information
//!
//! The data is collected using the sysinfo crate and stored in JSON format.

mod collector;
pub mod models;

pub use collector::VolatileDataCollector;
// Used in main.rs
#[allow(unused_imports)]
pub use models::VolatileDataSummary;

// Convenience functions for collecting specific volatile data
use anyhow::Result;

/// Collect all volatile system data
pub async fn collect_volatile_data() -> Result<models::VolatileData> {
    let mut collector = VolatileDataCollector::new();

    Ok(models::VolatileData {
        system_info: collector.collect_system_info()?,
        processes: collector.collect_processes()?,
        network: collector.collect_network()?,
        memory: collector.collect_memory()?,
        disks: collector.collect_disks()?,
    })
}

/// Collect process information
pub async fn collect_processes() -> Result<Vec<models::ProcessInfo>> {
    let collector = VolatileDataCollector::new();
    collector.collect_processes()
}
