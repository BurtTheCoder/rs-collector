use serde::{Serialize, Deserialize};

/// System information data structure
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemInfo {
    pub hostname: Option<String>,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub kernel_version: Option<String>,
    pub cpu_info: CpuInfo,
}

/// CPU information data structure
#[derive(Debug, Serialize, Deserialize)]
pub struct CpuInfo {
    pub count: usize,
    pub vendor: Option<String>,
    pub brand: Option<String>,
    pub frequency: u64,
}

/// Process information data structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cmd: Vec<String>,
    pub exe: Option<String>,
    pub status: String,
    pub start_time: u64,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub parent_pid: Option<u32>,
}

/// Network interface information
#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub mac: Option<String>,
    pub ips: Vec<String>,
    pub received_bytes: u64,
    pub transmitted_bytes: u64,
}

/// Network connection information
#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkConnection {
    pub protocol: String,
    pub local_address: String,
    pub local_port: u16,
    pub remote_address: Option<String>,
    pub remote_port: Option<u16>,
    pub state: Option<String>,
    pub process_id: Option<u32>,
}

/// Network information data structure
#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub interfaces: Vec<NetworkInterface>,
    pub connections: Vec<NetworkConnection>,
}

/// Memory information data structure
#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_memory: u64,
    pub used_memory: u64,
    pub total_swap: u64,
    pub used_swap: u64,
}

/// Disk information data structure
#[derive(Debug, Serialize, Deserialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: Option<String>,
    pub total_space: u64,
    pub available_space: u64,
    pub file_system: Option<String>,
    pub is_removable: bool,
}

/// Collection of all volatile data
#[derive(Debug, Serialize, Deserialize)]
pub struct VolatileData {
    pub system_info: SystemInfo,
    pub processes: Vec<ProcessInfo>,
    pub network: NetworkInfo,
    pub memory: MemoryInfo,
    pub disks: Vec<DiskInfo>,
}

/// Summary of volatile data collection for the collection summary
#[derive(Debug, Serialize, Deserialize)]
pub struct VolatileDataSummary {
    pub system_name: Option<String>,
    pub os_version: Option<String>,
    pub cpu_count: usize,
    pub total_memory_mb: u64,
    pub process_count: usize,
    pub network_interface_count: usize,
    pub disk_count: usize,
}
