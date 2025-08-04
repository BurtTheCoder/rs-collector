//! Benchmarks for artifact collection performance.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rust_collector::collectors::collector::collect_artifacts;
use rust_collector::config::{Artifact, ArtifactType};
use std::fs;
use tempfile::TempDir;

/// Benchmark artifact collection with different numbers of artifacts
fn bench_artifact_collection_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("artifact_collection_count");

    let artifact_counts = vec![1, 10, 50, 100];

    for count in artifact_counts {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = TempDir::new().unwrap();

        // Create test files and artifacts
        let mut artifacts = Vec::new();
        let mut total_size = 0u64;

        for i in 0..count {
            let file_path = temp_dir.path().join(format!("artifact_{}.txt", i));
            let data = format!("Test artifact content {}", i).repeat(100);
            fs::write(&file_path, &data).unwrap();
            total_size += data.len() as u64;

            artifacts.push(Artifact {
                name: format!("artifact_{}", i),
                artifact_type: ArtifactType::Logs,
                source_path: file_path.to_string_lossy().to_string(),
                destination_name: format!("collected_{}.txt", i),
                description: Some(format!("Test artifact {}", i)),
                required: true,
                metadata: std::collections::HashMap::new(),
                regex: None,
            });
        }

        group.throughput(Throughput::Bytes(total_size));
        group.bench_with_input(
            BenchmarkId::new("collect", format!("{}_artifacts", count)),
            &artifacts,
            |b, artifacts| {
                b.iter(|| {
                    let _ = collect_artifacts(black_box(artifacts), black_box(output_dir.path()));
                    // Clean output directory for next iteration
                    for entry in fs::read_dir(output_dir.path()).unwrap() {
                        if let Ok(entry) = entry {
                            fs::remove_file(entry.path()).ok();
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark collection with different file sizes
fn bench_artifact_file_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("artifact_file_sizes");

    let sizes = vec![
        (1024, "1KB"),
        (100 * 1024, "100KB"),
        (1024 * 1024, "1MB"),
        (10 * 1024 * 1024, "10MB"),
    ];

    for (size, label) in sizes {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = TempDir::new().unwrap();

        let file_path = temp_dir.path().join("large_artifact.bin");
        let data = vec![0u8; size];
        fs::write(&file_path, &data).unwrap();

        let artifacts = vec![Artifact {
            name: "large_artifact".to_string(),
            artifact_type: ArtifactType::FileSystem,
            source_path: file_path.to_string_lossy().to_string(),
            destination_name: "collected_large.bin".to_string(),
            description: Some(format!("Large file test {}", label)),
            required: true,
            metadata: std::collections::HashMap::new(),
            regex: None,
        }];

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("collect_size", label),
            &artifacts,
            |b, artifacts| {
                b.iter(|| {
                    let _ = collect_artifacts(black_box(artifacts), black_box(output_dir.path()));
                    // Clean output
                    fs::remove_file(output_dir.path().join("collected_large.bin")).ok();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark parallel collection performance
fn bench_parallel_collection(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_collection");

    // Create a mix of small and large files
    let temp_dir = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();

    let mut artifacts = Vec::new();
    let mut total_size = 0u64;

    // 20 small files (10KB each)
    for i in 0..20 {
        let file_path = temp_dir.path().join(format!("small_{}.txt", i));
        let data = vec![b'A'; 10 * 1024];
        fs::write(&file_path, &data).unwrap();
        total_size += data.len() as u64;

        artifacts.push(Artifact {
            name: format!("small_{}", i),
            artifact_type: ArtifactType::Logs,
            source_path: file_path.to_string_lossy().to_string(),
            destination_name: format!("small_{}.txt", i),
            description: None,
            required: true,
            metadata: std::collections::HashMap::new(),
            regex: None,
        });
    }

    // 5 large files (1MB each)
    for i in 0..5 {
        let file_path = temp_dir.path().join(format!("large_{}.bin", i));
        let data = vec![b'B'; 1024 * 1024];
        fs::write(&file_path, &data).unwrap();
        total_size += data.len() as u64;

        artifacts.push(Artifact {
            name: format!("large_{}", i),
            artifact_type: ArtifactType::FileSystem,
            source_path: file_path.to_string_lossy().to_string(),
            destination_name: format!("large_{}.bin", i),
            description: None,
            required: true,
            metadata: std::collections::HashMap::new(),
            regex: None,
        });
    }

    group.throughput(Throughput::Bytes(total_size));
    group.bench_function("mixed_parallel", |b| {
        b.iter(|| {
            let _ = collect_artifacts(black_box(&artifacts), black_box(output_dir.path()));
            // Clean output directory
            for entry in fs::read_dir(output_dir.path()).unwrap() {
                if let Ok(entry) = entry {
                    if entry.path().is_file() {
                        fs::remove_file(entry.path()).ok();
                    }
                }
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_artifact_collection_count,
    bench_artifact_file_sizes,
    bench_parallel_collection
);
criterion_main!(benches);
