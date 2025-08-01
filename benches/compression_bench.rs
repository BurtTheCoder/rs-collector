//! Benchmarks for compression performance.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use rust_collector::utils::compress::{create_zip_file, get_compression_options};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Benchmark ZIP creation with different file counts
fn bench_zip_file_counts(c: &mut Criterion) {
    let mut group = c.benchmark_group("zip_file_counts");
    
    let file_counts = vec![10, 50, 100, 500];
    let file_size = 10 * 1024; // 10KB per file
    
    for count in file_counts {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = TempDir::new().unwrap();
        
        // Create test files
        for i in 0..count {
            let file_path = temp_dir.path().join(format!("file_{}.txt", i));
            let data = vec![b'A'; file_size];
            fs::write(&file_path, data).unwrap();
        }
        
        let zip_path = output_dir.path().join("output.zip");
        
        group.throughput(Throughput::Bytes((count * file_size) as u64));
        group.bench_with_input(
            BenchmarkId::new("create_zip", format!("{}_files", count)),
            &(temp_dir.path(), &zip_path),
            |b, (source, dest)| {
                b.iter(|| {
                    create_zip_file(black_box(source), black_box(dest)).unwrap();
                    // Clean up for next iteration
                    fs::remove_file(dest).ok();
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark compression methods
fn bench_compression_methods(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_methods");
    let temp_dir = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();
    
    // Create different types of files
    let test_files = vec![
        ("text_file.txt", vec![b'A'; 100 * 1024], "text_repetitive"),
        ("random.bin", (0..100_000).map(|i| (i % 256) as u8).collect(), "binary_random"),
        ("zeros.dat", vec![0u8; 100 * 1024], "zeros"),
    ];
    
    for (filename, data, label) in test_files {
        let file_path = temp_dir.path().join(filename);
        fs::write(&file_path, &data).unwrap();
        
        let zip_path = output_dir.path().join(format!("{}.zip", label));
        
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("compress", label),
            &(&file_path, &zip_path),
            |b, (source, dest)| {
                let temp_dir = TempDir::new().unwrap();
                let temp_file = temp_dir.path().join(filename);
                fs::copy(source, &temp_file).unwrap();
                
                b.iter(|| {
                    create_zip_file(black_box(temp_dir.path()), black_box(dest)).unwrap();
                    fs::remove_file(dest).ok();
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark directory traversal and compression
fn bench_directory_structures(c: &mut Criterion) {
    let mut group = c.benchmark_group("directory_structures");
    
    // Different directory structures
    let structures = vec![
        ("flat", 100, 1),      // 100 files in root
        ("shallow", 10, 10),   // 10 dirs with 10 files each
        ("deep", 5, 20),       // 5 levels deep, 20 files per level
    ];
    
    for (name, dirs, files_per_dir) in structures {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = TempDir::new().unwrap();
        let total_size = create_directory_structure(&temp_dir, dirs, files_per_dir);
        
        let zip_path = output_dir.path().join(format!("{}.zip", name));
        
        group.throughput(Throughput::Bytes(total_size));
        group.bench_with_input(
            BenchmarkId::new("compress_structure", name),
            &(temp_dir.path(), &zip_path),
            |b, (source, dest)| {
                b.iter(|| {
                    create_zip_file(black_box(source), black_box(dest)).unwrap();
                    fs::remove_file(dest).ok();
                });
            },
        );
    }
    
    group.finish();
}

/// Helper function to create directory structures for benchmarking
fn create_directory_structure(base: &TempDir, num_dirs: usize, files_per_dir: usize) -> u64 {
    let mut total_size = 0u64;
    let file_size = 1024; // 1KB per file
    
    if num_dirs == 1 {
        // Flat structure
        for i in 0..files_per_dir {
            let file_path = base.path().join(format!("file_{}.txt", i));
            fs::write(&file_path, vec![b'A'; file_size]).unwrap();
            total_size += file_size as u64;
        }
    } else {
        // Nested structure
        for i in 0..num_dirs {
            let dir_path = base.path().join(format!("dir_{}", i));
            fs::create_dir_all(&dir_path).unwrap();
            
            for j in 0..files_per_dir {
                let file_path = dir_path.join(format!("file_{}.txt", j));
                fs::write(&file_path, vec![b'A'; file_size]).unwrap();
                total_size += file_size as u64;
            }
        }
    }
    
    total_size
}

criterion_group!(
    benches,
    bench_zip_file_counts,
    bench_compression_methods,
    bench_directory_structures
);
criterion_main!(benches);