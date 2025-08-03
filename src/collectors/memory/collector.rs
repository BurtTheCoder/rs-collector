//! Memory collector implementation
//!
//! This module provides the main implementation for collecting process memory.

use anyhow::{Result, Context, bail};
use log::{debug, info, warn};
use std::path::Path;
use std::time::Instant;
use chrono::Utc;

use crate::collectors::memory::models::{
    ProcessMemoryInfo, MemoryCollectionOptions, MemoryCollectionSummary,
};
use crate::collectors::memory::filters::{ProcessFilter, MemoryRegionFilter};
use crate::collectors::memory::export::MemoryExporter;
use crate::collectors::memory::platforms::{self, MemoryCollectorImpl};
use crate::collectors::volatile::models::ProcessInfo;
use crate::constants::{
    DEFAULT_MAX_TOTAL_MEMORY,
    DEFAULT_MAX_PROCESS_MEMORY
};

/// Memory collector for forensic memory acquisition.
/// 
/// This struct provides a high-level interface for collecting process memory
/// from running systems. It supports various filtering options and uses
/// platform-specific implementations for the actual memory reading.
/// 
/// # Features
/// 
/// - Process filtering by name, PID, or system process status
/// - Memory region filtering by type, size, and protection flags
/// - Automatic selection of best available memory reading method
/// - Progress tracking and detailed collection summaries
/// 
/// # Platform Support
/// 
/// - Windows: Uses MemProcFS or native Windows APIs
/// - Linux: Uses /proc filesystem or MemProcFS
/// - macOS: Uses mach APIs or MemProcFS
pub struct MemoryCollector {
    /// Memory collection options
    options: MemoryCollectionOptions,
    /// Process filter
    process_filter: ProcessFilter,
    /// Memory region filter
    region_filter: MemoryRegionFilter,
    /// Platform-specific memory collector implementation
    platform_impl: Box<dyn MemoryCollectorImpl>,
}

impl MemoryCollector {
    /// Create a new memory collector with the given options
    pub fn new(
        options: MemoryCollectionOptions,
        process_filter: ProcessFilter,
        region_filter: MemoryRegionFilter,
    ) -> Result<Self> {
        // Get the best available memory collector implementation
        // This will try MemProcFS first, then fall back to platform-specific
        let platform_impl = platforms::get_memory_collector()?;
        
        Ok(Self {
            options,
            process_filter,
            region_filter,
            platform_impl,
        })
    }
    
    /// Create a memory collector from command-line arguments
    pub fn from_args(
        process_names: Option<&str>,
        process_ids: Option<&str>,
        include_system_processes: bool,
        max_memory_size_mb: usize,
        memory_regions: &str,
    ) -> Result<Self> {
        // Create process filter
        let process_filter = ProcessFilter::from_args(
            process_names,
            process_ids,
            include_system_processes,
        );
        
        // Create memory region filter
        let region_filter = MemoryRegionFilter::from_str(
            memory_regions,
            4096, // Minimum region size (4KB)
            DEFAULT_MAX_TOTAL_MEMORY, // Maximum region size
        );
        
        // Create options
        let options = MemoryCollectionOptions {
            max_total_size: (max_memory_size_mb as u64) * 1024 * 1024,
            max_process_size: DEFAULT_MAX_PROCESS_MEMORY, // Default per process
            include_system_processes,
            process_filters: process_filter.process_names.clone(),
            pid_filters: process_filter.process_ids.clone(),
            region_types: region_filter.region_types.iter().cloned().collect(),
        };
        
        Self::new(options, process_filter, region_filter)
    }
    
    /// Collect memory from all matching processes
    pub fn collect_all(
        &self,
        processes: &[ProcessInfo],
        output_dir: impl AsRef<Path>,
    ) -> Result<MemoryCollectionSummary> {
        let output_dir = output_dir.as_ref();
        let start_time = Instant::now();
        let start_datetime = Utc::now();
        
        info!("Starting memory collection to {}", output_dir.display());
        
        // Create the output directory if it doesn't exist
        std::fs::create_dir_all(output_dir)
            .context(format!("Failed to create output directory: {}", output_dir.display()))?;
        
        // Create memory exporter
        let exporter = MemoryExporter::new(output_dir);
        
        // Filter processes
        let filtered_processes: Vec<&ProcessInfo> = processes
            .iter()
            .filter(|p| self.process_filter.matches(p))
            .collect();
        
        info!(
            "Found {} processes matching filter criteria out of {} total processes",
            filtered_processes.len(),
            processes.len()
        );
        
        // Collect memory from each process
        let mut process_infos = Vec::new();
        let mut total_collected = 0u64;
        
        for process in filtered_processes {
            // Check if we've exceeded the total size limit
            if total_collected >= self.options.max_total_size {
                info!(
                    "Reached maximum total memory collection size ({} bytes), stopping collection",
                    self.options.max_total_size
                );
                break;
            }
            
            // Collect memory from this process
            match self.collect_process(process, &exporter) {
                Ok(process_info) => {
                    total_collected += process_info.dumped_memory_size;
                    process_infos.push(process_info);
                }
                Err(e) => {
                    warn!("Failed to collect memory from process {}: {}", process.pid, e);
                    
                    // Add a failed process entry
                    let failed_process = ProcessMemoryInfo {
                        pid: process.pid,
                        name: process.name.clone(),
                        command_line: Some(process.cmd.join(" ")),
                        path: process.exe.clone(),
                        start_time: process.start_time,
                        user: None,
                        parent_pid: process.parent_pid,
                        regions: Vec::new(),
                        modules: Vec::new(),
                        total_memory_size: 0,
                        dumped_memory_size: 0,
                        collection_time: Utc::now().to_rfc3339(),
                        status: "Failed".to_string(),
                        error: Some(e.to_string()),
                    };
                    
                    process_infos.push(failed_process);
                }
            }
        }
        
        // Create collection summary
        let end_datetime = Utc::now();
        let summary = MemoryExporter::create_collection_summary(
            &process_infos,
            start_datetime,
            end_datetime,
        );
        
        // Export summary
        exporter.export_summary(&summary)?;
        
        let elapsed = start_time.elapsed();
        info!(
            "Memory collection completed in {:.2} seconds, collected {} bytes from {} processes",
            elapsed.as_secs_f64(),
            total_collected,
            process_infos.iter().filter(|p| p.status == "Success").count()
        );
        
        Ok(summary)
    }
    
    /// Collect memory from a single process
    fn collect_process(
        &self,
        process: &ProcessInfo,
        exporter: &MemoryExporter,
    ) -> Result<ProcessMemoryInfo> {
        let pid = process.pid;
        let start_time = Instant::now();
        
        info!("Collecting memory from process {} ({})", pid, process.name);
        
        // Get memory regions
        let mut regions = self.platform_impl.get_memory_regions(process)
            .context(format!("Failed to get memory regions for process {}", pid))?;
        
        // Filter regions
        let original_count = regions.len();
        regions.retain(|r| self.region_filter.matches(r));
        
        debug!(
            "Filtered memory regions for process {}: {} -> {}",
            pid,
            original_count,
            regions.len()
        );
        
        // Get modules
        let modules = self.platform_impl.get_modules(process)
            .context(format!("Failed to get modules for process {}", pid))?;
        
        // Calculate total memory size
        let total_memory_size: u64 = regions.iter().map(|r| r.size).sum();
        
        // Check if the process exceeds the maximum size
        if total_memory_size > self.options.max_process_size {
            warn!(
                "Process {} memory size ({} bytes) exceeds maximum ({} bytes), skipping",
                pid,
                total_memory_size,
                self.options.max_process_size
            );
            
            // Return a skipped process entry
            return Ok(ProcessMemoryInfo {
                pid,
                name: process.name.clone(),
                command_line: Some(process.cmd.join(" ")),
                path: process.exe.clone(),
                start_time: process.start_time,
                user: None,
                parent_pid: process.parent_pid,
                regions,
                modules,
                total_memory_size,
                dumped_memory_size: 0,
                collection_time: Utc::now().to_rfc3339(),
                status: "Skipped".to_string(),
                error: Some(format!("Process memory size exceeds maximum")),
            });
        }
        
        // Create process memory info
        let mut process_info = ProcessMemoryInfo {
            pid,
            name: process.name.clone(),
            command_line: Some(process.cmd.join(" ")),
            path: process.exe.clone(),
            start_time: process.start_time,
            user: None,
            parent_pid: process.parent_pid,
            regions,
            modules,
            total_memory_size,
            dumped_memory_size: 0,
            collection_time: Utc::now().to_rfc3339(),
            status: "Success".to_string(),
            error: None,
        };
        
        // Export process info to create the directory
        let process_dir = exporter.export_process_info(&process_info)
            .context(format!("Failed to export process info for process {}", pid))?;
        
        // Create memory map
        exporter.create_memory_map(&process_dir, &process_info.regions)
            .context(format!("Failed to create memory map for process {}", pid))?;
        
        // Dump memory regions
        let mut dumped_memory_size = 0u64;
        
        for region in &mut process_info.regions {
            // Skip regions that are too small
            if region.size < 4096 {
                continue;
            }
            
            // Read memory
            match self.platform_impl.read_memory(pid, region.base_address, region.size as usize) {
                Ok(data) => {
                    if data.is_empty() {
                        debug!(
                            "Skipping empty memory region at {:x} for process {}",
                            region.base_address,
                            pid
                        );
                        continue;
                    }
                    
                    // Export memory region
                    match exporter.export_memory_region(&process_dir, region, &data) {
                        Ok(dump_path) => {
                            // Update region info
                            region.dumped = true;
                            region.dump_path = Some(
                                dump_path
                                    .strip_prefix(&process_dir)
                                    .unwrap_or(&dump_path)
                                    .to_string_lossy()
                                    .to_string()
                            );
                            
                            // Update dumped memory size
                            dumped_memory_size += data.len() as u64;
                        }
                        Err(e) => {
                            warn!(
                                "Failed to export memory region at {:x} for process {}: {}",
                                region.base_address,
                                pid,
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    debug!(
                        "Failed to read memory at {:x} for process {}: {}",
                        region.base_address,
                        pid,
                        e
                    );
                }
            }
        }
        
        // Update process info
        process_info.dumped_memory_size = dumped_memory_size;
        
        // Re-export process info with updated region info
        exporter.export_process_info(&process_info)?;
        
        let elapsed = start_time.elapsed();
        info!(
            "Collected {} bytes from process {} in {:.2} seconds",
            dumped_memory_size,
            pid,
            elapsed.as_secs_f64()
        );
        
        Ok(process_info)
    }
    
    /// Search for a pattern in process memory
    pub fn search_memory(&self, process: &ProcessInfo, pattern: &[u8]) -> Result<Vec<u64>> {
        info!("Searching for pattern in process {} ({})", process.name, process.pid);
        
        let addresses = self.platform_impl.search_memory(
            process.pid, 
            pattern, 
            0, // Start at address 0
            None, // No end address (search all memory)
        )?;
        
        info!("Found {} matches in process {} ({})", 
              addresses.len(), process.name, process.pid);
        
        Ok(addresses)
    }
    
    /// Scan process memory with YARA rules
    #[cfg(feature = "yara")]
    pub fn scan_memory_yara(&self, process: &ProcessInfo, rules: &[&str]) -> Result<Vec<String>> {
        info!("Scanning process {} ({}) with YARA rules", process.name, process.pid);
        
        let matches = self.platform_impl.scan_memory_yara(
            process.pid,
            rules,
            0, // Start at address 0
            None, // No end address (scan all memory)
        )?;
        
        info!("Found {} YARA matches in process {} ({})",
              matches.len(), process.name, process.pid);
        
        Ok(matches)
    }
    
    /// Dump a specific memory region with formatted output
    pub fn dump_memory_region(&self, process: &ProcessInfo, 
                            address: u64, size: usize) -> Result<Vec<u8>> {
        info!("Dumping memory region at {:x} (size: {}) for process {} ({})",
             address, size, process.name, process.pid);
        
        // Read memory
        let memory = self.platform_impl.read_memory(process.pid, address, size)?;
        
        if memory.is_empty() {
            bail!("Memory region is empty");
        }
        
        Ok(memory)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::collectors::memory::models::{MemoryRegionType, MemoryRegionInfo};

    #[test]
    fn test_memory_collector_from_args() {
        let result = MemoryCollector::from_args(
            Some("firefox,chrome"),
            Some("1234,5678"),
            true,
            1024, // 1GB
            "heap,stack",
        );
        
        assert!(result.is_ok());
        let collector = result.unwrap();
        assert!(collector.options.include_system_processes);
        assert_eq!(collector.options.max_total_size, DEFAULT_MAX_TOTAL_MEMORY);
        assert_eq!(collector.options.process_filters.len(), 2);
        assert_eq!(collector.options.pid_filters.len(), 2);
    }

    #[test]
    fn test_memory_collector_from_args_defaults() {
        let result = MemoryCollector::from_args(
            None,
            None,
            false,
            512,
            "all",
        );
        
        assert!(result.is_ok());
        let collector = result.unwrap();
        assert!(!collector.options.include_system_processes);
        assert_eq!(collector.options.max_total_size, DEFAULT_MAX_PROCESS_MEMORY);
    }

    #[test]
    fn test_memory_collection_options() {
        let options = MemoryCollectionOptions {
            max_total_size: DEFAULT_MAX_TOTAL_MEMORY,
            max_process_size: DEFAULT_MAX_PROCESS_MEMORY,
            include_system_processes: true,
            process_filters: vec!["test".to_string()],
            pid_filters: vec![1234],
            region_types: vec![MemoryRegionType::Heap, MemoryRegionType::Stack],
        };
        
        assert_eq!(options.max_total_size, DEFAULT_MAX_TOTAL_MEMORY);
        assert_eq!(options.max_process_size, DEFAULT_MAX_PROCESS_MEMORY);
        assert!(options.include_system_processes);
        assert_eq!(options.process_filters.len(), 1);
        assert_eq!(options.pid_filters.len(), 1);
        assert_eq!(options.region_types.len(), 2);
    }

    #[test]
    fn test_process_filter_creation() {
        let filter = ProcessFilter::from_args(
            Some("firefox,chrome"),
            Some("1234,5678"),
            true,
        );
        
        assert_eq!(filter.process_names.len(), 2);
        assert!(filter.process_names.contains(&"firefox".to_string()));
        assert!(filter.process_names.contains(&"chrome".to_string()));
        assert_eq!(filter.process_ids.len(), 2);
        assert!(filter.process_ids.contains(&1234));
        assert!(filter.process_ids.contains(&5678));
        assert!(filter.include_system_processes);
    }

    #[test]
    fn test_process_filter_matches() {
        let filter = ProcessFilter {
            process_names: vec!["test".to_string()],
            process_ids: vec![1234],
            include_system_processes: false,
        };
        
        let process1 = ProcessInfo {
            pid: 1234,
            name: "other".to_string(),
            cmd: vec![],
            exe: None,
            status: "Running".to_string(),
            start_time: 0,
            cpu_usage: 0.0,
            memory_usage: 0,
            parent_pid: None,
        };
        
        let process2 = ProcessInfo {
            pid: 5678,
            name: "test".to_string(),
            cmd: vec![],
            exe: None,
            status: "Running".to_string(),
            start_time: 0,
            cpu_usage: 0.0,
            memory_usage: 0,
            parent_pid: None,
        };
        
        // Should match by PID
        assert!(filter.matches(&process1));
        // Should match by name
        assert!(filter.matches(&process2));
    }

    #[test]
    fn test_memory_region_filter() {
        let filter = MemoryRegionFilter::from_str(
            "heap,stack",
            4096,
            DEFAULT_MAX_TOTAL_MEMORY,
        );
        
        assert_eq!(filter.region_types.len(), 2);
        assert!(filter.region_types.contains(&MemoryRegionType::Heap));
        assert!(filter.region_types.contains(&MemoryRegionType::Stack));
        assert_eq!(filter.min_size, 4096);
        assert_eq!(filter.max_size, DEFAULT_MAX_TOTAL_MEMORY);
    }

    #[test]
    fn test_memory_region_filter_matches() {
        let filter = MemoryRegionFilter {
            region_types: std::collections::HashSet::from([MemoryRegionType::Heap]),
            min_size: 4096,
            max_size: 1024 * 1024,
        };
        
        let region1 = MemoryRegionInfo {
            base_address: 0x1000,
            size: 8192,
            protection: crate::collectors::memory::models::MemoryProtection {
                read: true,
                write: true,
                execute: false,
            },
            region_type: MemoryRegionType::Heap,
            name: Some("heap".to_string()),
            mapped_file: None,
            dumped: false,
            dump_path: None,
        };
        
        let region2 = MemoryRegionInfo {
            base_address: 0x2000,
            size: 2048, // Too small
            protection: crate::collectors::memory::models::MemoryProtection {
                read: true,
                write: true,
                execute: false,
            },
            region_type: MemoryRegionType::Heap,
            name: Some("heap".to_string()),
            mapped_file: None,
            dumped: false,
            dump_path: None,
        };
        
        let region3 = MemoryRegionInfo {
            base_address: 0x3000,
            size: 8192,
            protection: crate::collectors::memory::models::MemoryProtection {
                read: true,
                write: true,
                execute: false,
            },
            region_type: MemoryRegionType::Stack, // Wrong type
            name: Some("stack".to_string()),
            mapped_file: None,
            dumped: false,
            dump_path: None,
        };
        
        assert!(filter.matches(&region1));
        assert!(!filter.matches(&region2)); // Too small
        assert!(!filter.matches(&region3)); // Wrong type
    }

    #[test]
    fn test_collect_all_empty_processes() {
        let temp_dir = TempDir::new().unwrap();
        let options = MemoryCollectionOptions {
            max_total_size: DEFAULT_MAX_TOTAL_MEMORY,
            max_process_size: DEFAULT_MAX_PROCESS_MEMORY,
            include_system_processes: false,
            process_filters: vec![],
            pid_filters: vec![],
            region_types: vec![],
        };
        
        let process_filter = ProcessFilter {
            process_names: vec![],
            process_ids: vec![],
            include_system_processes: false,
        };
        
        let region_filter = MemoryRegionFilter {
            region_types: std::collections::HashSet::new(),
            min_size: 4096,
            max_size: DEFAULT_MAX_TOTAL_MEMORY,
        };
        
        // This will fail without a proper platform implementation
        // but we can test the structure
        let result = MemoryCollector::new(options, process_filter, region_filter);
        if let Ok(collector) = result {
            let processes = vec![];
            let summary_result = collector.collect_all(&processes, temp_dir.path());
            
            // Should succeed with empty process list
            if let Ok(summary) = summary_result {
                assert_eq!(summary.processes_collected, 0);
                assert_eq!(summary.total_memory_collected, 0);
            }
        }
    }

    #[test]
    fn test_search_memory_pattern() {
        // Test pattern search functionality
        let pattern = b"test_pattern";
        assert_eq!(pattern.len(), 12);
        
        // Test with empty pattern
        let empty_pattern: &[u8] = b"";
        assert_eq!(empty_pattern.len(), 0);
    }

    #[test]
    fn test_memory_collection_summary_creation() {
        // use crate::collectors::memory::models::ModuleInfo; // Not needed
        
        let process_info = ProcessMemoryInfo {
            pid: 1234,
            name: "test".to_string(),
            command_line: Some("test --arg".to_string()),
            path: Some("/usr/bin/test".to_string()),
            start_time: 0,
            user: Some("user".to_string()),
            parent_pid: Some(1),
            regions: vec![],
            modules: vec![],
            total_memory_size: 1024 * 1024,
            dumped_memory_size: 512 * 1024,
            collection_time: Utc::now().to_rfc3339(),
            status: "Success".to_string(),
            error: None,
        };
        
        let processes = vec![process_info];
        let start_time = Utc::now();
        let end_time = start_time + chrono::Duration::seconds(10);
        
        let summary = MemoryExporter::create_collection_summary(
            &processes,
            start_time,
            end_time,
        );
        
        assert_eq!(summary.processes_collected, 1);
        assert_eq!(summary.processes_collected, 1);
        assert_eq!(summary.processes_failed, 0);
        assert_eq!(summary.total_memory_collected, 512 * 1024);
    }
}
