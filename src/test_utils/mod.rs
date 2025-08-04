//! Test utilities for rs-collector
//!
//! This module provides common testing utilities, helpers, and mocks
//! for use across all test modules.

#![cfg(test)]

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::{NamedTempFile, TempDir};

// Note: Test constants are defined within the generators module to avoid scoping issues

/// Creates a temporary directory that is automatically cleaned up
pub fn create_temp_dir() -> Result<TempDir> {
    Ok(TempDir::new()?)
}

/// Creates a temporary file with the given content
pub fn create_temp_file(content: &[u8]) -> Result<NamedTempFile> {
    let mut file = NamedTempFile::new()?;
    use std::io::Write;
    file.write_all(content)?;
    file.flush()?;
    Ok(file)
}

/// Creates a test file structure in a temporary directory
pub fn create_test_file_structure() -> Result<TempDir> {
    let temp_dir = create_temp_dir()?;
    let base_path = temp_dir.path();

    // Create directory structure
    fs::create_dir_all(base_path.join("dir1/subdir1"))?;
    fs::create_dir_all(base_path.join("dir2"))?;

    // Create test files
    fs::write(base_path.join("file1.txt"), b"Test content 1")?;
    fs::write(base_path.join("file2.log"), b"Test log content")?;
    fs::write(base_path.join("dir1/file3.txt"), b"Test content 3")?;
    fs::write(base_path.join("dir1/subdir1/file4.txt"), b"Test content 4")?;
    fs::write(base_path.join("dir2/file5.log"), b"Another log file")?;

    Ok(temp_dir)
}

/// Creates a test YAML configuration file
pub fn create_test_config() -> Result<NamedTempFile> {
    let config_content = r#"
version: "1.0"
description: "Test configuration"
global_options:
  max_file_size_mb: "100"
  generate_bodyfile: "true"
  
artifacts:
  - name: "test_artifact"
    artifact_type:
      FileSystem: Generic
    source_path: "/tmp/test"
    destination_name: "test"
    description: "Test artifact"
    required: false
"#;

    create_temp_file(config_content.as_bytes())
}

/// Test data generators for common types
pub mod generators {
    use crate::collectors::volatile::models::*;
    use crate::models::ArtifactMetadata;
    use chrono::Utc;

    // Test constants defined locally within this module
    const TEST_DATA_SIZE: u64 = 2 * 1024 * 1024; // 2MB
    const TEST_TOTAL_MEMORY: u64 = 8 * 1024 * 1024 * 1024; // 8GB
    const TEST_USED_MEMORY: u64 = 4 * 1024 * 1024 * 1024; // 4GB
    const TEST_TOTAL_SWAP: u64 = 2 * 1024 * 1024 * 1024; // 2GB
    const TEST_USED_SWAP: u64 = 512 * 1024 * 1024; // 512MB
    const TEST_TOTAL_DISK_SPACE: u64 = 100 * 1024 * 1024 * 1024; // 100GB
    const TEST_AVAILABLE_DISK_SPACE: u64 = 50 * 1024 * 1024 * 1024; // 50GB

    /// Generate test ArtifactMetadata
    pub fn test_artifact_metadata(path: &str) -> ArtifactMetadata {
        ArtifactMetadata {
            original_path: path.to_string(),
            collection_time: Utc::now().to_rfc3339(),
            file_size: 1024,
            created_time: Some(Utc::now().to_rfc3339()),
            accessed_time: Some(Utc::now().to_rfc3339()),
            modified_time: Some(Utc::now().to_rfc3339()),
            is_locked: false,
        }
    }

    /// Generate test SystemInfo
    pub fn test_system_info() -> SystemInfo {
        SystemInfo {
            hostname: Some("test-host".to_string()),
            os_name: Some("Test OS".to_string()),
            os_version: Some("1.0.0".to_string()),
            kernel_version: Some("5.0.0".to_string()),
            cpu_info: CpuInfo {
                count: 4,
                vendor: Some("Test Vendor".to_string()),
                brand: Some("Test CPU".to_string()),
                frequency: 3600,
            },
        }
    }

    /// Generate test ProcessInfo
    pub fn test_process_info(pid: u32, name: &str) -> ProcessInfo {
        ProcessInfo {
            pid,
            name: name.to_string(),
            cmd: vec![name.to_string(), "--test".to_string()],
            exe: Some(format!("/usr/bin/{}", name)),
            status: "Running".to_string(),
            start_time: 0,
            cpu_usage: 10.5,
            memory_usage: TEST_DATA_SIZE / 2,
            parent_pid: Some(1),
        }
    }

    /// Generate test NetworkInfo
    pub fn test_network_info() -> NetworkInfo {
        NetworkInfo {
            interfaces: vec![NetworkInterface {
                name: "eth0".to_string(),
                ips: vec!["192.168.1.100".to_string()],
                mac: Some("00:11:22:33:44:55".to_string()),
                transmitted_bytes: TEST_DATA_SIZE / 2,
                received_bytes: TEST_DATA_SIZE,
            }],
            connections: vec![],
        }
    }

    /// Generate test MemoryInfo
    pub fn test_memory_info() -> MemoryInfo {
        MemoryInfo {
            total_memory: TEST_TOTAL_MEMORY,
            used_memory: TEST_USED_MEMORY,
            total_swap: TEST_TOTAL_SWAP,
            used_swap: TEST_USED_SWAP,
        }
    }

    /// Generate test DiskInfo
    pub fn test_disk_info() -> DiskInfo {
        DiskInfo {
            name: "/dev/sda1".to_string(),
            mount_point: Some("/".to_string()),
            total_space: TEST_TOTAL_DISK_SPACE,
            available_space: TEST_AVAILABLE_DISK_SPACE,
            file_system: Some("ext4".to_string()),
            is_removable: false,
        }
    }
}

/// Assertion helpers for custom types
pub mod assertions {
    use crate::models::ArtifactMetadata;

    /// Assert that an ArtifactMetadata has expected values
    pub fn assert_artifact_metadata(metadata: &ArtifactMetadata, path: &str, size: u64) {
        assert_eq!(metadata.original_path, path, "Artifact path mismatch");
        assert_eq!(metadata.file_size, size, "Artifact size mismatch");
        assert!(!metadata.is_locked, "File should not be locked");
    }
}

/// Mock filesystem utilities
pub mod mock_fs {
    use anyhow::{anyhow, Result};
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    /// Mock filesystem for testing
    pub struct MockFileSystem {
        files: HashMap<PathBuf, Vec<u8>>,
        directories: Vec<PathBuf>,
    }

    impl MockFileSystem {
        /// Create a new mock filesystem
        pub fn new() -> Self {
            Self {
                files: HashMap::new(),
                directories: vec![PathBuf::from("/")],
            }
        }

        /// Add a file to the mock filesystem
        pub fn add_file<P: AsRef<Path>>(&mut self, path: P, content: Vec<u8>) {
            let path = path.as_ref().to_path_buf();

            // Add parent directories
            if let Some(parent) = path.parent() {
                self.add_directory(parent);
            }

            self.files.insert(path, content);
        }

        /// Add a directory to the mock filesystem
        pub fn add_directory<P: AsRef<Path>>(&mut self, path: P) {
            let path = path.as_ref().to_path_buf();

            // Add all parent directories
            let mut current = path.clone();
            while let Some(parent) = current.parent() {
                if !self.directories.contains(&parent.to_path_buf()) {
                    self.directories.push(parent.to_path_buf());
                }
                current = parent.to_path_buf();
            }

            if !self.directories.contains(&path) {
                self.directories.push(path);
            }
        }

        /// Check if a path exists
        pub fn exists<P: AsRef<Path>>(&self, path: P) -> bool {
            let path = path.as_ref();
            self.files.contains_key(path) || self.directories.contains(&path.to_path_buf())
        }

        /// Read a file from the mock filesystem
        pub fn read_file<P: AsRef<Path>>(&self, path: P) -> Result<Vec<u8>> {
            self.files
                .get(path.as_ref())
                .cloned()
                .ok_or_else(|| anyhow!("File not found: {:?}", path.as_ref()))
        }

        /// List files in a directory
        pub fn list_dir<P: AsRef<Path>>(&self, dir: P) -> Result<Vec<PathBuf>> {
            let dir = dir.as_ref();
            if !self.directories.contains(&dir.to_path_buf()) {
                return Err(anyhow!("Directory not found: {:?}", dir));
            }

            let mut entries = Vec::new();

            // Add files
            for file_path in self.files.keys() {
                if let Some(parent) = file_path.parent() {
                    if parent == dir {
                        entries.push(file_path.clone());
                    }
                }
            }

            // Add subdirectories
            for dir_path in &self.directories {
                if let Some(parent) = dir_path.parent() {
                    if parent == dir && dir_path != dir {
                        entries.push(dir_path.clone());
                    }
                }
            }

            Ok(entries)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_mock_filesystem() {
            let mut fs = MockFileSystem::new();

            // Add files and directories
            fs.add_file("/test/file1.txt", b"content1".to_vec());
            fs.add_file("/test/dir/file2.txt", b"content2".to_vec());

            // Test existence
            assert!(fs.exists("/test"));
            assert!(fs.exists("/test/file1.txt"));
            assert!(fs.exists("/test/dir"));
            assert!(fs.exists("/test/dir/file2.txt"));
            assert!(!fs.exists("/nonexistent"));

            // Test reading
            assert_eq!(fs.read_file("/test/file1.txt").unwrap(), b"content1");
            assert_eq!(fs.read_file("/test/dir/file2.txt").unwrap(), b"content2");
            assert!(fs.read_file("/nonexistent").is_err());

            // Test listing
            let entries = fs.list_dir("/test").unwrap();
            assert_eq!(entries.len(), 2); // file1.txt and dir
        }
    }
}
