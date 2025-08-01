# Rust Collector

[![CI](https://github.com/BurtTheCoder/rs-collector/actions/workflows/ci.yml/badge.svg)](https://github.com/BurtTheCoder/rs-collector/actions/workflows/ci.yml)
[![Security Audit](https://github.com/BurtTheCoder/rs-collector/actions/workflows/security.yml/badge.svg)](https://github.com/BurtTheCoder/rs-collector/actions/workflows/security.yml)
[![codecov](https://codecov.io/gh/BurtTheCoder/rs-collector/branch/main/graph/badge.svg)](https://codecov.io/gh/BurtTheCoder/rs-collector)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A cross-platform DFIR (Digital Forensics and Incident Response) triage collection tool written in Rust. This tool collects forensic artifacts from Windows, Linux, and macOS systems, enabling access to important system files even when locked (on Windows).

## Features

- Cross-platform support for Windows, Linux, and macOS
- Collect forensic artifacts even if they're locked by the operating system (Windows)
- Volatile data collection for capturing system state at runtime:
  - System information (hostname, OS version, CPU details)
  - Running processes with command lines and resource usage
  - Network interfaces and statistics
  - Memory usage information
  - Disk information and usage statistics
- Advanced memory analysis capabilities:
  - Cross-platform memory collection using MemProcFS
  - Memory pattern searching for forensic analysis
  - YARA rule scanning for malware detection
  - Targeted memory region dumping
- Configurable via YAML files or embedded configuration
- Regex pattern matching for flexible artifact collection
- Bodyfile generation for forensic timeline analysis (Linux and macOS)
- OS-specific artifact types:
  - **Windows**: MFT, Registry hives, Event logs, Prefetch files, USN Journal
  - **Linux**: System logs, Journal logs, Audit logs, Bash history, Package management logs
  - **macOS**: Unified logs, FSEvents, Quarantine database, Launch Agents/Daemons, Plists
- Variable expansion in paths (e.g., %USERPROFILE% on Windows, $HOME on Unix)
- Artifact metadata collection
- Artifact compression and S3 upload
- Standalone executable option with embedded configuration
- File system-based organization that mirrors the original directory structure

## Usage

### Basic Usage

```bash
# Run with default configuration for current OS
./rust_collector

# Run with a specific configuration file
./rust_collector -c my_config.yaml

# Collect only specific artifact types
./rust_collector -t "Registry,EventLog"  # Windows
./rust_collector -t "SysLogs,Bash"       # Linux
./rust_collector -t "UnifiedLogs,Plist"  # macOS

# Specify output directory
./rust_collector -o /path/to/output

# Upload to S3
./rust_collector -b my-bucket -p "incident-response"

# Upload to SFTP server
./rust_collector --sftp-host example.com --sftp-user username --sftp-key ~/.ssh/id_rsa --sftp-path "/uploads"

# Stream artifacts directly to S3 without local storage
./rust_collector -b my-bucket -p "incident-response" --stream

# Stream artifacts directly to SFTP without local storage
./rust_collector --sftp-host example.com --sftp-user username --sftp-key ~/.ssh/id_rsa --sftp-path "/uploads" --stream

# Customize buffer size for streaming (in MB)
./rust_collector -b my-bucket -p "incident-response" --stream --buffer-size 16
./rust_collector --sftp-host example.com --sftp-user username --sftp-key ~/.ssh/id_rsa --sftp-path "/uploads" --stream --buffer-size 16
```

### Configuration Management

```bash
# Create a default configuration file for current OS
./rust_collector init-config config.yaml

# Create an OS-specific configuration file
./rust_collector init-config --target-os windows windows_config.yaml
./rust_collector init-config --target-os linux linux_config.yaml
./rust_collector init-config --target-os macos macos_config.yaml

# Build a standalone binary with embedded configuration
./rust_collector build -c my_config.yaml -n "custom_collector"

# Build for a specific OS
./rust_collector build -c windows_config.yaml --target-os windows -n "windows_collector"
./rust_collector build -c linux_config.yaml --target-os linux -n "linux_collector"
./rust_collector build -c macos_config.yaml --target-os macos -n "macos_collector"
```

### Command-line Options

```
Options:
  -b, --bucket <BUCKET>              S3 bucket name for uploading artifacts
  -p, --prefix <PREFIX>              S3 prefix for uploading artifacts
      --region <REGION>              AWS region for S3 uploads
      --profile <PROFILE>            AWS profile to use for S3 uploads
      --encrypt                      Enable server-side encryption for S3 uploads
      --sftp-host <HOST>             SFTP server hostname for uploading artifacts
      --sftp-port <PORT>             SFTP server port (default: 22)
      --sftp-user <USER>             SFTP username for authentication
      --sftp-key <KEY>               Path to private key file for SFTP authentication
      --sftp-path <PATH>             Remote path on SFTP server for uploading artifacts
      --sftp-connections <NUM>       Number of concurrent connections for SFTP uploads (default: 4)
  -o, --output <OUTPUT>              Local output path
      --skip-upload                  Skip uploading to cloud storage (S3 or SFTP)
  -v, --verbose                      Verbose logging
  -c, --config <CONFIG>              Path to configuration YAML file
  -t, --artifact-types <TYPES>       Override default artifact types to collect
      --target-os <OS>               Target operating system (windows, linux, macos)
      --stream                       Stream artifacts directly to cloud storage without local storage
      --buffer-size <SIZE>           Buffer size for streaming operations (in MB, default: 8)
      --no-volatile-data             Skip volatile data collection
      --force                        Continue even without elevated privileges
      --dump-process-memory          Dump process memory for forensic analysis
      --process <NAMES>              Specific processes to dump memory from (comma-separated names)
      --pid <PIDS>                   Specific process IDs to dump memory from (comma-separated PIDs)
      --max-memory-size <SIZE>       Maximum total size for memory dumps (in MB, default: 4096)
      --include-system-processes     Include system processes in memory dump
      --memory-regions <TYPES>       Memory regions to dump (comma-separated: heap,stack,code,all)
      --memory-search <PATTERN>      Search for a pattern in process memory (hex format)
      --memory-yara <RULE>           Scan process memory with YARA rules
      --dump-memory-region <SPEC>    Dump specific memory region (format: pid:address:size)
  -h, --help                         Print help
```

## Output Structure

The collected artifacts are organized in a file system-based structure that mirrors the original directory structure of the target system. This makes it easier to understand the context of each artifact and navigate the collected data.

```
[hostname]-[timestamp].zip
├── collection_summary.json
├── [hostname].body              # Bodyfile for timeline analysis (if enabled)
└── fs/
    ├── Windows/
    │   ├── System32/
    │   │   ├── config/
    │   │   │   ├── SYSTEM
    │   │   │   ├── SOFTWARE
    │   │   │   └── ...
    │   │   └── winevt/
    │   │       └── Logs/
    │   │           ├── System.evtx
    │   │           └── ...
    │   └── ...
    ├── Users/
    │   └── [username]/
    │       └── ...
    ├── $MFT
    └── ...
```

For Linux systems:

```
[hostname]-[timestamp].zip
├── collection_summary.json
├── [hostname].body              # Bodyfile for timeline analysis (if enabled)
└── fs/
    ├── etc/
    │   ├── passwd
    │   ├── shadow
    │   └── ...
    ├── var/
    │   ├── log/
    │   │   ├── syslog
    │   │   ├── auth.log
    │   │   └── ...
    │   └── ...
    └── ...
```

For macOS systems:

```
[hostname]-[timestamp].zip
├── collection_summary.json
├── [hostname].body              # Bodyfile for timeline analysis (if enabled)
└── fs/
    ├── Library/
    │   ├── Logs/
    │   │   └── ...
    │   └── ...
    ├── Users/
    │   └── [username]/
    │       ├── Library/
    │       │   ├── Preferences/
    │       │   │   └── ...
    │       │   └── ...
    │       └── ...
    └── ...
```

The `collection_summary.json` file contains metadata about all collected artifacts, including original paths, timestamps, and file sizes.

## Configuration

The YAML configuration file allows you to define which artifacts to collect. The format varies slightly by OS:

### Windows Configuration Example

```yaml
version: "1.0"
description: "Windows DFIR triage configuration"
global_options:
  skip_locked_files: "true"
  max_file_size_mb: "2048"
  generate_bodyfile: "true"
  bodyfile_calculate_hash: "false"  # Optional, disabled by default
  bodyfile_hash_max_size_mb: "100"  # Skip files larger than this
  bodyfile_skip_paths: "C:\\Windows\\WinSxS"  # Paths to skip for hashing
  bodyfile_use_iso8601: "true"  # Use ISO 8601 timestamps instead of Unix epoch
  
artifacts:
  - name: "MFT"
    artifact_type:
      Windows: MFT
    source_path: "\\\\?\\C:\\$MFT"
    destination_name: "MFT"
    description: "Master File Table"
    required: true
    
  - name: "SYSTEM"
    artifact_type:
      Windows: Registry
    source_path: "\\\\?\\C:\\Windows\\System32\\config\\SYSTEM"
    destination_name: "SYSTEM"
    description: "System registry hive"
    required: true
```

### Linux Configuration Example

```yaml
version: "1.0"
description: "Linux DFIR triage configuration"
global_options:
  max_file_size_mb: "1024"
  generate_bodyfile: "true"
  bodyfile_calculate_hash: "false"  # Optional, disabled by default
  bodyfile_hash_max_size_mb: "100"  # Skip files larger than this
  bodyfile_skip_paths: "/proc,/sys,/dev"  # Paths to skip for hashing
  bodyfile_use_iso8601: "true"  # Use ISO 8601 timestamps instead of Unix epoch
  
artifacts:
  - name: "syslog"
    artifact_type:
      Linux: SysLogs
    source_path: "/var/log/syslog"
    destination_name: "syslog"
    description: "System logs"
    required: true
    
  - name: "auth.log"
    artifact_type:
      Linux: SysLogs
    source_path: "/var/log/auth.log"
    destination_name: "auth.log"
    description: "Authentication logs"
    required: true
```

### macOS Configuration Example

```yaml
version: "1.0"
description: "macOS DFIR triage configuration"
global_options:
  max_file_size_mb: "1024"
  generate_bodyfile: "true"
  bodyfile_calculate_hash: "false"  # Optional, disabled by default
  bodyfile_hash_max_size_mb: "100"  # Skip files larger than this
  bodyfile_skip_paths: "/System/Volumes/Data/.Spotlight-V100"  # Paths to skip for hashing
  bodyfile_use_iso8601: "true"  # Use ISO 8601 timestamps instead of Unix epoch
  
artifacts:
  - name: "system.log"
    artifact_type:
      MacOS: UnifiedLogs
    source_path: "/var/log/system.log"
    destination_name: "system.log"
    description: "System logs"
    required: true
    
  - name: "quarantine"
    artifact_type:
      MacOS: Quarantine
    source_path: "$HOME/Library/Preferences/com.apple.LaunchServices.QuarantineEventsV2"
    destination_name: "QuarantineEventsV2"
    description: "Quarantine database"
    required: false
```

### Regex-Based Artifact Collection

The Rust Collector supports regex-based pattern matching for artifact collection. This allows you to collect multiple files that match specific patterns, rather than having to specify each file individually:

```yaml
# Collect all log files, excluding compressed ones
- name: "All Log Files"
  artifact_type:
    FileSystem: Logs
  source_path: "/var/log"
  destination_name: "logs"
  description: "All system log files"
  required: false
  regex:
    enabled: true
    recursive: true
    include_pattern: ".*\\.log$"
    exclude_pattern: ".*\\.gz$"
    max_depth: 2
```

The regex configuration supports:
- `enabled`: Enable regex matching for this artifact
- `recursive`: Recursively search directories
- `include_pattern`: Regex pattern for files to include
- `exclude_pattern`: Regex pattern for files to exclude (optional)
- `max_depth`: Maximum directory depth for recursive searches (optional)

See the `config` directory and `examples/regex_config.yaml` for more configuration examples.

## Advanced Features

### Cloud Storage Streaming Upload

The Rust Collector supports two methods for uploading artifacts to cloud storage (S3 or SFTP):

1. **Standard Method**: Collect artifacts locally, compress into a ZIP file, then upload to cloud storage
2. **Streaming Method**: Stream artifacts directly to cloud storage without storing the complete ZIP file locally

The streaming method offers several advantages:

- **Reduced Disk Usage**: Minimizes local storage requirements by not keeping the complete ZIP archive
- **Faster Completion**: Starts uploading immediately as artifacts are collected
- **Memory Efficiency**: Processes data in streams rather than loading entire files
- **Network Optimization**: Uses configurable buffer sizes for optimal performance
- **Real-time Progress Reporting**: Displays percentage complete, transfer speed, and bytes transferred
- **Intelligent Compression**: Automatically selects optimal compression settings based on file type
- **Robust Error Handling**: Automatically cleans up resources on failed uploads

#### S3 Streaming

To stream artifacts directly to S3, add the `--stream` flag when specifying an S3 bucket:

```bash
./rust_collector -b my-bucket -p "incident-response" --stream
```

You can also customize the buffer size (default is 8MB) to optimize for your network conditions:

```bash
./rust_collector -b my-bucket -p "incident-response" --stream --buffer-size 16
```

If streaming fails for any reason, the collector will automatically abort the multipart upload to clean up S3 resources and fall back to the standard method.

#### SFTP Streaming

To stream artifacts directly to an SFTP server, add the `--stream` flag when specifying SFTP connection details:

```bash
./rust_collector --sftp-host example.com --sftp-user username --sftp-key ~/.ssh/id_rsa --sftp-path "/uploads" --stream
```

You can customize the buffer size and number of concurrent connections:

```bash
./rust_collector --sftp-host example.com --sftp-user username --sftp-key ~/.ssh/id_rsa --sftp-path "/uploads" --stream --buffer-size 16 --sftp-connections 8
```

The SFTP streaming implementation includes:
- **SSH Key Authentication**: Secure authentication using SSH private keys
- **Automatic Retry**: Exponential backoff retry mechanism for transient errors
- **Connection Pooling**: Multiple concurrent connections for improved performance
- **Progress Tracking**: Real-time upload progress reporting

During the upload process, you'll see progress updates like this:

```
[INFO] Upload progress: 25% (25600000/102400000 bytes, 5.32 MB/s)
[INFO] Upload progress: 50% (51200000/102400000 bytes, 5.45 MB/s)
[INFO] Upload progress: 75% (76800000/102400000 bytes, 5.51 MB/s)
[INFO] Upload progress: 99% (101376000/102400000 bytes, 5.48 MB/s)
[INFO] Upload completed: 102400000 bytes transferred
```

The collector automatically optimizes compression based on file type:
- Already compressed files (ZIP, JPG, MP4, etc.) use no additional compression
- Large files (>100MB) use faster compression to improve performance
- Regular files use standard deflate compression for better space efficiency

### Volatile Data Collection

The Rust Collector automatically captures volatile system data during the collection process. This provides a snapshot of the system's state at the time of collection, which can be crucial for incident response and forensic analysis.

The volatile data collection includes:

- **System Information**: Basic system details including hostname, OS version, kernel version, and CPU information
- **Running Processes**: Complete list of running processes with their command lines, resource usage, parent-child relationships, and execution paths
- **Memory Usage**: System memory statistics including total memory, used memory, and swap usage
- **Network Interfaces**: Network interface information with traffic statistics
- **Disk Information**: Details about mounted disks including capacity, free space, and filesystem type

The collected data is stored in JSON format in the `volatile` directory within the artifact collection:

```
volatile/
├── system-info.json     # Basic system information
├── processes.json       # Running processes with details
├── network-connections.json  # Network interfaces and statistics
├── memory.json          # Memory usage information
├── disks.json           # Disk information and usage
```

This data is also summarized in the `collection_summary.json` file for easy reference.

#### Disabling Volatile Data Collection

In some environments, you may want to skip volatile data collection. You can do this by using the `--no-volatile-data` flag:

```bash
./rust_collector --no-volatile-data
```

This is useful in scenarios where:
- You're only interested in static artifacts
- You're running in a resource-constrained environment
- You want to minimize the collection's impact on the system
- You're collecting from a system where process enumeration might be problematic

### Memory Analysis

The Rust Collector provides advanced memory analysis capabilities across all supported platforms (Windows, Linux, and macOS) using a unified MemProcFS-based implementation:

#### Process Memory Collection

Collect memory from running processes with fine-grained control:

```bash
# Dump memory from all processes
./rust_collector --dump-process-memory

# Dump memory from specific processes by name
./rust_collector --dump-process-memory --process "chrome,firefox"

# Dump memory from specific processes by PID
./rust_collector --dump-process-memory --pid "1234,5678"

# Dump only specific memory region types
./rust_collector --dump-process-memory --memory-regions "heap,stack"

# Limit total memory collection size (in MB)
./rust_collector --dump-process-memory --max-memory-size 2048

# Include system processes in memory dump
./rust_collector --dump-process-memory --include-system-processes
```

#### Memory Pattern Searching

Search process memory for specific byte patterns (useful for finding credentials, encryption keys, or malware signatures):

```bash
# Search for a hex pattern in all processes
./rust_collector --memory-search "4D5A90"  # Search for MZ header

# Search in specific processes
./rust_collector --memory-search "70617373776F7264" --process "chrome,firefox"  # "password" in hex

# Search in specific processes by PID
./rust_collector --memory-search "4D5A90" --pid "1234,5678"
```

The search results are saved to `memory_search_results.json` in the output directory.

#### YARA Scanning

Scan process memory with YARA rules to detect malware or other patterns of interest:

```bash
# Scan with an inline YARA rule
./rust_collector --memory-yara "rule test { strings: $a = \"password\" condition: $a }"

# Scan with a YARA rule file
./rust_collector --memory-yara "/path/to/rules.yar"

# Scan specific processes
./rust_collector --memory-yara "/path/to/rules.yar" --process "chrome,firefox"
```

The YARA scan results are saved to `memory_yara_results.json` in the output directory.

#### Memory Region Dumping

Dump specific memory regions for detailed analysis:

```bash
# Dump a specific memory region (format: pid:address:size)
./rust_collector --dump-memory-region "1234:0x400000:4096"
```

The memory dump is saved as a binary file in the output directory.

#### Implementation Details

- **Cross-Platform**: Uses MemProcFS for consistent memory access across Windows, Linux, and macOS
- **Fallback Mechanism**: Automatically falls back to platform-specific implementations if MemProcFS is unavailable
- **Efficient Memory Handling**: Uses chunking for large memory regions to avoid allocation issues
- **Advanced Memory Analysis**: Provides detailed information about memory regions, modules, and memory contents

### Bodyfile Generation

The Rust Collector can generate bodyfiles for forensic timeline analysis on Linux and macOS systems (Windows support is planned for a future release):

- **Location**: The bodyfile is saved in the root of the artifact collection directory with the hostname as the filename (`[hostname].body`)
- **Format**: Standard TSK-compatible bodyfile format with SHA-256 hashing support
- **Timestamps**: ISO 8601 formatted timestamps for better readability
- **Performance**: Multi-threaded processing with Rayon for faster generation
- **Configuration Options**:
  - `generate_bodyfile`: Enable/disable bodyfile generation (default: true)
  - `bodyfile_calculate_hash`: Enable/disable SHA-256 hashing (default: false)
  - `bodyfile_hash_max_size_mb`: Maximum file size to hash (default: 100MB)
  - `bodyfile_skip_paths`: Comma-separated paths to skip during processing
  - `bodyfile_use_iso8601`: Use ISO 8601 timestamps (default: true)

The bodyfile can be used with tools like mactime for timeline analysis, helping investigators understand the sequence of events during an incident.

## Building from Source

### Basic Build

```bash
# Standard build
cargo build --release

# Build with embedded configuration
cargo build --release --features="embed_config"

# Build for specific target
cargo build --release --target x86_64-pc-windows-gnu
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target x86_64-apple-darwin
```

### Building with All Features

To build with all available features enabled:

```bash
cargo build --release --features="embed_config,memory_collection,yara"
```

This will include:
- `embed_config`: For embedding configuration into the binary
- `memory_collection`: For process memory collection and analysis
- `yara`: For YARA pattern scanning in memory

#### Dependencies for Full Build

Building with all features requires:

1. **YARA Libraries**: Required for the `yara` feature
   - **Ubuntu/Debian**: `apt install libyara-dev`
   - **macOS**: `brew install yara`
   - **Windows**: Download from [VirusTotal/yara GitHub](https://github.com/VirusTotal/yara/releases)

2. **MemProcFS Requirements**: For memory collection across platforms
   - Required for all operating systems when using `memory_collection`
   - See [MemProcFS documentation](https://github.com/ufrisk/MemProcFS) for platform-specific setup

#### Feature Combinations

You can also build with specific feature combinations:

```bash
# Just memory collection without YARA
cargo build --release --features="memory_collection"

# Embedded config with memory collection
cargo build --release --features="embed_config,memory_collection"

# Use the full toolkit with embedded configuration
./rust_collector build -c my_config.yaml -n "full_featured_collector" --features="memory_collection,yara"
```

#### Notes

- The resulting binary will be larger than the standard build
- Memory collection features require elevated privileges at runtime
- YARA scanning has additional runtime dependencies

### Enhanced Build System

RS-Collector includes an enhanced build system that automatically handles OS-specific configurations:

#### OS-Specific Configuration Handling

When building with the `embed_config` feature, the build system:

1. Automatically detects the target OS (Windows, Linux, macOS)
2. Embeds the appropriate OS-specific configuration file
3. Provides fallback mechanisms if the OS-specific config isn't available

```bash
# Build with OS-specific config for the current platform
cargo build --release --features="embed_config"

# Cross-compile with OS-specific config
cargo build --release --features="embed_config" --target x86_64-pc-windows-gnu
```

#### Using the Build Command

The `build` command provides a convenient way to create standalone binaries with embedded configurations:

```bash
# Build for current OS with specified config
./rust_collector build -c my_config.yaml -n "custom_collector"

# Build for specific OS
./rust_collector build -c windows_config.yaml --target-os windows -n "windows_collector"
./rust_collector build -c linux_config.yaml --target-os linux -n "linux_collector"
./rust_collector build -c macos_config.yaml --target-os macos -n "macos_collector"
```

#### CI/CD Integration

The project includes a comprehensive GitHub Actions workflow for automated builds across all supported platforms:

```yaml
name: RS-Collector Build

jobs:
  build:
    strategy:
      matrix:
        include:
          # Standard builds for each platform
          - os: ubuntu
            arch: x86_64
            os_normalized: linux
            target: x86_64-unknown-linux-gnu
            
          - os: macos
            arch: x86_64
            os_normalized: macos
            target: x86_64-apple-darwin
            
          - os: windows
            arch: x86_64
            os_normalized: windows
            target: x86_64-pc-windows-msvc
            
          # ARM64 builds
          - os: ubuntu
            arch: arm64
            target: aarch64-unknown-linux-gnu
            
          - os: macos
            arch: arm64
            target: aarch64-apple-darwin
            
          # Builds with embedded configs
          - os: ubuntu
            features: "embed_config"
            use_config_embedding: true
            
          # Builds using the build command
          - os: ubuntu
            use_build_command: true
```

The workflow handles:

1. **Multiple Architectures**: x86_64 and ARM64 builds
2. **Feature Combinations**: Standard, memory collection, YARA scanning
3. **OS-Specific Configurations**: Automatically embeds the correct config for each OS
4. **Cross-Compilation**: Builds for different target platforms
5. **Artifact Management**: Uploads build artifacts for each configuration

This ensures consistent builds across all platforms with proper OS-specific configuration handling.

## Release Process

Releases are automated through GitHub Actions. The release workflow automatically builds binaries for all supported platforms and creates a GitHub release with checksums.

### Creating a Release

1. **Update Version**
   - Update version in `Cargo.toml`
   - Update `CHANGELOG.md` with release notes

2. **Create and Push Tag**
   ```bash
   git tag v1.2.3
   git push origin v1.2.3
   ```

3. **Automated Release**
   The release workflow will automatically:
   - Build binaries for all platforms:
     - Linux: x86_64, aarch64
     - Windows: x86_64, aarch64
     - macOS: x86_64, aarch64
   - Create stripped, optimized release binaries
   - Generate SHA256 checksums for all artifacts
   - Create a GitHub release with all artifacts
   - Provide a combined checksums.txt file

### Manual Release Trigger

You can also trigger a release manually from GitHub Actions:
1. Go to Actions → Release workflow
2. Click "Run workflow"
3. Enter the version tag (e.g., v1.2.3)

### Release Artifacts

Each release includes:
- Platform-specific binaries (e.g., `rs-collector-linux-amd64`)
- Compressed archives (.tar.gz for Unix, .zip for Windows)
- SHA256 checksums for verification
- A special Linux build with all features enabled (`rs-collector-linux-amd64-full`)

### Verifying Downloads

Always verify the checksum of downloaded binaries:
```bash
# Download the binary and checksum
wget https://github.com/BurtTheCoder/rs-collector/releases/download/v1.2.3/rs-collector-linux-amd64
wget https://github.com/BurtTheCoder/rs-collector/releases/download/v1.2.3/checksums.txt

# Verify checksum
sha256sum -c checksums.txt --ignore-missing
```

## Deployment Options

1. **Runtime Configuration**: Deploy the executable and a configuration file
2. **Embedded Configuration**: Create a standalone binary with embedded configuration

The latter is useful for incident response where deploying multiple files may be challenging.

## Platform-Specific Notes

### Windows
- Requires Administrator privileges for accessing locked files
- Uses Windows Backup API for raw file access
- Supports Windows 7/Server 2008 R2 or newer

### Linux
- Requires root privileges for accessing most system files
- Supports most modern Linux distributions

### macOS
- Requires root privileges for accessing system files
- Supports macOS 10.15 (Catalina) or newer

## License

[MIT License](LICENSE)
