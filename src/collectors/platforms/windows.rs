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
