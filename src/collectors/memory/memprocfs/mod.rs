//! MemProcFS-based memory collection
//!
//! This module provides unified memory collection across all platforms
//! using the MemProcFS library.

pub mod collector;
mod helpers;

// Platform-specific initialization modules
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

// Re-export collector
pub use collector::MemProcFSCollector;

// Platform-specific initialization functions
#[cfg(target_os = "linux")]
pub use linux::create_collector;
#[cfg(target_os = "macos")]
pub use macos::create_collector;
#[cfg(target_os = "windows")]
pub use windows::create_collector;
