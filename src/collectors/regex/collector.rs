use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, Context};
use log::{debug, info, warn};

use crate::models::ArtifactMetadata;
use crate::config::{Artifact, ArtifactType, RegexConfig};
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
