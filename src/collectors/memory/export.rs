//! Memory export functionality
//!
//! This module handles exporting memory dumps to files and generating metadata.

use anyhow::{Result, Context};
use log::debug;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use chrono::Utc;
use serde_json;

use crate::collectors::memory::models::{
    ProcessMemoryInfo, MemoryRegionInfo, MemoryCollectionSummary, ProcessSummary,
};

/// Memory export handler
pub struct MemoryExporter {
    /// Base output directory
    base_dir: PathBuf,
}

impl MemoryExporter {
    /// Create a new memory exporter
    pub fn new(base_dir: impl AsRef<Path>) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    /// Export process memory information to a directory
    pub fn export_process_info(&self, process_info: &ProcessMemoryInfo) -> Result<PathBuf> {
        // Create process directory path: [base_dir]/[process_name]_[pid]
        let process_dir = self.base_dir.join(format!("{}_{}", process_info.name, process_info.pid));
        
        // Create the directory if it doesn't exist
        fs::create_dir_all(&process_dir)
            .context(format!("Failed to create process directory: {}", process_dir.display()))?;
        
        // Write process metadata to JSON file
        let metadata_path = process_dir.join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(process_info)
            .context("Failed to serialize process metadata to JSON")?;
        
        fs::write(&metadata_path, metadata_json)
            .context(format!("Failed to write process metadata to file: {}", metadata_path.display()))?;
        
        debug!("Exported process metadata to {}", metadata_path.display());
        
        // Return the process directory path
        Ok(process_dir)
    }

    /// Export memory region data to a file
    pub fn export_memory_region(
        &self,
        process_dir: impl AsRef<Path>,
        region: &MemoryRegionInfo,
        data: &[u8],
    ) -> Result<PathBuf> {
        let process_dir = process_dir.as_ref();
        
        // Create a filename for the region dump
        let region_type_str = match region.region_type {
            crate::collectors::memory::models::MemoryRegionType::Heap => "heap",
            crate::collectors::memory::models::MemoryRegionType::Stack => "stack",
            crate::collectors::memory::models::MemoryRegionType::Code => "code",
            crate::collectors::memory::models::MemoryRegionType::MappedFile => "mapped",
            crate::collectors::memory::models::MemoryRegionType::Other => "other",
        };
        
        let filename = format!(
            "{}_{:x}_{:x}.dmp",
            region_type_str,
            region.base_address,
            region.size
        );
        
        let dump_path = process_dir.join(&filename);
        
        // Write the memory data to the file
        let mut file = File::create(&dump_path)
            .context(format!("Failed to create memory dump file: {}", dump_path.display()))?;
        
        file.write_all(data)
            .context(format!("Failed to write memory data to file: {}", dump_path.display()))?;
        
        debug!(
            "Exported memory region ({:x}-{:x}) to {}",
            region.base_address,
            region.base_address + region.size,
            dump_path.display()
        );
        
        // Return the dump file path
        Ok(dump_path)
    }

    /// Export memory collection summary
    pub fn export_summary(&self, summary: &MemoryCollectionSummary) -> Result<PathBuf> {
        let summary_path = self.base_dir.join("memory_collection_summary.json");
        
        let summary_json = serde_json::to_string_pretty(summary)
            .context("Failed to serialize memory collection summary to JSON")?;
        
        fs::write(&summary_path, summary_json)
            .context(format!("Failed to write summary to file: {}", summary_path.display()))?;
        
        debug!("Exported memory collection summary to {}", summary_path.display());
        
        Ok(summary_path)
    }

    /// Create a memory map file for a process
    pub fn create_memory_map(
        &self,
        process_dir: impl AsRef<Path>,
        regions: &[MemoryRegionInfo],
    ) -> Result<PathBuf> {
        let process_dir = process_dir.as_ref();
        let memory_map_path = process_dir.join("memory_map.txt");
        
        let mut file = File::create(&memory_map_path)
            .context(format!("Failed to create memory map file: {}", memory_map_path.display()))?;
        
        // Write header
        writeln!(file, "Address Range                Size       Type       Permissions  Name")?;
        writeln!(file, "-------------------------- ------------ ---------- ------------ ----------------")?;
        
        // Write each region
        for region in regions {
            let end_address = region.base_address + region.size;
            
            let region_type = match region.region_type {
                crate::collectors::memory::models::MemoryRegionType::Heap => "Heap",
                crate::collectors::memory::models::MemoryRegionType::Stack => "Stack",
                crate::collectors::memory::models::MemoryRegionType::Code => "Code",
                crate::collectors::memory::models::MemoryRegionType::MappedFile => "Mapped",
                crate::collectors::memory::models::MemoryRegionType::Other => "Other",
            };
            
            let permissions = format!(
                "{}{}{}",
                if region.protection.read { "r" } else { "-" },
                if region.protection.write { "w" } else { "-" },
                if region.protection.execute { "x" } else { "-" },
            );
            
            let name = region.name.as_deref().unwrap_or("");
            
            writeln!(
                file,
                "{:016x}-{:016x} {:12} {:10} {:12} {}",
                region.base_address,
                end_address,
                region.size,
                region_type,
                permissions,
                name
            )?;
        }
        
        debug!("Created memory map at {}", memory_map_path.display());
        
        Ok(memory_map_path)
    }

    /// Create a process summary from process memory info
    pub fn create_process_summary(process_info: &ProcessMemoryInfo) -> ProcessSummary {
        let regions_dumped = process_info.regions.iter().filter(|r| r.dumped).count();
        
        ProcessSummary {
            pid: process_info.pid,
            name: process_info.name.clone(),
            region_count: process_info.regions.len(),
            regions_dumped,
            total_memory_size: process_info.total_memory_size,
            dumped_memory_size: process_info.dumped_memory_size,
            status: process_info.status.clone(),
        }
    }

    /// Create a collection summary
    pub fn create_collection_summary(
        processes: &[ProcessMemoryInfo],
        start_time: chrono::DateTime<Utc>,
        end_time: chrono::DateTime<Utc>,
    ) -> MemoryCollectionSummary {
        let processes_examined = processes.len();
        let processes_collected = processes.iter().filter(|p| p.status == "Success").count();
        let processes_skipped = processes.iter().filter(|p| p.status == "Skipped").count();
        let processes_failed = processes.iter().filter(|p| p.status == "Failed").count();
        
        let total_memory_collected = processes
            .iter()
            .map(|p| p.dumped_memory_size)
            .sum();
        
        let duration_seconds = (end_time - start_time).num_seconds() as f64;
        
        // Create process summaries
        let mut process_summaries = std::collections::HashMap::new();
        for process in processes {
            let summary = Self::create_process_summary(process);
            process_summaries.insert(format!("{}_{}", process.name, process.pid), summary);
        }
        
        MemoryCollectionSummary {
            processes_examined,
            processes_collected,
            processes_skipped,
            processes_failed,
            total_memory_collected,
            start_time: start_time.to_rfc3339(),
            end_time: end_time.to_rfc3339(),
            duration_seconds,
            process_summaries,
        }
    }
}
