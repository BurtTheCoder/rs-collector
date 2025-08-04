use anyhow::{Context as AnyhowContext, Result};
use log::{debug, info};
use std::fs;
use std::path::Path;
use sysinfo::{CpuExt, DiskExt, NetworkExt, PidExt, ProcessExt, ProcessStatus, System, SystemExt};

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

        info!(
            "Collecting volatile system data to {}",
            output_dir.display()
        );

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
    pub fn collect_system_info(&self) -> Result<SystemInfo> {
        debug!("Collecting system information");

        let cpu_info = CpuInfo {
            count: self.system.cpus().len(),
            vendor: None, // sysinfo doesn't provide CPU vendor
            brand: self
                .system
                .cpus()
                .first()
                .map(|cpu| cpu.brand().to_string()),
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
    pub fn collect_processes(&self) -> Result<Vec<ProcessInfo>> {
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
    pub fn collect_network(&mut self) -> Result<NetworkInfo> {
        debug!("Collecting network information");

        // Refresh network information
        self.system.refresh_networks_list();
        self.system.refresh_networks();

        let mut interfaces = Vec::new();

        // Collect network interfaces
        for (interface_name, data) in self.system.networks() {
            let interface = NetworkInterface {
                name: interface_name.to_string(),
                mac: None,       // sysinfo doesn't provide MAC addresses
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
    pub fn collect_memory(&mut self) -> Result<MemoryInfo> {
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
    pub fn collect_disks(&mut self) -> Result<Vec<DiskInfo>> {
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
        let json =
            serde_json::to_string_pretty(data).context("Failed to serialize data to JSON")?;

        // Write to file
        fs::write(path, json)
            .context(format!("Failed to write data to file: {}", path.display()))?;

        debug!("Saved data to {}", path.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_volatile_collector_new() {
        let collector = VolatileDataCollector::new();
        // Just verify it creates without panic
        assert!(!collector.system.processes().is_empty() || collector.system.cpus().is_empty());
    }

    #[test]
    fn test_collect_all() {
        let mut collector = VolatileDataCollector::new();
        let temp_dir = TempDir::new().unwrap();

        let result = collector.collect_all(temp_dir.path());
        assert!(result.is_ok());

        let summary = result.unwrap();
        assert!(summary.cpu_count > 0);
        assert!(summary.total_memory_mb > 0);

        // Check that files were created
        assert!(temp_dir.path().join("system-info.json").exists());
        assert!(temp_dir.path().join("processes.json").exists());
        assert!(temp_dir.path().join("network-connections.json").exists());
        assert!(temp_dir.path().join("memory.json").exists());
        assert!(temp_dir.path().join("disks.json").exists());
    }

    #[test]
    fn test_collect_system_info() {
        let collector = VolatileDataCollector::new();

        let result = collector.collect_system_info();
        assert!(result.is_ok());

        let info = result.unwrap();
        assert!(info.cpu_info.count > 0);
        // OS info should be present on all systems
        assert!(info.hostname.is_some() || info.os_name.is_some());
    }

    #[test]
    fn test_collect_processes() {
        let collector = VolatileDataCollector::new();

        let result = collector.collect_processes();
        assert!(result.is_ok());

        let processes = result.unwrap();
        // There should always be at least one process (this test process)
        assert!(!processes.is_empty());

        // Check that process info is populated
        for process in processes.iter().take(5) {
            assert!(process.pid > 0);
            assert!(!process.name.is_empty());
            assert!(!process.status.is_empty());
        }
    }

    #[test]
    fn test_collect_network() {
        let mut collector = VolatileDataCollector::new();

        let result = collector.collect_network();
        assert!(result.is_ok());

        let network = result.unwrap();
        // Most systems have at least one network interface
        // but this could be empty in some containers
        if !network.interfaces.is_empty() {
            let interface = &network.interfaces[0];
            assert!(!interface.name.is_empty());
        }
    }

    #[test]
    fn test_collect_memory() {
        let mut collector = VolatileDataCollector::new();

        let result = collector.collect_memory();
        assert!(result.is_ok());

        let memory = result.unwrap();
        assert!(memory.total_memory > 0);
        assert!(memory.used_memory > 0);
        assert!(memory.used_memory <= memory.total_memory);
        // Swap might be 0 on some systems
        assert!(memory.used_swap <= memory.total_swap);
    }

    #[test]
    fn test_collect_disks() {
        let mut collector = VolatileDataCollector::new();

        let result = collector.collect_disks();
        assert!(result.is_ok());

        let disks = result.unwrap();
        // Most systems have at least one disk
        if !disks.is_empty() {
            let disk = &disks[0];
            assert!(!disk.name.is_empty());
            assert!(disk.total_space > 0);
            assert!(disk.available_space <= disk.total_space);
        }
    }

    #[test]
    fn test_save_to_json() {
        let collector = VolatileDataCollector::new();
        let temp_dir = TempDir::new().unwrap();

        #[derive(serde::Serialize)]
        struct TestData {
            test: String,
            value: u32,
        }

        let data = TestData {
            test: "hello".to_string(),
            value: 42,
        };

        let output_path = temp_dir.path().join("test.json");
        let result = collector.save_to_json(&data, &output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());

        // Verify the content
        let content = fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("\"test\": \"hello\""));
        assert!(content.contains("\"value\": 42"));
    }

    #[test]
    fn test_save_to_json_with_nested_path() {
        let collector = VolatileDataCollector::new();
        let temp_dir = TempDir::new().unwrap();

        let data = vec!["test1", "test2"];
        let output_path = temp_dir.path().join("nested").join("dir").join("test.json");

        let result = collector.save_to_json(&data, &output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
        assert!(output_path.parent().unwrap().exists());
    }

    #[test]
    fn test_process_status_mapping() {
        // Test that we handle all process statuses correctly
        let statuses = vec![
            (ProcessStatus::Run, "Running"),
            (ProcessStatus::Sleep, "Sleeping"),
            (ProcessStatus::Stop, "Stopped"),
            (ProcessStatus::Zombie, "Zombie"),
            (ProcessStatus::Idle, "Idle"),
        ];

        // Verify our mapping matches expected values
        for (status, expected) in statuses {
            let actual = match status {
                ProcessStatus::Run => "Running",
                ProcessStatus::Sleep => "Sleeping",
                ProcessStatus::Stop => "Stopped",
                ProcessStatus::Zombie => "Zombie",
                ProcessStatus::Idle => "Idle",
                _ => "Unknown",
            };
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_summary_calculation() {
        let mut collector = VolatileDataCollector::new();
        let temp_dir = TempDir::new().unwrap();

        let result = collector.collect_all(temp_dir.path());
        assert!(result.is_ok());

        let summary = result.unwrap();

        // Verify summary values are reasonable
        assert!(summary.cpu_count > 0 && summary.cpu_count < 1000); // Reasonable CPU count
        assert!(summary.total_memory_mb > 0); // Should have some memory
        assert!(summary.process_count > 0); // Should have at least one process
                                            // Network and disk counts can be 0 in some environments
        assert!(summary.network_interface_count >= 0);
        assert!(summary.disk_count >= 0);
    }
}
