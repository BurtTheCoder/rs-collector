# Memory Collection Implementation

This document provides detailed information about the memory collection system in the Rust Collector, which uses MemProcFS for cross-platform memory analysis.

## Overview

The memory collection system provides a unified approach to memory analysis across Windows, Linux, and macOS using the MemProcFS library. This enables consistent memory collection capabilities regardless of the underlying operating system, with platform-specific initialization handled transparently.

## Architecture

### Core Components

1. **MemProcFSCollector**
   - Main implementation class that provides cross-platform memory collection
   - Implements the `MemoryCollectorImpl` trait
   - Handles memory region enumeration, memory reading, module information, and advanced memory analysis

2. **Platform-Specific Initializers**
   - `windows.rs`: Windows-specific initialization using FPGA device
   - `linux.rs`: Linux-specific initialization using PROC device
   - `macos.rs`: macOS-specific initialization using PMEM device

3. **Helper Functions**
   - Library path detection
   - Memory region classification
   - Memory protection conversion
   - Memory dump formatting

### Directory Structure

```
src/collectors/memory/
├── collector.rs      # Main memory collector interface
├── export.rs         # Export functionality
├── filters.rs        # Memory region filtering
├── mod.rs            # Module definitions
├── models.rs         # Data models 
├── memprocfs/        # MemProcFS implementation
│   ├── mod.rs        # Module definitions
│   ├── collector.rs  # Core unified collector implementation
│   ├── helpers.rs    # Common helper functions
│   ├── windows.rs    # Windows-specific initialization
│   ├── linux.rs      # Linux-specific initialization
│   └── macos.rs      # macOS-specific initialization
└── platforms/        # Legacy platform implementations
```

## Implementation Details

### MemProcFS Integration

The MemProcFS library provides a unified API for memory analysis across different platforms. The integration works as follows:

1. **Library Detection**
   - Automatically detects the MemProcFS library (vmm.dll, vmm.so, or vmm.dylib)
   - Supports custom library paths via the `RUST_COLLECTOR_MEMPROCFS_PATH` environment variable
   - Checks common installation locations based on the platform

2. **Initialization**
   - Platform-specific initialization with appropriate device types
   - Windows: Uses FPGA device for live memory access
   - Linux: Uses PROC device for /proc-based memory access
   - macOS: Uses PMEM device for task-based memory access

3. **Process Handling**
   - Process enumeration via MemProcFS API
   - Process handle caching for improved performance
   - Robust error handling for process access failures

### Memory Region Handling

1. **Region Enumeration**
   - Uses MemProcFS VAD (Virtual Address Descriptor) map for consistent region information
   - Classifies regions into types (Stack, Heap, Code, MappedFile, Other)
   - Provides protection information (Read, Write, Execute)

2. **Memory Reading**
   - Direct memory access via MemProcFS API
   - Chunked reading for large memory regions to avoid allocation issues
   - Error handling with partial success support

3. **Module Information**
   - Retrieves loaded module information (DLLs, shared libraries)
   - Provides base address, size, path, name, and version information
   - Consistent module representation across platforms

### Advanced Capabilities

1. **Memory Pattern Searching**
   - Searches process memory for specific byte patterns
   - Supports hex patterns with wildcards
   - Returns addresses of matches for further analysis

2. **YARA Scanning**
   - Scans process memory with YARA rules
   - Supports both inline rules and rule files
   - Returns detailed match information including rule names and match locations

3. **Memory Region Dumping**
   - Dumps specific memory regions for detailed analysis
   - Formats memory dumps for readability
   - Supports binary output for external analysis

### Fallback Mechanism

The system includes a fallback mechanism to ensure compatibility:

1. **Primary Approach**: Try to use MemProcFS for memory collection
2. **Fallback**: If MemProcFS is unavailable, fall back to platform-specific implementations
3. **Error Handling**: Provide clear error messages if memory collection is not available

## Memory Collection Process

The memory collection process follows these steps:

1. **Process Selection**
   - Filter processes based on name, PID, or system process status
   - Apply user-defined filters to focus on specific processes

2. **Memory Region Enumeration**
   - Enumerate memory regions for each selected process
   - Apply region filters based on type, size, and protection

3. **Memory Reading**
   - Read memory from filtered regions
   - Handle errors and partial reads gracefully
   - Skip empty or inaccessible regions

4. **Export**
   - Export memory dumps to files
   - Create memory maps for analysis
   - Generate summary information

## Command-Line Interface

The memory collection system is exposed through several command-line options:

```
--dump-process-memory             Dump process memory for forensic analysis
--process <NAMES>                 Specific processes to dump memory from (comma-separated names)
--pid <PIDS>                      Specific process IDs to dump memory from (comma-separated PIDs)
--max-memory-size <SIZE>          Maximum total size for memory dumps (in MB)
--include-system-processes        Include system processes in memory dump
--memory-regions <TYPES>          Memory regions to dump (comma-separated: heap,stack,code,all)
--memory-search <PATTERN>         Search for a pattern in process memory (hex format)
--memory-yara <RULE>              Scan process memory with YARA rules
--dump-memory-region <SPEC>       Dump specific memory region (format: pid:address:size)
```

## Feature Flags

The memory collection system uses feature flags to control compilation:

- `memory_collection`: Enables the unified MemProcFS-based memory collection
- `yara`: Enables YARA scanning support (requires the `memory_collection` feature)

## Platform-Specific Considerations

### Windows

- Uses MemProcFS with FPGA device for live memory access
- Requires Administrator privileges for full access
- Supports Windows 7/Server 2008 R2 or newer

### Linux

- Uses MemProcFS with PROC device for /proc-based memory access
- Requires root privileges for full access
- Checks for /proc filesystem availability

### macOS

- Uses MemProcFS with PMEM device for task-based memory access
- Requires root privileges for task_for_pid access
- Checks for root privileges during initialization

## Performance Considerations

1. **Memory Usage**
   - Chunked memory reading to avoid large allocations
   - Process handle caching to reduce overhead
   - Efficient error handling to avoid unnecessary operations

2. **Speed Optimization**
   - Parallel processing where appropriate
   - Early filtering to reduce unnecessary memory reads
   - Efficient memory region classification

3. **Resource Limits**
   - Maximum total memory size limit
   - Maximum process memory size limit
   - Skip regions that are too small or too large

## Error Handling

The memory collection system includes robust error handling:

1. **Initialization Errors**
   - Library not found
   - Insufficient privileges
   - Unsupported platform

2. **Process Access Errors**
   - Process not found
   - Access denied
   - Process terminated during collection

3. **Memory Reading Errors**
   - Invalid memory address
   - Inaccessible memory region
   - Partial read success

4. **Export Errors**
   - File creation failure
   - Disk space issues
   - Permission problems

## Future Enhancements

Potential future enhancements to the memory collection system:

1. **Memory Analysis**
   - String extraction and analysis
   - Automatic detection of interesting memory regions
   - Integration with external analysis tools

2. **Performance Improvements**
   - More efficient memory region filtering
   - Better handling of very large processes
   - Improved caching mechanisms

3. **Additional Capabilities**
   - Memory diffing between snapshots
   - Process hollowing detection
   - Rootkit detection via memory analysis
