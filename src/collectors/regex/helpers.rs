use anyhow::{Context, Result};
use regex::Regex;
use std::path::{Path, PathBuf};

/// Check if a path matches the given regex pattern
pub fn path_matches_pattern(path: &Path, base_path: &Path, pattern: &Regex) -> bool {
    // Get path relative to base for regex matching
    if let Ok(relative_path) = path.strip_prefix(base_path) {
        let path_str = relative_path.to_string_lossy();
        pattern.is_match(&path_str)
    } else {
        // If we can't get a relative path, use the full path
        let path_str = path.to_string_lossy();
        pattern.is_match(&path_str)
    }
}

/// Check if a path should be excluded based on the exclude pattern
pub fn should_exclude_path(path: &Path, base_path: &Path, exclude_regex: &Option<Regex>) -> bool {
    if let Some(exclude) = exclude_regex {
        if let Ok(relative_path) = path.strip_prefix(base_path) {
            let path_str = relative_path.to_string_lossy();
            exclude.is_match(&path_str)
        } else {
            let path_str = path.to_string_lossy();
            exclude.is_match(&path_str)
        }
    } else {
        false
    }
}

/// Create a destination path that preserves the directory structure
pub fn create_destination_path(
    source_path: &Path,
    base_path: &Path,
    output_base: &Path,
) -> Result<PathBuf> {
    // Get path relative to base
    let relative_path = source_path.strip_prefix(base_path).context(format!(
        "Failed to get relative path for {}",
        source_path.display()
    ))?;

    // Create destination path
    let dest_path = output_base.join(relative_path);

    // Create parent directories if they don't exist
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)
            .context(format!("Failed to create directory: {}", parent.display()))?;
    }

    Ok(dest_path)
}
