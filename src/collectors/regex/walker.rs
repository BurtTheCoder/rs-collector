use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use log::{debug, warn};
use regex::Regex;

use crate::collectors::platforms::common::FallbackCollector;
use crate::collectors::regex::helpers::{
    create_destination_path, path_matches_pattern, should_exclude_path,
};
use crate::models::ArtifactMetadata;
// Path validation is handled by the FallbackCollector

/// Directory walker for regex-based artifact collection
pub struct DirectoryWalker<'a> {
    fallback: &'a FallbackCollector,
    base_path: PathBuf,
    output_base: PathBuf,
    include_regex: Regex,
    exclude_regex: Option<Regex>,
    recursive: bool,
    max_depth: Option<usize>,
}

impl<'a> DirectoryWalker<'a> {
    /// Create a new directory walker
    pub fn new(
        fallback: &'a FallbackCollector,
        base_path: &Path,
        output_base: &Path,
        include_pattern: &str,
        exclude_pattern: &str,
        recursive: bool,
        max_depth: Option<usize>,
    ) -> Result<Self> {
        let include_regex = Regex::new(include_pattern).context("Invalid include pattern regex")?;

        let exclude_regex = if !exclude_pattern.is_empty() {
            Some(Regex::new(exclude_pattern).context("Invalid exclude pattern regex")?)
        } else {
            None
        };

        Ok(DirectoryWalker {
            fallback,
            base_path: base_path.to_path_buf(),
            output_base: output_base.to_path_buf(),
            include_regex,
            exclude_regex,
            recursive,
            max_depth,
        })
    }

    /// Walk the directory and collect matching files
    pub async fn walk(&self) -> Result<Vec<(PathBuf, ArtifactMetadata)>> {
        // Instead of spawning a blocking task, just perform the work directly
        // This avoids the lifetime issue with the closure
        let mut results = Vec::new();
        self.walk_directory_recursive(&self.base_path, 0, &mut results)?;

        Ok(results)
    }

    /// Recursively walk a directory and collect matching files
    fn walk_directory_recursive(
        &self,
        current_path: &Path,
        current_depth: usize,
        results: &mut Vec<(PathBuf, ArtifactMetadata)>,
    ) -> Result<()> {
        // Skip if we've exceeded max_depth
        if let Some(depth) = self.max_depth {
            if current_depth > depth {
                return Ok(());
            }
        }

        // Check if the path exists and is a directory
        if !current_path.exists() {
            return Err(anyhow::anyhow!(
                "Path does not exist: {}",
                current_path.display()
            ));
        }

        if !current_path.is_dir() {
            return Err(anyhow::anyhow!(
                "Path is not a directory: {}",
                current_path.display()
            ));
        }

        // Read directory entries
        let entries = fs::read_dir(current_path).context(format!(
            "Failed to read directory: {}",
            current_path.display()
        ))?;

        // Process each entry
        for entry in entries {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            // Skip if path should be excluded
            if should_exclude_path(&path, &self.base_path, &self.exclude_regex) {
                debug!("Skipping excluded path: {}", path.display());
                continue;
            }

            if path.is_dir() {
                if self.recursive {
                    // Recursively process subdirectory
                    self.walk_directory_recursive(&path, current_depth + 1, results)?;
                }
            } else if path_matches_pattern(&path, &self.base_path, &self.include_regex) {
                // Path matches include pattern, collect it
                debug!("Collecting file: {}", path.display());

                // Create destination path
                let dest_path = create_destination_path(&path, &self.base_path, &self.output_base)?;

                // Collect the file
                match self.fallback.collect_standard_file(&path, &dest_path) {
                    Ok(metadata) => {
                        results.push((dest_path, metadata));
                    }
                    Err(e) => {
                        warn!("Failed to collect {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(())
    }
}

// Make DirectoryWalker cloneable for use in async blocks
impl<'a> Clone for DirectoryWalker<'a> {
    fn clone(&self) -> Self {
        DirectoryWalker {
            fallback: self.fallback,
            base_path: self.base_path.clone(),
            output_base: self.output_base.clone(),
            include_regex: self.include_regex.clone(),
            exclude_regex: self.exclude_regex.clone(),
            recursive: self.recursive,
            max_depth: self.max_depth,
        }
    }
}
