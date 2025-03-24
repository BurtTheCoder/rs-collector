use std::path::{Path, PathBuf};
use std::fs;
use std::io;

use anyhow::{Result, Context};
use log::debug;

use crate::models::ArtifactMetadata;
use crate::config::{Artifact, ArtifactType};
use crate::collectors::collector::ArtifactCollector;

/// Fallback collector for platforms without specific implementations
pub struct FallbackCollector;

impl FallbackCollector {
    pub fn new() -> Self {
        FallbackCollector
    }
    
    /// Standard file collection method that works on all platforms
    pub fn collect_standard_file(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        debug!("Collecting standard file from {} to {}", source.display(), dest.display());
        
        // Create parent directories if they don't exist
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .context(format!("Failed to create directory: {}", parent.display()))?;
        }
        
        // Get file metadata before copying
        let metadata = fs::metadata(source)
            .context(format!("Failed to get metadata for {}", source.display()))?;
        
        // Copy the file
        fs::copy(source, dest)
            .context(format!("Failed to copy {} to {}", source.display(), dest.display()))?;
        
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
    
    /// Directory collection method that recursively copies directories
    pub fn collect_directory(&self, source: &Path, dest: &Path) -> Result<ArtifactMetadata> {
        debug!("Collecting directory from {} to {}", source.display(), dest.display());
        
        // Create the destination directory
        fs::create_dir_all(dest)
            .context(format!("Failed to create directory: {}", dest.display()))?;
        
        // Get directory metadata
        let metadata = fs::metadata(source)
            .context(format!("Failed to get metadata for {}", source.display()))?;
        
        // Recursively copy directory contents
        self.copy_dir_contents(source, dest)?;
        
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
            file_size: 0, // Will be updated with total size
            created_time,
            accessed_time,
            modified_time,
            is_locked: false,
        };
        
        Ok(artifact_metadata)
    }
    
    /// Helper method to recursively copy directory contents
    fn copy_dir_contents(&self, source: &Path, dest: &Path) -> Result<()> {
        for entry in fs::read_dir(source)
            .context(format!("Failed to read directory: {}", source.display()))? {
            
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();
            let file_name = entry.file_name();
            let dest_path = dest.join(file_name);
            
            if path.is_dir() {
                fs::create_dir_all(&dest_path)
                    .context(format!("Failed to create directory: {}", dest_path.display()))?;
                self.copy_dir_contents(&path, &dest_path)?;
            } else {
                fs::copy(&path, &dest_path)
                    .context(format!("Failed to copy {} to {}", path.display(), dest_path.display()))?;
            }
        }
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl ArtifactCollector for FallbackCollector {
    async fn collect(&self, artifact: &Artifact, output_dir: &Path) -> Result<ArtifactMetadata> {
        let source_path = PathBuf::from(&artifact.source_path);
        
        // Use the output directory directly instead of joining with destination name
        // The path structure is now handled by the main collector function
        let output_path = output_dir.to_path_buf();
        
        debug!("Collecting {} from {} to {}", artifact.name, source_path.display(), output_path.display());
        
        // Check if source is a directory or file
        let metadata = match fs::metadata(&source_path) {
            Ok(meta) => meta,
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    return Err(anyhow::anyhow!("Source not found: {}", source_path.display()));
                } else {
                    return Err(anyhow::anyhow!("Failed to access source: {}", e));
                }
            }
        };
        
        // Clone self for the async block
        let collector = self.clone();
        let source_path_clone = source_path.clone();
        let output_path_clone = output_path.clone();
        
        // Use tokio::task::spawn_blocking for file I/O operations
        let result = tokio::task::spawn_blocking(move || {
            if metadata.is_dir() {
                collector.collect_directory(&source_path_clone, &output_path_clone)
            } else {
                collector.collect_standard_file(&source_path_clone, &output_path_clone)
            }
        }).await.context("Task join error")??;
        
        Ok(result)
    }
    
    fn supports_artifact_type(&self, _artifact_type: &ArtifactType) -> bool {
        // Fallback collector supports all artifact types, but with limited functionality
        true
    }
}

// Make FallbackCollector cloneable for use in async blocks
impl Clone for FallbackCollector {
    fn clone(&self) -> Self {
        FallbackCollector
    }
}
