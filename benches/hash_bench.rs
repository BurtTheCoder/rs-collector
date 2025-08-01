//! Benchmarks for hash calculation performance.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use rust_collector::utils::hash::calculate_sha256;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Benchmark SHA256 calculation for different file sizes
fn bench_sha256_file_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha256_file_sizes");
    let temp_dir = TempDir::new().unwrap();
    
    // Test different file sizes
    let sizes = vec![
        (1024, "1KB"),
        (10 * 1024, "10KB"),
        (100 * 1024, "100KB"),
        (1024 * 1024, "1MB"),
        (10 * 1024 * 1024, "10MB"),
    ];
    
    for (size, name) in sizes {
        let file_path = temp_dir.path().join(format!("test_{}.bin", name));
        let data = vec![0u8; size];
        fs::write(&file_path, &data).unwrap();
        
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("calculate_sha256", name),
            &file_path,
            |b, path| {
                b.iter(|| {
                    calculate_sha256(black_box(path)).unwrap()
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark SHA256 calculation with different buffer sizes
fn bench_sha256_buffer_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha256_buffer_sizes");
    let temp_dir = TempDir::new().unwrap();
    
    // Create a 5MB test file
    let file_path = temp_dir.path().join("test_5mb.bin");
    let data = vec![0u8; 5 * 1024 * 1024];
    fs::write(&file_path, &data).unwrap();
    
    // Note: In real implementation, we'd need to modify calculate_sha256
    // to accept a buffer size parameter for this benchmark
    group.throughput(Throughput::Bytes(5 * 1024 * 1024));
    group.bench_function("default_buffer", |b| {
        b.iter(|| {
            calculate_sha256(black_box(&file_path)).unwrap()
        });
    });
    
    group.finish();
}

/// Benchmark parallel vs sequential hash calculation
fn bench_parallel_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_hashing");
    let temp_dir = TempDir::new().unwrap();
    
    // Create multiple files
    let num_files = 10;
    let file_size = 1024 * 1024; // 1MB each
    let mut file_paths = Vec::new();
    
    for i in 0..num_files {
        let file_path = temp_dir.path().join(format!("file_{}.bin", i));
        let data = vec![0u8; file_size];
        fs::write(&file_path, &data).unwrap();
        file_paths.push(file_path);
    }
    
    group.throughput(Throughput::Bytes((num_files * file_size) as u64));
    
    // Sequential hashing
    group.bench_function("sequential", |b| {
        b.iter(|| {
            for path in &file_paths {
                calculate_sha256(black_box(path)).unwrap();
            }
        });
    });
    
    // Parallel hashing using rayon
    group.bench_function("parallel", |b| {
        use rayon::prelude::*;
        b.iter(|| {
            file_paths.par_iter().for_each(|path| {
                calculate_sha256(black_box(path)).unwrap();
            });
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_sha256_file_sizes,
    bench_sha256_buffer_sizes,
    bench_parallel_hashing
);
criterion_main!(benches);