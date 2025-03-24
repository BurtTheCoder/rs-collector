# Rust Collector Implementation Details

## Configuration System

The Rust Collector now features a flexible configuration system with two deployment options:

1. **Runtime Configuration**: Load YAML configuration at execution time
2. **Compile-time Embedding**: Bake configuration into binary during build

This approach provides maximum flexibility for different incident response scenarios, especially in environments where deploying separate files might be challenging.

## Configuration Format

The YAML configuration file defines:

- Global options like file size limits and compression settings
- Individual artifacts to collect, each with:
  - Name and type (MFT, Registry, EventLog, Custom)
  - Source path with environment variable support (%USERPROFILE%)
  - Destination name and other metadata
  - Required flag to distinguish critical vs. optional artifacts

Example:
```yaml
version: "1.0"
description: "DFIR triage configuration"
global_options:
  skip_locked_files: "true"
  
artifacts:
  - name: "MFT"
    artifact_type: MFT
    source_path: "\\\\?\\C:\\$MFT"
    destination_name: "MFT"
    description: "Master File Table"
    required: true
    metadata:
      category: "filesystem"
      priority: "high"
```

## Collector Architecture

The collection system is now fully modular:

1. **Configuration Loading**: Load from file or embedded config
   - Environment variable expansion in paths
   - Default configuration if none provided
   - Filter by artifact types if requested

2. **Collection Process**:
   - Volatile data collection (system state, processes, network, memory, disks)
   - File-based artifact collection using appropriate collectors
   - Success/failure tracking with detailed logging
   - Directory structure creation based on artifact types

3. **Standalone Binary Creation**:
   - Build script generation with embedded configuration
   - Simplified deployment with no external dependencies

## Volatile Data Collection

The volatile data collection module captures the system state at runtime:

1. **Implementation**:
   - Uses the `sysinfo` crate for cross-platform system information gathering
   - Collects data in a structured format using custom data models
   - Handles platform-specific differences transparently

2. **Data Categories**:
   - **System Information**: Hostname, OS details, CPU information
   - **Process Information**: Running processes with command lines, resource usage, parent-child relationships
   - **Network Information**: Interface details and traffic statistics
   - **Memory Information**: System memory usage and swap statistics
   - **Disk Information**: Mounted disks with capacity and usage information

3. **Integration**:
   - Runs before file-based artifact collection to capture system state
   - Stores data in JSON format for easy analysis
   - Includes summary in the collection summary file
   - Can be disabled with the `--no-volatile-data` flag

## Deployment Options

### Option 1: Runtime Configuration
- Deploy executable + configuration file
- Great for flexible environments where configuration might change
- Example: `./rust_collector -c my_config.yaml`

### Option 2: Embedded Configuration
- Generate a standalone binary with baked-in configuration
- Ideal for incident response where deploying multiple files is challenging
- Example: 
  1. `./rust_collector build -c my_config.yaml -n "ir_collector"`
  2. Deploy `ir_collector` to target system
  3. Run with just `./ir_collector`

## Memory Collection System

The memory collection system has been implemented with a unified approach using MemProcFS across all supported platforms:

### Architecture

1. **Unified Implementation**:
   - Uses MemProcFS for consistent memory access across Windows, Linux, and macOS
   - Platform-specific initialization handled transparently
   - Automatic fallback to legacy platform-specific implementations if MemProcFS is unavailable

2. **Core Components**:
   - `MemProcFSCollector`: Main implementation that provides cross-platform memory collection
   - Platform-specific initializers for Windows, Linux, and macOS
   - Helper functions for common operations like memory region classification

3. **Advanced Capabilities**:
   - Memory pattern searching with support for hex patterns
   - YARA rule scanning for malware detection (with the `yara` feature)
   - Targeted memory region dumping for detailed analysis
   - Efficient handling of large memory regions through chunking

4. **Integration with Volatile Data**:
   - Uses process information from volatile data collection
   - Provides detailed memory maps and module information
   - Exports memory dumps in a structured format

### Implementation Details

1. **Memory Region Classification**:
   - Stack regions
   - Heap regions
   - Code regions
   - Mapped file regions
   - Other regions

2. **Memory Access Optimization**:
   - Chunked memory reading for large regions
   - Caching of process handles
   - Efficient error handling with partial success support

3. **Cross-Platform Consistency**:
   - Same memory region model across all platforms
   - Consistent memory protection flags
   - Unified module information format

## Future Improvements

Potential enhancements:

1. **Collection Plugins**: Dynamic loading of collection modules
2. **Remote Configuration**: Pull configuration from a remote source
3. **Artifact Prioritization**: Collect most critical artifacts first
4. **Real-time Analysis**: Preliminary analysis during collection
5. **Remote Command & Control**: Receive remote instructions during collection
