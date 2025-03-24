use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use log::{info, debug};

use crate::models::ArtifactMetadata;

/// Mock implementation of privilege elevation for non-Windows platforms
#[allow(dead_code)]
pub fn enable_privileges() -> Result<()> {
    info!("Running on non-Windows platform, privileges are mocked");
    info!("On Windows, this would enable SeBackupPrivilege, SeRestorePrivilege, etc.");
    Ok(())
}

/// Mock implementation of raw file access for non-Windows platforms
pub fn collect_with_raw_handle(source_path: &str, dest_path: &Path) -> Result<ArtifactMetadata> {
    debug!("Mock collecting {} to {}", source_path, dest_path.display());
    
    // In a real implementation, we would use Windows API to open files with backup semantics
    // Instead, we'll create an empty file for testing purposes
    
    // Create parent directories if they don't exist
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)
            .context(format!("Failed to create parent directories for {}", dest_path.display()))?;
    }
    
    // Create the file
    fs::File::create(dest_path)
        .context(format!("Failed to create output file: {}", dest_path.display()))?;
    
    // Get current time for metadata
    let collection_time = chrono::Utc::now().to_rfc3339();
    
    // Create metadata with mock values
    let metadata = ArtifactMetadata {
        original_path: source_path.to_string(),
        collection_time: collection_time.clone(),
        file_size: 0, // Mock file size
        created_time: Some(collection_time.clone()),
        accessed_time: Some(collection_time.clone()),
        modified_time: Some(collection_time),
        is_locked: false,
    };
    
    info!("Mock implementation: File would be collected with backup semantics on Windows");
    Ok(metadata)
}
