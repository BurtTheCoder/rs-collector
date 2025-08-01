//! Data models for process memory collection
//!
//! This module defines the data structures used for process memory collection,
//! including memory regions, process information, and collection summaries.

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Memory region type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryRegionType {
    /// Heap memory
    Heap,
    /// Stack memory
    Stack,
    /// Code/executable memory
    Code,
    /// Memory-mapped files
    MappedFile,
    /// Other memory regions
    Other,
}

impl std::fmt::Display for MemoryRegionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryRegionType::Heap => write!(f, "Heap"),
            MemoryRegionType::Stack => write!(f, "Stack"),
            MemoryRegionType::Code => write!(f, "Code"),
            MemoryRegionType::MappedFile => write!(f, "MappedFile"),
            MemoryRegionType::Other => write!(f, "Other"),
        }
    }
}

/// Memory protection flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryProtection {
    /// Read permission
    pub read: bool,
    /// Write permission
    pub write: bool,
    /// Execute permission
    pub execute: bool,
}

/// Memory region information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRegionInfo {
    /// Base address of the memory region
    pub base_address: u64,
    /// Size of the memory region in bytes
    pub size: u64,
    /// Type of memory region
    pub region_type: MemoryRegionType,
    /// Memory protection flags
    pub protection: MemoryProtection,
    /// Name or description of the region (e.g., module name)
    pub name: Option<String>,
    /// Path to the file if memory-mapped
    pub mapped_file: Option<String>,
    /// Whether the region was successfully dumped
    pub dumped: bool,
    /// Path to the dump file (relative to process directory)
    pub dump_path: Option<String>,
}

/// Module information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    /// Base address of the module
    pub base_address: u64,
    /// Size of the module in bytes
    pub size: u64,
    /// Path to the module file
    pub path: String,
    /// Module name
    pub name: String,
    /// Module version information
    pub version: Option<String>,
}

/// Process memory information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMemoryInfo {
    /// Process ID
    pub pid: u32,
    /// Process name
    pub name: String,
    /// Full command line
    pub command_line: Option<String>,
    /// Process path
    pub path: Option<String>,
    /// Process start time
    pub start_time: u64,
    /// User that owns the process
    pub user: Option<String>,
    /// Parent process ID
    pub parent_pid: Option<u32>,
    /// Memory regions
    pub regions: Vec<MemoryRegionInfo>,
    /// Loaded modules
    pub modules: Vec<ModuleInfo>,
    /// Total memory size in bytes
    pub total_memory_size: u64,
    /// Total dumped memory size in bytes
    pub dumped_memory_size: u64,
    /// Collection timestamp
    pub collection_time: String,
    /// Collection status
    pub status: String,
    /// Error message if collection failed
    pub error: Option<String>,
}

/// Memory collection options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCollectionOptions {
    /// Maximum total memory size to collect (in bytes)
    pub max_total_size: u64,
    /// Maximum size per process (in bytes)
    pub max_process_size: u64,
    /// Whether to include system processes
    pub include_system_processes: bool,
    /// Process name filters (if empty, all processes are included)
    pub process_filters: Vec<String>,
    /// Process ID filters (if empty, all processes are included)
    pub pid_filters: Vec<u32>,
    /// Memory region types to collect
    pub region_types: Vec<MemoryRegionType>,
}

impl Default for MemoryCollectionOptions {
    fn default() -> Self {
        Self {
            max_total_size: 4 * 1024 * 1024 * 1024, // 4 GB
            max_process_size: 512 * 1024 * 1024,    // 512 MB
            include_system_processes: false,
            process_filters: Vec::new(),
            pid_filters: Vec::new(),
            region_types: vec![
                MemoryRegionType::Heap,
                MemoryRegionType::Stack,
                MemoryRegionType::Code,
            ],
        }
    }
}

/// Memory collection summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCollectionSummary {
    /// Number of processes examined
    pub processes_examined: usize,
    /// Number of processes collected
    pub processes_collected: usize,
    /// Number of processes skipped
    pub processes_skipped: usize,
    /// Number of processes that failed
    pub processes_failed: usize,
    /// Total memory size collected (in bytes)
    pub total_memory_collected: u64,
    /// Collection start time
    pub start_time: String,
    /// Collection end time
    pub end_time: String,
    /// Collection duration in seconds
    pub duration_seconds: f64,
    /// Process summaries
    pub process_summaries: HashMap<String, ProcessSummary>,
}

/// Process summary for the collection summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSummary {
    /// Process ID
    pub pid: u32,
    /// Process name
    pub name: String,
    /// Number of memory regions
    pub region_count: usize,
    /// Number of memory regions dumped
    pub regions_dumped: usize,
    /// Total memory size (in bytes)
    pub total_memory_size: u64,
    /// Dumped memory size (in bytes)
    pub dumped_memory_size: u64,
    /// Collection status
    pub status: String,
}
