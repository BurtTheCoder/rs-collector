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
        if source.starts_with("/proc/self") || source.starts_with("/proc/thread-self") {
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
