# Performance Guide for rs-collector

## Overview

This document describes the performance characteristics of rs-collector and provides guidance for optimizing collection speed and resource usage.

## Benchmarks

### Running Benchmarks

To run all performance benchmarks:

```bash
cargo bench
```

To run specific benchmarks:

```bash
# Hash calculation benchmarks
cargo bench --bench hash_bench

# Compression benchmarks
cargo bench --bench compression_bench

# Collection benchmarks
cargo bench --bench collector_bench

# Path validation benchmarks
cargo bench --bench path_validation_bench

# Bodyfile generation benchmarks
cargo bench --bench bodyfile_bench
```

### Benchmark Results

Benchmarks generate HTML reports in `target/criterion/` that show:
- Performance over time
- Statistical analysis
- Comparison between runs

## Performance Characteristics

### 1. Hash Calculation

**SHA256 Performance**:
- Small files (< 1MB): ~500 MB/s
- Medium files (1-10MB): ~450 MB/s
- Large files (> 10MB): ~400 MB/s

**Optimization Tips**:
- Use `--no-hash` flag to skip hash calculation for faster collection
- Large files (> 100MB) automatically skip hashing unless forced

### 2. Compression

**ZIP Creation Performance**:
- Text files: 50-200 MB/s (high compression ratio)
- Binary files: 300-500 MB/s (low compression ratio)
- Already compressed: 500+ MB/s (stored without compression)

**Optimization Tips**:
- Use `--no-compress` for already compressed data
- Adjust compression level with `--compression-level` (0-9)

### 3. Artifact Collection

**Collection Speed**:
- Small files: 10,000+ files/second
- Large files: Limited by I/O speed (typically 100-500 MB/s)
- Network files: Limited by network bandwidth

**Parallel Collection**:
- Default: Uses all available CPU cores
- Adjustable with `--threads` parameter

### 4. Memory Usage

**Typical Memory Usage**:
- Base: ~50MB
- Per artifact: ~1KB metadata
- Buffer usage: 8MB per concurrent operation
- Peak during compression: ~100MB + file size

**Memory Optimization**:
- Use streaming for large files
- Limit concurrent operations with `--max-concurrent`
- Use `--low-memory` mode for constrained environments

## Performance Tuning

### 1. CPU Optimization

```bash
# Use all available cores (default)
rs-collector -o output/

# Limit to 4 threads
rs-collector -o output/ --threads 4

# Single-threaded operation
rs-collector -o output/ --threads 1
```

### 2. I/O Optimization

```bash
# Increase buffer size for large files
rs-collector -o output/ --buffer-size 16

# Use direct I/O for better performance
rs-collector -o output/ --direct-io

# Skip file system cache
rs-collector -o output/ --no-cache
```

### 3. Network Optimization

```bash
# S3 upload with larger chunks
rs-collector --s3-bucket mybucket --s3-chunk-size 16

# SFTP with compression
rs-collector --sftp-host server --sftp-compress

# Parallel uploads
rs-collector --upload-threads 8
```

### 4. Memory Optimization

```bash
# Low memory mode
rs-collector -o output/ --low-memory

# Limit cache size
rs-collector -o output/ --cache-size 100

# Stream large files
rs-collector -o output/ --stream-threshold 50
```

## Platform-Specific Performance

### Windows

**Best Performance**:
- Run from SSD
- Disable Windows Defender real-time scanning for output directory
- Use VSS for locked files

**Common Bottlenecks**:
- Antivirus scanning
- File system filters
- Network shares

### Linux

**Best Performance**:
- Use ext4 or XFS file systems
- Mount with `noatime` option
- Increase file descriptor limits

**Common Bottlenecks**:
- SELinux policies
- Audit subsystem
- Network file systems (NFS)

### macOS

**Best Performance**:
- Grant full disk access
- Use APFS file system
- Disable Spotlight for output directory

**Common Bottlenecks**:
- System Integrity Protection (SIP)
- Time Machine backups
- FileVault encryption

## Profiling

### CPU Profiling

```bash
# Profile with perf (Linux)
perf record -g rs-collector -o output/
perf report

# Profile with Instruments (macOS)
instruments -t "Time Profiler" rs-collector -o output/
```

### Memory Profiling

```bash
# Use Valgrind (Linux)
valgrind --tool=massif rs-collector -o output/

# Use heaptrack (Linux)
heaptrack rs-collector -o output/
```

### I/O Profiling

```bash
# Use iotop (Linux)
sudo iotop -p $(pgrep rs-collector)

# Use fs_usage (macOS)
sudo fs_usage -w rs-collector
```

## Optimization Strategies

### 1. For Speed

```yaml
# Fast collection configuration
performance:
  threads: 0  # Use all cores
  buffer_size_mb: 16
  compression_level: 1
  skip_hash: true
  parallel_uploads: true
```

### 2. For Low Resource Usage

```yaml
# Low resource configuration
performance:
  threads: 2
  buffer_size_mb: 4
  compression_level: 6
  stream_large_files: true
  max_memory_mb: 256
```

### 3. For Network Collection

```yaml
# Network optimized configuration
performance:
  compression_level: 9
  chunk_size_mb: 8
  retry_count: 5
  connection_timeout: 60
  parallel_connections: 4
```

## Performance Metrics

### Expected Performance

| Operation | Small Files | Large Files | Network |
|-----------|-------------|-------------|---------|
| Read | 50k files/sec | 500 MB/s | N/A |
| Hash | 10k files/sec | 400 MB/s | N/A |
| Compress | 5k files/sec | 200 MB/s | N/A |
| Upload S3 | N/A | 100 MB/s | 50 MB/s |
| Upload SFTP | N/A | 50 MB/s | 25 MB/s |

### Resource Usage

| Resource | Idle | Active | Peak |
|----------|------|--------|------|
| CPU | 1% | 50-100% | 100% |
| Memory | 50MB | 200MB | 1GB |
| Disk I/O | 0 | 100-500 MB/s | 1GB/s |
| Network | 0 | 10-100 MB/s | 1Gb/s |

## Troubleshooting Performance Issues

### Slow Collection

1. Check disk I/O: `iostat -x 1`
2. Check CPU usage: `top` or `htop`
3. Check memory: `free -h` or `vm_stat`
4. Disable unnecessary features:
   - Skip hashing: `--no-hash`
   - Skip compression: `--no-compress`
   - Reduce threads: `--threads 2`

### High Memory Usage

1. Enable streaming: `--stream-large-files`
2. Reduce buffers: `--buffer-size 4`
3. Limit concurrent operations: `--max-concurrent 2`
4. Use low memory mode: `--low-memory`

### Network Bottlenecks

1. Enable compression: `--compress-uploads`
2. Increase chunk size: `--chunk-size 16`
3. Use parallel uploads: `--parallel-uploads 4`
4. Check bandwidth: `speedtest-cli`

## Conclusion

rs-collector is designed to balance performance with reliability. The default settings work well for most scenarios, but the extensive configuration options allow optimization for specific use cases. Always test performance settings in your environment before production use.