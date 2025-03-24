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
