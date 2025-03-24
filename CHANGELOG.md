# Changelog

## 0.3.0 (2025-03-23)

### Added
- Advanced memory analysis capabilities using MemProcFS
  - Cross-platform memory collection for Windows, Linux, and macOS
  - Memory pattern searching with `--memory-search` option
  - YARA rule scanning with `--memory-yara` option
  - Targeted memory region dumping with `--dump-memory-region` option
  - Efficient handling of large memory regions through chunking
- New command-line options for memory analysis:
  - `--dump-process-memory`: Dump process memory for forensic analysis
  - `--process`: Specify processes by name for memory operations
  - `--pid`: Specify processes by PID for memory operations
  - `--max-memory-size`: Limit total memory collection size
  - `--include-system-processes`: Include system processes in memory operations
  - `--memory-regions`: Specify memory region types to collect
- Automatic fallback to platform-specific implementations if MemProcFS is unavailable
- Comprehensive documentation for memory analysis capabilities

## 0.2.0 (2025-03-23)

### Added
- Volatile data collection for capturing system state at runtime
  - System information (hostname, OS version, CPU details)
  - Running processes with command lines and resource usage
  - Network interfaces and statistics
  - Memory usage information
  - Disk information and usage statistics
- New `--no-volatile-data` flag to disable volatile data collection when needed
- Integrated volatile data in collection summary for easier analysis
- SFTP support for secure artifact uploads
  - SSH key-based authentication
  - Configurable connection parameters
  - Concurrent connection support
- New `--stream` option to stream artifacts directly to cloud storage (S3 or SFTP) without local storage
- Configurable buffer size with `--buffer-size` option for streaming operations
- Real-time progress reporting with percentage complete, transfer speed, and bytes transferred
- Improved performance for large collections with streaming upload
- Automatic cleanup of cloud storage resources when uploads fail
- Automatic fallback to standard upload method if streaming fails

### Changed
- Refactored upload logic to support both streaming and standard methods
- Optimized compression settings for different file types:
  - No compression for already compressed files (ZIP, JPG, MP4, etc.)
  - Faster compression for large files (>100MB)
  - Standard deflate compression for regular files
- Enhanced error handling with exponential backoff retry mechanism
- Improved resource cleanup with proper abort handling for failed uploads
- Fixed CRC32 hasher finalization in ZIP writer to prevent partial moves

### Technical Improvements
- Implemented atomic counters for thread-safe progress tracking
- Added background task for non-blocking progress reporting
- Enhanced ZIP header handling with proper CRC32 calculation
- Improved error context for better debugging
- Implemented AsyncWrite trait for SFTP streaming

## 0.1.0 (Initial Release)

### Added
- Cross-platform support for Windows, Linux, and macOS
- Collection of forensic artifacts with OS-specific implementations
- Raw file access for locked files on Windows
- Configurable artifact collection via YAML files
- Bodyfile generation for forensic timeline analysis
- Artifact compression and S3 upload
- Variable expansion in paths
- Standalone executable option with embedded configuration
