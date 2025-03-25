use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, Context};
use futures::future::{self, FutureExt};
use log::{info, warn, debug};
use tokio::sync::{Mutex, Semaphore};

use crate::models::ArtifactMetadata;
use crate::config::{Artifact, ArtifactType, WindowsArtifactType};
use crate::collectors::platforms;
use crate::collectors::regex::RegexCollector;

/// Trait for artifact collectors
#[async_trait::async_trait]
pub trait ArtifactCollector: Send + Sync {
    async fn collect(&self, artifact: &Artifact, output_dir: &Path) -> Result<ArtifactMetadata>;
    fn supports_artifact_type(&self, artifact_type: &ArtifactType) -> bool;
}

/// Check if an artifact is a special case that doesn't have a standard file path
fn is_special_artifact(artifact_type: &ArtifactType) -> bool {
    match artifact_type {
        // Windows special artifacts
        ArtifactType::Windows(WindowsArtifactType::MFT) => true,
        ArtifactType::Windows(WindowsArtifactType::USNJournal) => true,
        
        // Other special artifacts that might not have standard paths
        _ => false
    }
}

/// Determine the destination path for an artifact based on its original path
fn get_destination_path(fs_dir: &Path, artifact: &Artifact) -> PathBuf {
    // For special artifacts that don't have a standard file path
    if is_special_artifact(&artifact.artifact_type) {
        return fs_dir.join(&artifact.destination_name);
    }
    
    // For regular files, preserve the original path structure
    let source_path = Path::new(&artifact.source_path);
    
    // Handle absolute paths by removing the leading separator
    let rel_path = if source_path.is_absolute() {
        // Convert /etc/passwd to etc/passwd or C:\Windows\System32 to Windows\System32
        let path_str = source_path.to_string_lossy();
        
        // Handle Windows paths with drive letters
        if cfg!(windows) && path_str.chars().nth(1) == Some(':') {
            // Remove drive letter (e.g., C:\Windows -> Windows)
            let without_drive = path_str.chars().skip(3).collect::<String>();
            PathBuf::from(without_drive)
        } else {
            // Remove leading slash for Unix paths
            let path_without_root = path_str.trim_start_matches('/').trim_start_matches('\\');
            PathBuf::from(path_without_root)
        }
    } else {
        source_path.to_path_buf()
    };
    
    fs_dir.join(rel_path)
}

/// Handle potential duplicate filenames by adding a numeric suffix
fn handle_duplicate_filename(dest_path: &Path) -> PathBuf {
    if dest_path.exists() {
        // Add a numeric suffix to the filename
        let file_stem = dest_path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());
        
        let extension = dest_path.extension()
            .map(|s| format!(".{}", s.to_string_lossy()))
            .unwrap_or_else(|| "".to_string());
        
        let mut counter = 1;
        loop {
            let new_name = format!("{}_{}{}", file_stem, counter, extension);
            let new_path = dest_path.with_file_name(new_name);
            if !new_path.exists() {
                return new_path;
            }
            counter += 1;
        }
    } else {
        dest_path.to_path_buf()
    }
}

/// Normalize path for storage (convert backslashes to forward slashes)
fn normalize_path_for_storage(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Collect artifacts based on configuration with parallel execution
pub async fn collect_artifacts_parallel(
    artifacts: &[Artifact],
    base_dir: &Path
) -> Result<HashMap<String, ArtifactMetadata>> {
    // Make sure base directory exists
    tokio::fs::create_dir_all(base_dir)
        .await
        .context("Failed to create base directory")?;
    
    // Create a single 'fs' directory instead of type-based directories
    let fs_dir = base_dir.join("fs");
    tokio::fs::create_dir_all(&fs_dir)
        .await
        .context("Failed to create fs directory")?;
    
    // Create a rate limiter to control concurrent artifact collection
    // This prevents overwhelming the system with too many concurrent I/O operations
    let max_concurrent = std::cmp::min(num_cpus::get() * 2, 32); // Limit concurrency
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    
    // Get the platform-specific collector
    let collector = Arc::new(platforms::get_platform_collector());
    
    // Filter artifacts for the current platform
    let platform_artifacts = platforms::filter_artifacts_for_platform(artifacts);
    
    // Shared results map protected by a mutex
    let results = Arc::new(Mutex::new(HashMap::new()));
    
    // Process all artifacts in parallel with controlled concurrency
    let futures = platform_artifacts.iter().map(|artifact| {
        // Clone references for the async block
        let collector = Arc::clone(&collector);
        let results = Arc::clone(&results);
        let semaphore = Arc::clone(&semaphore);
        let artifact = artifact.clone(); // Clone the artifact for the async move block
        let fs_dir = fs_dir.clone();
        let base_dir = base_dir.to_path_buf();
        
        async move {
            // Acquire a permit from the semaphore, limiting concurrency
            let _permit = semaphore.acquire().await.unwrap();
            
            info!("Collecting artifact: {}", artifact.name);
            
            // Determine output path based on original file path
            let output_path = get_destination_path(&fs_dir, &artifact);
            
            // Handle potential duplicate filenames
            let final_output_path = handle_duplicate_filename(&output_path);
            
            // Create parent directories if they don't exist
            if let Some(parent) = final_output_path.parent() {
                if let Err(e) = tokio::fs::create_dir_all(parent).await {
                    warn!("Failed to create directory {}: {}", parent.display(), e);
                }
            }
            
            // Check if this is a regex-based artifact
            if let Some(regex_config) = &artifact.regex {
                if regex_config.enabled {
                    // Use regex collector for this artifact
                    let regex_collector = RegexCollector::new();
                    let source_path = PathBuf::from(&artifact.source_path);
                    
                    match regex_collector.collect_with_regex(
                        &artifact,
                        &source_path,
                        &final_output_path.parent().unwrap_or(&fs_dir)
                    ).await {
                        Ok(collected_items) => {
                            let mut map = results.lock().await;
                            for (path, metadata) in collected_items {
                                let relative_path = normalize_path_for_storage(&path.strip_prefix(&base_dir).unwrap_or(&path));
                                map.insert(relative_path, metadata);
                            }
                            info!("Successfully collected regex artifact: {}", artifact.name);
                        },
                        Err(e) => {
                            // If the artifact is required, report the error but continue
                            if artifact.required {
                                warn!("Failed to collect required regex artifact {}: {}", artifact.name, e);
                            } else {
                                debug!("Failed to collect optional regex artifact {}: {}", artifact.name, e);
                            }
                        }
                    }
                } else {
                    // Standard collection for non-regex artifacts
                    match collector.collect(&artifact, &final_output_path.parent().unwrap_or(&fs_dir)).await {
                        Ok(metadata) => {
                            // Create a relative path for the result that preserves the original structure
                            let relative_path = normalize_path_for_storage(&final_output_path.strip_prefix(&base_dir).unwrap_or(&final_output_path));
                            
                            // Add result to the shared map
                            let mut map = results.lock().await;
                            map.insert(relative_path, metadata);
                            info!("Successfully collected: {}", artifact.name);
                        },
                        Err(e) => {
                            // If the artifact is required, report the error but continue
                            if artifact.required {
                                warn!("Failed to collect required artifact {}: {}", artifact.name, e);
                            } else {
                                debug!("Failed to collect optional artifact {}: {}", artifact.name, e);
                            }
                        }
                    }
                }
            } else {
                // Standard collection for non-regex artifacts
                match collector.collect(&artifact, &final_output_path.parent().unwrap_or(&fs_dir)).await {
                    Ok(metadata) => {
                        // Create a relative path for the result that preserves the original structure
                        let relative_path = normalize_path_for_storage(&final_output_path.strip_prefix(&base_dir).unwrap_or(&final_output_path));
                        
                        // Add result to the shared map
                        let mut map = results.lock().await;
                        map.insert(relative_path, metadata);
                        info!("Successfully collected: {}", artifact.name);
                    },
                    Err(e) => {
                        // If the artifact is required, report the error but continue
                        if artifact.required {
                            warn!("Failed to collect required artifact {}: {}", artifact.name, e);
                        } else {
                            debug!("Failed to collect optional artifact {}: {}", artifact.name, e);
                        }
                    }
                }
            }
        }.boxed()
    });
    
    // Execute all futures concurrently with controlled parallelism
    future::join_all(futures).await;
    
    // Extract results from the mutex
    let final_results = results.lock().await.clone();
    Ok(final_results)
}

/// Legacy synchronous collection function that calls the async implementation
pub fn collect_artifacts(
    artifacts: &[Artifact],
    base_dir: &Path
) -> Result<HashMap<String, ArtifactMetadata>> {
    // Create a new runtime for running the async function
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(num_cpus::get())
        .enable_all()
        .build()
        .context("Failed to create Tokio runtime")?;
    
    // Run the async function in the runtime
    runtime.block_on(collect_artifacts_parallel(artifacts, base_dir))
}
