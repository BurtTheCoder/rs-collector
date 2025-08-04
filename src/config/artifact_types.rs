use serde::{Deserialize, Serialize};
use std::fmt;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artifact_type_serialization() {
        // Test common types
        let fs_type = ArtifactType::FileSystem;
        let serialized = serde_json::to_string(&fs_type).unwrap();
        assert_eq!(serialized, "\"FileSystem\"");
        let deserialized: ArtifactType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, fs_type);

        // Test OS-specific types
        let win_type = ArtifactType::Windows(WindowsArtifactType::MFT);
        let serialized = serde_json::to_string(&win_type).unwrap();
        assert!(serialized.contains("Windows"));
        assert!(serialized.contains("MFT"));
        let deserialized: ArtifactType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, win_type);

        // Test volatile data types
        let volatile_type = ArtifactType::VolatileData(VolatileDataType::Processes);
        let serialized = serde_json::to_string(&volatile_type).unwrap();
        assert!(serialized.contains("VolatileData"));
        assert!(serialized.contains("Processes"));
        let deserialized: ArtifactType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, volatile_type);
    }

    #[test]
    fn test_artifact_type_display() {
        // Test Display implementation
        assert_eq!(ArtifactType::FileSystem.to_string(), "FileSystem");
        assert_eq!(ArtifactType::Logs.to_string(), "Logs");
        assert_eq!(ArtifactType::Custom.to_string(), "Custom");

        // OS-specific
        assert_eq!(
            ArtifactType::Windows(WindowsArtifactType::Registry).to_string(),
            "Windows-Registry"
        );
        assert_eq!(
            ArtifactType::Linux(LinuxArtifactType::SysLogs).to_string(),
            "Linux-SysLogs"
        );
        assert_eq!(
            ArtifactType::MacOS(MacOSArtifactType::UnifiedLogs).to_string(),
            "MacOS-UnifiedLogs"
        );

        // Volatile data
        assert_eq!(
            ArtifactType::VolatileData(VolatileDataType::SystemInfo).to_string(),
            "VolatileData-SystemInfo"
        );
    }

    #[test]
    fn test_volatile_data_type_display() {
        assert_eq!(VolatileDataType::SystemInfo.to_string(), "SystemInfo");
        assert_eq!(VolatileDataType::Processes.to_string(), "Processes");
        assert_eq!(
            VolatileDataType::NetworkConnections.to_string(),
            "NetworkConnections"
        );
        assert_eq!(VolatileDataType::Memory.to_string(), "Memory");
        assert_eq!(VolatileDataType::Disks.to_string(), "Disks");
    }

    #[test]
    fn test_artifact_type_equality() {
        // Test PartialEq implementation
        assert_eq!(ArtifactType::FileSystem, ArtifactType::FileSystem);
        assert_ne!(ArtifactType::FileSystem, ArtifactType::Logs);

        assert_eq!(
            ArtifactType::Windows(WindowsArtifactType::MFT),
            ArtifactType::Windows(WindowsArtifactType::MFT)
        );
        assert_ne!(
            ArtifactType::Windows(WindowsArtifactType::MFT),
            ArtifactType::Windows(WindowsArtifactType::Registry)
        );
    }

    #[test]
    fn test_artifact_type_hash() {
        use std::collections::HashSet;

        // Test that artifact types can be used in HashSet
        let mut set = HashSet::new();
        set.insert(ArtifactType::FileSystem);
        set.insert(ArtifactType::Logs);
        set.insert(ArtifactType::Windows(WindowsArtifactType::EventLog));

        assert!(set.contains(&ArtifactType::FileSystem));
        assert!(set.contains(&ArtifactType::Windows(WindowsArtifactType::EventLog)));
        assert!(!set.contains(&ArtifactType::Custom));
    }

    #[test]
    fn test_windows_artifact_types() {
        // Test all Windows artifact types
        let types = vec![
            WindowsArtifactType::MFT,
            WindowsArtifactType::Registry,
            WindowsArtifactType::EventLog,
            WindowsArtifactType::Prefetch,
            WindowsArtifactType::USNJournal,
            WindowsArtifactType::ShimCache,
            WindowsArtifactType::AmCache,
        ];

        for win_type in types {
            let artifact = ArtifactType::Windows(win_type.clone());
            let serialized = serde_json::to_string(&artifact).unwrap();
            let deserialized: ArtifactType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, artifact);
        }
    }

    #[test]
    fn test_linux_artifact_types() {
        // Test all Linux artifact types
        let types = vec![
            LinuxArtifactType::SysLogs,
            LinuxArtifactType::Journal,
            LinuxArtifactType::Proc,
            LinuxArtifactType::Audit,
            LinuxArtifactType::Cron,
            LinuxArtifactType::Bash,
            LinuxArtifactType::Apt,
            LinuxArtifactType::Dpkg,
            LinuxArtifactType::Yum,
            LinuxArtifactType::Systemd,
        ];

        for linux_type in types {
            let artifact = ArtifactType::Linux(linux_type.clone());
            let serialized = serde_json::to_string(&artifact).unwrap();
            let deserialized: ArtifactType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, artifact);
        }
    }

    #[test]
    fn test_macos_artifact_types() {
        // Test all macOS artifact types
        let types = vec![
            MacOSArtifactType::UnifiedLogs,
            MacOSArtifactType::Plist,
            MacOSArtifactType::Spotlight,
            MacOSArtifactType::FSEvents,
            MacOSArtifactType::Quarantine,
            MacOSArtifactType::KnowledgeC,
            MacOSArtifactType::LaunchAgents,
            MacOSArtifactType::LaunchDaemons,
        ];

        for macos_type in types {
            let artifact = ArtifactType::MacOS(macos_type.clone());
            let serialized = serde_json::to_string(&artifact).unwrap();
            let deserialized: ArtifactType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, artifact);
        }
    }

    #[test]
    fn test_clone_implementation() {
        // Test that Clone works correctly
        let original = ArtifactType::Windows(WindowsArtifactType::Registry);
        let cloned = original.clone();
        assert_eq!(original, cloned);

        let volatile_original = VolatileDataType::Processes;
        let volatile_cloned = volatile_original.clone();
        assert_eq!(volatile_original, volatile_cloned);
    }

    #[test]
    fn test_yaml_serialization() {
        // Test YAML serialization compatibility
        let artifact = ArtifactType::Linux(LinuxArtifactType::Journal);
        let yaml = serde_yaml::to_string(&artifact).unwrap();
        assert!(yaml.contains("Linux"));
        assert!(yaml.contains("Journal"));

        let deserialized: ArtifactType = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(deserialized, artifact);
    }
}
