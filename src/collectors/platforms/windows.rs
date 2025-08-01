use std::path::{Path, PathBuf};

use anyhow::{Result, Context};
use log::{debug, info, warn};
use tokio::task;

use crate::models::ArtifactMetadata;
use crate::config::{Artifact, ArtifactType, WindowsArtifactType};
use crate::collectors::collector::ArtifactCollector;
use crate::config::parse_windows_env_vars;
use crate::windows::{collect_with_raw_handle, check_backup_api_available};

/// Windows-specific artifact collector
pub struct WindowsCollector {
    has_backup_api: bool
}

impl WindowsCollector {
    #[allow(dead_code)]
    pub fn new() -> Self {
        info!("Initializing Windows artifact collector");
        
        // Check for required Windows features
        let has_backup_api = check_backup_api_available();
        if !has_backup_api {
            warn!("Windows Backup API not available - some locked files may be inaccessible");
        }
        
        WindowsCollector {
            has_backup_api
        }
    }
    
    /// Collect MFT using raw file access
    fn collect_mft(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        if self.has_backup_api {
            info!("Collecting MFT using raw file access with Backup API");
        } else {
            info!("Collecting MFT using raw file access (Backup API unavailable)");
        }
        collect_with_raw_handle(&source.to_string_lossy(), dest)
    }
    
    /// Collect registry hive using raw file access
    fn collect_registry(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        if self.has_backup_api {
            info!("Collecting registry hive using raw file access with Backup API");
        } else {
            info!("Collecting registry hive using raw file access (Backup API unavailable)");
        }
        collect_with_raw_handle(&source.to_string_lossy(), dest)
    }
    
    /// Collect event log using raw file access
    fn collect_eventlog(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        if self.has_backup_api {
            info!("Collecting event log using raw file access with Backup API");
        } else {
            info!("Collecting event log using raw file access (Backup API unavailable)");
        }
        collect_with_raw_handle(&source.to_string_lossy(), dest)
    }
    
    /// Collect prefetch files using raw file access
    fn collect_prefetch(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        if self.has_backup_api {
            info!("Collecting prefetch files using raw file access with Backup API");
        } else {
            info!("Collecting prefetch files using raw file access (Backup API unavailable)");
        }
        collect_with_raw_handle(&source.to_string_lossy(), dest)
    }
    
    /// Collect USN journal using raw file access
    fn collect_usn_journal(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        if self.has_backup_api {
            info!("Collecting USN journal using raw file access with Backup API");
        } else {
            info!("Collecting USN journal using raw file access (Backup API unavailable)");
        }
        collect_with_raw_handle(&source.to_string_lossy(), dest)
    }
}

#[async_trait::async_trait]
impl ArtifactCollector for WindowsCollector {
    async fn collect(&self, artifact: &Artifact, output_dir: &Path) -> Result<ArtifactMetadata> {
        let source_path = PathBuf::from(parse_windows_env_vars(&artifact.source_path));
        
        // Use the output directory directly instead of joining with destination name
        // The path structure is now handled by the main collector function
        let output_path = output_dir.to_path_buf();
        
        debug!("Collecting {} from {} to {}", artifact.name, source_path.display(), output_path.display());
        
        // Clone self for the async block
        let collector = self.clone();
        let source_path_clone = source_path.clone();
        let output_path_clone = output_path.clone();
        let artifact_type = artifact.artifact_type.clone();
        
        // Choose appropriate collection method based on artifact type
        let result = task::spawn_blocking(move || {
            match &artifact_type {
                ArtifactType::Windows(WindowsArtifactType::MFT) => {
                    collector.collect_mft(&source_path_clone, &output_path_clone)
                },
                ArtifactType::Windows(WindowsArtifactType::Registry) => {
                    collector.collect_registry(&source_path_clone, &output_path_clone)
                },
                ArtifactType::Windows(WindowsArtifactType::EventLog) => {
                    collector.collect_eventlog(&source_path_clone, &output_path_clone)
                },
                ArtifactType::Windows(WindowsArtifactType::Prefetch) => {
                    collector.collect_prefetch(&source_path_clone, &output_path_clone)
                },
                ArtifactType::Windows(WindowsArtifactType::USNJournal) => {
                    collector.collect_usn_journal(&source_path_clone, &output_path_clone)
                },
                _ => {
                    // For other artifact types, use raw file access
                    if collector.has_backup_api {
                        debug!("Using Backup API for generic file collection");
                    } else {
                        debug!("Using standard file access (Backup API unavailable)");
                    }
                    collect_with_raw_handle(&source_path_clone.to_string_lossy(), &output_path_clone)
                }
            }
        }).await.context("Task join error")??;
        
        Ok(result)
    }
    
    fn supports_artifact_type(&self, artifact_type: &ArtifactType) -> bool {
        matches!(artifact_type, 
            ArtifactType::Windows(_) | 
            ArtifactType::FileSystem | 
            ArtifactType::Logs | 
            ArtifactType::UserData | 
            ArtifactType::SystemInfo | 
            ArtifactType::Memory | 
            ArtifactType::Network | 
            ArtifactType::Custom
        )
    }
}

// Make WindowsCollector cloneable for use in async blocks
impl Clone for WindowsCollector {
    fn clone(&self) -> Self {
        WindowsCollector {
            has_backup_api: self.has_backup_api
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::config::WindowsArtifactType;

    #[test]
    fn test_windows_collector_new() {
        let collector = WindowsCollector::new();
        // Just verify it creates without panic
        // The has_backup_api field is private, so we can't directly test it
        assert!(collector.supports_artifact_type(&ArtifactType::Windows(WindowsArtifactType::MFT)));
    }

    #[test]
    fn test_windows_collector_clone() {
        let collector1 = WindowsCollector::new();
        let collector2 = collector1.clone();
        
        // Both should support the same artifact types
        assert_eq!(
            collector1.supports_artifact_type(&ArtifactType::Windows(WindowsArtifactType::Registry)),
            collector2.supports_artifact_type(&ArtifactType::Windows(WindowsArtifactType::Registry))
        );
    }

    #[test]
    fn test_supports_artifact_type() {
        let collector = WindowsCollector::new();
        
        // Windows-specific types
        assert!(collector.supports_artifact_type(&ArtifactType::Windows(WindowsArtifactType::MFT)));
        assert!(collector.supports_artifact_type(&ArtifactType::Windows(WindowsArtifactType::Registry)));
        assert!(collector.supports_artifact_type(&ArtifactType::Windows(WindowsArtifactType::EventLog)));
        assert!(collector.supports_artifact_type(&ArtifactType::Windows(WindowsArtifactType::Prefetch)));
        assert!(collector.supports_artifact_type(&ArtifactType::Windows(WindowsArtifactType::USNJournal)));
        
        // Generic types
        assert!(collector.supports_artifact_type(&ArtifactType::FileSystem));
        assert!(collector.supports_artifact_type(&ArtifactType::Logs));
        assert!(collector.supports_artifact_type(&ArtifactType::UserData));
        assert!(collector.supports_artifact_type(&ArtifactType::SystemInfo));
        assert!(collector.supports_artifact_type(&ArtifactType::Memory));
        assert!(collector.supports_artifact_type(&ArtifactType::Network));
        assert!(collector.supports_artifact_type(&ArtifactType::Custom));
        
        // Unsupported types
        assert!(!collector.supports_artifact_type(&ArtifactType::Linux(crate::config::LinuxArtifactType::SysLogs)));
        assert!(!collector.supports_artifact_type(&ArtifactType::MacOS(crate::config::MacOSArtifactType::UnifiedLogs)));
    }

    #[tokio::test]
    async fn test_collect_windows_artifact_mft() {
        let collector = WindowsCollector::new();
        let temp_dir = TempDir::new().unwrap();
        
        let artifact = Artifact {
            name: "MFT".to_string(),
            artifact_type: ArtifactType::Windows(WindowsArtifactType::MFT),
            source_path: r"\\?\C:\$MFT".to_string(),
            destination_name: "MFT".to_string(),
            description: Some("Master File Table".to_string()),
            required: true,
            metadata: std::collections::HashMap::new(),
            regex: None,
        };
        
        // Note: This will fail on non-Windows systems or without admin rights
        // But it tests the code path
        let result = collector.collect(&artifact, temp_dir.path()).await;
        
        // We expect this to fail in test environment
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_collect_windows_artifact_registry() {
        let collector = WindowsCollector::new();
        let temp_dir = TempDir::new().unwrap();
        
        let artifact = Artifact {
            name: "SYSTEM".to_string(),
            artifact_type: ArtifactType::Windows(WindowsArtifactType::Registry),
            source_path: r"\\?\C:\Windows\System32\config\SYSTEM".to_string(),
            destination_name: "SYSTEM".to_string(),
            description: Some("System registry hive".to_string()),
            required: true,
            metadata: std::collections::HashMap::new(),
            regex: None,
        };
        
        let result = collector.collect(&artifact, temp_dir.path()).await;
        
        // We expect this to fail in test environment
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_collect_with_env_vars() {
        let collector = WindowsCollector::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Set a test environment variable
        std::env::set_var("TESTDIR", r"C:\TestDirectory");
        
        let artifact = Artifact {
            name: "TestFile".to_string(),
            artifact_type: ArtifactType::Windows(WindowsArtifactType::Registry),
            source_path: r"%TESTDIR%\test.dat".to_string(),
            destination_name: "test.dat".to_string(),
            description: Some("Test file with env var".to_string()),
            required: false,
            metadata: std::collections::HashMap::new(),
            regex: None,
        };
        
        let result = collector.collect(&artifact, temp_dir.path()).await;
        
        // Cleanup
        std::env::remove_var("TESTDIR");
        
        // We expect this to fail but it should have expanded the env var
        assert!(result.is_err());
    }

    #[test]
    fn test_artifact_type_matching() {
        let collector = WindowsCollector::new();
        
        // Create test artifacts for each Windows type
        let test_cases = vec![
            (WindowsArtifactType::MFT, "MFT"),
            (WindowsArtifactType::Registry, "Registry"),
            (WindowsArtifactType::EventLog, "EventLog"),
            (WindowsArtifactType::Prefetch, "Prefetch"),
            (WindowsArtifactType::USNJournal, "USNJournal"),
        ];
        
        for (win_type, name) in test_cases {
            let artifact_type = ArtifactType::Windows(win_type);
            assert!(
                collector.supports_artifact_type(&artifact_type),
                "Should support {} artifact type", name
            );
        }
    }

    #[test]
    fn test_generic_artifact_support() {
        let collector = WindowsCollector::new();
        
        let generic_types = vec![
            ArtifactType::FileSystem,
            ArtifactType::Logs,
            ArtifactType::UserData,
            ArtifactType::SystemInfo,
            ArtifactType::Memory,
            ArtifactType::Network,
            ArtifactType::Custom,
        ];
        
        for artifact_type in generic_types {
            assert!(
                collector.supports_artifact_type(&artifact_type),
                "Should support {:?} artifact type", artifact_type
            );
        }
    }

    #[test]
    fn test_non_windows_artifact_rejection() {
        let collector = WindowsCollector::new();
        
        // Test Linux artifact types
        let linux_types = vec![
            crate::config::LinuxArtifactType::SysLogs,
            crate::config::LinuxArtifactType::Journal,
            crate::config::LinuxArtifactType::Proc,
        ];
        
        for linux_type in linux_types {
            assert!(
                !collector.supports_artifact_type(&ArtifactType::Linux(linux_type)),
                "Should not support Linux artifact type"
            );
        }
        
        // Test macOS artifact types
        let macos_types = vec![
            crate::config::MacOSArtifactType::UnifiedLogs,
            crate::config::MacOSArtifactType::FSEvents,
            crate::config::MacOSArtifactType::Quarantine,
        ];
        
        for macos_type in macos_types {
            assert!(
                !collector.supports_artifact_type(&ArtifactType::MacOS(macos_type)),
                "Should not support macOS artifact type"
            );
        }
    }

    #[tokio::test]
    async fn test_collect_all_windows_types() {
        let collector = WindowsCollector::new();
        let temp_dir = TempDir::new().unwrap();
        
        let test_artifacts = vec![
            (WindowsArtifactType::MFT, r"\\?\C:\$MFT", "MFT"),
            (WindowsArtifactType::Registry, r"\\?\C:\Windows\System32\config\SYSTEM", "SYSTEM"),
            (WindowsArtifactType::EventLog, r"\\?\C:\Windows\System32\winevt\Logs\System.evtx", "System.evtx"),
            (WindowsArtifactType::Prefetch, r"\\?\C:\Windows\Prefetch\TEST.pf", "TEST.pf"),
            (WindowsArtifactType::USNJournal, r"\\?\C:\$Extend\$UsnJrnl:$J", "UsnJrnl"),
        ];
        
        for (win_type, path, name) in test_artifacts {
            let artifact = Artifact {
                name: name.to_string(),
                artifact_type: ArtifactType::Windows(win_type),
                source_path: path.to_string(),
                destination_name: name.to_string(),
                description: Some(format!("Test {}", name)),
                required: false,
                metadata: std::collections::HashMap::new(),
                regex: None,
            };
            
            let result = collector.collect(&artifact, temp_dir.path()).await;
            
            // All should fail in test environment but exercise the code paths
            assert!(result.is_err());
        }
    }
}
