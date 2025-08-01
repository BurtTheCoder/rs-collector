//! Benchmarks for path validation and security checks.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rust_collector::security::path_validator::{validate_path, sanitize_filename, validate_output_path};
use std::path::Path;
use tempfile::TempDir;

/// Benchmark path validation with different path types
fn bench_path_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_validation");
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    
    let test_paths = vec![
        ("simple", "test.txt"),
        ("nested", "dir1/dir2/file.txt"),
        ("absolute", "/tmp/test.txt"),
        ("relative", "./subdir/file.txt"),
        ("complex", "dir1/./dir2/../dir3/file.txt"),
    ];
    
    for (name, path) in test_paths {
        group.bench_with_input(
            BenchmarkId::new("validate_path", name),
            path,
            |b, path| {
                b.iter(|| {
                    let _ = validate_path(
                        black_box(Path::new(path)),
                        black_box(Some(base_path))
                    );
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark filename sanitization
fn bench_filename_sanitization(c: &mut Criterion) {
    let mut group = c.benchmark_group("filename_sanitization");
    
    let test_filenames = vec![
        ("clean", "document.pdf"),
        ("spaces", "my document.pdf"),
        ("special", "file<>:\"|?*.txt"),
        ("unicode", "文档.txt"),
        ("path_traversal", "../../etc/passwd"),
        ("null_bytes", "file\0name.txt"),
        ("control_chars", "file\n\r\t.txt"),
        ("long", "a".repeat(255).as_str()),
    ];
    
    for (name, filename) in test_filenames {
        group.bench_with_input(
            BenchmarkId::new("sanitize", name),
            filename,
            |b, filename| {
                b.iter(|| {
                    sanitize_filename(black_box(filename));
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark output path validation
fn bench_output_path_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("output_path_validation");
    
    let test_paths = vec![
        ("safe_tmp", "/tmp/output.txt"),
        ("safe_home", "/home/user/output.txt"),
        ("unsafe_etc", "/etc/passwd"),
        ("unsafe_windows", "C:\\Windows\\System32\\config.sys"),
        ("nested_safe", "/tmp/dir1/dir2/output.txt"),
        ("nested_unsafe", "/sys/kernel/security/output"),
    ];
    
    for (name, path) in test_paths {
        group.bench_with_input(
            BenchmarkId::new("validate_output", name),
            path,
            |b, path| {
                b.iter(|| {
                    let _ = validate_output_path(black_box(Path::new(path)));
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark path validation under load
fn bench_path_validation_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_validation_throughput");
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    
    // Generate many paths to validate
    let path_counts = vec![100, 1000, 10000];
    
    for count in path_counts {
        let paths: Vec<String> = (0..count)
            .map(|i| format!("dir_{}/subdir_{}/file_{}.txt", i % 10, i % 5, i))
            .collect();
        
        group.bench_with_input(
            BenchmarkId::new("validate_many", count),
            &paths,
            |b, paths| {
                b.iter(|| {
                    for path in paths {
                        let _ = validate_path(
                            black_box(Path::new(path)),
                            black_box(Some(base_path))
                        );
                    }
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_path_validation,
    bench_filename_sanitization,
    bench_output_path_validation,
    bench_path_validation_throughput
);
criterion_main!(benches);