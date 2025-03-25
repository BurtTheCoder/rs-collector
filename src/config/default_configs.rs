use std::collections::HashMap;
use crate::config::artifact_types::{ArtifactType, WindowsArtifactType, LinuxArtifactType, MacOSArtifactType};
use crate::config::collection_config::{CollectionConfig, Artifact};

impl CollectionConfig {
    /// Default configuration for Windows
    pub fn default_windows() -> Self {
        CollectionConfig {
            version: "1.0".into(),
            description: "Default Windows DFIR triage configuration".into(),
            artifacts: vec![
                // MFT
                Artifact {
                    name: "MFT".into(),
                    artifact_type: ArtifactType::Windows(WindowsArtifactType::MFT),
                    source_path: r"\\?\C:\$MFT".into(),
                    destination_name: "MFT".into(),
                    description: Some("Master File Table".into()),
                    required: true,
                    metadata: HashMap::new(),
                    regex: None,
                },
                // Registry hives
                Artifact {
                    name: "SYSTEM".into(),
                    artifact_type: ArtifactType::Windows(WindowsArtifactType::Registry),
                    source_path: r"\\?\C:\Windows\System32\config\SYSTEM".into(),
                    destination_name: "SYSTEM".into(),
                    description: Some("System registry hive".into()),
                    required: true,
                    metadata: HashMap::new(),
                    regex: None,
                },
                Artifact {
                    name: "SOFTWARE".into(),
                    artifact_type: ArtifactType::Windows(WindowsArtifactType::Registry),
                    source_path: r"\\?\C:\Windows\System32\config\SOFTWARE".into(),
                    destination_name: "SOFTWARE".into(),
                    description: Some("Software registry hive".into()),
                    required: true,
                    metadata: HashMap::new(),
                    regex: None,
                },
                Artifact {
                    name: "SECURITY".into(),
                    artifact_type: ArtifactType::Windows(WindowsArtifactType::Registry),
                    source_path: r"\\?\C:\Windows\System32\config\SECURITY".into(),
                    destination_name: "SECURITY".into(),
                    description: Some("Security registry hive".into()),
                    required: true,
                    metadata: HashMap::new(),
                    regex: None,
                },
                Artifact {
                    name: "SAM".into(),
                    artifact_type: ArtifactType::Windows(WindowsArtifactType::Registry),
                    source_path: r"\\?\C:\Windows\System32\config\SAM".into(),
                    destination_name: "SAM".into(),
                    description: Some("SAM registry hive".into()),
                    required: true,
                    metadata: HashMap::new(),
                    regex: None,
                },
                Artifact {
                    name: "NTUSER.DAT".into(),
                    artifact_type: ArtifactType::Windows(WindowsArtifactType::Registry),
                    source_path: r"\\?\%USERPROFILE%\NTUSER.DAT".into(),
                    destination_name: "NTUSER.DAT".into(),
                    description: Some("User registry hive".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
                // Event logs
                Artifact {
                    name: "System.evtx".into(),
                    artifact_type: ArtifactType::Windows(WindowsArtifactType::EventLog),
                    source_path: r"\\?\C:\Windows\System32\winevt\Logs\System.evtx".into(),
                    destination_name: "System.evtx".into(),
                    description: Some("System event log".into()),
                    required: true,
                    metadata: HashMap::new(),
                    regex: None,
                },
                Artifact {
                    name: "Security.evtx".into(),
                    artifact_type: ArtifactType::Windows(WindowsArtifactType::EventLog),
                    source_path: r"\\?\C:\Windows\System32\winevt\Logs\Security.evtx".into(),
                    destination_name: "Security.evtx".into(),
                    description: Some("Security event log".into()),
                    required: true,
                    metadata: HashMap::new(),
                    regex: None,
                },
                Artifact {
                    name: "Application.evtx".into(),
                    artifact_type: ArtifactType::Windows(WindowsArtifactType::EventLog),
                    source_path: r"\\?\C:\Windows\System32\winevt\Logs\Application.evtx".into(),
                    destination_name: "Application.evtx".into(),
                    description: Some("Application event log".into()),
                    required: true,
                    metadata: HashMap::new(),
                    regex: None,
                },
                Artifact {
                    name: "PowerShell.evtx".into(),
                    artifact_type: ArtifactType::Windows(WindowsArtifactType::EventLog),
                    source_path: r"\\?\C:\Windows\System32\winevt\Logs\Microsoft-Windows-PowerShell%4Operational.evtx".into(),
                    destination_name: "PowerShell-Operational.evtx".into(),
                    description: Some("PowerShell event log".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
                Artifact {
                    name: "Sysmon.evtx".into(),
                    artifact_type: ArtifactType::Windows(WindowsArtifactType::EventLog),
                    source_path: r"\\?\C:\Windows\System32\winevt\Logs\Microsoft-Windows-Sysmon%4Operational.evtx".into(),
                    destination_name: "Sysmon-Operational.evtx".into(),
                    description: Some("Sysmon event log".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
                // Prefetch files
                Artifact {
                    name: "Prefetch".into(),
                    artifact_type: ArtifactType::Windows(WindowsArtifactType::Prefetch),
                    source_path: r"\\?\C:\Windows\Prefetch".into(),
                    destination_name: "Prefetch".into(),
                    description: Some("Prefetch files".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
                // USN Journal
                Artifact {
                    name: "USN Journal".into(),
                    artifact_type: ArtifactType::Windows(WindowsArtifactType::USNJournal),
                    source_path: r"\\?\C:\$Extend\$UsnJrnl:$J".into(),
                    destination_name: "UsnJrnl".into(),
                    description: Some("USN Journal".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
            ],
            global_options: HashMap::new(),
        }
    }

    /// Default configuration for Linux
    pub fn default_linux() -> Self {
        CollectionConfig {
            version: "1.0".into(),
            description: "Default Linux DFIR triage configuration".into(),
            artifacts: vec![
                // System logs
                Artifact {
                    name: "syslog".into(),
                    artifact_type: ArtifactType::Linux(LinuxArtifactType::SysLogs),
                    source_path: "/var/log/syslog".into(),
                    destination_name: "syslog".into(),
                    description: Some("System logs".into()),
                    required: true,
                    metadata: HashMap::new(),
                    regex: None,
                },
                Artifact {
                    name: "auth.log".into(),
                    artifact_type: ArtifactType::Linux(LinuxArtifactType::SysLogs),
                    source_path: "/var/log/auth.log".into(),
                    destination_name: "auth.log".into(),
                    description: Some("Authentication logs".into()),
                    required: true,
                    metadata: HashMap::new(),
                    regex: None,
                },
                // Journal logs
                Artifact {
                    name: "journal".into(),
                    artifact_type: ArtifactType::Linux(LinuxArtifactType::Journal),
                    source_path: "/var/log/journal".into(),
                    destination_name: "journal".into(),
                    description: Some("Systemd journal logs".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
                // Proc filesystem
                Artifact {
                    name: "proc-cmdline".into(),
                    artifact_type: ArtifactType::Linux(LinuxArtifactType::Proc),
                    source_path: "/proc/cmdline".into(),
                    destination_name: "proc_cmdline".into(),
                    description: Some("Kernel command line".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
                Artifact {
                    name: "proc-modules".into(),
                    artifact_type: ArtifactType::Linux(LinuxArtifactType::Proc),
                    source_path: "/proc/modules".into(),
                    destination_name: "proc_modules".into(),
                    description: Some("Loaded kernel modules".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
                // Audit logs
                Artifact {
                    name: "audit.log".into(),
                    artifact_type: ArtifactType::Linux(LinuxArtifactType::Audit),
                    source_path: "/var/log/audit/audit.log".into(),
                    destination_name: "audit.log".into(),
                    description: Some("Audit logs".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
                // Cron
                Artifact {
                    name: "crontab".into(),
                    artifact_type: ArtifactType::Linux(LinuxArtifactType::Cron),
                    source_path: "/etc/crontab".into(),
                    destination_name: "crontab".into(),
                    description: Some("System crontab".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
                Artifact {
                    name: "cron.d".into(),
                    artifact_type: ArtifactType::Linux(LinuxArtifactType::Cron),
                    source_path: "/etc/cron.d".into(),
                    destination_name: "cron.d".into(),
                    description: Some("System cron jobs".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
                // Bash history
                Artifact {
                    name: "bash_history".into(),
                    artifact_type: ArtifactType::Linux(LinuxArtifactType::Bash),
                    source_path: "$HOME/.bash_history".into(),
                    destination_name: "bash_history".into(),
                    description: Some("Bash command history".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
                // Package management
                Artifact {
                    name: "dpkg.log".into(),
                    artifact_type: ArtifactType::Linux(LinuxArtifactType::Dpkg),
                    source_path: "/var/log/dpkg.log".into(),
                    destination_name: "dpkg.log".into(),
                    description: Some("Package installation logs".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
                // Systemd
                Artifact {
                    name: "systemd-units".into(),
                    artifact_type: ArtifactType::Linux(LinuxArtifactType::Systemd),
                    source_path: "/etc/systemd/system".into(),
                    destination_name: "systemd_units".into(),
                    description: Some("Systemd unit files".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
            ],
            global_options: HashMap::new(),
        }
    }

    /// Default configuration for macOS
    pub fn default_macos() -> Self {
        CollectionConfig {
            version: "1.0".into(),
            description: "Default macOS DFIR triage configuration".into(),
            artifacts: vec![
                // System logs
                Artifact {
                    name: "system.log".into(),
                    artifact_type: ArtifactType::MacOS(MacOSArtifactType::UnifiedLogs),
                    source_path: "/var/log/system.log".into(),
                    destination_name: "system.log".into(),
                    description: Some("System logs".into()),
                    required: true,
                    metadata: HashMap::new(),
                    regex: None,
                },
                // Unified logs
                Artifact {
                    name: "unified_logs".into(),
                    artifact_type: ArtifactType::MacOS(MacOSArtifactType::UnifiedLogs),
                    source_path: "/private/var/db/diagnostics".into(),
                    destination_name: "unified_logs".into(),
                    description: Some("Unified logging system".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
                // FSEvents
                Artifact {
                    name: "fseventsd".into(),
                    artifact_type: ArtifactType::MacOS(MacOSArtifactType::FSEvents),
                    source_path: "/System/Volumes/Data/.fseventsd".into(),
                    destination_name: "fseventsd".into(),
                    description: Some("File system events".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
                // Quarantine database
                Artifact {
                    name: "quarantine".into(),
                    artifact_type: ArtifactType::MacOS(MacOSArtifactType::Quarantine),
                    source_path: "$HOME/Library/Preferences/com.apple.LaunchServices.QuarantineEventsV2".into(),
                    destination_name: "QuarantineEventsV2".into(),
                    description: Some("Quarantine database".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
                // KnowledgeC database
                Artifact {
                    name: "knowledgec".into(),
                    artifact_type: ArtifactType::MacOS(MacOSArtifactType::KnowledgeC),
                    source_path: "$HOME/Library/Application Support/Knowledge/knowledgeC.db".into(),
                    destination_name: "knowledgeC.db".into(),
                    description: Some("User activity database".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
                // Launch Agents
                Artifact {
                    name: "launch_agents".into(),
                    artifact_type: ArtifactType::MacOS(MacOSArtifactType::LaunchAgents),
                    source_path: "/Library/LaunchAgents".into(),
                    destination_name: "LaunchAgents".into(),
                    description: Some("System launch agents".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
                Artifact {
                    name: "user_launch_agents".into(),
                    artifact_type: ArtifactType::MacOS(MacOSArtifactType::LaunchAgents),
                    source_path: "$HOME/Library/LaunchAgents".into(),
                    destination_name: "UserLaunchAgents".into(),
                    description: Some("User launch agents".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
                // Launch Daemons
                Artifact {
                    name: "launch_daemons".into(),
                    artifact_type: ArtifactType::MacOS(MacOSArtifactType::LaunchDaemons),
                    source_path: "/Library/LaunchDaemons".into(),
                    destination_name: "LaunchDaemons".into(),
                    description: Some("System launch daemons".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
                // Spotlight
                Artifact {
                    name: "spotlight_store".into(),
                    artifact_type: ArtifactType::MacOS(MacOSArtifactType::Spotlight),
                    source_path: "/.Spotlight-V100".into(),
                    destination_name: "Spotlight".into(),
                    description: Some("Spotlight metadata".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
                // Plists
                Artifact {
                    name: "system_plists".into(),
                    artifact_type: ArtifactType::MacOS(MacOSArtifactType::Plist),
                    source_path: "/Library/Preferences".into(),
                    destination_name: "SystemPreferences".into(),
                    description: Some("System preference plists".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
            ],
            global_options: HashMap::new(),
        }
    }

    /// Minimal configuration for unsupported OS
    pub fn default_minimal() -> Self {
        CollectionConfig {
            version: "1.0".into(),
            description: "Minimal DFIR triage configuration".into(),
            artifacts: vec![
                // Basic system info
                Artifact {
                    name: "hostname".into(),
                    artifact_type: ArtifactType::SystemInfo,
                    source_path: "/etc/hostname".into(),
                    destination_name: "hostname".into(),
                    description: Some("System hostname".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
                // Basic logs
                Artifact {
                    name: "logs".into(),
                    artifact_type: ArtifactType::Logs,
                    source_path: "/var/log".into(),
                    destination_name: "logs".into(),
                    description: Some("System logs".into()),
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
            ],
            global_options: HashMap::new(),
        }
    }
}
