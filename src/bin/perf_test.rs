//! Simple performance testing tool for rs-collector.
//!
//! This provides basic performance metrics without requiring criterion.

use anyhow::Result;
use chrono;
use rust_collector::collectors::collector::collect_artifacts;
use rust_collector::config::{Artifact, ArtifactType};
use rust_collector::utils::compress::compress_artifacts;
use rust_collector::utils::hash::calculate_sha256;
use std::fs;
use std::path::Path;
use std::time::Instant;

fn main() -> Result<()> {
    println!("rs-collector Performance Test Suite\n");

    // Create test data directory
    let test_dir = Path::new("./perf_test_data");
    fs::create_dir_all(&test_dir)?;
    println!("Creating test data in: {}", test_dir.display());

    // Run tests
    test_hash_performance(&test_dir)?;
    test_compression_performance(&test_dir)?;
    test_collection_performance(&test_dir)?;

    // Cleanup
    fs::remove_dir_all(&test_dir)?;

    println!("\nPerformance tests completed!");
    Ok(())
}

/// Test hash calculation performance
fn test_hash_performance(test_dir: &Path) -> Result<()> {
    println!("\n=== Hash Performance Test ===");

    let sizes = vec![
        (1024 * 1024, "1MB"),
        (10 * 1024 * 1024, "10MB"),
        (100 * 1024 * 1024, "100MB"),
    ];

    for (size, label) in sizes {
        let file_path = test_dir.join(format!("hash_test_{}.bin", label));

        // Create test file
        println!("Creating {} test file...", label);
        let data = vec![0u8; size];
        fs::write(&file_path, &data)?;

        // Time hash calculation
        let start = Instant::now();
        let hash = calculate_sha256(&file_path, 1024)?; // 1GB max
        let duration = start.elapsed();

        let throughput = (size as f64) / duration.as_secs_f64() / 1024.0 / 1024.0;

        let hash_str = hash.unwrap_or_else(|| "NO_HASH".to_string());
        println!(
            "  {} file: {:?} ({:.2} MB/s) - Hash: {}",
            label,
            duration,
            throughput,
            if hash_str.len() >= 16 {
                &hash_str[..16]
            } else {
                &hash_str
            }
        );
    }

    Ok(())
}

/// Test compression performance
fn test_compression_performance(test_dir: &Path) -> Result<()> {
    println!("\n=== Compression Performance Test ===");

    // Create test files
    let file_count = 100;
    let file_size = 100 * 1024; // 100KB each

    let source_dir = test_dir.join("compress_test");
    fs::create_dir_all(&source_dir)?;

    println!("Creating {} test files...", file_count);
    for i in 0..file_count {
        let file_path = source_dir.join(format!("file_{:03}.txt", i));
        let data = format!("Test data {}\n", i).repeat(file_size / 20);
        fs::write(&file_path, data)?;
    }

    // Time compression using compress_artifacts
    let start = Instant::now();
    let hostname = "test-host";
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let output_path = compress_artifacts(&source_dir, hostname, &timestamp)?;
    let duration = start.elapsed();

    let input_size = (file_count * file_size) as f64 / 1024.0 / 1024.0;
    let output_size = fs::metadata(&output_path)?.len() as f64 / 1024.0 / 1024.0;
    let compression_ratio = (1.0 - (output_size / input_size)) * 100.0;

    println!("  Compressed {} files in {:?}", file_count, duration);
    println!(
        "  Input: {:.2}MB, Output: {:.2}MB, Compression: {:.1}%",
        input_size, output_size, compression_ratio
    );
    println!(
        "  Throughput: {:.2} MB/s",
        input_size / duration.as_secs_f64()
    );

    Ok(())
}

/// Test artifact collection performance
fn test_collection_performance(test_dir: &Path) -> Result<()> {
    println!("\n=== Collection Performance Test ===");

    let source_dir = test_dir.join("collect_test");
    let output_dir = test_dir.join("collected");
    fs::create_dir_all(&source_dir)?;
    fs::create_dir_all(&output_dir)?;

    // Create test artifacts
    let artifact_count = 50;
    let mut artifacts = Vec::new();

    println!("Creating {} test artifacts...", artifact_count);
    for i in 0..artifact_count {
        let file_path = source_dir.join(format!("artifact_{:03}.log", i));
        let data = format!("Log entry {}\n", i).repeat(1000);
        fs::write(&file_path, &data)?;

        artifacts.push(Artifact {
            name: format!("artifact_{}", i),
            artifact_type: ArtifactType::Logs,
            source_path: file_path.to_string_lossy().to_string(),
            destination_name: format!("logs/artifact_{:03}.log", i),
            description: Some(format!("Test artifact {}", i)),
            required: true,
            metadata: std::collections::HashMap::new(),
            regex: None,
        });
    }

    // Time collection
    let start = Instant::now();
    let results = collect_artifacts(&artifacts, &output_dir)?;
    let duration = start.elapsed();

    println!("  Collected {} artifacts in {:?}", results.len(), duration);
    println!(
        "  Rate: {:.2} files/second",
        artifact_count as f64 / duration.as_secs_f64()
    );

    // Calculate total size collected
    let mut total_size = 0u64;
    for (_, metadata) in &results {
        total_size += metadata.file_size;
    }

    println!(
        "  Total size: {:.2} MB",
        total_size as f64 / 1024.0 / 1024.0
    );
    println!(
        "  Throughput: {:.2} MB/s",
        (total_size as f64 / 1024.0 / 1024.0) / duration.as_secs_f64()
    );

    Ok(())
}
