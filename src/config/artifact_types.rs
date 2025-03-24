use std::fmt;
use serde::{Serialize, Deserialize};

/// OS-agnostic artifact types
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum ArtifactType {
    // Common across platforms
    FileSystem,
    Logs,
    UserData,
    SystemInfo,
    Memory,
    Network,
    
    // OS-specific types
    Windows(WindowsArtifactType),
    Linux(LinuxArtifactType),
    MacOS(MacOSArtifactType),
    
    // Volatile data collection
    VolatileData(VolatileDataType),
    
    // For custom artifacts
    Custom,
}

/// Volatile data artifact types
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum VolatileDataType {
    SystemInfo,
    Processes,
    NetworkConnections,
    Memory,
    Disks,
}

/// Windows-specific artifact types
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum WindowsArtifactType {
    MFT,
    Registry,
    EventLog,
    Prefetch,
    USNJournal,
    ShimCache,
    AmCache,
}

/// Linux-specific artifact types
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum LinuxArtifactType {
    SysLogs,
    Journal,
    Proc,
    Audit,
    Cron,
    Bash,
    Apt,
    Dpkg,
    Yum,
    Systemd,
}

/// macOS-specific artifact types
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum MacOSArtifactType {
    UnifiedLogs,
    Plist,
    Spotlight,
    FSEvents,
    Quarantine,
    KnowledgeC,
    LaunchAgents,
    LaunchDaemons,
}

impl fmt::Display for ArtifactType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArtifactType::FileSystem => write!(f, "FileSystem"),
            ArtifactType::Logs => write!(f, "Logs"),
            ArtifactType::UserData => write!(f, "UserData"),
            ArtifactType::SystemInfo => write!(f, "SystemInfo"),
            ArtifactType::Memory => write!(f, "Memory"),
            ArtifactType::Network => write!(f, "Network"),
            ArtifactType::Windows(wtype) => write!(f, "Windows-{:?}", wtype),
            ArtifactType::Linux(ltype) => write!(f, "Linux-{:?}", ltype),
            ArtifactType::MacOS(mtype) => write!(f, "MacOS-{:?}", mtype),
            ArtifactType::VolatileData(vtype) => write!(f, "VolatileData-{:?}", vtype),
            ArtifactType::Custom => write!(f, "Custom"),
        }
    }
}

impl fmt::Display for VolatileDataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VolatileDataType::SystemInfo => write!(f, "SystemInfo"),
            VolatileDataType::Processes => write!(f, "Processes"),
            VolatileDataType::NetworkConnections => write!(f, "NetworkConnections"),
            VolatileDataType::Memory => write!(f, "Memory"),
            VolatileDataType::Disks => write!(f, "Disks"),
        }
    }
}
