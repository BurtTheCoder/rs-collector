//! Integration tests for compression and ZIP creation functionality.
//!
//! These tests verify that artifacts are correctly compressed and
//! organized into ZIP files during collection.

use std::fs;
use std::io::Read;
use tempfile::TempDir;
use anyhow::Result;
use zip::ZipArchive;

use rust_collector::config::{
    Artifact, ArtifactType
};
use rust_collector::collectors::collector::collect_artifacts;
use rust_collector::utils::compress::create_zip_file;

/// Test basic ZIP file creation with multiple files
#[test]
fn test_zip_creation_basic() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = TempDir::new()?;
    
    // Create test files
    let files = vec![
        ("file1.txt", "Content of file 1"),
        ("file2.log", "Log file content"),
        ("data.json", r#"{"test": "data"}"#),
    ];
    
    for (filename, content) in &files {
        fs::write(temp_dir.path().join(filename), content)?;
    }
    
    // Create ZIP file
    let zip_path = output_dir.path().join("test_archive.zip");
    create_zip_file(temp_dir.path(), &zip_path)?;
    
    // Verify ZIP file exists and has correct content
    assert!(zip_path.exists());
    
    // Open and verify ZIP contents
    let file = fs::File::open(&zip_path)?;
    let mut archive = ZipArchive::new(file)?;
    
    assert_eq!(archive.len(), files.len());
    
    // Check each file in the archive
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name();
        
        // Find matching original file
        let original = files.iter().find(|(f, _)| name.contains(f));
        assert!(original.is_some(), "File {} not found in original files", name);
        
        // Read and compare content
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        assert_eq!(content, original.unwrap().1);
    }
    
    Ok(())
}

/// Test ZIP creation with directory structure preservation
#[test]
fn test_zip_directory_structure() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = TempDir::new()?;
    
    // Create directory structure
    let sub_dir = temp_dir.path().join("subdir");
    let deep_dir = sub_dir.join("deep");
    fs::create_dir_all(&deep_dir)?;
    
    // Create files at different levels
    fs::write(temp_dir.path().join("root.txt"), "Root file")?;
    fs::write(sub_dir.join("sub.txt"), "Subdirectory file")?;
    fs::write(deep_dir.join("deep.txt"), "Deep file")?;
    
    // Create ZIP file
    let zip_path = output_dir.path().join("structured.zip");
    create_zip_file(temp_dir.path(), &zip_path)?;
    
    // Verify directory structure in ZIP
    let file = fs::File::open(&zip_path)?;
    let mut archive = ZipArchive::new(file)?;
    
    // Check that all paths are preserved
    let mut found_files = vec![];
    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        found_files.push(file.name().to_string());
    }
    
    // Verify expected files exist (paths may vary by platform)
    assert!(found_files.iter().any(|f| f.contains("root.txt")));
    assert!(found_files.iter().any(|f| f.contains("sub.txt") && f.contains("subdir")));
    assert!(found_files.iter().any(|f| f.contains("deep.txt") && f.contains("deep")));
    
    Ok(())
}

/// Test compression of different file types
#[test]
fn test_compression_by_file_type() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = TempDir::new()?;
    
    // Create files that should be compressed
    let text_content = "This is a text file that should be compressed. ".repeat(100);
    fs::write(temp_dir.path().join("text.txt"), &text_content)?;
    
    // Create a file that's already compressed (simulate with random bytes)
    let compressed_content = vec![0x50, 0x4B, 0x03, 0x04]; // ZIP magic bytes
    fs::write(temp_dir.path().join("already.zip"), &compressed_content)?;
    
    // Create artifacts
    let artifacts = vec![
        Artifact {
            name: "text_file".to_string(),
            artifact_type: match std::env::consts::OS {
                "windows" => ArtifactType::FileSystem,
                "linux" => ArtifactType::FileSystem,
                "macos" => ArtifactType::FileSystem,
                _ => ArtifactType::FileSystem,
            },
            source_path: temp_dir.path().join("text.txt").to_string_lossy().to_string(),
            destination_name: "collected_text.txt".to_string(),
            description: Some("Text file for compression test".to_string()),
            required: true,
            metadata: std::collections::HashMap::new(),
            regex: None,
        },
        Artifact {
            name: "zip_file".to_string(),
            artifact_type: match std::env::consts::OS {
                "windows" => ArtifactType::FileSystem,
                "linux" => ArtifactType::FileSystem,
                "macos" => ArtifactType::FileSystem,
                _ => ArtifactType::FileSystem,
            },
            source_path: temp_dir.path().join("already.zip").to_string_lossy().to_string(),
            destination_name: "collected_zip.zip".to_string(),
            description: Some("Already compressed file".to_string()),
            required: true,
            metadata: std::collections::HashMap::new(),
            regex: None,
        },
    ];
    
    // Collect artifacts
    let results = collect_artifacts(&artifacts, output_dir.path())?;
    assert_eq!(results.len(), 2);
    
    // Create a ZIP of the collected files
    let zip_path = output_dir.path().join("compressed.zip");
    create_zip_file(output_dir.path(), &zip_path)?;
    
    // Verify the ZIP was created
    assert!(zip_path.exists());
    let metadata = fs::metadata(&zip_path)?;
    assert!(metadata.len() > 0);
    
    Ok(())
}

/// Test handling of empty directories in ZIP
#[test]
fn test_zip_empty_directories() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = TempDir::new()?;
    
    // Create empty directories
    fs::create_dir(temp_dir.path().join("empty1"))?;
    fs::create_dir(temp_dir.path().join("empty2"))?;
    
    // Create one file to ensure ZIP isn't completely empty
    fs::write(temp_dir.path().join("readme.txt"), "Not empty")?;
    
    // Create ZIP
    let zip_path = output_dir.path().join("with_empty_dirs.zip");
    create_zip_file(temp_dir.path(), &zip_path)?;
    
    // Verify ZIP exists
    assert!(zip_path.exists());
    
    Ok(())
}

/// Test collection with ZIP output
#[test]
fn test_collection_to_zip() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = TempDir::new()?;
    
    // Create test files
    let test_files = vec![
        ("evidence1.txt", "Evidence content 1"),
        ("evidence2.log", "Evidence content 2"),
        ("data.bin", "Binary data content"),
    ];
    
    let mut artifacts = Vec::new();
    for (filename, content) in &test_files {
        let file_path = temp_dir.path().join(filename);
        fs::write(&file_path, content)?;
        
        artifacts.push(Artifact {
            name: filename.to_string(),
            artifact_type: match std::env::consts::OS {
                "windows" => ArtifactType::FileSystem,
                "linux" => ArtifactType::FileSystem,
                "macos" => ArtifactType::FileSystem,
                _ => ArtifactType::FileSystem,
            },
            source_path: file_path.to_string_lossy().to_string(),
            destination_name: format!("collected_{}", filename),
            description: Some(format!("Test file: {}", filename)),
            required: true,
            metadata: std::collections::HashMap::new(),
            regex: None,
        });
    }
    
    // Collect artifacts
    let results = collect_artifacts(&artifacts, output_dir.path())?;
    assert_eq!(results.len(), test_files.len());
    
    // Create ZIP of collected artifacts
    let zip_path = output_dir.path().join("collection.zip");
    create_zip_file(output_dir.path(), &zip_path)?;
    
    // Verify ZIP contents
    let file = fs::File::open(&zip_path)?;
    let mut archive = ZipArchive::new(file)?;
    
    // Should have at least our collected files
    assert!(archive.len() >= test_files.len());
    
    // Verify each collected file is in the ZIP
    for (filename, expected_content) in &test_files {
        let collected_name = format!("collected_{}", filename);
        let mut found = false;
        
        for i in 0..archive.len() {
            let mut zip_file = archive.by_index(i)?;
            if zip_file.name().contains(&collected_name) {
                let mut content = String::new();
                zip_file.read_to_string(&mut content)?;
                assert_eq!(content, *expected_content);
                found = true;
                break;
            }
        }
        
        assert!(found, "File {} not found in ZIP", collected_name);
    }
    
    Ok(())
}

/// Test handling of large files in ZIP
#[test]
fn test_zip_large_file_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = TempDir::new()?;
    
    // Create a "large" file (1MB for testing)
    let large_content = vec![b'A'; 1024 * 1024];
    fs::write(temp_dir.path().join("large.bin"), &large_content)?;
    
    // Create a small file
    fs::write(temp_dir.path().join("small.txt"), "Small file")?;
    
    // Create ZIP
    let zip_path = output_dir.path().join("mixed_sizes.zip");
    create_zip_file(temp_dir.path(), &zip_path)?;
    
    // Verify ZIP was created and contains files
    assert!(zip_path.exists());
    
    let file = fs::File::open(&zip_path)?;
    let archive = ZipArchive::new(file)?;
    assert_eq!(archive.len(), 2);
    
    Ok(())
}

/// Test special characters in filenames
#[test]
fn test_zip_special_characters() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = TempDir::new()?;
    
    // Create files with special characters (safe for most filesystems)
    let special_files = vec![
        ("file with spaces.txt", "Content with spaces"),
        ("file-with-dashes.txt", "Content with dashes"),
        ("file_with_underscores.txt", "Content with underscores"),
    ];
    
    for (filename, content) in &special_files {
        fs::write(temp_dir.path().join(filename), content)?;
    }
    
    // Create ZIP
    let zip_path = output_dir.path().join("special_chars.zip");
    create_zip_file(temp_dir.path(), &zip_path)?;
    
    // Verify files in ZIP
    let file = fs::File::open(&zip_path)?;
    let archive = ZipArchive::new(file)?;
    assert_eq!(archive.len(), special_files.len());
    
    Ok(())
}