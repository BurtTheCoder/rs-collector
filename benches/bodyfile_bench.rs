//! Benchmarks for bodyfile generation performance.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rust_collector::utils::bodyfile::{generate_bodyfile, BodyfileOptions};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Benchmark bodyfile generation with different file counts
fn bench_bodyfile_file_counts(c: &mut Criterion) {
    let mut group = c.benchmark_group("bodyfile_file_counts");

    let file_counts = vec![10, 100, 500, 1000];

    for count in file_counts {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = TempDir::new().unwrap();

        // Create test files
        for i in 0..count {
            let file_path = temp_dir.path().join(format!("file_{}.txt", i));
            fs::write(&file_path, format!("Content of file {}", i)).unwrap();
        }

        let output_path = output_dir.path().join("bodyfile.txt");

        group.bench_with_input(
            BenchmarkId::new("generate", format!("{}_files", count)),
            &(temp_dir.path(), &output_path),
            |b, (source, output)| {
                b.iter(|| {
                    generate_bodyfile(
                        black_box(source),
                        black_box(output),
                        black_box(&BodyfileOptions::default()),
                    )
                    .unwrap();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark bodyfile generation with hash calculation
fn bench_bodyfile_with_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("bodyfile_hashing");
    let temp_dir = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();

    // Create test files of different sizes
    let files = vec![
        ("small", 100, 1024),       // 100 files of 1KB
        ("medium", 50, 100 * 1024), // 50 files of 100KB
        ("large", 10, 1024 * 1024), // 10 files of 1MB
    ];

    for (name, count, size) in files {
        let test_dir = TempDir::new().unwrap();
        let mut total_size = 0u64;

        for i in 0..count {
            let file_path = test_dir.path().join(format!("{}_{}.bin", name, i));
            let data = vec![0u8; size];
            fs::write(&file_path, &data).unwrap();
            total_size += size as u64;
        }

        let output_path = output_dir.path().join(format!("bodyfile_{}.txt", name));

        group.throughput(Throughput::Bytes(total_size));

        // With hashing
        group.bench_with_input(
            BenchmarkId::new("with_hash", name),
            &(test_dir.path(), &output_path),
            |b, (source, output)| {
                let options = BodyfileOptions {
                    include_hash: true,
                    hash_large_files: true,
                    large_file_threshold: 10 * 1024 * 1024,
                    ..Default::default()
                };
                b.iter(|| {
                    generate_bodyfile(black_box(source), black_box(output), black_box(&options))
                        .unwrap();
                });
            },
        );

        // Without hashing
        group.bench_with_input(
            BenchmarkId::new("no_hash", name),
            &(test_dir.path(), &output_path),
            |b, (source, output)| {
                let options = BodyfileOptions {
                    include_hash: false,
                    ..Default::default()
                };
                b.iter(|| {
                    generate_bodyfile(black_box(source), black_box(output), black_box(&options))
                        .unwrap();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark bodyfile generation with directory traversal
fn bench_bodyfile_directory_depth(c: &mut Criterion) {
    let mut group = c.benchmark_group("bodyfile_directory_depth");

    let structures = vec![
        ("flat", 1, 1000),    // 1000 files in root
        ("shallow", 10, 100), // 10 dirs with 100 files each
        ("deep", 100, 10),    // 100 dirs with 10 files each
        ("nested", 5, 200),   // 5 levels deep, 200 files per level
    ];

    for (name, depth, files) in structures {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = TempDir::new().unwrap();

        create_nested_structure(&temp_dir, depth, files);

        let output_path = output_dir.path().join(format!("bodyfile_{}.txt", name));

        group.bench_with_input(
            BenchmarkId::new("generate_structure", name),
            &(temp_dir.path(), &output_path),
            |b, (source, output)| {
                b.iter(|| {
                    generate_bodyfile(
                        black_box(source),
                        black_box(output),
                        black_box(&BodyfileOptions::default()),
                    )
                    .unwrap();
                });
            },
        );
    }

    group.finish();
}

/// Helper to create nested directory structures
fn create_nested_structure(base: &TempDir, depth: usize, files_per_level: usize) {
    if depth == 1 {
        // Create files in root
        for i in 0..files_per_level {
            let file_path = base.path().join(format!("file_{}.txt", i));
            fs::write(&file_path, format!("Content {}", i)).unwrap();
        }
    } else {
        // Create nested directories
        let mut current_path = base.path().to_path_buf();
        for level in 0..depth {
            let dir_name = format!("level_{}", level);
            current_path = current_path.join(&dir_name);
            fs::create_dir_all(&current_path).unwrap();

            // Create files at this level
            for i in 0..(files_per_level / depth) {
                let file_path = current_path.join(format!("file_{}_{}.txt", level, i));
                fs::write(&file_path, format!("Content {} {}", level, i)).unwrap();
            }
        }
    }
}

criterion_group!(
    benches,
    bench_bodyfile_file_counts,
    bench_bodyfile_with_hashing,
    bench_bodyfile_directory_depth
);
criterion_main!(benches);
