use anyhow::{Result, Context as AnyhowContext};
use log::{info, debug};
use std::path::Path;
use std::fs;
use sysinfo::{System, SystemExt, ProcessExt, DiskExt, NetworkExt, ProcessStatus, PidExt, CpuExt};

use crate::collectors::volatile::models::*;

/// Collector for volatile system data
pub struct VolatileDataCollector {
    system: System,
}

impl VolatileDataCollector {
    /// Create a new volatile data collector
    pub fn new() -> Self {
        info!("Initializing volatile data collector");
        let mut system = System::new_all();
        system.refresh_all();
        Self { system }
    }
    
    /// Collect all volatile data and save to the specified directory
    pub fn collect_all(&mut self, output_dir: impl AsRef<Path>) -> Result<VolatileDataSummary> {
        let output_dir = output_dir.as_ref();
        
        // Create the output directory if it doesn't exist
        fs::create_dir_all(output_dir)
            .context("Failed to create volatile data output directory")?;
        
        info!("Collecting volatile system data to {}", output_dir.display());
        
        // Refresh all system information
        self.system.refresh_all();
        
        // Collect and save system information
        let system_info = self.collect_system_info()?;
        self.save_to_json(&system_info, output_dir.join("system-info.json"))?;
        
        // Collect and save process information
        let processes = self.collect_processes()?;
        self.save_to_json(&processes, output_dir.join("processes.json"))?;
        
        // Collect and save network information
        let network = self.collect_network()?;
        self.save_to_json(&network, output_dir.join("network-connections.json"))?;
        
        // Collect and save memory information
        let memory = self.collect_memory()?;
        self.save_to_json(&memory, output_dir.join("memory.json"))?;
        
        // Collect and save disk information
        let disks = self.collect_disks()?;
        self.save_to_json(&disks, output_dir.join("disks.json"))?;
        
        // Create a summary for the collection summary
        let summary = VolatileDataSummary {
            system_name: system_info.hostname.clone(),
            os_version: system_info.os_version.clone(),
            cpu_count: system_info.cpu_info.count,
            total_memory_mb: memory.total_memory / 1024, // Convert KB to MB
            process_count: processes.len(),
            network_interface_count: network.interfaces.len(),
            disk_count: disks.len(),
        };
        
        info!("Volatile data collection completed successfully");
        Ok(summary)
    }
    
    /// Collect system information
    fn collect_system_info(&self) -> Result<SystemInfo> {
        debug!("Collecting system information");
        
        let cpu_info = CpuInfo {
            count: self.system.cpus().len(),
            vendor: None, // sysinfo doesn't provide CPU vendor
            brand: self.system.cpus().first().map(|cpu| cpu.brand().to_string()),
            frequency: self.system.cpus().first().map_or(0, |cpu| cpu.frequency()),
        };
        
        let system_info = SystemInfo {
            hostname: self.system.host_name(),
            os_name: self.system.name(),
            os_version: self.system.os_version(),
            kernel_version: self.system.kernel_version(),
            cpu_info,
        };
        
        Ok(system_info)
    }
    
    /// Collect process information
    fn collect_processes(&self) -> Result<Vec<ProcessInfo>> {
        debug!("Collecting process information");
        
        let mut processes = Vec::new();
        
        for (pid, process) in self.system.processes() {
            let status = match process.status() {
                ProcessStatus::Run => "Running",
                ProcessStatus::Sleep => "Sleeping",
                ProcessStatus::Stop => "Stopped",
                ProcessStatus::Zombie => "Zombie",
                ProcessStatus::Idle => "Idle",
                _ => "Unknown",
            };
            
            let process_info = ProcessInfo {
                pid: pid.as_u32(),
                name: process.name().to_string(),
                cmd: process.cmd().to_vec(),
                exe: Some(process.exe().to_string_lossy().to_string()),
                status: status.to_string(),
                start_time: process.start_time(),
                cpu_usage: process.cpu_usage(),
                memory_usage: process.memory(),
                parent_pid: process.parent().map(|p| p.as_u32()),
            };
            
            processes.push(process_info);
        }
        
        Ok(processes)
    }
    
    /// Collect network information
    fn collect_network(&mut self) -> Result<NetworkInfo> {
        debug!("Collecting network information");
        
        // Refresh network information
        self.system.refresh_networks_list();
        self.system.refresh_networks();
        
        let mut interfaces = Vec::new();
        
        // Collect network interfaces
        for (interface_name, data) in self.system.networks() {
            let interface = NetworkInterface {
                name: interface_name.to_string(),
                mac: None, // sysinfo doesn't provide MAC addresses
                ips: Vec::new(), // sysinfo doesn't provide IP addresses
                received_bytes: data.total_received(),
                transmitted_bytes: data.total_transmitted(),
            };
            
            interfaces.push(interface);
        }
        
        // For network connections, we would need platform-specific code
        // sysinfo doesn't provide network connection information directly
        let connections = Vec::new();
        
        let network_info = NetworkInfo {
            interfaces,
            connections,
        };
        
        Ok(network_info)
    }
    
    /// Collect memory information
    fn collect_memory(&mut self) -> Result<MemoryInfo> {
        debug!("Collecting memory information");
        
        // Refresh memory information
        self.system.refresh_memory();
        
        let memory_info = MemoryInfo {
            total_memory: self.system.total_memory(),
            used_memory: self.system.used_memory(),
            total_swap: self.system.total_swap(),
            used_swap: self.system.used_swap(),
        };
        
        Ok(memory_info)
    }
    
    /// Collect disk information
    fn collect_disks(&mut self) -> Result<Vec<DiskInfo>> {
        debug!("Collecting disk information");
        
        // Refresh disks list
        self.system.refresh_disks_list();
        
        let mut disks = Vec::new();
        
        for disk in self.system.disks() {
            // Convert file system bytes to string if possible
            let file_system = std::str::from_utf8(disk.file_system())
                .ok()
                .map(|s| s.to_string());
            
            let disk_info = DiskInfo {
                name: disk.name().to_string_lossy().to_string(),
                mount_point: Some(disk.mount_point().to_string_lossy().to_string()),
                total_space: disk.total_space(),
                available_space: disk.available_space(),
                file_system,
                is_removable: disk.is_removable(),
            };
            
            disks.push(disk_info);
        }
        
        Ok(disks)
    }
    
    /// Save data to a JSON file
    fn save_to_json<T: serde::Serialize>(&self, data: &T, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .context(format!("Failed to create directory: {}", parent.display()))?;
        }
        
        // Serialize to JSON with pretty formatting
        let json = serde_json::to_string_pretty(data)
            .context("Failed to serialize data to JSON")?;
        
        // Write to file
        fs::write(path, json)
            .context(format!("Failed to write data to file: {}", path.display()))?;
        
        debug!("Saved data to {}", path.display());
        Ok(())
    }
}
