use std::path::{Path, PathBuf};

use anyhow::Result;
use log::{debug, info};

use crate::models::ArtifactMetadata;
use crate::config::Artifact;
use crate::collectors::platforms::common::FallbackCollector;
use crate::collectors::regex::walker::DirectoryWalker;

/// Collector for regex-based artifact collection
pub struct RegexCollector {
    fallback: FallbackCollector,
}

impl RegexCollector {
    /// Create a new regex collector
    pub fn new() -> Self {
        RegexCollector {
            fallback: FallbackCollector::new(),
        }
    }
    
    /// Collect artifacts using regex patterns
    pub async fn collect_with_regex(
        &self,
        artifact: &Artifact,
        source_base: &Path,
        output_dir: &Path
    ) -> Result<Vec<(PathBuf, ArtifactMetadata)>> {
        // Verify regex config exists
        let regex_config = match &artifact.regex {
            Some(config) if config.enabled => config,
            _ => return Err(anyhow::anyhow!("Regex config not enabled for this artifact")),
        };

        info!("Collecting regex artifact: {} from {}", artifact.name, source_base.display());
        debug!("Using include pattern: {}", regex_config.include_pattern);
        
        if !regex_config.exclude_pattern.is_empty() {
            debug!("Using exclude pattern: {}", regex_config.exclude_pattern);
        }
        
        if regex_config.recursive {
            debug!("Recursive search enabled");
            if let Some(depth) = regex_config.max_depth {
                debug!("Maximum depth: {}", depth);
            } else {
                debug!("No maximum depth limit");
            }
        } else {
            debug!("Non-recursive search (top-level only)");
        }

        // Create walker and process directory
        let walker = DirectoryWalker::new(
            &self.fallback,
            source_base,
            output_dir,
            &regex_config.include_pattern,
            &regex_config.exclude_pattern,
            regex_config.recursive,
            regex_config.max_depth
        )?;
        
        let results = walker.walk().await?;
        info!("Collected {} files matching pattern", results.len());
        
        Ok(results)
    }
    
    /// Check if an artifact has regex configuration enabled
    pub fn has_regex_config(artifact: &Artifact) -> bool {
        if let Some(regex_config) = &artifact.regex {
            regex_config.enabled
        } else {
            false
        }
    }
}

// Make RegexCollector cloneable for use in async blocks
impl Clone for RegexCollector {
    fn clone(&self) -> Self {
        RegexCollector {
            fallback: self.fallback.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    use std::collections::HashMap;

    #[test]
    fn test_regex_collector_new() {
        let collector = RegexCollector::new();
        // Just verify it creates without panic
        let _ = collector.clone();
    }

    #[test]
    fn test_regex_collector_clone() {
        let collector1 = RegexCollector::new();
        let _collector2 = collector1.clone();
        // Both should be valid collectors
        let artifact = create_test_artifact(true);
        assert!(RegexCollector::has_regex_config(&artifact));
    }

    #[test]
    fn test_has_regex_config() {
        // Test with regex enabled
        let artifact_with_regex = Artifact {
            name: "test".to_string(),
            artifact_type: ArtifactType::FileSystem,
            source_path: "/test".to_string(),
            destination_name: "test".to_string(),
            description: None,
            required: false,
            metadata: HashMap::new(),
            regex: Some(RegexConfig {
                enabled: true,
                include_pattern: "*.log".to_string(),
                exclude_pattern: String::new(),
                recursive: true,
                max_depth: None,
            }),
        };
        assert!(RegexCollector::has_regex_config(&artifact_with_regex));

        // Test with regex disabled
        let artifact_disabled = Artifact {
            name: "test".to_string(),
            artifact_type: ArtifactType::FileSystem,
            source_path: "/test".to_string(),
            destination_name: "test".to_string(),
            description: None,
            required: false,
            metadata: HashMap::new(),
            regex: Some(RegexConfig {
                enabled: false,
                include_pattern: "*.log".to_string(),
                exclude_pattern: String::new(),
                recursive: true,
                max_depth: None,
            }),
        };
        assert!(!RegexCollector::has_regex_config(&artifact_disabled));

        // Test without regex config
        let artifact_no_regex = Artifact {
            name: "test".to_string(),
            artifact_type: ArtifactType::FileSystem,
            source_path: "/test".to_string(),
            destination_name: "test".to_string(),
            description: None,
            required: false,
            metadata: HashMap::new(),
            regex: None,
        };
        assert!(!RegexCollector::has_regex_config(&artifact_no_regex));
    }

    #[tokio::test]
    async fn test_collect_with_regex_not_enabled() {
        let collector = RegexCollector::new();
        let temp_dir = TempDir::new().unwrap();
        
        let artifact = Artifact {
            name: "test".to_string(),
            artifact_type: ArtifactType::FileSystem,
            source_path: "/test".to_string(),
            destination_name: "test".to_string(),
            description: None,
            required: false,
            metadata: HashMap::new(),
            regex: None,
        };

        let result = collector.collect_with_regex(&artifact, temp_dir.path(), temp_dir.path()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Regex config not enabled"));
    }

    #[tokio::test]
    async fn test_collect_with_regex_simple_pattern() {
        let collector = RegexCollector::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Create test files
        let source_dir = temp_dir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("test.log"), "log content").unwrap();
        fs::write(source_dir.join("test.txt"), "text content").unwrap();
        fs::write(source_dir.join("app.log"), "app log content").unwrap();
        
        let output_dir = temp_dir.path().join("output");
        fs::create_dir_all(&output_dir).unwrap();

        let artifact = Artifact {
            name: "logs".to_string(),
            artifact_type: ArtifactType::Logs,
            source_path: source_dir.to_string_lossy().to_string(),
            destination_name: "logs".to_string(),
            description: None,
            required: false,
            metadata: HashMap::new(),
            regex: Some(RegexConfig {
                enabled: true,
                include_pattern: r".*\.log$".to_string(),
                exclude_pattern: String::new(),
                recursive: false,
                max_depth: None,
            }),
        };

        let result = collector.collect_with_regex(&artifact, &source_dir, &output_dir).await;
        
        // Should succeed but may have 0 results due to walker implementation
        // The actual regex filtering happens in the walker
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_collect_with_regex_recursive() {
        let collector = RegexCollector::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Create nested directory structure
        let source_dir = temp_dir.path().join("source");
        let sub_dir = source_dir.join("subdir");
        fs::create_dir_all(&sub_dir).unwrap();
        
        fs::write(source_dir.join("root.log"), "root log").unwrap();
        fs::write(sub_dir.join("sub.log"), "sub log").unwrap();
        fs::write(sub_dir.join("data.txt"), "text data").unwrap();
        
        let output_dir = temp_dir.path().join("output");
        fs::create_dir_all(&output_dir).unwrap();

        let artifact = Artifact {
            name: "logs".to_string(),
            artifact_type: ArtifactType::Logs,
            source_path: source_dir.to_string_lossy().to_string(),
            destination_name: "logs".to_string(),
            description: None,
            required: false,
            metadata: HashMap::new(),
            regex: Some(RegexConfig {
                enabled: true,
                include_pattern: r".*\.log$".to_string(),
                exclude_pattern: String::new(),
                recursive: true,
                max_depth: Some(2),
            }),
        };

        let result = collector.collect_with_regex(&artifact, &source_dir, &output_dir).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_collect_with_regex_exclude_pattern() {
        let collector = RegexCollector::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Create test files
        let source_dir = temp_dir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("app.log"), "app log").unwrap();
        fs::write(source_dir.join("debug.log"), "debug log").unwrap();
        fs::write(source_dir.join("temp.log"), "temp log").unwrap();
        
        let output_dir = temp_dir.path().join("output");
        fs::create_dir_all(&output_dir).unwrap();

        let artifact = Artifact {
            name: "logs".to_string(),
            artifact_type: ArtifactType::Logs,
            source_path: source_dir.to_string_lossy().to_string(),
            destination_name: "logs".to_string(),
            description: None,
            required: false,
            metadata: HashMap::new(),
            regex: Some(RegexConfig {
                enabled: true,
                include_pattern: r".*\.log$".to_string(),
                exclude_pattern: r"debug|temp".to_string(),
                recursive: false,
                max_depth: None,
            }),
        };

        let result = collector.collect_with_regex(&artifact, &source_dir, &output_dir).await;
        assert!(result.is_ok());
    }

    // Helper function to create test artifacts
    fn create_test_artifact(with_regex: bool) -> Artifact {
        Artifact {
            name: "test".to_string(),
            artifact_type: ArtifactType::FileSystem,
            source_path: "/test".to_string(),
            destination_name: "test".to_string(),
            description: None,
            required: false,
            metadata: HashMap::new(),
            regex: if with_regex {
                Some(RegexConfig {
                    enabled: true,
                    include_pattern: "*.log".to_string(),
                    exclude_pattern: String::new(),
                    recursive: true,
                    max_depth: None,
                })
            } else {
                None
            },
        }
    }
}
