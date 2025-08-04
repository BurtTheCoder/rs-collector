use crate::config::artifact_types::{
    ArtifactType, LinuxArtifactType, MacOSArtifactType, WindowsArtifactType,
};
use crate::config::collection_config::{Artifact, CollectionConfig};
use std::collections::HashMap;

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
                    source_path:
                        "$HOME/Library/Preferences/com.apple.LaunchServices.QuarantineEventsV2"
                            .into(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_windows_config() {
        let config = CollectionConfig::default_windows();

        // Test basic properties
        assert_eq!(config.version, "1.0");
        assert_eq!(
            config.description,
            "Default Windows DFIR triage configuration"
        );
        assert!(!config.artifacts.is_empty());

        // Test specific artifacts exist
        let artifact_names: Vec<&str> = config.artifacts.iter().map(|a| a.name.as_str()).collect();

        assert!(artifact_names.contains(&"MFT"));
        assert!(artifact_names.contains(&"SYSTEM"));
        assert!(artifact_names.contains(&"SOFTWARE"));
        assert!(artifact_names.contains(&"SECURITY"));
        assert!(artifact_names.contains(&"SAM"));
        assert!(artifact_names.contains(&"NTUSER.DAT"));
        assert!(artifact_names.contains(&"System.evtx"));
        assert!(artifact_names.contains(&"Security.evtx"));
        assert!(artifact_names.contains(&"Application.evtx"));
        assert!(artifact_names.contains(&"Prefetch"));
        assert!(artifact_names.contains(&"USN Journal"));

        // Test MFT artifact specifically
        let mft = config
            .artifacts
            .iter()
            .find(|a| a.name == "MFT")
            .expect("MFT artifact should exist");

        assert!(matches!(
            mft.artifact_type,
            ArtifactType::Windows(WindowsArtifactType::MFT)
        ));
        assert_eq!(mft.source_path, r"\\?\C:\$MFT");
        assert_eq!(mft.destination_name, "MFT");
        assert!(mft.required);
        assert_eq!(mft.description, Some("Master File Table".to_string()));

        // Test registry artifact
        let system_hive = config
            .artifacts
            .iter()
            .find(|a| a.name == "SYSTEM")
            .expect("SYSTEM artifact should exist");

        assert!(matches!(
            system_hive.artifact_type,
            ArtifactType::Windows(WindowsArtifactType::Registry)
        ));
        assert_eq!(
            system_hive.source_path,
            r"\\?\C:\Windows\System32\config\SYSTEM"
        );
        assert!(system_hive.required);
    }

    #[test]
    fn test_default_linux_config() {
        let config = CollectionConfig::default_linux();

        // Test basic properties
        assert_eq!(config.version, "1.0");
        assert_eq!(
            config.description,
            "Default Linux DFIR triage configuration"
        );
        assert!(!config.artifacts.is_empty());

        // Test specific artifacts exist
        let artifact_names: Vec<&str> = config.artifacts.iter().map(|a| a.name.as_str()).collect();

        assert!(artifact_names.contains(&"syslog"));
        assert!(artifact_names.contains(&"auth.log"));
        assert!(artifact_names.contains(&"journal"));
        assert!(artifact_names.contains(&"proc-cmdline"));
        assert!(artifact_names.contains(&"proc-modules"));
        assert!(artifact_names.contains(&"audit.log"));
        assert!(artifact_names.contains(&"crontab"));
        assert!(artifact_names.contains(&"bash_history"));
        assert!(artifact_names.contains(&"dpkg.log"));
        assert!(artifact_names.contains(&"systemd-units"));

        // Test syslog artifact specifically
        let syslog = config
            .artifacts
            .iter()
            .find(|a| a.name == "syslog")
            .expect("syslog artifact should exist");

        assert!(matches!(
            syslog.artifact_type,
            ArtifactType::Linux(LinuxArtifactType::SysLogs)
        ));
        assert_eq!(syslog.source_path, "/var/log/syslog");
        assert_eq!(syslog.destination_name, "syslog");
        assert!(syslog.required);

        // Test bash history artifact
        let bash_history = config
            .artifacts
            .iter()
            .find(|a| a.name == "bash_history")
            .expect("bash_history artifact should exist");

        assert!(matches!(
            bash_history.artifact_type,
            ArtifactType::Linux(LinuxArtifactType::Bash)
        ));
        assert_eq!(bash_history.source_path, "$HOME/.bash_history");
        assert!(!bash_history.required);
    }

    #[test]
    fn test_default_macos_config() {
        let config = CollectionConfig::default_macos();

        // Test basic properties
        assert_eq!(config.version, "1.0");
        assert_eq!(
            config.description,
            "Default macOS DFIR triage configuration"
        );
        assert!(!config.artifacts.is_empty());

        // Test specific artifacts exist
        let artifact_names: Vec<&str> = config.artifacts.iter().map(|a| a.name.as_str()).collect();

        assert!(artifact_names.contains(&"system.log"));
        assert!(artifact_names.contains(&"unified_logs"));
        assert!(artifact_names.contains(&"fseventsd"));
        assert!(artifact_names.contains(&"quarantine"));
        assert!(artifact_names.contains(&"knowledgec"));
        assert!(artifact_names.contains(&"launch_agents"));
        assert!(artifact_names.contains(&"user_launch_agents"));
        assert!(artifact_names.contains(&"launch_daemons"));
        assert!(artifact_names.contains(&"spotlight_store"));
        assert!(artifact_names.contains(&"system_plists"));

        // Test unified logs artifact specifically
        let unified_logs = config
            .artifacts
            .iter()
            .find(|a| a.name == "unified_logs")
            .expect("unified_logs artifact should exist");

        assert!(matches!(
            unified_logs.artifact_type,
            ArtifactType::MacOS(MacOSArtifactType::UnifiedLogs)
        ));
        assert_eq!(unified_logs.source_path, "/private/var/db/diagnostics");
        assert_eq!(unified_logs.destination_name, "unified_logs");
        assert!(!unified_logs.required);

        // Test quarantine database
        let quarantine = config
            .artifacts
            .iter()
            .find(|a| a.name == "quarantine")
            .expect("quarantine artifact should exist");

        assert!(matches!(
            quarantine.artifact_type,
            ArtifactType::MacOS(MacOSArtifactType::Quarantine)
        ));
        assert!(quarantine.source_path.contains("QuarantineEventsV2"));
    }

    #[test]
    fn test_default_minimal_config() {
        let config = CollectionConfig::default_minimal();

        // Test basic properties
        assert_eq!(config.version, "1.0");
        assert_eq!(config.description, "Minimal DFIR triage configuration");
        assert_eq!(config.artifacts.len(), 2);

        // Test hostname artifact
        let hostname = config
            .artifacts
            .iter()
            .find(|a| a.name == "hostname")
            .expect("hostname artifact should exist");

        assert!(matches!(hostname.artifact_type, ArtifactType::SystemInfo));
        assert_eq!(hostname.source_path, "/etc/hostname");
        assert_eq!(hostname.destination_name, "hostname");
        assert!(!hostname.required);

        // Test logs artifact
        let logs = config
            .artifacts
            .iter()
            .find(|a| a.name == "logs")
            .expect("logs artifact should exist");

        assert!(matches!(logs.artifact_type, ArtifactType::Logs));
        assert_eq!(logs.source_path, "/var/log");
        assert_eq!(logs.destination_name, "logs");
        assert!(!logs.required);
    }

    #[test]
    fn test_all_configs_have_valid_version() {
        let configs = vec![
            CollectionConfig::default_windows(),
            CollectionConfig::default_linux(),
            CollectionConfig::default_macos(),
            CollectionConfig::default_minimal(),
        ];

        for config in configs {
            assert!(!config.version.is_empty());
            assert!(config.version.contains('.'));
        }
    }

    #[test]
    fn test_all_configs_have_description() {
        let configs = vec![
            CollectionConfig::default_windows(),
            CollectionConfig::default_linux(),
            CollectionConfig::default_macos(),
            CollectionConfig::default_minimal(),
        ];

        for config in configs {
            assert!(!config.description.is_empty());
            assert!(config.description.contains("DFIR"));
        }
    }

    #[test]
    fn test_artifact_metadata_and_regex() {
        let configs = vec![
            CollectionConfig::default_windows(),
            CollectionConfig::default_linux(),
            CollectionConfig::default_macos(),
            CollectionConfig::default_minimal(),
        ];

        // All default artifacts should have empty metadata and no regex
        for config in configs {
            for artifact in &config.artifacts {
                assert!(artifact.metadata.is_empty());
                assert!(artifact.regex.is_none());
            }
        }
    }

    #[test]
    fn test_global_options_empty() {
        let configs = vec![
            CollectionConfig::default_windows(),
            CollectionConfig::default_linux(),
            CollectionConfig::default_macos(),
            CollectionConfig::default_minimal(),
        ];

        // All default configs should have empty global options
        for config in configs {
            assert!(config.global_options.is_empty());
        }
    }

    #[test]
    fn test_windows_artifact_types() {
        let config = CollectionConfig::default_windows();

        // Count different artifact types
        let mut type_counts = HashMap::new();

        for artifact in &config.artifacts {
            match &artifact.artifact_type {
                ArtifactType::Windows(win_type) => {
                    let type_name = format!("{:?}", win_type);
                    *type_counts.entry(type_name).or_insert(0) += 1;
                }
                _ => panic!("Non-Windows artifact type in Windows config"),
            }
        }

        // Verify we have multiple of certain types
        assert!(type_counts.get("Registry").unwrap_or(&0) >= &5);
        assert!(type_counts.get("EventLog").unwrap_or(&0) >= &3);
        assert_eq!(type_counts.get("MFT").unwrap_or(&0), &1);
        assert_eq!(type_counts.get("Prefetch").unwrap_or(&0), &1);
        assert_eq!(type_counts.get("USNJournal").unwrap_or(&0), &1);
    }

    #[test]
    fn test_linux_artifact_types() {
        let config = CollectionConfig::default_linux();

        // Verify all artifacts are Linux type
        for artifact in &config.artifacts {
            assert!(matches!(artifact.artifact_type, ArtifactType::Linux(_)));
        }

        // Count required vs optional
        let required_count = config.artifacts.iter().filter(|a| a.required).count();
        let optional_count = config.artifacts.iter().filter(|a| !a.required).count();

        assert_eq!(required_count, 2); // syslog and auth.log
        assert!(optional_count > 5);
    }

    #[test]
    fn test_macos_artifact_types() {
        let config = CollectionConfig::default_macos();

        // Verify all artifacts are macOS type
        for artifact in &config.artifacts {
            assert!(matches!(artifact.artifact_type, ArtifactType::MacOS(_)));
        }

        // Only system.log should be required
        let required_artifacts: Vec<&str> = config
            .artifacts
            .iter()
            .filter(|a| a.required)
            .map(|a| a.name.as_str())
            .collect();

        assert_eq!(required_artifacts, vec!["system.log"]);
    }

    #[test]
    fn test_windows_special_paths() {
        let config = CollectionConfig::default_windows();

        // Check for Windows special path prefix
        for artifact in &config.artifacts {
            if artifact.source_path.starts_with(r"\\?\") {
                // Verify it's followed by a drive letter
                let after_prefix = &artifact.source_path[4..];
                assert!(after_prefix.starts_with("C:") || after_prefix.starts_with("%"));
            }
        }

        // Check specific special files
        let mft = config.artifacts.iter().find(|a| a.name == "MFT").unwrap();
        assert!(mft.source_path.contains("$MFT"));

        let usn = config
            .artifacts
            .iter()
            .find(|a| a.name == "USN Journal")
            .unwrap();
        assert!(usn.source_path.contains("$Extend"));
        assert!(usn.source_path.contains("$UsnJrnl:$J"));
    }

    #[test]
    fn test_environment_variables_in_paths() {
        let config_windows = CollectionConfig::default_windows();
        let config_linux = CollectionConfig::default_linux();
        let config_macos = CollectionConfig::default_macos();

        // Windows uses %VAR% style
        let ntuser = config_windows
            .artifacts
            .iter()
            .find(|a| a.name == "NTUSER.DAT")
            .unwrap();
        assert!(ntuser.source_path.contains("%USERPROFILE%"));

        // Linux uses $VAR style
        let bash_history = config_linux
            .artifacts
            .iter()
            .find(|a| a.name == "bash_history")
            .unwrap();
        assert!(bash_history.source_path.contains("$HOME"));

        // macOS uses $VAR style
        let quarantine = config_macos
            .artifacts
            .iter()
            .find(|a| a.name == "quarantine")
            .unwrap();
        assert!(quarantine.source_path.contains("$HOME"));
    }
}
