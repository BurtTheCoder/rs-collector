use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;

use anyhow::{Result, Context};
use log::{debug, info, warn};
use tokio::task;

use crate::models::ArtifactMetadata;
use crate::config::{Artifact, ArtifactType, MacOSArtifactType};
use crate::collectors::collector::ArtifactCollector;
use crate::config::parse_unix_env_vars;
use crate::collectors::platforms::common::FallbackCollector;
use crate::privileges::is_elevated;

/// macOS-specific artifact collector
pub struct MacOSCollector {
    fallback: FallbackCollector,
}

impl MacOSCollector {
    pub fn new() -> Self {
        info!("Initializing macOS artifact collector");
        
        // Check for root privileges
        if !is_elevated() {
            warn!("Not running as root - some system files may be inaccessible");
        }
        
        // Check for log command availability
        let has_log_cmd = Command::new("which")
            .arg("log")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
            
        if !has_log_cmd {
            warn!("log command not found - unified log collection may be limited");
        }
        
        // Check for plutil command availability
        let has_plutil = Command::new("which")
            .arg("plutil")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
            
        if !has_plutil {
            warn!("plutil command not found - plist conversion will be skipped");
        }
        
        MacOSCollector {
            fallback: FallbackCollector::new(),
        }
    }
    
    /// Collect unified logs using log command
    fn collect_unified_logs(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        info!("Collecting unified logs");
        
        // If source is a directory, collect it normally
        if source.is_dir() {
            return self.fallback.collect_directory(source, dest);
        }
        
        // If source is a file, collect it normally
        if source.is_file() {
            return self.fallback.collect_standard_file(source, dest);
        }
        
        // Otherwise, try to use the log command to export logs
        // Create parent directories if they don't exist
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .context(format!("Failed to create directory: {}", parent.display()))?;
        }
        
        // Use log command to export logs
        let output = Command::new("log")
            .arg("show")
            .arg("--style=json")
            .arg("--last=1d")
            .output()
            .context("Failed to execute log command")?;
        
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            warn!("log command failed: {}", error);
            
            // Fall back to copying the file if it exists
            if source.exists() {
                return self.fallback.collect_standard_file(source, dest);
            } else {
                return Err(anyhow::anyhow!("Failed to collect unified logs"));
            }
        }
        
        // Write the output to the destination file
        fs::write(dest, output.stdout)
            .context(format!("Failed to write unified logs to {}", dest.display()))?;
        
        // Get file metadata
        let metadata = fs::metadata(dest)
            .context(format!("Failed to get metadata for {}", dest.display()))?;
        
        // Get current time for metadata
        let collection_time = chrono::Utc::now().to_rfc3339();
        
        // Convert file times to RFC3339 strings
        let created_time = metadata.created()
            .ok()
            .map(|time| chrono::DateTime::<chrono::Utc>::from(time).to_rfc3339());
            
        let accessed_time = metadata.accessed()
            .ok()
            .map(|time| chrono::DateTime::<chrono::Utc>::from(time).to_rfc3339());
            
        let modified_time = metadata.modified()
            .ok()
            .map(|time| chrono::DateTime::<chrono::Utc>::from(time).to_rfc3339());
        
        // Create artifact metadata
        let artifact_metadata = ArtifactMetadata {
            original_path: source.to_string_lossy().to_string(),
            collection_time,
            file_size: metadata.len(),
            created_time,
            accessed_time,
            modified_time,
            is_locked: false,
        };
        
        Ok(artifact_metadata)
    }
    
    /// Collect FSEvents
    fn collect_fsevents(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        info!("Collecting FSEvents");
        
        if source.is_dir() {
            self.fallback.collect_directory(source, dest)
        } else {
            self.fallback.collect_standard_file(source, dest)
        }
    }
    
    /// Collect property list files
    fn collect_plist(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        info!("Collecting property list file");
        
        // If source is a directory, collect it normally
        if source.is_dir() {
            return self.fallback.collect_directory(source, dest);
        }
        
        // If source is a file, try to convert binary plist to XML if needed
        if source.is_file() {
            // Check if it's a binary plist
            let is_binary = Command::new("file")
                .arg(source)
                .output()
                .map(|output| {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    output_str.contains("binary property list")
                })
                .unwrap_or(false);
            
            if is_binary {
                info!("Converting binary plist to XML format");
                
                // Create parent directories if they don't exist
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)
                        .context(format!("Failed to create directory: {}", parent.display()))?;
                }
                
                // Convert binary plist to XML
                let output = Command::new("plutil")
                    .arg("-convert")
                    .arg("xml1")
                    .arg("-o")
                    .arg(dest)
                    .arg(source)
                    .output()
                    .context("Failed to execute plutil command")?;
                
                if !output.status.success() {
                    let error = String::from_utf8_lossy(&output.stderr);
                    warn!("plutil command failed: {}", error);
                    
                    // Fall back to copying the file
                    return self.fallback.collect_standard_file(source, dest);
                }
                
                // Get file metadata
                let metadata = fs::metadata(dest)
                    .context(format!("Failed to get metadata for {}", dest.display()))?;
                
                // Get current time for metadata
                let collection_time = chrono::Utc::now().to_rfc3339();
                
                // Convert file times to RFC3339 strings
                let created_time = metadata.created()
                    .ok()
                    .map(|time| chrono::DateTime::<chrono::Utc>::from(time).to_rfc3339());
                    
                let accessed_time = metadata.accessed()
                    .ok()
                    .map(|time| chrono::DateTime::<chrono::Utc>::from(time).to_rfc3339());
                    
                let modified_time = metadata.modified()
                    .ok()
                    .map(|time| chrono::DateTime::<chrono::Utc>::from(time).to_rfc3339());
                
                // Create artifact metadata
                let artifact_metadata = ArtifactMetadata {
                    original_path: source.to_string_lossy().to_string(),
                    collection_time,
                    file_size: metadata.len(),
                    created_time,
                    accessed_time,
                    modified_time,
                    is_locked: false,
                };
                
                return Ok(artifact_metadata);
            }
            
            // If not binary or conversion failed, collect normally
            return self.fallback.collect_standard_file(source, dest);
        }
        
        Err(anyhow::anyhow!("Source does not exist: {}", source.display()))
    }
    
    /// Collect Spotlight metadata
    fn collect_spotlight(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        info!("Collecting Spotlight metadata");
        
        if source.is_dir() {
            self.fallback.collect_directory(source, dest)
        } else {
            self.fallback.collect_standard_file(source, dest)
        }
    }
    
    /// Collect Quarantine database
    fn collect_quarantine(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        info!("Collecting Quarantine database");
        self.fallback.collect_standard_file(source, dest)
    }
    
    /// Collect KnowledgeC database
    fn collect_knowledgec(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        info!("Collecting KnowledgeC database");
        self.fallback.collect_standard_file(source, dest)
    }
    
    /// Collect Launch Agents
    fn collect_launch_agents(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        info!("Collecting Launch Agents");
        
        if source.is_dir() {
            self.fallback.collect_directory(source, dest)
        } else {
            self.fallback.collect_standard_file(source, dest)
        }
    }
    
    /// Collect Launch Daemons
    fn collect_launch_daemons(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        info!("Collecting Launch Daemons");
        
        if source.is_dir() {
            self.fallback.collect_directory(source, dest)
        } else {
            self.fallback.collect_standard_file(source, dest)
        }
    }
}

#[async_trait::async_trait]
impl ArtifactCollector for MacOSCollector {
    async fn collect(&self, artifact: &Artifact, output_dir: &Path) -> Result<ArtifactMetadata> {
        let source_path = PathBuf::from(parse_unix_env_vars(&artifact.source_path));
        
        // Use the output directory directly instead of joining with destination name
        // The path structure is now handled by the main collector function
        let output_path = output_dir.to_path_buf();
        
        debug!("Collecting {} from {} to {}", artifact.name, source_path.display(), output_path.display());
        
        // Clone self and data for the async block
        let collector = self.clone();
        let source_path_clone = source_path.clone();
        let output_path_clone = output_path.clone();
        let artifact_type = artifact.artifact_type.clone();
        
        // Choose appropriate collection method based on artifact type
        let result = task::spawn_blocking(move || {
            match &artifact_type {
                ArtifactType::MacOS(MacOSArtifactType::UnifiedLogs) => {
                    collector.collect_unified_logs(&source_path_clone, &output_path_clone)
                },
                ArtifactType::MacOS(MacOSArtifactType::FSEvents) => {
                    collector.collect_fsevents(&source_path_clone, &output_path_clone)
                },
                ArtifactType::MacOS(MacOSArtifactType::Plist) => {
                    collector.collect_plist(&source_path_clone, &output_path_clone)
                },
                ArtifactType::MacOS(MacOSArtifactType::Spotlight) => {
                    collector.collect_spotlight(&source_path_clone, &output_path_clone)
                },
                ArtifactType::MacOS(MacOSArtifactType::Quarantine) => {
                    collector.collect_quarantine(&source_path_clone, &output_path_clone)
                },
                ArtifactType::MacOS(MacOSArtifactType::KnowledgeC) => {
                    collector.collect_knowledgec(&source_path_clone, &output_path_clone)
                },
                ArtifactType::MacOS(MacOSArtifactType::LaunchAgents) => {
                    collector.collect_launch_agents(&source_path_clone, &output_path_clone)
                },
                ArtifactType::MacOS(MacOSArtifactType::LaunchDaemons) => {
                    collector.collect_launch_daemons(&source_path_clone, &output_path_clone)
                },
                _ => {
                    // For other artifact types, use standard file collection
                    if source_path_clone.is_dir() {
                        collector.fallback.collect_directory(&source_path_clone, &output_path_clone)
                    } else {
                        collector.fallback.collect_standard_file(&source_path_clone, &output_path_clone)
                    }
                }
            }
        }).await.context("Task join error")??;
        
        Ok(result)
    }
    
    fn supports_artifact_type(&self, artifact_type: &ArtifactType) -> bool {
        matches!(artifact_type, 
            ArtifactType::MacOS(_) | 
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

// Make MacOSCollector cloneable for use in async blocks
impl Clone for MacOSCollector {
    fn clone(&self) -> Self {
        MacOSCollector {
            fallback: self.fallback.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::config::MacOSArtifactType;

    #[test]
    fn test_macos_collector_new() {
        let collector = MacOSCollector::new();
        // Just verify it creates without panic
        assert!(collector.supports_artifact_type(&ArtifactType::MacOS(MacOSArtifactType::UnifiedLogs)));
    }

    #[test]
    fn test_macos_collector_clone() {
        let collector1 = MacOSCollector::new();
        let collector2 = collector1.clone();
        
        // Both should support the same artifact types
        assert_eq!(
            collector1.supports_artifact_type(&ArtifactType::MacOS(MacOSArtifactType::FSEvents)),
            collector2.supports_artifact_type(&ArtifactType::MacOS(MacOSArtifactType::FSEvents))
        );
    }

    #[test]
    fn test_supports_artifact_type() {
        let collector = MacOSCollector::new();
        
        // macOS-specific types
        assert!(collector.supports_artifact_type(&ArtifactType::MacOS(MacOSArtifactType::UnifiedLogs)));
        assert!(collector.supports_artifact_type(&ArtifactType::MacOS(MacOSArtifactType::FSEvents)));
        assert!(collector.supports_artifact_type(&ArtifactType::MacOS(MacOSArtifactType::Plist)));
        assert!(collector.supports_artifact_type(&ArtifactType::MacOS(MacOSArtifactType::Spotlight)));
        assert!(collector.supports_artifact_type(&ArtifactType::MacOS(MacOSArtifactType::Quarantine)));
        assert!(collector.supports_artifact_type(&ArtifactType::MacOS(MacOSArtifactType::KnowledgeC)));
        assert!(collector.supports_artifact_type(&ArtifactType::MacOS(MacOSArtifactType::LaunchAgents)));
        assert!(collector.supports_artifact_type(&ArtifactType::MacOS(MacOSArtifactType::LaunchDaemons)));
        
        // Generic types
        assert!(collector.supports_artifact_type(&ArtifactType::FileSystem));
        assert!(collector.supports_artifact_type(&ArtifactType::Logs));
        assert!(collector.supports_artifact_type(&ArtifactType::UserData));
        assert!(collector.supports_artifact_type(&ArtifactType::SystemInfo));
        assert!(collector.supports_artifact_type(&ArtifactType::Memory));
        assert!(collector.supports_artifact_type(&ArtifactType::Network));
        assert!(collector.supports_artifact_type(&ArtifactType::Custom));
        
        // Unsupported types
        assert!(!collector.supports_artifact_type(&ArtifactType::Windows(crate::config::WindowsArtifactType::MFT)));
        assert!(!collector.supports_artifact_type(&ArtifactType::Linux(crate::config::LinuxArtifactType::SysLogs)));
    }

    #[tokio::test]
    async fn test_collect_macos_artifact_unified_logs() {
        let collector = MacOSCollector::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Create a test log file
        let test_log_file = temp_dir.path().join("system.log");
        fs::write(&test_log_file, "Test system log content\n").unwrap();
        
        let artifact = Artifact {
            name: "system.log".to_string(),
            artifact_type: ArtifactType::MacOS(MacOSArtifactType::UnifiedLogs),
            source_path: test_log_file.to_string_lossy().to_string(),
            destination_name: "system.log".to_string(),
            description: Some("System logs".to_string()),
            required: true,
            metadata: std::collections::HashMap::new(),
            regex: None,
        };
        
        let output_path = temp_dir.path().join("output").join("system.log");
        let result = collector.collect(&artifact, &output_path).await;
        
        assert!(result.is_ok());
        assert!(output_path.exists());
        let content = fs::read_to_string(&output_path).unwrap();
        assert_eq!(content, "Test system log content\n");
    }

    #[tokio::test]
    async fn test_collect_macos_artifact_fsevents() {
        let collector = MacOSCollector::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Create a test FSEvents directory
        let test_fsevents_dir = temp_dir.path().join("fseventsd");
        fs::create_dir_all(&test_fsevents_dir).unwrap();
        fs::write(test_fsevents_dir.join("0000000000000001"), "fake fsevents data\n").unwrap();
        
        let artifact = Artifact {
            name: "fseventsd".to_string(),
            artifact_type: ArtifactType::MacOS(MacOSArtifactType::FSEvents),
            source_path: test_fsevents_dir.to_string_lossy().to_string(),
            destination_name: "fseventsd".to_string(),
            description: Some("File system events".to_string()),
            required: false,
            metadata: std::collections::HashMap::new(),
            regex: None,
        };
        
        let output_path = temp_dir.path().join("output").join("fseventsd");
        let result = collector.collect(&artifact, &output_path).await;
        
        assert!(result.is_ok());
        assert!(output_path.exists());
        assert!(output_path.is_dir());
    }

    #[tokio::test]
    async fn test_collect_with_env_vars() {
        let collector = MacOSCollector::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Create test directory structure
        let test_home = temp_dir.path().join("Users").join("testuser");
        let library_dir = test_home.join("Library").join("Preferences");
        fs::create_dir_all(&library_dir).unwrap();
        let test_file = library_dir.join("com.apple.LaunchServices.QuarantineEventsV2");
        fs::write(&test_file, "quarantine data\n").unwrap();
        
        // Set environment variable
        std::env::set_var("HOME", test_home.to_string_lossy().to_string());
        
        let artifact = Artifact {
            name: "quarantine".to_string(),
            artifact_type: ArtifactType::MacOS(MacOSArtifactType::Quarantine),
            source_path: "$HOME/Library/Preferences/com.apple.LaunchServices.QuarantineEventsV2".to_string(),
            destination_name: "QuarantineEventsV2".to_string(),
            description: Some("Quarantine database".to_string()),
            required: false,
            metadata: std::collections::HashMap::new(),
            regex: None,
        };
        
        let output_path = temp_dir.path().join("output").join("QuarantineEventsV2");
        let result = collector.collect(&artifact, &output_path).await;
        
        // Cleanup
        std::env::remove_var("HOME");
        
        assert!(result.is_ok());
        assert!(output_path.exists());
        let content = fs::read_to_string(&output_path).unwrap();
        assert_eq!(content, "quarantine data\n");
    }

    #[test]
    fn test_artifact_type_matching() {
        let collector = MacOSCollector::new();
        
        // Create test artifacts for each macOS type
        let test_cases = vec![
            (MacOSArtifactType::UnifiedLogs, "UnifiedLogs"),
            (MacOSArtifactType::FSEvents, "FSEvents"),
            (MacOSArtifactType::Plist, "Plist"),
            (MacOSArtifactType::Spotlight, "Spotlight"),
            (MacOSArtifactType::Quarantine, "Quarantine"),
            (MacOSArtifactType::KnowledgeC, "KnowledgeC"),
            (MacOSArtifactType::LaunchAgents, "LaunchAgents"),
            (MacOSArtifactType::LaunchDaemons, "LaunchDaemons"),
        ];
        
        for (macos_type, name) in test_cases {
            let artifact_type = ArtifactType::MacOS(macos_type);
            assert!(
                collector.supports_artifact_type(&artifact_type),
                "Should support {} artifact type", name
            );
        }
    }

    #[test]
    fn test_non_macos_artifact_rejection() {
        let collector = MacOSCollector::new();
        
        // Test Windows artifact types
        let windows_types = vec![
            crate::config::WindowsArtifactType::MFT,
            crate::config::WindowsArtifactType::Registry,
            crate::config::WindowsArtifactType::EventLog,
        ];
        
        for windows_type in windows_types {
            assert!(
                !collector.supports_artifact_type(&ArtifactType::Windows(windows_type)),
                "Should not support Windows artifact type"
            );
        }
        
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
    }

    #[tokio::test]
    async fn test_collect_plist_text() {
        let collector = MacOSCollector::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Create a test text plist file
        let test_plist = temp_dir.path().join("test.plist");
        let plist_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>TestKey</key>
    <string>TestValue</string>
</dict>
</plist>"#;
        fs::write(&test_plist, plist_content).unwrap();
        
        let artifact = Artifact {
            name: "test.plist".to_string(),
            artifact_type: ArtifactType::MacOS(MacOSArtifactType::Plist),
            source_path: test_plist.to_string_lossy().to_string(),
            destination_name: "test.plist".to_string(),
            description: Some("Test plist".to_string()),
            required: false,
            metadata: std::collections::HashMap::new(),
            regex: None,
        };
        
        let output_path = temp_dir.path().join("output").join("test.plist");
        let result = collector.collect(&artifact, &output_path).await;
        
        assert!(result.is_ok());
        assert!(output_path.exists());
        let content = fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("TestKey"));
        assert!(content.contains("TestValue"));
    }

    #[tokio::test]
    async fn test_collect_launch_agents_directory() {
        let collector = MacOSCollector::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Create a test LaunchAgents directory with plist files
        let launch_agents_dir = temp_dir.path().join("LaunchAgents");
        fs::create_dir_all(&launch_agents_dir).unwrap();
        fs::write(launch_agents_dir.join("com.test.agent1.plist"), "agent1 content\n").unwrap();
        fs::write(launch_agents_dir.join("com.test.agent2.plist"), "agent2 content\n").unwrap();
        
        let artifact = Artifact {
            name: "launch_agents".to_string(),
            artifact_type: ArtifactType::MacOS(MacOSArtifactType::LaunchAgents),
            source_path: launch_agents_dir.to_string_lossy().to_string(),
            destination_name: "LaunchAgents".to_string(),
            description: Some("Launch agents".to_string()),
            required: false,
            metadata: std::collections::HashMap::new(),
            regex: None,
        };
        
        let output_path = temp_dir.path().join("output").join("LaunchAgents");
        let result = collector.collect(&artifact, &output_path).await;
        
        assert!(result.is_ok());
        assert!(output_path.exists());
        assert!(output_path.is_dir());
        assert!(output_path.join("com.test.agent1.plist").exists());
        assert!(output_path.join("com.test.agent2.plist").exists());
    }

    #[tokio::test]
    async fn test_collect_knowledgec_database() {
        let collector = MacOSCollector::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Create a test database file
        let test_db = temp_dir.path().join("knowledgeC.db");
        fs::write(&test_db, "fake database content\n").unwrap();
        
        let artifact = Artifact {
            name: "knowledgec".to_string(),
            artifact_type: ArtifactType::MacOS(MacOSArtifactType::KnowledgeC),
            source_path: test_db.to_string_lossy().to_string(),
            destination_name: "knowledgeC.db".to_string(),
            description: Some("User activity database".to_string()),
            required: false,
            metadata: std::collections::HashMap::new(),
            regex: None,
        };
        
        let output_path = temp_dir.path().join("output").join("knowledgeC.db");
        let result = collector.collect(&artifact, &output_path).await;
        
        assert!(result.is_ok());
        assert!(output_path.exists());
        let content = fs::read_to_string(&output_path).unwrap();
        assert_eq!(content, "fake database content\n");
    }

    #[tokio::test]
    async fn test_collect_spotlight_directory() {
        let collector = MacOSCollector::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Create a test Spotlight directory
        let spotlight_dir = temp_dir.path().join(".Spotlight-V100");
        fs::create_dir_all(&spotlight_dir).unwrap();
        fs::write(spotlight_dir.join("Store-V2"), "spotlight index data\n").unwrap();
        
        let artifact = Artifact {
            name: "spotlight_store".to_string(),
            artifact_type: ArtifactType::MacOS(MacOSArtifactType::Spotlight),
            source_path: spotlight_dir.to_string_lossy().to_string(),
            destination_name: "Spotlight".to_string(),
            description: Some("Spotlight metadata".to_string()),
            required: false,
            metadata: std::collections::HashMap::new(),
            regex: None,
        };
        
        let output_path = temp_dir.path().join("output").join("Spotlight");
        let result = collector.collect(&artifact, &output_path).await;
        
        assert!(result.is_ok());
        assert!(output_path.exists());
        assert!(output_path.is_dir());
        assert!(output_path.join("Store-V2").exists());
    }
}
