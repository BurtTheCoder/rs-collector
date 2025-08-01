use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;

use anyhow::{Result, Context};
use log::{debug, info, warn};
use tokio::task;

use crate::models::ArtifactMetadata;
use crate::config::{Artifact, ArtifactType, LinuxArtifactType};
use crate::collectors::collector::ArtifactCollector;
use crate::config::parse_unix_env_vars;
use crate::collectors::platforms::common::FallbackCollector;
use crate::privileges::is_elevated;
use crate::constants::{PROC_PATH};

/// Linux-specific artifact collector
pub struct LinuxCollector {
    fallback: FallbackCollector,
}

impl LinuxCollector {
    #[allow(dead_code)]
    pub fn new() -> Self {
        info!("Initializing Linux artifact collector");
        
        // Check for root privileges
        if !is_elevated() {
            warn!("Not running as root - some system files may be inaccessible");
        }
        
        // Check for journalctl availability
        let has_journalctl = Command::new("which")
            .arg("journalctl")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
            
        if !has_journalctl {
            warn!("journalctl not found - journal collection may be limited");
        }
        
        LinuxCollector {
            fallback: FallbackCollector::new(),
        }
    }
    
    /// Collect system logs
    fn collect_syslogs(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        info!("Collecting system logs");
        
        if source.is_dir() {
            self.fallback.collect_directory(source, dest)
        } else {
            self.fallback.collect_standard_file(source, dest)
        }
    }
    
    /// Collect journal logs using journalctl
    fn collect_journal(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        info!("Collecting journal logs");
        
        // Create parent directories if they don't exist
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .context(format!("Failed to create directory: {}", parent.display()))?;
        }
        
        // Use journalctl to export logs
        let output = Command::new("journalctl")
            .arg("--no-pager")
            .arg("--output=json")
            .arg("--since=yesterday")
            .output()
            .context("Failed to execute journalctl")?;
        
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            warn!("journalctl command failed: {}", error);
            
            // Fall back to copying the journal directory
            return self.fallback.collect_directory(source, dest);
        }
        
        // Write the output to the destination file
        fs::write(dest, output.stdout)
            .context(format!("Failed to write journal logs to {}", dest.display()))?;
        
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
    
    /// Collect proc filesystem entries
    fn collect_proc(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        info!("Collecting proc filesystem entry");
        
        // Special handling for /proc
        let proc_self = PathBuf::from(PROC_PATH).join("self");
        let proc_thread_self = PathBuf::from(PROC_PATH).join("thread-self");
        if source.starts_with(&proc_self) || source.starts_with(&proc_thread_self) {
            warn!("Skipping self-referential proc entry: {}", source.display());
            
            // Create an empty file with a note
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)
                    .context(format!("Failed to create directory: {}", parent.display()))?;
            }
            
            fs::write(dest, format!("Skipped self-referential proc entry: {}\n", source.display()))
                .context(format!("Failed to write note to {}", dest.display()))?;
            
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
            
            return Ok(ArtifactMetadata {
                original_path: source.to_string_lossy().to_string(),
                collection_time,
                file_size: metadata.len(),
                created_time,
                accessed_time,
                modified_time,
                is_locked: false,
            });
        }
        
        // For other proc entries, try standard collection
        if source.is_dir() {
            self.fallback.collect_directory(source, dest)
        } else {
            self.fallback.collect_standard_file(source, dest)
        }
    }
    
    /// Collect audit logs
    fn collect_audit(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        info!("Collecting audit logs");
        self.fallback.collect_standard_file(source, dest)
    }
    
    /// Collect cron jobs
    fn collect_cron(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        info!("Collecting cron jobs");
        
        if source.is_dir() {
            self.fallback.collect_directory(source, dest)
        } else {
            self.fallback.collect_standard_file(source, dest)
        }
    }
    
    /// Collect bash history
    fn collect_bash(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        info!("Collecting bash history");
        self.fallback.collect_standard_file(source, dest)
    }
    
    /// Collect package manager logs
    fn collect_package_logs(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        info!("Collecting package manager logs");
        
        if source.is_dir() {
            self.fallback.collect_directory(source, dest)
        } else {
            self.fallback.collect_standard_file(source, dest)
        }
    }
    
    /// Collect systemd units
    fn collect_systemd(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        info!("Collecting systemd units");
        
        if source.is_dir() {
            self.fallback.collect_directory(source, dest)
        } else {
            self.fallback.collect_standard_file(source, dest)
        }
    }
}

#[async_trait::async_trait]
impl ArtifactCollector for LinuxCollector {
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
                ArtifactType::Linux(LinuxArtifactType::SysLogs) => {
                    collector.collect_syslogs(&source_path_clone, &output_path_clone)
                },
                ArtifactType::Linux(LinuxArtifactType::Journal) => {
                    collector.collect_journal(&source_path_clone, &output_path_clone)
                },
                ArtifactType::Linux(LinuxArtifactType::Proc) => {
                    collector.collect_proc(&source_path_clone, &output_path_clone)
                },
                ArtifactType::Linux(LinuxArtifactType::Audit) => {
                    collector.collect_audit(&source_path_clone, &output_path_clone)
                },
                ArtifactType::Linux(LinuxArtifactType::Cron) => {
                    collector.collect_cron(&source_path_clone, &output_path_clone)
                },
                ArtifactType::Linux(LinuxArtifactType::Bash) => {
                    collector.collect_bash(&source_path_clone, &output_path_clone)
                },
                ArtifactType::Linux(LinuxArtifactType::Apt) |
                ArtifactType::Linux(LinuxArtifactType::Dpkg) |
                ArtifactType::Linux(LinuxArtifactType::Yum) => {
                    collector.collect_package_logs(&source_path_clone, &output_path_clone)
                },
                ArtifactType::Linux(LinuxArtifactType::Systemd) => {
                    collector.collect_systemd(&source_path_clone, &output_path_clone)
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
            ArtifactType::Linux(_) | 
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

// Make LinuxCollector cloneable for use in async blocks
impl Clone for LinuxCollector {
    fn clone(&self) -> Self {
        LinuxCollector {
            fallback: self.fallback.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::config::LinuxArtifactType;

    #[test]
    fn test_linux_collector_new() {
        let collector = LinuxCollector::new();
        // Just verify it creates without panic
        assert!(collector.supports_artifact_type(&ArtifactType::Linux(LinuxArtifactType::SysLogs)));
    }

    #[test]
    fn test_linux_collector_clone() {
        let collector1 = LinuxCollector::new();
        let collector2 = collector1.clone();
        
        // Both should support the same artifact types
        assert_eq!(
            collector1.supports_artifact_type(&ArtifactType::Linux(LinuxArtifactType::Journal)),
            collector2.supports_artifact_type(&ArtifactType::Linux(LinuxArtifactType::Journal))
        );
    }

    #[test]
    fn test_supports_artifact_type() {
        let collector = LinuxCollector::new();
        
        // Linux-specific types
        assert!(collector.supports_artifact_type(&ArtifactType::Linux(LinuxArtifactType::SysLogs)));
        assert!(collector.supports_artifact_type(&ArtifactType::Linux(LinuxArtifactType::Journal)));
        assert!(collector.supports_artifact_type(&ArtifactType::Linux(LinuxArtifactType::Proc)));
        assert!(collector.supports_artifact_type(&ArtifactType::Linux(LinuxArtifactType::Audit)));
        assert!(collector.supports_artifact_type(&ArtifactType::Linux(LinuxArtifactType::Cron)));
        assert!(collector.supports_artifact_type(&ArtifactType::Linux(LinuxArtifactType::Bash)));
        assert!(collector.supports_artifact_type(&ArtifactType::Linux(LinuxArtifactType::Apt)));
        assert!(collector.supports_artifact_type(&ArtifactType::Linux(LinuxArtifactType::Dpkg)));
        assert!(collector.supports_artifact_type(&ArtifactType::Linux(LinuxArtifactType::Yum)));
        assert!(collector.supports_artifact_type(&ArtifactType::Linux(LinuxArtifactType::Systemd)));
        
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
        assert!(!collector.supports_artifact_type(&ArtifactType::MacOS(crate::config::MacOSArtifactType::UnifiedLogs)));
    }

    #[tokio::test]
    async fn test_collect_linux_artifact_syslog() {
        let collector = LinuxCollector::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Create a test syslog file
        let test_log_dir = temp_dir.path().join("var").join("log");
        fs::create_dir_all(&test_log_dir).unwrap();
        let test_log_file = test_log_dir.join("syslog");
        fs::write(&test_log_file, "Test syslog content\n").unwrap();
        
        let artifact = Artifact {
            name: "syslog".to_string(),
            artifact_type: ArtifactType::Linux(LinuxArtifactType::SysLogs),
            source_path: test_log_file.to_string_lossy().to_string(),
            destination_name: "syslog".to_string(),
            description: Some("System logs".to_string()),
            required: true,
            metadata: std::collections::HashMap::new(),
            regex: None,
        };
        
        let output_path = temp_dir.path().join("output").join("syslog");
        let result = collector.collect(&artifact, &output_path).await;
        
        assert!(result.is_ok());
        assert!(output_path.exists());
        let content = fs::read_to_string(&output_path).unwrap();
        assert_eq!(content, "Test syslog content\n");
    }

    #[tokio::test]
    async fn test_collect_linux_artifact_proc() {
        let collector = LinuxCollector::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Create a test proc file
        let test_proc_file = temp_dir.path().join("cmdline");
        fs::write(&test_proc_file, "test command line\n").unwrap();
        
        let artifact = Artifact {
            name: "proc-cmdline".to_string(),
            artifact_type: ArtifactType::Linux(LinuxArtifactType::Proc),
            source_path: test_proc_file.to_string_lossy().to_string(),
            destination_name: "proc_cmdline".to_string(),
            description: Some("Kernel command line".to_string()),
            required: false,
            metadata: std::collections::HashMap::new(),
            regex: None,
        };
        
        let output_path = temp_dir.path().join("output").join("proc_cmdline");
        let result = collector.collect(&artifact, &output_path).await;
        
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[tokio::test]
    async fn test_collect_proc_self_referential() {
        let collector = LinuxCollector::new();
        let temp_dir = TempDir::new().unwrap();
        
        let artifact = Artifact {
            name: "proc-self".to_string(),
            artifact_type: ArtifactType::Linux(LinuxArtifactType::Proc),
            source_path: format!("{}/self/status", PROC_PATH),
            destination_name: "proc_self_status".to_string(),
            description: Some("Process status".to_string()),
            required: false,
            metadata: std::collections::HashMap::new(),
            regex: None,
        };
        
        let output_path = temp_dir.path().join("output").join("proc_self_status");
        let result = collector.collect(&artifact, &output_path).await;
        
        // Should succeed but create a note file
        assert!(result.is_ok());
        assert!(output_path.exists());
        let content = fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("Skipped self-referential proc entry"));
    }

    #[tokio::test]
    async fn test_collect_with_env_vars() {
        let collector = LinuxCollector::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Create test directory structure
        let test_home = temp_dir.path().join("home").join("user");
        fs::create_dir_all(&test_home).unwrap();
        let test_file = test_home.join(".bash_history");
        fs::write(&test_file, "history content\n").unwrap();
        
        // Set environment variable
        std::env::set_var("HOME", test_home.to_string_lossy().to_string());
        
        let artifact = Artifact {
            name: "bash_history".to_string(),
            artifact_type: ArtifactType::Linux(LinuxArtifactType::Bash),
            source_path: "$HOME/.bash_history".to_string(),
            destination_name: "bash_history".to_string(),
            description: Some("Bash command history".to_string()),
            required: false,
            metadata: std::collections::HashMap::new(),
            regex: None,
        };
        
        let output_path = temp_dir.path().join("output").join("bash_history");
        let result = collector.collect(&artifact, &output_path).await;
        
        // Cleanup
        std::env::remove_var("HOME");
        
        assert!(result.is_ok());
        assert!(output_path.exists());
        let content = fs::read_to_string(&output_path).unwrap();
        assert_eq!(content, "history content\n");
    }

    #[test]
    fn test_artifact_type_matching() {
        let collector = LinuxCollector::new();
        
        // Create test artifacts for each Linux type
        let test_cases = vec![
            (LinuxArtifactType::SysLogs, "SysLogs"),
            (LinuxArtifactType::Journal, "Journal"),
            (LinuxArtifactType::Proc, "Proc"),
            (LinuxArtifactType::Audit, "Audit"),
            (LinuxArtifactType::Cron, "Cron"),
            (LinuxArtifactType::Bash, "Bash"),
            (LinuxArtifactType::Apt, "Apt"),
            (LinuxArtifactType::Dpkg, "Dpkg"),
            (LinuxArtifactType::Yum, "Yum"),
            (LinuxArtifactType::Systemd, "Systemd"),
        ];
        
        for (linux_type, name) in test_cases {
            let artifact_type = ArtifactType::Linux(linux_type);
            assert!(
                collector.supports_artifact_type(&artifact_type),
                "Should support {} artifact type", name
            );
        }
    }

    #[test]
    fn test_non_linux_artifact_rejection() {
        let collector = LinuxCollector::new();
        
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
    async fn test_collect_directory_artifact() {
        let collector = LinuxCollector::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Create a test directory with files
        let test_cron_dir = temp_dir.path().join("etc").join("cron.d");
        fs::create_dir_all(&test_cron_dir).unwrap();
        fs::write(test_cron_dir.join("job1"), "0 * * * * root /bin/test1\n").unwrap();
        fs::write(test_cron_dir.join("job2"), "0 * * * * root /bin/test2\n").unwrap();
        
        let artifact = Artifact {
            name: "cron.d".to_string(),
            artifact_type: ArtifactType::Linux(LinuxArtifactType::Cron),
            source_path: test_cron_dir.to_string_lossy().to_string(),
            destination_name: "cron.d".to_string(),
            description: Some("System cron jobs".to_string()),
            required: false,
            metadata: std::collections::HashMap::new(),
            regex: None,
        };
        
        let output_path = temp_dir.path().join("output").join("cron.d");
        let result = collector.collect(&artifact, &output_path).await;
        
        assert!(result.is_ok());
        assert!(output_path.exists());
        assert!(output_path.is_dir());
        assert!(output_path.join("job1").exists());
        assert!(output_path.join("job2").exists());
    }

    #[tokio::test]
    async fn test_collect_journal_fallback() {
        let collector = LinuxCollector::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Create a fake journal directory
        let journal_dir = temp_dir.path().join("var").join("log").join("journal");
        fs::create_dir_all(&journal_dir).unwrap();
        fs::write(journal_dir.join("system.journal"), "fake journal data\n").unwrap();
        
        let artifact = Artifact {
            name: "journal".to_string(),
            artifact_type: ArtifactType::Linux(LinuxArtifactType::Journal),
            source_path: journal_dir.to_string_lossy().to_string(),
            destination_name: "journal".to_string(),
            description: Some("Systemd journal logs".to_string()),
            required: false,
            metadata: std::collections::HashMap::new(),
            regex: None,
        };
        
        let output_path = temp_dir.path().join("output").join("journal");
        let result = collector.collect(&artifact, &output_path).await;
        
        // Should succeed (either with journalctl or fallback)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_collect_package_logs() {
        let collector = LinuxCollector::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Create test package log files
        let dpkg_log = temp_dir.path().join("dpkg.log");
        fs::write(&dpkg_log, "2024-01-01 package installed\n").unwrap();
        
        let test_cases = vec![
            (LinuxArtifactType::Dpkg, "dpkg.log"),
            (LinuxArtifactType::Apt, "apt.log"),
            (LinuxArtifactType::Yum, "yum.log"),
        ];
        
        for (pkg_type, filename) in test_cases {
            let artifact = Artifact {
                name: filename.to_string(),
                artifact_type: ArtifactType::Linux(pkg_type),
                source_path: dpkg_log.to_string_lossy().to_string(),
                destination_name: filename.to_string(),
                description: Some("Package logs".to_string()),
                required: false,
                metadata: std::collections::HashMap::new(),
                regex: None,
            };
            
            let output_path = temp_dir.path().join("output").join(filename);
            let result = collector.collect(&artifact, &output_path).await;
            
            assert!(result.is_ok());
            assert!(output_path.exists());
        }
    }
}
