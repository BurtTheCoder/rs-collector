# rs-collector API Documentation

## Overview

rs-collector is a high-performance, cross-platform Digital Forensics and Incident Response (DFIR) artifact collector written in Rust. This document provides comprehensive API documentation for developers integrating or extending rs-collector.

## Building Documentation

To generate the full API documentation with all details:

```bash
# Generate documentation
cargo doc --all-features --no-deps

# Open in browser
cargo doc --all-features --no-deps --open
```

## Core Components

### 1. Configuration API

The configuration system allows defining what artifacts to collect:

```rust
use rust_collector::config::{CollectionConfig, Artifact, ArtifactType};

// Load configuration from file
let config = CollectionConfig::from_yaml_file(Path::new("config.yaml"))?;

// Or create programmatically
let artifact = Artifact {
    name: "System Logs".to_string(),
    artifact_type: ArtifactType::Linux(LinuxArtifactType::SysLogs),
    source_path: "/var/log/syslog".to_string(),
    destination_name: "syslog".to_string(),
    description: Some("System log file".to_string()),
    required: true,
    metadata: HashMap::new(),
    regex: None,
};
```

### 2. Collector API

The main collector trait and implementations:

```rust
use rust_collector::collectors::collector::{ArtifactCollector, collect_artifacts_parallel};

// Synchronous collection
let results = collect_artifacts(&artifacts, output_dir)?;

// Asynchronous parallel collection
let results = collect_artifacts_parallel(&artifacts, output_dir).await?;
```

### 3. Streaming Upload API

For direct cloud uploads without local storage:

#### S3 Streaming

```rust
use rust_collector::cloud::s3::{S3Config, create_s3_client};
use rust_collector::cloud::streaming::S3UploadStream;

let config = S3Config {
    bucket: "my-bucket".to_string(),
    region: "us-east-1".to_string(),
    access_key_id: "KEY".to_string(),
    secret_access_key: "SECRET".to_string(),
    buffer_size_mb: 5,
};

let client = create_s3_client(&config)?;
let mut stream = S3UploadStream::new(
    client,
    &config.bucket,
    "output.zip",
    config.buffer_size_mb
).await?;

// Use stream with AsyncWrite trait
stream.write_all(data).await?;
stream.complete().await?;
```

#### SFTP Streaming

```rust
use rust_collector::cloud::sftp::SFTPConfig;
use rust_collector::cloud::sftp_streaming::SFTPUploadStream;

let config = SFTPConfig {
    host: "server.example.com".to_string(),
    port: 22,
    username: "user".to_string(),
    private_key_path: PathBuf::from("/home/user/.ssh/id_rsa"),
    remote_base_path: "/uploads/".to_string(),
    connection_timeout_sec: 30,
    concurrent_connections: 4,
};

let mut stream = SFTPUploadStream::new(
    config,
    "/uploads/output.zip",
    5 // buffer size in MB
).await?;
```

### 4. Memory Collection API

Process memory collection with search capabilities:

```rust
use rust_collector::collectors::memory::MemoryCollector;
use rust_collector::collectors::memory::models::{ProcessFilter, MemoryRegionFilter};

let collector = MemoryCollector::new()?;

// List processes
let processes = collector.list_processes()?;

// Filter processes
let filter = ProcessFilter {
    pids: Some(vec![1234]),
    names: Some(vec!["chrome.exe".to_string()]),
    exclude_pids: None,
    exclude_names: None,
};
let filtered = collector.filter_processes(&processes, &filter);

// Collect memory
let results = collector.collect_from_processes(
    &filtered,
    output_dir,
    &MemoryRegionFilter::default()
).await?;
```

### 5. Volatile Data Collection API

Collect runtime system information:

```rust
use rust_collector::collectors::volatile::{
    collect_volatile_data,
    collect_processes,
    collect_network_connections,
    collect_system_info
};

// Collect all volatile data
let volatile_data = collect_volatile_data().await?;

// Or collect individually
let processes = collect_processes()?;
let connections = collect_network_connections()?;
let system_info = collect_system_info()?;
```

### 6. Utility APIs

#### Compression

```rust
use rust_collector::utils::compress::{compress_artifacts, create_streaming_zip};

// Standard compression
let zip_path = compress_artifacts(source_dir, hostname, timestamp)?;

// Streaming compression
let writer = create_streaming_zip(output_stream).await?;
```

#### Hashing

```rust
use rust_collector::utils::hash::calculate_sha256;

// Calculate SHA-256 with size limit
let hash = calculate_sha256(file_path, max_size_bytes)?;
```

#### Bodyfile Timeline

```rust
use rust_collector::utils::bodyfile::generate_bodyfile;

// Generate timeline in Sleuthkit bodyfile format
let entry_count = generate_bodyfile(root_path, output_path).await?;
```

### 7. Security APIs

#### Path Validation

```rust
use rust_collector::security::path_validator::{validate_path, sanitize_filename};

// Validate paths to prevent directory traversal
let safe_path = validate_path(user_input, base_dir)?;

// Sanitize filenames
let safe_name = sanitize_filename(user_filename);
```

#### Credential Scrubbing

```rust
use rust_collector::security::credential_scrubber::{scrub_credentials, scrub_path};

// Remove sensitive data from strings
let clean_text = scrub_credentials(potentially_sensitive_text);

// Scrub file paths
let clean_path = scrub_path(file_path);
```

## Error Handling

All API functions return `Result<T, anyhow::Error>` for consistent error handling:

```rust
use anyhow::{Result, Context};

fn collect_artifacts_with_context() -> Result<()> {
    let config = CollectionConfig::from_yaml_file(Path::new("config.yaml"))
        .context("Failed to load configuration")?;
    
    let results = collect_artifacts(&config.artifacts, output_dir)
        .context("Failed to collect artifacts")?;
    
    Ok(())
}
```

## Platform Support

The API automatically handles platform differences:

```rust
use rust_collector::collectors::platforms::{get_platform_collector, filter_artifacts_for_platform};

// Get platform-specific collector
let collector = get_platform_collector();

// Filter artifacts for current platform
let artifacts = filter_artifacts_for_platform(&all_artifacts);
```

## Feature Flags

Enable specific features in `Cargo.toml`:

```toml
[dependencies]
rust_collector = { version = "0.3.0", features = ["memory_collection", "yara"] }
```

Available features:
- `memory_collection`: Process memory collection support
- `yara`: YARA rule scanning in memory dumps
- `embed_config`: Embed default configurations

## Thread Safety

All collectors implement `Send + Sync` and are safe for concurrent use:

```rust
use std::sync::Arc;
use tokio::task;

let collector = Arc::new(get_platform_collector());

let handles: Vec<_> = artifacts.iter().map(|artifact| {
    let collector = Arc::clone(&collector);
    let artifact = artifact.clone();
    
    task::spawn(async move {
        collector.collect(&artifact, output_dir).await
    })
}).collect();

// Wait for all collections
for handle in handles {
    handle.await??;
}
```

## Best Practices

1. **Error Handling**: Always use `.context()` for meaningful error messages
2. **Resource Management**: Use streaming APIs for large files
3. **Concurrency**: Leverage parallel collection for better performance
4. **Security**: Always validate user input paths
5. **Progress Tracking**: Implement progress callbacks for long operations

## Example: Complete Collection Workflow

```rust
use rust_collector::{
    config::{CollectionConfig, load_or_create_config},
    collectors::collector::collect_artifacts_parallel,
    utils::compress::compress_artifacts,
    cloud::s3::{S3Config, upload_to_s3},
};

async fn forensic_collection() -> Result<()> {
    // 1. Load configuration
    let config = load_or_create_config(None)?;
    
    // 2. Collect artifacts
    let output_dir = Path::new("/tmp/collection");
    let results = collect_artifacts_parallel(&config.artifacts, output_dir).await?;
    
    // 3. Generate timeline
    let bodyfile_path = output_dir.join("timeline.bodyfile");
    generate_bodyfile(output_dir, &bodyfile_path).await?;
    
    // 4. Compress results
    let hostname = hostname::get()?.to_string_lossy().to_string();
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let zip_path = compress_artifacts(output_dir, &hostname, &timestamp)?;
    
    // 5. Upload to cloud (optional)
    if let Some(s3_config) = load_s3_config()? {
        upload_to_s3(&s3_config, &zip_path, "forensics/cases/").await?;
    }
    
    println!("Collection complete: {} artifacts collected", results.len());
    Ok(())
}
```

## Further Reading

- [GitHub Repository](https://github.com/yourusername/rs-collector)
- [User Guide](./README.md)
- [Contributing Guidelines](./CONTRIBUTING.md)

For detailed API documentation, run `cargo doc --open` to view the generated documentation in your browser.