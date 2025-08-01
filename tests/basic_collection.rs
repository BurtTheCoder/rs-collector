//! Integration tests for basic artifact collection scenarios.
//!
//! These tests verify end-to-end functionality of the collector
//! in common usage scenarios.

use std::fs;
use tempfile::TempDir;
use anyhow::Result;

use rust_collector::config::{
    Artifact, ArtifactType
};
use rust_collector::collectors::collector::collect_artifacts;

/// Test basic file collection on the current platform
#[test]
fn test_basic_file_collection() -> Result<()> {
    // Create a temporary directory for test files
    let test_dir = TempDir::new()?;
    let test_file_path = test_dir.path().join("test_file.txt");
    fs::write(&test_file_path, "Test content for integration test")?;

    // Create output directory
    let output_dir = TempDir::new()?;

    // Create a simple artifact configuration
    let artifacts = vec![
        Artifact {
            name: "test_file".to_string(),
            artifact_type: ArtifactType::FileSystem,
            source_path: test_file_path.to_string_lossy().to_string(),
            destination_name: "collected_test_file.txt".to_string(),
            description: Some("Test file for integration testing".to_string()),
            required: true,
            metadata: std::collections::HashMap::new(),
            regex: None,
        }
    ];

    // Collect the artifact
    let results = collect_artifacts(&artifacts, output_dir.path())?;

    // Verify the file was collected
    assert_eq!(results.len(), 1);
    
    // Check that the output file exists
    let collected_path = output_dir.path().join("collected_test_file.txt");
    assert!(collected_path.exists(), "Collected file should exist");

    // Verify content
    let collected_content = fs::read_to_string(&collected_path)?;
    assert_eq!(collected_content, "Test content for integration test");

    // Verify metadata
    let (path, metadata) = results.iter().next().unwrap();
    assert_eq!(metadata.file_size, 33); // Length of test content
    assert!(metadata.collection_time.len() > 0);

    Ok(())
}

/// Test collection with multiple artifacts
#[test]
fn test_multiple_artifact_collection() -> Result<()> {
    // Create test files
    let test_dir = TempDir::new()?;
    let files = vec![
        ("file1.txt", "Content of file 1"),
        ("file2.log", "Log file content"),
        ("data.json", r#"{"test": "data"}"#),
    ];

    let mut artifacts = Vec::new();
    for (filename, content) in &files {
        let file_path = test_dir.path().join(filename);
        fs::write(&file_path, content)?;

        artifacts.push(Artifact {
            name: filename.to_string(),
            artifact_type: ArtifactType::FileSystem,
            source_path: file_path.to_string_lossy().to_string(),
            destination_name: format!("collected_{}", filename),
            description: Some(format!("Test file: {}", filename)),
            required: true,
            metadata: std::collections::HashMap::new(),
            regex: None,
        });
    }

    // Create output directory
    let output_dir = TempDir::new()?;

    // Collect all artifacts
    let results = collect_artifacts(&artifacts, output_dir.path())?;

    // Verify all files were collected
    assert_eq!(results.len(), 3);

    // Check each collected file
    for (filename, expected_content) in &files {
        let collected_path = output_dir.path().join(format!("collected_{}", filename));
        assert!(collected_path.exists(), "File {} should be collected", filename);
        
        let collected_content = fs::read_to_string(&collected_path)?;
        assert_eq!(&collected_content, expected_content);
    }

    Ok(())
}

/// Test collection with non-existent files (should handle gracefully)
#[test]
fn test_missing_artifact_collection() -> Result<()> {
    let output_dir = TempDir::new()?;

    // Create artifacts with non-existent paths
    let artifacts = vec![
        Artifact {
            name: "missing_required".to_string(),
            artifact_type: ArtifactType::FileSystem,
            source_path: "/non/existent/path/file.txt".to_string(),
            destination_name: "missing.txt".to_string(),
            description: Some("This file doesn't exist".to_string()),
            required: true, // Required but missing - should log warning
            metadata: std::collections::HashMap::new(),
            regex: None,
        },
        Artifact {
            name: "missing_optional".to_string(),
            artifact_type: ArtifactType::FileSystem,
            source_path: "/another/missing/file.txt".to_string(),
            destination_name: "optional_missing.txt".to_string(),
            description: Some("Optional file that doesn't exist".to_string()),
            required: false, // Optional - should be silently skipped
            metadata: std::collections::HashMap::new(),
            regex: None,
        },
    ];

    // Collection should complete without error
    let results = collect_artifacts(&artifacts, output_dir.path())?;

    // No files should be collected since they don't exist
    assert_eq!(results.len(), 0);

    Ok(())
}

/// Test collection with directory structure preservation
#[test]
fn test_directory_structure_collection() -> Result<()> {
    // Create a directory structure
    let test_dir = TempDir::new()?;
    let sub_dir = test_dir.path().join("subdir");
    fs::create_dir(&sub_dir)?;
    
    let file1_path = test_dir.path().join("root_file.txt");
    let file2_path = sub_dir.join("sub_file.txt");
    
    fs::write(&file1_path, "Root level file")?;
    fs::write(&file2_path, "Subdirectory file")?;

    // Create output directory
    let output_dir = TempDir::new()?;

    // Create artifacts that preserve directory structure
    let artifacts = vec![
        Artifact {
            name: "root_file".to_string(),
            artifact_type: ArtifactType::FileSystem,
            source_path: file1_path.to_string_lossy().to_string(),
            destination_name: "collection/root_file.txt".to_string(),
            description: Some("Root level file".to_string()),
            required: true,
            metadata: std::collections::HashMap::new(),
            regex: None,
        },
        Artifact {
            name: "sub_file".to_string(),
            artifact_type: ArtifactType::FileSystem,
            source_path: file2_path.to_string_lossy().to_string(),
            destination_name: "collection/subdir/sub_file.txt".to_string(),
            description: Some("Subdirectory file".to_string()),
            required: true,
            metadata: std::collections::HashMap::new(),
            regex: None,
        },
    ];

    // Collect artifacts
    let results = collect_artifacts(&artifacts, output_dir.path())?;

    // Verify collection
    assert_eq!(results.len(), 2);

    // Check directory structure was created
    let collection_dir = output_dir.path().join("collection");
    assert!(collection_dir.exists());
    assert!(collection_dir.join("root_file.txt").exists());
    
    let sub_collection_dir = collection_dir.join("subdir");
    assert!(sub_collection_dir.exists());
    assert!(sub_collection_dir.join("sub_file.txt").exists());

    Ok(())
}