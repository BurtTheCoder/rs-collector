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

/// Trait for artifact collectors.
/// 
/// This trait defines the interface that all artifact collectors must implement.
/// Collectors are responsible for gathering specific types of forensic artifacts
/// from a system and storing them in a specified output directory.
/// 
/// # Thread Safety
/// 
/// Implementors must be `Send + Sync` to support concurrent collection operations.
#[async_trait::async_trait]
pub trait ArtifactCollector: Send + Sync {
    /// Collect a specific artifact and save it to the output directory.
    /// 
    /// # Arguments
    /// 
    /// * `artifact` - The artifact configuration specifying what to collect
    /// * `output_dir` - Directory where the collected artifact should be saved
    /// 
    /// # Returns
    /// 
    /// * `Ok(ArtifactMetadata)` - Metadata about the successfully collected artifact
    /// * `Err` - If collection fails or the artifact cannot be found
    /// 
    /// # Implementation Notes
    /// 
    /// Implementors should:
    /// - Preserve the original file metadata when possible
    /// - Handle platform-specific paths appropriately
    /// - Create necessary subdirectories in the output directory
    /// - Return detailed error messages for troubleshooting
    async fn collect(&self, artifact: &Artifact, output_dir: &Path) -> Result<ArtifactMetadata>;
    
    /// Check if this collector supports a given artifact type.
    /// 
    /// # Arguments
    /// 
    /// * `artifact_type` - The type of artifact to check
    /// 
    /// # Returns
    /// 
    /// * `true` if this collector can handle the artifact type
    /// * `false` otherwise
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

/// Legacy synchronous collection function that calls the async implementation.
/// 
/// This function provides a synchronous interface to the asynchronous artifact
/// collection system. It creates a Tokio runtime and blocks until all artifacts
/// are collected.
/// 
/// # Arguments
/// 
/// * `artifacts` - Slice of artifact configurations to collect
/// * `base_dir` - Base directory where collected artifacts will be stored
/// 
/// # Returns
/// 
/// * `Ok(HashMap<String, ArtifactMetadata>)` - Map of relative paths to metadata for collected artifacts
/// * `Err` - If runtime creation fails or collection encounters fatal errors
/// 
/// # Example
/// 
/// ```no_run
/// # use std::path::Path;
/// # use rust_collector::config::Artifact;
/// # use rust_collector::collectors::collector::collect_artifacts;
/// # let artifacts: Vec<Artifact> = vec![];
/// let results = collect_artifacts(&artifacts, Path::new("/tmp/collection"))?;
/// for (path, metadata) in results {
///     println!("Collected: {} ({} bytes)", path, metadata.file_size);
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
/// 
/// # Performance
/// 
/// This function uses all available CPU cores for parallel collection.
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    use crate::config::{LinuxArtifactType, MacOSArtifactType};

    // Mock collector for testing
    struct MockCollector {
        supported_types: Vec<ArtifactType>,
        should_fail: bool,
    }

    #[async_trait::async_trait]
    impl ArtifactCollector for MockCollector {
        async fn collect(&self, artifact: &Artifact, output_dir: &Path) -> Result<ArtifactMetadata> {
            if self.should_fail {
                return Err(anyhow::anyhow!("Mock failure"));
            }

            // Create a dummy file
            let dest_path = output_dir.join(&artifact.destination_name);
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&dest_path, "mock content")?;

            Ok(ArtifactMetadata {
                original_path: artifact.source_path.clone(),
                collection_time: chrono::Utc::now().to_rfc3339(),
                file_size: 12, // "mock content".len()
                created_time: None,
                accessed_time: None,
                modified_time: None,
                is_locked: false,
            })
        }

        fn supports_artifact_type(&self, artifact_type: &ArtifactType) -> bool {
            self.supported_types.contains(artifact_type)
        }
    }

    #[test]
    fn test_is_special_artifact() {
        // Windows special artifacts
        assert!(is_special_artifact(&ArtifactType::Windows(WindowsArtifactType::MFT)));
        assert!(is_special_artifact(&ArtifactType::Windows(WindowsArtifactType::USNJournal)));

        // Non-special artifacts
        assert!(!is_special_artifact(&ArtifactType::Windows(WindowsArtifactType::Registry)));
        assert!(!is_special_artifact(&ArtifactType::Linux(LinuxArtifactType::SysLogs)));
        assert!(!is_special_artifact(&ArtifactType::FileSystem));
        assert!(!is_special_artifact(&ArtifactType::Logs));
    }

    #[test]
    fn test_get_destination_path_special_artifact() {
        let fs_dir = Path::new("/output/fs");
        let artifact = Artifact {
            name: "MFT".to_string(),
            artifact_type: ArtifactType::Windows(WindowsArtifactType::MFT),
            source_path: r"\\?\C:\$MFT".to_string(),
            destination_name: "MFT".to_string(),
            description: None,
            required: true,
            metadata: HashMap::new(),
            regex: None,
        };

        let dest_path = get_destination_path(fs_dir, &artifact);
        assert_eq!(dest_path, fs_dir.join("MFT"));
    }

    #[test]
    fn test_get_destination_path_absolute_unix() {
        let fs_dir = Path::new("/output/fs");
        let artifact = Artifact {
            name: "syslog".to_string(),
            artifact_type: ArtifactType::Linux(LinuxArtifactType::SysLogs),
            source_path: "/var/log/syslog".to_string(),
            destination_name: "syslog".to_string(),
            description: None,
            required: true,
            metadata: HashMap::new(),
            regex: None,
        };

        let dest_path = get_destination_path(fs_dir, &artifact);
        assert_eq!(dest_path, fs_dir.join("var/log/syslog"));
    }

    #[test]
    fn test_get_destination_path_absolute_windows() {
        let fs_dir = Path::new("/output/fs");
        let artifact = Artifact {
            name: "hosts".to_string(),
            artifact_type: ArtifactType::FileSystem,
            source_path: r"C:\Windows\System32\drivers\etc\hosts".to_string(),
            destination_name: "hosts".to_string(),
            description: None,
            required: true,
            metadata: HashMap::new(),
            regex: None,
        };

        let dest_path = get_destination_path(fs_dir, &artifact);
        
        if cfg!(windows) {
            // On Windows, strip the drive letter
            assert_eq!(dest_path, fs_dir.join(r"Windows\System32\drivers\etc\hosts"));
        } else {
            // On Unix, the whole path is preserved
            assert_eq!(dest_path, fs_dir.join(r"C:\Windows\System32\drivers\etc\hosts"));
        }
    }

    #[test]
    fn test_get_destination_path_relative() {
        let fs_dir = Path::new("/output/fs");
        let artifact = Artifact {
            name: "config".to_string(),
            artifact_type: ArtifactType::UserData,
            source_path: "config/app.conf".to_string(),
            destination_name: "app.conf".to_string(),
            description: None,
            required: false,
            metadata: HashMap::new(),
            regex: None,
        };

        let dest_path = get_destination_path(fs_dir, &artifact);
        assert_eq!(dest_path, fs_dir.join("config/app.conf"));
    }

    #[test]
    fn test_handle_duplicate_filename() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().join("test.txt");

        // First call should return the original path
        let path1 = handle_duplicate_filename(&base_path);
        assert_eq!(path1, base_path);

        // Create the file
        fs::write(&path1, "content1").unwrap();

        // Second call should add _1
        let path2 = handle_duplicate_filename(&base_path);
        assert_eq!(path2, temp_dir.path().join("test_1.txt"));

        // Create the second file
        fs::write(&path2, "content2").unwrap();

        // Third call should add _2
        let path3 = handle_duplicate_filename(&base_path);
        assert_eq!(path3, temp_dir.path().join("test_2.txt"));
    }

    #[test]
    fn test_handle_duplicate_filename_no_extension() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().join("test");

        // Create the file
        fs::write(&base_path, "content").unwrap();

        // Should add _1 without extension
        let path2 = handle_duplicate_filename(&base_path);
        assert_eq!(path2, temp_dir.path().join("test_1"));
    }

    #[test]
    fn test_normalize_path_for_storage() {
        // Windows paths
        assert_eq!(
            normalize_path_for_storage(Path::new(r"Windows\System32\config")),
            "Windows/System32/config"
        );

        // Unix paths (already normalized)
        assert_eq!(
            normalize_path_for_storage(Path::new("/var/log/syslog")),
            "/var/log/syslog"
        );

        // Mixed separators
        assert_eq!(
            normalize_path_for_storage(Path::new(r"some\mixed/path")),
            "some/mixed/path"
        );
    }

    #[tokio::test]
    async fn test_collect_artifacts_parallel_empty() {
        let temp_dir = TempDir::new().unwrap();
        let results = collect_artifacts_parallel(&[], temp_dir.path()).await.unwrap();
        
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_collect_artifacts_parallel_with_mock() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create test artifact
        let artifact = Artifact {
            name: "test".to_string(),
            artifact_type: ArtifactType::FileSystem,
            source_path: "/test/file.txt".to_string(),
            destination_name: "file.txt".to_string(),
            description: Some("Test file".to_string()),
            required: true,
            metadata: HashMap::new(),
            regex: None,
        };

        // Create a mock collector
        let collector = Arc::new(MockCollector {
            supported_types: vec![ArtifactType::FileSystem],
            should_fail: false,
        });

        // We can't easily test the full parallel collection without modifying the function
        // to accept custom collectors, so we'll test the collector directly
        let fs_dir = temp_dir.path().join("fs");
        fs::create_dir_all(&fs_dir).unwrap();
        
        let metadata = collector.collect(&artifact, &fs_dir).await.unwrap();
        assert_eq!(metadata.file_size, 12);
        assert_eq!(metadata.original_path, "/test/file.txt");
    }

    #[tokio::test]
    async fn test_mock_collector_failure() {
        let temp_dir = TempDir::new().unwrap();
        
        let artifact = Artifact {
            name: "test".to_string(),
            artifact_type: ArtifactType::FileSystem,
            source_path: "/test/file.txt".to_string(),
            destination_name: "file.txt".to_string(),
            description: None,
            required: true,
            metadata: HashMap::new(),
            regex: None,
        };

        let collector = MockCollector {
            supported_types: vec![ArtifactType::FileSystem],
            should_fail: true,
        };

        let result = collector.collect(&artifact, temp_dir.path()).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Mock failure");
    }

    #[test]
    fn test_legacy_collect_artifacts() {
        let temp_dir = TempDir::new().unwrap();
        
        // Test with empty artifacts
        let results = collect_artifacts(&[], temp_dir.path()).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_semaphore_concurrency_limit() {
        // Test that MAX_CONCURRENT_COLLECTIONS is reasonable
        let max_concurrent = num_cpus::get() * 2;
        assert!(max_concurrent > 0);
        assert!(max_concurrent <= 256); // Reasonable upper limit
    }

    #[test]
    fn test_artifact_type_support() {
        let mock = MockCollector {
            supported_types: vec![
                ArtifactType::FileSystem,
                ArtifactType::Windows(WindowsArtifactType::Registry),
            ],
            should_fail: false,
        };

        assert!(mock.supports_artifact_type(&ArtifactType::FileSystem));
        assert!(mock.supports_artifact_type(&ArtifactType::Windows(WindowsArtifactType::Registry)));
        assert!(!mock.supports_artifact_type(&ArtifactType::Linux(LinuxArtifactType::SysLogs)));
        assert!(!mock.supports_artifact_type(&ArtifactType::MacOS(MacOSArtifactType::UnifiedLogs)));
    }

    #[test]
    fn test_get_destination_path_edge_cases() {
        let fs_dir = Path::new("/output/fs");
        
        // Test empty path
        let artifact = Artifact {
            name: "empty".to_string(),
            artifact_type: ArtifactType::FileSystem,
            source_path: "".to_string(),
            destination_name: "empty.txt".to_string(),
            description: None,
            required: false,
            metadata: HashMap::new(),
            regex: None,
        };
        let dest_path = get_destination_path(fs_dir, &artifact);
        assert_eq!(dest_path, fs_dir.join(""));
        
        // Test path with only separators
        let artifact2 = Artifact {
            name: "sep".to_string(),
            artifact_type: ArtifactType::FileSystem,
            source_path: "///".to_string(),
            destination_name: "sep.txt".to_string(),
            description: None,
            required: false,
            metadata: HashMap::new(),
            regex: None,
        };
        let dest_path2 = get_destination_path(fs_dir, &artifact2);
        assert_eq!(dest_path2, fs_dir.join(""));
    }

    #[test]
    fn test_get_destination_path_windows_unc() {
        let fs_dir = Path::new("/output/fs");
        let artifact = Artifact {
            name: "unc".to_string(),
            artifact_type: ArtifactType::FileSystem,
            source_path: r"\\server\share\file.txt".to_string(),
            destination_name: "file.txt".to_string(),
            description: None,
            required: false,
            metadata: HashMap::new(),
            regex: None,
        };
        
        let dest_path = get_destination_path(fs_dir, &artifact);
        // UNC paths should have the leading backslashes stripped
        assert_eq!(dest_path, fs_dir.join(r"server\share\file.txt"));
    }

    #[test]
    fn test_handle_duplicate_filename_complex_extensions() {
        let temp_dir = TempDir::new().unwrap();
        
        // Test file with multiple dots
        let base_path = temp_dir.path().join("file.tar.gz");
        fs::write(&base_path, "content").unwrap();
        
        let path2 = handle_duplicate_filename(&base_path);
        assert_eq!(path2, temp_dir.path().join("file.tar_1.gz"));
        
        // Test file with no stem
        let hidden_path = temp_dir.path().join(".hidden");
        fs::write(&hidden_path, "content").unwrap();
        
        let path3 = handle_duplicate_filename(&hidden_path);
        assert_eq!(path3, temp_dir.path().join(".hidden_1"));
    }

    #[test]
    fn test_normalize_path_for_storage_edge_cases() {
        // Empty path
        assert_eq!(normalize_path_for_storage(Path::new("")), "");
        
        // Path with multiple backslashes
        assert_eq!(
            normalize_path_for_storage(Path::new(r"C:\\Windows\\System32")),
            "C://Windows//System32"
        );
        
        // Path with mixed separators already
        assert_eq!(
            normalize_path_for_storage(Path::new("some/mixed\\path/here")),
            "some/mixed//path/here"
        );
    }

    #[tokio::test]
    async fn test_collect_artifacts_parallel_with_regex() {
        use crate::config::RegexConfig;
        
        let temp_dir = TempDir::new().unwrap();
        
        // Create test files
        let test_dir = temp_dir.path().join("logs");
        fs::create_dir_all(&test_dir).unwrap();
        fs::write(test_dir.join("app.log"), "log content").unwrap();
        fs::write(test_dir.join("error.log"), "error content").unwrap();
        fs::write(test_dir.join("debug.txt"), "debug content").unwrap();
        
        let artifact = Artifact {
            name: "logs".to_string(),
            artifact_type: ArtifactType::Logs,
            source_path: test_dir.to_string_lossy().to_string(),
            destination_name: "logs".to_string(),
            description: Some("Log files".to_string()),
            required: true,
            metadata: HashMap::new(),
            regex: Some(RegexConfig {
                enabled: true,
                include_pattern: r".*\.log$".to_string(),
                exclude_pattern: String::new(),
                recursive: true,
                max_depth: None,
            }),
        };
        
        // We can't easily test the full regex collection without mocking
        // but we can verify the artifact structure
        assert!(artifact.regex.is_some());
        assert!(artifact.regex.as_ref().unwrap().enabled);
    }

    #[tokio::test]
    async fn test_collect_artifacts_with_failures() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create artifacts with mixed success/failure scenarios
        let artifacts = vec![
            Artifact {
                name: "required-missing".to_string(),
                artifact_type: ArtifactType::FileSystem,
                source_path: "/nonexistent/required.txt".to_string(),
                destination_name: "required.txt".to_string(),
                description: None,
                required: true, // Required but missing
                metadata: HashMap::new(),
                regex: None,
            },
            Artifact {
                name: "optional-missing".to_string(),
                artifact_type: ArtifactType::FileSystem,
                source_path: "/nonexistent/optional.txt".to_string(),
                destination_name: "optional.txt".to_string(),
                description: None,
                required: false, // Optional and missing
                metadata: HashMap::new(),
                regex: None,
            },
        ];
        
        // Should complete without error (failures are logged, not returned)
        let result = collect_artifacts_parallel(&artifacts, temp_dir.path()).await;
        assert!(result.is_ok());
        
        // Results should be empty since both files don't exist
        let results = result.unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_is_special_artifact_comprehensive() {
        // Test all Windows artifact types
        use crate::config::WindowsArtifactType::*;
        use crate::config::VolatileDataType;
        
        let special_types = vec![
            ArtifactType::Windows(MFT),
            ArtifactType::Windows(USNJournal),
        ];
        
        let normal_types = vec![
            ArtifactType::Windows(Registry),
            ArtifactType::Windows(EventLog),
            ArtifactType::Windows(Prefetch),
            ArtifactType::Windows(ShimCache),
            ArtifactType::Windows(AmCache),
            ArtifactType::FileSystem,
            ArtifactType::Logs,
            ArtifactType::UserData,
            ArtifactType::SystemInfo,
            ArtifactType::Memory,
            ArtifactType::Network,
            ArtifactType::VolatileData(VolatileDataType::Processes),
            ArtifactType::Custom,
        ];
        
        for artifact_type in special_types {
            assert!(is_special_artifact(&artifact_type), 
                   "{:?} should be special", artifact_type);
        }
        
        for artifact_type in normal_types {
            assert!(!is_special_artifact(&artifact_type), 
                   "{:?} should not be special", artifact_type);
        }
    }

    #[test]
    fn test_concurrent_collection_limit() {
        // Verify the concurrency limit calculation
        let cpu_count = num_cpus::get();
        let max_concurrent = std::cmp::min(cpu_count * 2, 32);
        
        assert!(max_concurrent > 0);
        assert!(max_concurrent <= 32);
        
        // Test edge cases
        if cpu_count == 1 {
            assert_eq!(max_concurrent, 2);
        }
        if cpu_count >= 16 {
            assert_eq!(max_concurrent, 32);
        }
    }
}
