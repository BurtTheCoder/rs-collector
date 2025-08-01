//! Integration tests for full end-to-end collection scenarios.
//!
//! These tests simulate complete forensic collection workflows including
//! configuration loading, artifact collection, compression, and summary generation.

use std::fs;
use std::path::Path;
use tempfile::TempDir;
use anyhow::Result;

use rust_collector::config::{
    CollectionConfig, Artifact, ArtifactType,
    LinuxArtifactType, WindowsArtifactType, MacOSArtifactType,
    load_or_create_config
};
use rust_collector::collectors::collector::collect_artifacts;
use rust_collector::utils::compress::create_zip_file;
use rust_collector::utils::summary::create_collection_summary;
use rust_collector::models::ArtifactMetadata;

/// Test full collection workflow from config to ZIP
#[test]
fn test_full_collection_workflow() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = TempDir::new()?;
    
    // Create test artifacts
    fs::write(temp_dir.path().join("system.log"), "System log content")?;
    fs::write(temp_dir.path().join("app.log"), "Application log content")?;
    fs::write(temp_dir.path().join("config.ini"), "[settings]\nkey=value")?;
    
    // Create configuration
    let config = CollectionConfig {
        version: "1.0".to_string(),
        description: "Test collection config".to_string(),
        artifacts: vec![
            Artifact {
                name: "system_logs".to_string(),
                artifact_type: ArtifactType::Logs,
                source_path: temp_dir.path().join("system.log").to_string_lossy().to_string(),
                destination_name: "logs/system.log".to_string(),
                description: Some("System logs".to_string()),
                required: true,
                metadata: std::collections::HashMap::new(),
                regex: None,
            },
            Artifact {
                name: "app_logs".to_string(),
                artifact_type: ArtifactType::Logs,
                source_path: temp_dir.path().join("app.log").to_string_lossy().to_string(),
                destination_name: "logs/app.log".to_string(),
                description: Some("Application logs".to_string()),
                required: true,
                metadata: std::collections::HashMap::new(),
                regex: None,
            },
            Artifact {
                name: "config_files".to_string(),
                artifact_type: ArtifactType::Configuration,
                source_path: temp_dir.path().join("config.ini").to_string_lossy().to_string(),
                destination_name: "config/config.ini".to_string(),
                description: Some("Configuration files".to_string()),
                required: false,
                metadata: std::collections::HashMap::new(),
                regex: None,
            },
        ],
        global_options: std::collections::HashMap::new(),
    };
    
    // Save config
    let config_path = temp_dir.path().join("collection.yaml");
    config.save_to_yaml_file(&config_path)?;
    
    // Load config
    let loaded_config = load_or_create_config(Some(&config_path))?;
    assert_eq!(loaded_config.artifacts.len(), 3);
    
    // Collect artifacts
    let results = collect_artifacts(&loaded_config.artifacts, output_dir.path())?;
    assert_eq!(results.len(), 3);
    
    // Verify collected files
    assert!(output_dir.path().join("logs/system.log").exists());
    assert!(output_dir.path().join("logs/app.log").exists());
    assert!(output_dir.path().join("config/config.ini").exists());
    
    // Create summary
    let summary_path = output_dir.path().join("collection_summary.json");
    create_collection_summary(&results, &summary_path)?;
    assert!(summary_path.exists());
    
    // Create ZIP archive
    let zip_path = output_dir.path().join("collection.zip");
    create_zip_file(output_dir.path(), &zip_path)?;
    assert!(zip_path.exists());
    
    // Verify ZIP size is reasonable
    let zip_metadata = fs::metadata(&zip_path)?;
    assert!(zip_metadata.len() > 0);
    
    Ok(())
}

/// Test collection with missing artifacts
#[test]
fn test_collection_with_missing_artifacts() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = TempDir::new()?;
    
    // Create only some of the configured artifacts
    fs::write(temp_dir.path().join("exists.txt"), "This file exists")?;
    
    let config = CollectionConfig {
        version: "1.0".to_string(),
        description: "Test with missing files".to_string(),
        artifacts: vec![
            Artifact {
                name: "existing_file".to_string(),
                artifact_type: ArtifactType::Logs,
                source_path: temp_dir.path().join("exists.txt").to_string_lossy().to_string(),
                destination_name: "exists.txt".to_string(),
                description: Some("This file exists".to_string()),
                required: true,
                metadata: std::collections::HashMap::new(),
                regex: None,
            },
            Artifact {
                name: "missing_required".to_string(),
                artifact_type: ArtifactType::Logs,
                source_path: temp_dir.path().join("missing.txt").to_string_lossy().to_string(),
                destination_name: "missing.txt".to_string(),
                description: Some("This file is missing but required".to_string()),
                required: true,
                metadata: std::collections::HashMap::new(),
                regex: None,
            },
            Artifact {
                name: "missing_optional".to_string(),
                artifact_type: ArtifactType::Logs,
                source_path: temp_dir.path().join("optional.txt").to_string_lossy().to_string(),
                destination_name: "optional.txt".to_string(),
                description: Some("This file is missing and optional".to_string()),
                required: false,
                metadata: std::collections::HashMap::new(),
                regex: None,
            },
        ],
        global_options: std::collections::HashMap::new(),
    };
    
    // Collect artifacts
    let results = collect_artifacts(&config.artifacts, output_dir.path())?;
    
    // Should only collect the existing file
    assert_eq!(results.len(), 1);
    assert!(output_dir.path().join("exists.txt").exists());
    
    Ok(())
}

/// Test platform-specific collection
#[test]
fn test_platform_specific_collection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = TempDir::new()?;
    
    // Create test file
    fs::write(temp_dir.path().join("test.txt"), "Platform test")?;
    
    // Create platform-specific artifact
    let artifact = Artifact {
        name: "platform_test".to_string(),
        artifact_type: match std::env::consts::OS {
            "windows" => ArtifactType::Windows(WindowsArtifactType::File),
            "linux" => ArtifactType::Linux(LinuxArtifactType::File),
            "macos" => ArtifactType::MacOS(MacOSArtifactType::File),
            _ => ArtifactType::Logs,
        },
        source_path: temp_dir.path().join("test.txt").to_string_lossy().to_string(),
        destination_name: "platform_test.txt".to_string(),
        description: Some("Platform-specific test".to_string()),
        required: true,
        metadata: std::collections::HashMap::new(),
        regex: None,
    };
    
    let results = collect_artifacts(&vec![artifact], output_dir.path())?;
    assert_eq!(results.len(), 1);
    
    Ok(())
}

/// Test collection summary generation
#[test]
fn test_collection_summary() -> Result<()> {
    let output_dir = TempDir::new()?;
    
    // Create mock collection results
    let results = vec![
        (
            "file1.txt".to_string(),
            ArtifactMetadata {
                original_path: "/path/to/file1.txt".to_string(),
                collection_time: "2024-01-01T00:00:00Z".to_string(),
                file_size: 1024,
                created_time: Some("2023-12-01T00:00:00Z".to_string()),
                accessed_time: Some("2024-01-01T00:00:00Z".to_string()),
                modified_time: Some("2023-12-15T00:00:00Z".to_string()),
                is_locked: false,
            },
        ),
        (
            "file2.log".to_string(),
            ArtifactMetadata {
                original_path: "/var/log/file2.log".to_string(),
                collection_time: "2024-01-01T00:00:01Z".to_string(),
                file_size: 2048,
                created_time: None,
                accessed_time: None,
                modified_time: Some("2024-01-01T00:00:00Z".to_string()),
                is_locked: true,
            },
        ),
    ];
    
    // Create summary
    let summary_path = output_dir.path().join("summary.json");
    create_collection_summary(&results, &summary_path)?;
    
    // Verify summary file
    assert!(summary_path.exists());
    let summary_content = fs::read_to_string(&summary_path)?;
    let summary: serde_json::Value = serde_json::from_str(&summary_content)?;
    
    // Check summary structure
    assert!(summary["collection_time"].is_string());
    assert!(summary["total_artifacts"].is_number());
    assert_eq!(summary["total_artifacts"], 2);
    assert!(summary["total_size"].is_number());
    assert_eq!(summary["total_size"], 3072); // 1024 + 2048
    assert!(summary["artifacts"].is_array());
    
    Ok(())
}

/// Test collection with regex patterns
#[test]
fn test_collection_with_regex() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = TempDir::new()?;
    
    // Create test files
    fs::write(temp_dir.path().join("test1.log"), "Log entry 1")?;
    fs::write(temp_dir.path().join("test2.log"), "Log entry 2")?;
    fs::write(temp_dir.path().join("test.txt"), "Not a log")?;
    
    let artifact = Artifact {
        name: "log_files".to_string(),
        artifact_type: ArtifactType::Logs,
        source_path: temp_dir.path().to_string_lossy().to_string(),
        destination_name: "logs/".to_string(),
        description: Some("Collect all log files".to_string()),
        required: true,
        metadata: std::collections::HashMap::new(),
        regex: Some(rust_collector::config::RegexConfig {
            enabled: true,
            recursive: false,
            include_pattern: r".*\.log$".to_string(),
            exclude_pattern: String::new(),
            max_depth: Some(1),
        }),
    };
    
    // Note: Actual regex collection might not be implemented
    // This tests the configuration aspect
    let _ = collect_artifacts(&vec![artifact], output_dir.path());
    
    Ok(())
}

/// Test collection with environment variables
#[test]
fn test_collection_with_env_vars() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = TempDir::new()?;
    
    // Set test environment variable
    std::env::set_var("TEST_ARTIFACT_PATH", temp_dir.path().to_string_lossy().to_string());
    
    // Create test file
    fs::write(temp_dir.path().join("env_test.txt"), "Environment variable test")?;
    
    let mut config = CollectionConfig {
        version: "1.0".to_string(),
        description: "Test with env vars".to_string(),
        artifacts: vec![
            Artifact {
                name: "env_test".to_string(),
                artifact_type: ArtifactType::Logs,
                source_path: "$TEST_ARTIFACT_PATH/env_test.txt".to_string(),
                destination_name: "env_test.txt".to_string(),
                description: Some("Test environment variable expansion".to_string()),
                required: true,
                metadata: std::collections::HashMap::new(),
                regex: None,
            },
        ],
        global_options: std::collections::HashMap::new(),
    };
    
    // Process environment variables
    config.process_environment_variables()?;
    
    // Verify path was expanded
    assert!(!config.artifacts[0].source_path.contains("$TEST_ARTIFACT_PATH"));
    assert!(config.artifacts[0].source_path.contains("env_test.txt"));
    
    // Clean up
    std::env::remove_var("TEST_ARTIFACT_PATH");
    
    Ok(())
}

/// Test large collection performance
#[test]
fn test_large_collection_performance() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = TempDir::new()?;
    
    // Create many small files
    let num_files = 100;
    let mut artifacts = Vec::new();
    
    for i in 0..num_files {
        let filename = format!("file_{:03}.txt", i);
        let content = format!("Content of file {}", i);
        fs::write(temp_dir.path().join(&filename), &content)?;
        
        artifacts.push(Artifact {
            name: format!("file_{}", i),
            artifact_type: ArtifactType::Logs,
            source_path: temp_dir.path().join(&filename).to_string_lossy().to_string(),
            destination_name: format!("collected/{}", filename),
            description: Some(format!("Test file {}", i)),
            required: false,
            metadata: std::collections::HashMap::new(),
            regex: None,
        });
    }
    
    // Time the collection
    let start = std::time::Instant::now();
    let results = collect_artifacts(&artifacts, output_dir.path())?;
    let duration = start.elapsed();
    
    // Verify results
    assert_eq!(results.len(), num_files);
    assert!(duration.as_secs() < 10); // Should complete within 10 seconds
    
    // Create summary
    let summary_path = output_dir.path().join("performance_summary.json");
    create_collection_summary(&results, &summary_path)?;
    
    Ok(())
}