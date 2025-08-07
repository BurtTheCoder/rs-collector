//! Path validation utilities for preventing path traversal attacks.
//!
//! This module provides functions to validate and sanitize file paths
//! to ensure they remain within expected boundaries and don't allow
//! access to unauthorized locations.

use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};

/// Validates that a path is safe and doesn't contain directory traversal attempts.
///
/// This function checks for common path traversal patterns and ensures
/// the path doesn't escape the intended directory structure.
///
/// # Arguments
///
/// * `path` - The path to validate
/// * `base_dir` - Optional base directory that the path must be within
///
/// # Returns
///
/// * `Ok(PathBuf)` - The canonicalized safe path
/// * `Err` - If the path is unsafe or invalid
///
/// # Security
///
/// This function prevents:
/// - Path traversal using `..` sequences
/// - Absolute paths when a base directory is specified
/// - Symbolic links that point outside the base directory
/// - Invalid path characters
pub fn validate_path(path: &Path, base_dir: Option<&Path>) -> Result<PathBuf> {
    // Check for path traversal attempts
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                bail!("Path traversal attempt detected: path contains '..'");
            }
            std::path::Component::RootDir => {
                if base_dir.is_some() {
                    bail!("Absolute paths not allowed when base directory is specified");
                }
            }
            _ => {}
        }
    }

    // Validate path doesn't contain null bytes
    if let Some(path_str) = path.to_str() {
        if path_str.contains('\0') {
            bail!("Path contains null bytes");
        }
    }

    // If base directory is specified, ensure path stays within it
    if let Some(base) = base_dir {
        let base_canonical = base
            .canonicalize()
            .context("Failed to canonicalize base directory")?;

        // Resolve the full path
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            base.join(path)
        };

        // Check if the path exists and canonicalize it
        if full_path.exists() {
            let canonical = full_path
                .canonicalize()
                .context("Failed to canonicalize path")?;

            // Ensure the canonical path starts with the base directory
            if !canonical.starts_with(&base_canonical) {
                return Err(anyhow!("Path escapes base directory: {:?}", canonical));
            }

            Ok(canonical)
        } else {
            // For non-existent paths, manually resolve and validate
            let mut resolved = base_canonical.clone();
            for component in path.components() {
                match component {
                    std::path::Component::Normal(name) => {
                        resolved.push(name);
                    }
                    std::path::Component::CurDir => {
                        // Current directory - do nothing
                    }
                    _ => {
                        return Err(anyhow!("Invalid path component: {:?}", component));
                    }
                }
            }

            // Final check that we're still within base directory
            if !resolved.starts_with(&base_canonical) {
                bail!("Resolved path escapes base directory");
            }

            Ok(resolved)
        }
    } else {
        // No base directory specified, just canonicalize if possible
        if path.exists() {
            path.canonicalize().context("Failed to canonicalize path")
        } else {
            Ok(path.to_path_buf())
        }
    }
}

/// Sanitizes a filename to remove potentially dangerous characters.
///
/// This function removes or replaces characters that could be problematic
/// in filenames across different operating systems.
///
/// # Arguments
///
/// * `filename` - The filename to sanitize
///
/// # Returns
///
/// A sanitized filename safe for use on all platforms
pub fn sanitize_filename(filename: &str) -> String {
    let mut sanitized = String::with_capacity(filename.len());

    for ch in filename.chars() {
        match ch {
            // Replace path separators
            '/' | '\\' => sanitized.push('_'),
            // Remove null bytes
            '\0' => continue,
            // Replace other problematic characters
            '<' | '>' | ':' | '"' | '|' | '?' | '*' => sanitized.push('_'),
            // Control characters
            c if c.is_control() => sanitized.push('_'),
            // Keep everything else
            c => sanitized.push(c),
        }
    }

    // Don't allow only dots
    if sanitized.chars().all(|c| c == '.') {
        sanitized = format!("_{}", sanitized);
    }

    // Don't allow empty names
    if sanitized.is_empty() {
        sanitized = "unnamed".to_string();
    }

    // Trim dots and spaces from ends
    sanitized.trim_matches(|c| c == '.' || c == ' ').to_string()
}

/// Validates that a path is safe for output.
///
/// This ensures that output paths don't overwrite system files
/// or write to dangerous locations.
///
/// # Arguments
///
/// * `path` - The output path to validate
///
/// # Returns
///
/// * `Ok(())` - If the path is safe for output
/// * `Err` - If the path is unsafe
pub fn validate_output_path(path: &Path) -> Result<()> {
    // Prevent writing to system directories
    let path_str = path.to_string_lossy().to_lowercase();

    let dangerous_paths = vec![
        "/etc",
        "/sys",
        "/proc",
        "/dev",
        "/boot",
        "c:\\windows",
        "c:\\program files",
        "c:\\programdata",
        "/system",
        "/library",
        "/usr",
    ];

    for dangerous in dangerous_paths {
        if path_str.starts_with(dangerous) {
            return Err(anyhow!(
                "Cannot write to system directory: {}",
                path.display()
            ));
        }
    }

    // Check if parent directory exists and is writable
    if let Some(parent) = path.parent() {
        if parent.exists() && parent.metadata()?.permissions().readonly() {
            return Err(anyhow!(
                "Parent directory is read-only: {}",
                parent.display()
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_path_traversal() {
        let base = Path::new("/tmp/safe");

        // Should fail on path traversal
        assert!(validate_path(Path::new("../etc/passwd"), Some(base)).is_err());
        assert!(validate_path(Path::new("./../../etc/passwd"), Some(base)).is_err());
        assert!(validate_path(Path::new("subdir/../../../etc/passwd"), Some(base)).is_err());
    }

    #[test]
    fn test_validate_path_absolute() {
        let base = Path::new("/tmp/safe");

        // Should fail on absolute paths with base dir
        assert!(validate_path(Path::new("/etc/passwd"), Some(base)).is_err());

        // Should work without base dir
        assert!(validate_path(Path::new("/tmp/file"), None).is_ok());
    }

    #[test]
    fn test_validate_path_null_bytes() {
        let path_with_null = Path::new("file\0name");
        assert!(validate_path(path_with_null, None).is_err());
    }

    #[test]
    fn test_validate_path_valid() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Create a test file
        let test_file = base.join("test.txt");
        std::fs::write(&test_file, "test").unwrap();

        // Should succeed for valid paths
        assert!(validate_path(Path::new("test.txt"), Some(base)).is_ok());
        assert!(validate_path(Path::new("./test.txt"), Some(base)).is_ok());
        assert!(validate_path(Path::new("subdir/file.txt"), Some(base)).is_ok());
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("normal.txt"), "normal.txt");
        assert_eq!(
            sanitize_filename("../../etc/passwd"),
            "_.._.._.._etc_passwd"
        );
        assert_eq!(sanitize_filename("file<>:\"|?*.txt"), "file_______.txt");
        assert_eq!(sanitize_filename("file\0name"), "filename");
        assert_eq!(sanitize_filename(""), "unnamed");
        assert_eq!(sanitize_filename("..."), "_...");
        assert_eq!(sanitize_filename("  spaces  "), "spaces");
        assert_eq!(sanitize_filename("file."), "file");
    }

    #[test]
    fn test_validate_output_path() {
        // Should fail for system paths
        assert!(validate_output_path(Path::new("/etc/passwd")).is_err());
        assert!(validate_output_path(Path::new("/sys/kernel")).is_err());
        assert!(validate_output_path(Path::new("C:\\Windows\\System32\\config")).is_err());

        // Should succeed for safe paths
        assert!(validate_output_path(Path::new("/tmp/output.txt")).is_ok());
        assert!(validate_output_path(Path::new("/home/user/output")).is_ok());
    }

    #[test]
    fn test_path_escape_attempts() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Various escape attempts
        let escape_attempts = vec![
            "subdir/../../..",
            "../../../etc/passwd",
            "./../..",
            "valid/../../../escape",
        ];

        for attempt in escape_attempts {
            assert!(
                validate_path(Path::new(attempt), Some(base)).is_err(),
                "Failed to catch escape attempt: {}",
                attempt
            );
        }
    }
}
