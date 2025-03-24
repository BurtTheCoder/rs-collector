//! Memory collector implementation
//!
//! This module provides the main implementation for collecting process memory.

use anyhow::{Result, Context, bail};
use log::{debug, info, warn, error};
use std::path::{Path, PathBuf};
use std::time::Instant;
use chrono::Utc;
use std::collections::HashMap;

use crate::collectors::memory::models::{
    ProcessMemoryInfo, MemoryCollectionOptions, MemoryCollectionSummary,
};
use crate::collectors::memory::filters::{ProcessFilter, MemoryRegionFilter};
use crate::collectors::memory::export::MemoryExporter;
use crate::collectors::memory::platforms::{self, MemoryCollectorImpl};
use crate::collectors::volatile::models::ProcessInfo;

/// Memory collector
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
        // Try to use MemProcFS implementation first
        #[cfg(feature = "memory_collection")]
        {
            match crate::collectors::memory::memprocfs::MemProcFSCollector::new() {
                Ok(impl_collector) => {
                    info!("Using MemProcFS for memory collection");
                    return Ok(Self {
                        options,
                        process_filter,
                        region_filter,
                        platform_impl: Box::new(impl_collector),
                    });
                },
                Err(e) => {
                    warn!("MemProcFS not available, falling back to platform-specific implementation: {}", e);
                }
            }
        }
        
        // Fall back to platform-specific implementation
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
            1024 * 1024 * 1024, // Maximum region size (1GB)
        );
        
        // Create options
        let options = MemoryCollectionOptions {
            max_total_size: (max_memory_size_mb as u64) * 1024 * 1024,
            max_process_size: 512 * 1024 * 1024, // 512MB per process
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
