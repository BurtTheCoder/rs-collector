//! Filters for process memory collection
//!
//! This module provides filtering capabilities for processes and memory regions
//! to control which data is collected during memory acquisition.

use crate::collectors::memory::models::{MemoryRegionType, MemoryRegionInfo};
use crate::collectors::volatile::models::ProcessInfo;
use std::collections::HashSet;

/// Process filter for memory collection
#[derive(Debug, Clone)]
pub struct ProcessFilter {
    /// Process names to include (if empty, all processes are included)
    pub process_names: Vec<String>,
    /// Process IDs to include (if empty, all processes are included)
    pub process_ids: Vec<u32>,
    /// Whether to include system processes
    pub include_system_processes: bool,
}

impl ProcessFilter {
    /// Create a new process filter
    pub fn new(
        process_names: Vec<String>,
        process_ids: Vec<u32>,
        include_system_processes: bool,
    ) -> Self {
        Self {
            process_names,
            process_ids,
            include_system_processes,
        }
    }

    /// Create a process filter from command-line arguments
    pub fn from_args(
        process_names_str: Option<&str>,
        process_ids_str: Option<&str>,
        include_system_processes: bool,
    ) -> Self {
        let process_names = process_names_str
            .map(|s| s.split(',').map(|name| name.trim().to_string()).collect())
            .unwrap_or_default();

        let process_ids = process_ids_str
            .map(|s| {
                s.split(',')
                    .filter_map(|id| id.trim().parse::<u32>().ok())
                    .collect()
            })
            .unwrap_or_default();

        Self::new(process_names, process_ids, include_system_processes)
    }

    /// Check if a process matches the filter
    pub fn matches(&self, process: &ProcessInfo) -> bool {
        // If both filters are empty, include all processes
        let empty_filters = self.process_names.is_empty() && self.process_ids.is_empty();

        // Check if the process is a system process
        let is_system_process = Self::is_system_process(process);
        if is_system_process && !self.include_system_processes {
            return false;
        }

        // If filters are empty, include based on system process setting
        if empty_filters {
            return true;
        }

        // Check if the process name matches any of the filters
        if !self.process_names.is_empty() && self.process_names.iter().any(|name| {
            process.name.to_lowercase().contains(&name.to_lowercase())
        }) {
            return true;
        }

        // Check if the process ID matches any of the filters
        if !self.process_ids.is_empty() && self.process_ids.contains(&process.pid) {
            return true;
        }

        // If we have filters but no matches, exclude the process
        false
    }

    /// Check if a process is a system process
    fn is_system_process(process: &ProcessInfo) -> bool {
        // Common system process names
        const SYSTEM_PROCESS_NAMES: [&str; 15] = [
            "system", "smss", "csrss", "wininit", "services", "lsass", "svchost",
            "winlogon", "explorer", "dwm", "fontdrvhost", "runtimebroker",
            "systemd", "init", "kernel",
        ];

        // Check if the process name matches any of the system process names
        let name_lower = process.name.to_lowercase();
        SYSTEM_PROCESS_NAMES.iter().any(|&sys_name| name_lower == sys_name)
            // Check for common system process patterns
            || name_lower.starts_with("system")
            || name_lower.ends_with("d") && name_lower.len() <= 8 // Common daemon naming pattern
            || process.pid < 10 // Very low PIDs are typically system processes
    }
}

/// Memory region filter for memory collection
#[derive(Debug, Clone)]
pub struct MemoryRegionFilter {
    /// Memory region types to include
    pub region_types: HashSet<MemoryRegionType>,
    /// Minimum region size in bytes
    pub min_size: u64,
    /// Maximum region size in bytes
    pub max_size: u64,
}

impl MemoryRegionFilter {
    /// Create a new memory region filter
    pub fn new(
        region_types: Vec<MemoryRegionType>,
        min_size: u64,
        max_size: u64,
    ) -> Self {
        Self {
            region_types: region_types.into_iter().collect(),
            min_size,
            max_size,
        }
    }

    /// Create a memory region filter from a comma-separated string of region types
    pub fn from_str(region_types_str: &str, min_size: u64, max_size: u64) -> Self {
        let region_types = if region_types_str.to_lowercase() == "all" {
            vec![
                MemoryRegionType::Heap,
                MemoryRegionType::Stack,
                MemoryRegionType::Code,
                MemoryRegionType::MappedFile,
                MemoryRegionType::Other,
            ]
        } else {
            region_types_str
                .split(',')
                .filter_map(|s| match s.trim().to_lowercase().as_str() {
                    "heap" => Some(MemoryRegionType::Heap),
                    "stack" => Some(MemoryRegionType::Stack),
                    "code" => Some(MemoryRegionType::Code),
                    "mapped" | "mappedfile" => Some(MemoryRegionType::MappedFile),
                    "other" => Some(MemoryRegionType::Other),
                    _ => None,
                })
                .collect()
        };

        Self::new(region_types, min_size, max_size)
    }

    /// Check if a memory region matches the filter
    pub fn matches(&self, region: &MemoryRegionInfo) -> bool {
        // Check if the region type is included
        if !self.region_types.contains(&region.region_type) {
            return false;
        }

        // Check if the region size is within the limits
        if region.size < self.min_size || (self.max_size > 0 && region.size > self.max_size) {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_filter_matches() {
        let process = ProcessInfo {
            pid: 1234,
            name: "test_process".to_string(),
            cmd: vec!["test_process".to_string(), "--arg".to_string()],
            exe: Some("/usr/bin/test_process".to_string()),
            status: "Running".to_string(),
            start_time: 0,
            cpu_usage: 0.0,
            memory_usage: 0,
            parent_pid: None,
        };

        // Test empty filter (should match all)
        let filter = ProcessFilter::new(vec![], vec![], true);
        assert!(filter.matches(&process));

        // Test name filter
        let filter = ProcessFilter::new(vec!["test".to_string()], vec![], true);
        assert!(filter.matches(&process));

        // Test PID filter
        let filter = ProcessFilter::new(vec![], vec![1234], true);
        assert!(filter.matches(&process));

        // Test non-matching filter
        let filter = ProcessFilter::new(vec!["other".to_string()], vec![5678], true);
        assert!(!filter.matches(&process));
    }

    #[test]
    fn test_memory_region_filter_matches() {
        let region = MemoryRegionInfo {
            base_address: 0x1000,
            size: 4096,
            region_type: MemoryRegionType::Heap,
            protection: crate::collectors::memory::models::MemoryProtection {
                read: true,
                write: true,
                execute: false,
            },
            name: None,
            mapped_file: None,
            dumped: false,
            dump_path: None,
        };

        // Test with matching type and size
        let filter = MemoryRegionFilter::new(vec![MemoryRegionType::Heap], 1024, 8192);
        assert!(filter.matches(&region));

        // Test with non-matching type
        let filter = MemoryRegionFilter::new(vec![MemoryRegionType::Stack], 1024, 8192);
        assert!(!filter.matches(&region));

        // Test with non-matching size (too small)
        let filter = MemoryRegionFilter::new(vec![MemoryRegionType::Heap], 8192, 16384);
        assert!(!filter.matches(&region));

        // Test with non-matching size (too large)
        let filter = MemoryRegionFilter::new(vec![MemoryRegionType::Heap], 1024, 2048);
        assert!(!filter.matches(&region));
    }
}
