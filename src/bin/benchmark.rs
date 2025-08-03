//! Comprehensive performance benchmarking tool for rs-collector.
//!
//! This tool provides detailed performance metrics for various operations
//! including parallel collection, memory collection, and streaming uploads.

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use anyhow::Result;
use tokio::runtime::Runtime;
use rust_collector::{
    utils::{hash::calculate_sha256, compress::compress_artifacts, bodyfile::generate_bodyfile},
    config::{Artifact, ArtifactType, LinuxArtifactType},
    collectors::{
        collector::collect_artifacts_parallel,
        volatile::{collect_volatile_data, collect_processes},
    },
};

#[cfg(feature = "memory_collection")]
use rust_collector::collectors::memory::{
    collector::MemoryCollector,
    models::MemoryCollectionOptions,
    filters::{ProcessFilter, MemoryRegionFilter},
};

/// Benchmark results for a single operation
#[derive(Debug)]
struct BenchmarkResult {
    operation: String,
    iterations: usize,
    total_time: Duration,
    min_time: Duration,
    max_time: Duration,
    avg_time: Duration,
    throughput_mb_per_sec: Option<f64>,
}

impl BenchmarkResult {
    fn print(&self) {
        println!("\n{}", self.operation);
        println!("  Iterations: {}", self.iterations);
        println!("  Total time: {:?}", self.total_time);
        println!("  Min time:   {:?}", self.min_time);
        println!("  Max time:   {:?}", self.max_time);
        println!("  Avg time:   {:?}", self.avg_time);
        if let Some(throughput) = self.throughput_mb_per_sec {
            println!("  Throughput: {:.2} MB/s", throughput);
        }
    }
}

fn main() -> Result<()> {
    println!("🚀 rs-collector Comprehensive Performance Benchmark\n");
    
    let rt = Runtime::new()?;
    
    // Create test environment
    let test_dir = Path::new("./benchmark_data");
    fs::create_dir_all(&test_dir)?;
    println!("Created test directory: {}", test_dir.display());
    
    // Run benchmarks
    let mut results = Vec::new();
    
    results.push(benchmark_hash_calculation(&test_dir)?);
    results.push(benchmark_compression(&test_dir)?);
    results.push(benchmark_bodyfile_generation(&test_dir)?);
    results.push(benchmark_parallel_collection(&test_dir, &rt)?);
    results.push(benchmark_volatile_collection(&rt)?);
    
    #[cfg(feature = "memory_collection")]
    results.push(benchmark_memory_collection(&test_dir, &rt)?);
    
    // Print summary
    println!("\n{}", "=".repeat(80));
    println!("BENCHMARK SUMMARY");
    println!("{}", "=".repeat(80));
    
    for result in &results {
        result.print();
    }
    
    // Cleanup
    fs::remove_dir_all(&test_dir)?;
    
    println!("\n✅ Benchmark completed successfully!");
    Ok(())
}

/// Benchmark SHA-256 hash calculation
fn benchmark_hash_calculation(test_dir: &Path) -> Result<BenchmarkResult> {
    println!("\n📊 Benchmarking Hash Calculation...");
    
    let file_sizes = vec![
        (1024 * 1024, "1MB"),
        (10 * 1024 * 1024, "10MB"),
        (50 * 1024 * 1024, "50MB"),
    ];
    
    let mut times = Vec::new();
    let mut total_bytes = 0u64;
    
    for (size, label) in file_sizes {
        let file_path = test_dir.join(format!("hash_test_{}.bin", label));
        
        // Create test file with random data
        let data = vec![42u8; size];
        fs::write(&file_path, &data)?;
        
        // Warm up
        let _ = calculate_sha256(&file_path, size as u64 * 2)?;
        
        // Benchmark
        let start = Instant::now();
        let _ = calculate_sha256(&file_path, size as u64 * 2)?;
        let duration = start.elapsed();
        
        times.push(duration);
        total_bytes += size as u64;
        
        println!("  {} file: {:?}", label, duration);
    }
    
    let total_time: Duration = times.iter().sum();
    let throughput = (total_bytes as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64();
    
    Ok(BenchmarkResult {
        operation: "SHA-256 Hash Calculation".to_string(),
        iterations: times.len(),
        total_time,
        min_time: *times.iter().min().unwrap(),
        max_time: *times.iter().max().unwrap(),
        avg_time: total_time / times.len() as u32,
        throughput_mb_per_sec: Some(throughput),
    })
}

/// Benchmark compression performance
fn benchmark_compression(test_dir: &Path) -> Result<BenchmarkResult> {
    println!("\n📊 Benchmarking Compression...");
    
    let source_dir = test_dir.join("compress_test");
    fs::create_dir_all(&source_dir)?;
    
    // Create test files
    let file_counts = vec![10, 50, 100];
    let mut times = Vec::new();
    let mut total_input_size = 0u64;
    
    for count in file_counts {
        // Create test files
        for i in 0..count {
            let file_path = source_dir.join(format!("file_{:04}.txt", i));
            let data = format!("Test data line {}\n", i).repeat(1000);
            fs::write(&file_path, data)?;
        }
        
        let input_size = fs::read_dir(&source_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
            .sum::<u64>();
        
        total_input_size += input_size;
        
        // Benchmark
        let start = Instant::now();
        let output_path = compress_artifacts(&source_dir, "bench", &format!("{}", count))?;
        let duration = start.elapsed();
        
        times.push(duration);
        
        let output_size = fs::metadata(&output_path)?.len();
        let compression_ratio = (1.0 - (output_size as f64 / input_size as f64)) * 100.0;
        
        println!("  {} files: {:?} (compression: {:.1}%)", count, duration, compression_ratio);
        
        // Cleanup
        fs::remove_file(output_path)?;
    }
    
    let total_time: Duration = times.iter().sum();
    let throughput = (total_input_size as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64();
    
    Ok(BenchmarkResult {
        operation: "ZIP Compression".to_string(),
        iterations: times.len(),
        total_time,
        min_time: *times.iter().min().unwrap(),
        max_time: *times.iter().max().unwrap(),
        avg_time: total_time / times.len() as u32,
        throughput_mb_per_sec: Some(throughput),
    })
}

/// Benchmark bodyfile generation
fn benchmark_bodyfile_generation(test_dir: &Path) -> Result<BenchmarkResult> {
    println!("\n📊 Benchmarking Bodyfile Generation...");
    
    let bodyfile_dir = test_dir.join("bodyfile_test");
    fs::create_dir_all(&bodyfile_dir)?;
    
    // Create directory structure
    let file_counts = vec![100, 500, 1000];
    let mut times = Vec::new();
    
    for count in file_counts {
        // Create nested directory structure
        for i in 0..count {
            let subdir = bodyfile_dir.join(format!("dir_{:03}", i % 10));
            fs::create_dir_all(&subdir)?;
            let file_path = subdir.join(format!("file_{:04}.txt", i));
            fs::write(&file_path, format!("Content {}", i))?;
        }
        
        let output_path = test_dir.join(format!("timeline_{}.bodyfile", count));
        
        // Benchmark
        let start = Instant::now();
        let mut options = std::collections::HashMap::new();
        options.insert("root_path".to_string(), bodyfile_dir.to_string_lossy().to_string());
        generate_bodyfile(&output_path, &options)?;
        let duration = start.elapsed();
        
        times.push(duration);
        
        // Count the entries in the file (approximate)
        let entries = count;
        let rate = entries as f64 / duration.as_secs_f64();
        println!("  {} files: {:?} ({:.0} entries/sec)", count, duration, rate);
        
        // Cleanup
        fs::remove_file(output_path)?;
    }
    
    let total_time: Duration = times.iter().sum();
    
    Ok(BenchmarkResult {
        operation: "Bodyfile Generation".to_string(),
        iterations: times.len(),
        total_time,
        min_time: *times.iter().min().unwrap(),
        max_time: *times.iter().max().unwrap(),
        avg_time: total_time / times.len() as u32,
        throughput_mb_per_sec: None,
    })
}

/// Benchmark parallel artifact collection
fn benchmark_parallel_collection(test_dir: &Path, rt: &Runtime) -> Result<BenchmarkResult> {
    println!("\n📊 Benchmarking Parallel Collection...");
    
    let source_dir = test_dir.join("parallel_test");
    let output_dir = test_dir.join("parallel_output");
    fs::create_dir_all(&source_dir)?;
    
    // Create test artifacts
    let artifact_counts = vec![10, 25, 50];
    let mut times = Vec::new();
    
    for count in artifact_counts {
        // Create test files
        let mut artifacts = Vec::new();
        for i in 0..count {
            let file_path = source_dir.join(format!("artifact_{:03}.log", i));
            fs::write(&file_path, format!("Log data {}\n", i).repeat(100))?;
            
            artifacts.push(Artifact {
                name: format!("artifact_{}", i),
                artifact_type: ArtifactType::Linux(LinuxArtifactType::SysLogs),
                source_path: file_path.to_string_lossy().to_string(),
                destination_name: format!("logs/artifact_{:03}.log", i),
                description: Some(format!("Test artifact {}", i)),
                required: true,
                metadata: std::collections::HashMap::new(),
                regex: None,
            });
        }
        
        // Clean output directory
        let _ = fs::remove_dir_all(&output_dir);
        fs::create_dir_all(&output_dir)?;
        
        // Benchmark parallel collection
        let start = Instant::now();
        let results = rt.block_on(collect_artifacts_parallel(&artifacts, &output_dir))?;
        let duration = start.elapsed();
        
        times.push(duration);
        
        let rate = results.len() as f64 / duration.as_secs_f64();
        println!("  {} artifacts: {:?} ({:.0} artifacts/sec)", count, duration, rate);
    }
    
    let total_time: Duration = times.iter().sum();
    
    Ok(BenchmarkResult {
        operation: "Parallel Artifact Collection".to_string(),
        iterations: times.len(),
        total_time,
        min_time: *times.iter().min().unwrap(),
        max_time: *times.iter().max().unwrap(),
        avg_time: total_time / times.len() as u32,
        throughput_mb_per_sec: None,
    })
}

/// Benchmark volatile data collection
fn benchmark_volatile_collection(rt: &Runtime) -> Result<BenchmarkResult> {
    println!("\n📊 Benchmarking Volatile Data Collection...");
    
    let iterations = 5;
    let mut times = Vec::new();
    
    for i in 0..iterations {
        // Warm up on first iteration
        if i == 0 {
            rt.block_on(collect_volatile_data())?;
        }
        
        let start = Instant::now();
        let data = rt.block_on(collect_volatile_data())?;
        let duration = start.elapsed();
        
        times.push(duration);
        
        println!("  Iteration {}: {:?} ({} processes)", 
                 i + 1, duration, data.processes.len());
    }
    
    let total_time: Duration = times.iter().sum();
    
    Ok(BenchmarkResult {
        operation: "Volatile Data Collection".to_string(),
        iterations,
        total_time,
        min_time: *times.iter().min().unwrap(),
        max_time: *times.iter().max().unwrap(),
        avg_time: total_time / iterations as u32,
        throughput_mb_per_sec: None,
    })
}

/// Benchmark memory collection (if feature enabled)
#[cfg(feature = "memory_collection")]
fn benchmark_memory_collection(test_dir: &Path, rt: &Runtime) -> Result<BenchmarkResult> {
    println!("\n📊 Benchmarking Memory Collection...");
    
    use rust_collector::collectors::memory::is_memory_collection_available;
    
    if !is_memory_collection_available() {
        println!("  Memory collection not available on this platform");
        return Ok(BenchmarkResult {
            operation: "Memory Collection".to_string(),
            iterations: 0,
            total_time: Duration::from_secs(0),
            min_time: Duration::from_secs(0),
            max_time: Duration::from_secs(0),
            avg_time: Duration::from_secs(0),
            throughput_mb_per_sec: None,
        });
    }
    
    let memory_dir = test_dir.join("memory_output");
    fs::create_dir_all(&memory_dir)?;
    
    // Create memory collector
    let collector = MemoryCollector::new(
        MemoryCollectionOptions::default(),
        ProcessFilter::new(
            vec![],
            vec![std::process::id()],
            false
        ),
        MemoryRegionFilter::new(
            vec![],
            0,
            u64::MAX
        ),
    )?;
    
    // Get current process info
    let processes = rt.block_on(collect_processes())?;
    let current_process = processes.iter()
        .find(|p| p.pid == std::process::id());
    
    if let Some(process) = current_process {
        let start = Instant::now();
        let summary = collector.collect_all(&[process.clone()], &memory_dir)?;
        let duration = start.elapsed();
        
        let total_size = summary.total_memory_collected;
        
        let throughput = (total_size as f64 / 1024.0 / 1024.0) / duration.as_secs_f64();
        
        println!("  Collected {} MB in {:?}", total_size / 1024 / 1024, duration);
        
        Ok(BenchmarkResult {
            operation: "Memory Collection (Current Process)".to_string(),
            iterations: 1,
            total_time: duration,
            min_time: duration,
            max_time: duration,
            avg_time: duration,
            throughput_mb_per_sec: Some(throughput),
        })
    } else {
        Ok(BenchmarkResult {
            operation: "Memory Collection".to_string(),
            iterations: 0,
            total_time: Duration::from_secs(0),
            min_time: Duration::from_secs(0),
            max_time: Duration::from_secs(0),
            avg_time: Duration::from_secs(0),
            throughput_mb_per_sec: None,
        })
    }
}

#[cfg(not(feature = "memory_collection"))]
fn benchmark_memory_collection(_test_dir: &Path, _rt: &Runtime) -> Result<BenchmarkResult> {
    println!("\n📊 Memory Collection benchmark skipped (feature not enabled)");
    Ok(BenchmarkResult {
        operation: "Memory Collection".to_string(),
        iterations: 0,
        total_time: Duration::from_secs(0),
        min_time: Duration::from_secs(0),
        max_time: Duration::from_secs(0),
        avg_time: Duration::from_secs(0),
        throughput_mb_per_sec: None,
    })
}