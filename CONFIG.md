# Rust Collector Configuration Guide

## Configuration File Format

The YAML configuration defines what artifacts to collect and how:

```yaml
version: "1.0"
description: "DFIR triage configuration"
global_options:
  skip_locked_files: "true"
  max_file_size_mb: "2048"
  compress_artifacts: "true"

artifacts:
  - name: "Example Artifact"
    artifact_type: 
      Windows: Registry  # OS-specific artifact type
    source_path: "\\\\?\\C:\\Windows\\System32\\config\\SYSTEM"
    destination_name: "SYSTEM"
    description: "System registry hive"
    required: true
    metadata:
      category: "registry"
      priority: "high"
  
  # More artifacts...
  
  # Regex-based artifact collection
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

## Regex-Based Artifact Collection

The Rust Collector supports regex-based pattern matching for artifact collection. This allows you to collect multiple files that match specific patterns, rather than having to specify each file individually.

### Regex Configuration

To use regex-based collection, add a `regex` section to your artifact configuration:

```yaml
regex:
  enabled: true                # Enable regex matching for this artifact
  recursive: true              # Recursively search directories
  include_pattern: ".*\\.log$" # Regex pattern for files to include
  exclude_pattern: ".*\\.gz$"  # Regex pattern for files to exclude (optional)
  max_depth: 2                 # Maximum directory depth for recursive searches (optional)
```

### Regex Pattern Syntax

The regex patterns use Rust's regex syntax, which is similar to Perl-compatible regular expressions (PCRE). Some common patterns:

- `.*\\.log$` - Match files ending with .log
- `.*\\.conf$` - Match files ending with .conf
- `.*/(access|error)\\.log$` - Match files named access.log or error.log
- `.*/.bash_history$` - Match .bash_history files in any directory

### Examples

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

# Collect all Windows event logs
- name: "Windows Event Logs"
  artifact_type:
    Windows: EventLog
  source_path: "C:\\Windows\\System32\\winevt\\Logs"
  destination_name: "EventLogs"
  description: "Windows event logs"
  required: false
  regex:
    enabled: true
    recursive: false
    include_pattern: ".*\\.evtx$"

# Collect all user bash histories
- name: "User Bash Histories"
  artifact_type:
    Linux: Bash
  source_path: "/home"
  destination_name: "bash_histories"
  description: "User bash command histories"
  required: false
  regex:
    enabled: true
    recursive: false
    include_pattern: ".*/.bash_history$"
```

See the `examples/regex_config.yaml` file for more examples.

## Artifact Types

The following artifact types are supported:

### Common Types
- `FileSystem`: File system artifacts
- `Logs`: Log files
- `UserData`: User-specific data
- `SystemInfo`: System information
- `Memory`: Memory dumps and related files
- `Network`: Network configuration and logs
- `Custom`: Any other files or artifacts

### Windows-Specific Types
- `Windows:MFT`: Master File Table
- `Windows:Registry`: Windows Registry hives
- `Windows:EventLog`: Windows Event Logs
- `Windows:Prefetch`: Prefetch files
- `Windows:USNJournal`: USN Journal
- `Windows:ShimCache`: Application Compatibility Cache
- `Windows:AmCache`: AmCache hive

### Linux-Specific Types
- `Linux:SysLogs`: System logs
- `Linux:Journal`: Systemd journal logs
- `Linux:Proc`: Proc filesystem entries
- `Linux:Audit`: Audit logs
- `Linux:Cron`: Cron jobs and schedules
- `Linux:Bash`: Bash history and configuration
- `Linux:Apt`: APT package manager logs
- `Linux:Dpkg`: DPKG package manager logs
- `Linux:Yum`: YUM package manager logs
- `Linux:Systemd`: Systemd configuration and units

### macOS-Specific Types
- `MacOS:UnifiedLogs`: Unified logging system
- `MacOS:Plist`: Property list files
- `MacOS:Spotlight`: Spotlight metadata
- `MacOS:FSEvents`: File system events
- `MacOS:Quarantine`: Quarantine database
- `MacOS:KnowledgeC`: User activity database
- `MacOS:LaunchAgents`: Launch agents
- `MacOS:LaunchDaemons`: Launch daemons

## Path Variables

You can use environment variables in paths, with different syntax depending on the OS:

### Windows Path Variables
```yaml
source_path: "\\\\?\\%USERPROFILE%\\NTUSER.DAT"
```

Common Windows variables:
- `%USERPROFILE%`: Current user's profile directory
- `%SYSTEMROOT%`: Windows system directory
- `%PROGRAMDATA%`: Program data directory
- `%WINDIR%`: Windows directory
- `%TEMP%`: Temporary directory

### Unix Path Variables (Linux and macOS)
```yaml
source_path: "$HOME/.bash_history"
```

Common Unix variables:
- `$HOME`: User's home directory
- `$USER`: Current username
- `$TMPDIR`: Temporary directory
- `$PATH`: System path

## OS-Specific Configuration Examples

### Windows Configuration

```yaml
version: "1.0"
description: "Windows DFIR triage configuration"
artifacts:
  - name: "SYSTEM"
    artifact_type:
      Windows: Registry
    source_path: "\\\\?\\C:\\Windows\\System32\\config\\SYSTEM"
    destination_name: "SYSTEM"
    description: "System registry hive"
    required: true
    
  - name: "Security.evtx"
    artifact_type:
      Windows: EventLog
    source_path: "\\\\?\\C:\\Windows\\System32\\winevt\\Logs\\Security.evtx"
    destination_name: "Security.evtx"
    description: "Security event log"
    required: true
```

### Linux Configuration

```yaml
version: "1.0"
description: "Linux DFIR triage configuration"
artifacts:
  - name: "auth.log"
    artifact_type:
      Linux: SysLogs
    source_path: "/var/log/auth.log"
    destination_name: "auth.log"
    description: "Authentication logs"
    required: true
    
  - name: "bash_history"
    artifact_type:
      Linux: Bash
    source_path: "$HOME/.bash_history"
    destination_name: "bash_history"
    description: "Bash command history"
    required: false
```

### macOS Configuration

```yaml
version: "1.0"
description: "macOS DFIR triage configuration"
artifacts:
  - name: "system.log"
    artifact_type:
      MacOS: UnifiedLogs
    source_path: "/var/log/system.log"
    destination_name: "system.log"
    description: "System logs"
    required: true
    
  - name: "launch_agents"
    artifact_type:
      MacOS: LaunchAgents
    source_path: "/Library/LaunchAgents"
    destination_name: "LaunchAgents"
    description: "System launch agents"
    required: false
```

## Deployment Options

### Runtime Configuration

Load configuration at runtime:

```bash
# Initialize a default config for your current OS
./rust_collector init-config my_config.yaml

# Initialize an OS-specific config
./rust_collector init-config --target-os windows windows_config.yaml
./rust_collector init-config --target-os linux linux_config.yaml
./rust_collector init-config --target-os macos macos_config.yaml

# Run with your custom config
./rust_collector -c my_config.yaml
```

### Embedded Configuration

Build a standalone binary with embedded configuration:

```bash
# Create a standalone binary for current OS
./rust_collector build -c my_config.yaml -n "incident_collector"

# Create a standalone binary for specific OS
./rust_collector build -c windows_config.yaml --target-os windows -n "windows_collector"
./rust_collector build -c linux_config.yaml --target-os linux -n "linux_collector"
./rust_collector build -c macos_config.yaml --target-os macos -n "macos_collector"
```

## Command Line Options

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
  -t, --artifact-types <TYPES>       Artifact types to collect (e.g., "Registry,EventLog")
      --target-os <OS>               Target operating system (windows, linux, macos)
      --stream                       Stream artifacts directly to cloud storage without local storage
      --buffer-size <SIZE>           Buffer size for streaming operations (in MB, default: 8)
      --no-volatile-data             Skip volatile data collection (running processes, network connections, etc.)
      --force                        Continue even without elevated privileges

  # Process Memory Collection Options
      --dump-process-memory          Dump process memory for forensic analysis
      --process <NAMES>              Specific processes to dump memory from (comma-separated names)
      --pid <PIDS>                   Specific process IDs to dump memory from (comma-separated PIDs)
      --max-memory-size <SIZE>       Maximum total size for memory dumps (in MB, default: 4096)
      --include-system-processes     Include system processes in memory dump
      --memory-regions <REGIONS>     Memory regions to dump (comma-separated: heap,stack,code,all)
```

## Subcommands

```
Subcommands:
  init-config    Create a default configuration file
  build          Build a standalone binary with embedded configuration
```

## Cloud Storage Configuration

### S3 Configuration

To upload artifacts to Amazon S3:

```bash
# Basic S3 upload
./rust_collector -b my-bucket -p "incident-response"

# With region and profile
./rust_collector -b my-bucket -p "incident-response" --region us-west-2 --profile incident-response

# With server-side encryption
./rust_collector -b my-bucket -p "incident-response" --encrypt

# Streaming upload
./rust_collector -b my-bucket -p "incident-response" --stream --buffer-size 16
```

### SFTP Configuration

To upload artifacts to an SFTP server:

```bash
# Basic SFTP upload
./rust_collector --sftp-host example.com --sftp-user username --sftp-key ~/.ssh/id_rsa --sftp-path "/uploads"

# With custom port
./rust_collector --sftp-host example.com --sftp-port 2222 --sftp-user username --sftp-key ~/.ssh/id_rsa --sftp-path "/uploads"

# With concurrent connections
./rust_collector --sftp-host example.com --sftp-user username --sftp-key ~/.ssh/id_rsa --sftp-path "/uploads" --sftp-connections 8

# Streaming upload
./rust_collector --sftp-host example.com --sftp-user username --sftp-key ~/.ssh/id_rsa --sftp-path "/uploads" --stream --buffer-size 16
```

## Volatile Data Collection

The Rust Collector automatically captures volatile system data during the collection process. This provides a snapshot of the system's state at the time of collection, which can be crucial for incident response and forensic analysis.

### Collected Volatile Data

The volatile data collection includes:

- **System Information**: Basic system details including hostname, OS version, kernel version, and CPU information
- **Running Processes**: Complete list of running processes with their command lines, resource usage, parent-child relationships, and execution paths
- **Memory Usage**: System memory statistics including total memory, used memory, and swap usage
- **Network Interfaces**: Network interface information with traffic statistics
- **Disk Information**: Details about mounted disks including capacity, free space, and filesystem type

### Output Format

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

### Disabling Volatile Data Collection

In some environments, you may want to skip volatile data collection. You can do this by using the `--no-volatile-data` flag:

```bash
./rust_collector --no-volatile-data
```

This is useful in scenarios where:
- You're only interested in static artifacts
- You're running in a resource-constrained environment
- You want to minimize the collection's impact on the system
- You're collecting from a system where process enumeration might be problematic

## Process Memory Collection

The Rust Collector can dump memory from running processes for forensic analysis. This feature allows you to capture the memory space of specific processes or all processes on the system, providing valuable evidence for malware analysis, incident response, and forensic investigations.

### Memory Collection Options

Process memory collection is controlled by the following command-line options:

```bash
# Dump memory from all processes (requires volatile data collection)
./rust_collector --dump-process-memory

# Dump memory from specific processes by name
./rust_collector --dump-process-memory --process "chrome,firefox,explorer"

# Dump memory from specific processes by PID
./rust_collector --dump-process-memory --pid "1234,5678"

# Include system processes (normally excluded)
./rust_collector --dump-process-memory --include-system-processes

# Limit the total memory collection size
./rust_collector --dump-process-memory --max-memory-size 2048

# Collect only specific memory region types
./rust_collector --dump-process-memory --memory-regions "heap,stack"
```

### Output Format

The collected memory is stored in the `process_memory` directory within the artifact collection:

```
process_memory/
├── chrome_1234/
│   ├── metadata.json           # Process metadata
│   ├── memory_map.txt          # Memory map showing all regions
│   ├── heap_00a10000_4096.dmp  # Heap memory region
│   ├── stack_7ff00000_8192.dmp # Stack memory region
│   └── code_00400000_65536.dmp # Code memory region
├── firefox_5678/
│   └── ...
└── memory_collection_summary.json  # Collection summary
```

Each process gets its own directory named `[process_name]_[pid]` containing:
- A metadata JSON file with process details
- A memory map text file showing all memory regions
- Individual dump files for each memory region that was collected

### Memory Region Types

The following memory region types can be collected:

- **heap**: Process heap memory (often contains valuable artifacts)
- **stack**: Process stack memory (call stacks, local variables)
- **code**: Executable code regions (useful for detecting code injection)
- **mapped**: Memory-mapped files
- **all**: All memory regions

### Platform-Specific Implementations

#### Windows Implementation

The Windows implementation uses the MemProcFS library to access process memory:

- **Memory Region Enumeration**: Uses MemProcFS's VAD (Virtual Address Descriptor) map to enumerate memory regions
- **Memory Reading**: Uses MemProcFS's memory reading capabilities to access process memory
- **Module Information**: Extracts loaded module information including base addresses and paths
- **Region Classification**: Accurately identifies heap, stack, and code regions based on VAD tags
- **Performance**: Optimized for large memory regions with efficient memory access

#### Linux Implementation

The Linux implementation uses the `/proc` filesystem to access process memory:

- **Memory Region Enumeration**: Parses `/proc/<pid>/maps` to enumerate memory regions
- **Memory Reading**: Reads directly from `/proc/<pid>/mem` to access process memory
- **Module Information**: Identifies loaded modules from memory maps
- **Region Classification**: Identifies region types based on permissions and path information
- **Performance**: Implements chunked reading for large memory regions to avoid allocation issues
- **Error Handling**: Robust error handling for permission issues and special memory regions

#### macOS Implementation

The macOS implementation uses the Mach kernel APIs to access process memory:

- **Memory Region Enumeration**: Uses `mach_vm_region` to enumerate memory regions
- **Memory Reading**: Uses `mach_vm_read_overwrite` to access process memory
- **Module Information**: Extracts loaded module information from the dyld shared cache
- **Region Classification**: Uses heuristics to identify heap, stack, and code regions
- **Task Port Caching**: Caches task ports for efficient repeated access to the same process

### Requirements

Process memory collection requires:

1. Elevated privileges (administrator/root)
2. Volatile data collection to be enabled (provides process list)
3. The `memory_collection` feature to be enabled at compile time
4. Platform-specific requirements:
   - Windows: Requires the MemProcFS library
   - Linux: Requires access to the `/proc` filesystem
   - macOS: Requires SIP to be disabled for accessing certain processes

### Compilation Options

The memory collection feature can be enabled with different compilation options:

```bash
# Enable all memory collection features
cargo build --features memory_collection

# Enable only Windows memory collection
cargo build --features windows_memory

# Enable only macOS memory collection
cargo build --features macos_memory
```

### Limitations

- Memory collection may be blocked by security software
- Some processes may have protected memory regions
- Collection size can be very large for systems with many processes
- Not all memory regions can be read, especially for system processes
- macOS System Integrity Protection (SIP) may prevent access to certain processes
- Windows may require special privileges for accessing protected processes

## Custom Artifact Examples

### Windows Custom Artifacts

```yaml
# USN Journal
- name: "UsnJrnl"
  artifact_type:
    Windows: USNJournal
  source_path: "\\\\?\\C:\\$Extend\\$UsnJrnl:$J"
  destination_name: "UsnJrnl"
  description: "USN Journal"
  required: false

# Prefetch files
- name: "Prefetch"
  artifact_type:
    Windows: Prefetch
  source_path: "\\\\?\\C:\\Windows\\Prefetch"
  destination_name: "Prefetch"
  description: "Prefetch files"
  required: false
```

### Linux Custom Artifacts

```yaml
# SSH Configuration
- name: "ssh_config"
  artifact_type:
    Linux: Custom
  source_path: "/etc/ssh/sshd_config"
  destination_name: "sshd_config"
  description: "SSH server configuration"
  required: false

# Web Server Logs
- name: "apache_logs"
  artifact_type:
    Linux: Custom
  source_path: "/var/log/apache2"
  destination_name: "apache_logs"
  description: "Apache web server logs"
  required: false
```

### macOS Custom Artifacts

```yaml
# Safari History
- name: "safari_history"
  artifact_type:
    MacOS: Custom
  source_path: "$HOME/Library/Safari/History.db"
  destination_name: "safari_history.db"
  description: "Safari browsing history"
  required: false

# User Keychain
- name: "user_keychain"
  artifact_type:
    MacOS: Custom
  source_path: "$HOME/Library/Keychains"
  destination_name: "user_keychains"
  description: "User keychain files"
  required: false
