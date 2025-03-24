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
