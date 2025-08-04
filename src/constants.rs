//! Global constants for the rs-collector application.
//!
//! This module centralizes all hardcoded values to improve maintainability
//! and make configuration changes easier.

// Memory and buffer size constants
/// Default buffer size for file operations (1MB)
pub const DEFAULT_BUFFER_SIZE: usize = 1024 * 1024;

/// Chunk size for compression operations (512KB)
pub const COMPRESSION_CHUNK_SIZE: usize = 512 * 1024;

/// Chunk size for memory reading operations (1MB)
pub const MEMORY_CHUNK_SIZE: usize = 1024 * 1024;

/// Small buffer size for streaming operations (64KB)
pub const STREAMING_BUFFER_SIZE: usize = 64 * 1024;

/// Maximum memory region size to collect (1GB)
pub const MAX_MEMORY_REGION_SIZE: u64 = 1024 * 1024 * 1024;

/// Default maximum total memory to collect (1GB)
pub const DEFAULT_MAX_TOTAL_MEMORY: u64 = 1024 * 1024 * 1024;

/// Default maximum memory per process (512MB)
pub const DEFAULT_MAX_PROCESS_MEMORY: u64 = 512 * 1024 * 1024;

// Cloud storage constants
/// S3 upload chunk size (8MB, S3 minimum is 5MB)
pub const S3_UPLOAD_CHUNK_SIZE: usize = 8 * 1024 * 1024;

/// S3 minimum part size for multipart uploads (5MB)
pub const S3_MIN_PART_SIZE: usize = 5 * 1024 * 1024;

/// S3 maximum parts per upload
pub const S3_MAX_PARTS: usize = 10000;

/// Large file threshold for multipart uploads (50MB)
pub const LARGE_FILE_THRESHOLD: u64 = 50 * 1024 * 1024;

/// Default SFTP port
pub const SFTP_DEFAULT_PORT: u16 = 22;

/// Default concurrent SFTP connections
pub const SFTP_DEFAULT_CONCURRENT_CONNECTIONS: usize = 4;

/// SFTP buffer size (8MB)
pub const SFTP_BUFFER_SIZE: usize = 8 * 1024 * 1024;

// Timeout and retry constants
/// Default connection timeout in seconds
pub const DEFAULT_CONNECTION_TIMEOUT_SECS: u64 = 30;

/// Maximum upload retry attempts
pub const MAX_UPLOAD_RETRIES: usize = 3;

/// Base retry delay in milliseconds
pub const RETRY_BASE_DELAY_MS: u64 = 250;

/// Maximum retry delay in seconds
pub const RETRY_MAX_DELAY_SECS: u64 = 30;

/// Progress reporting interval in seconds
pub const PROGRESS_REPORT_INTERVAL_SECS: u64 = 2;

/// Progress reporting interval for uploads in seconds
pub const UPLOAD_PROGRESS_INTERVAL_SECS: u64 = 5;

// ZIP format constants
/// ZIP local file header signature
pub const ZIP_LOCAL_FILE_HEADER_SIGNATURE: u32 = 0x04034b50;

/// ZIP central directory header signature
pub const ZIP_CENTRAL_DIR_HEADER_SIGNATURE: u32 = 0x02014b50;

/// ZIP end of central directory signature
pub const ZIP_END_OF_CENTRAL_DIR_SIGNATURE: u32 = 0x06054b50;

/// ZIP version needed to extract
pub const ZIP_VERSION_NEEDED: u16 = 20; // 2.0

/// ZIP version made by (UNIX + 3.0)
pub const ZIP_VERSION_MADE_BY: u16 = 0x031e;

/// ZIP compression method: deflate
pub const ZIP_COMPRESSION_METHOD_DEFLATE: u16 = 8;

/// ZIP compression method: store (no compression)
pub const ZIP_COMPRESSION_METHOD_STORE: u16 = 0;

/// ZIP default bit flag
pub const ZIP_DEFAULT_BIT_FLAG: u16 = 0;

// File size thresholds
/// Large file threshold for compression decisions (100MB)
pub const LARGE_FILE_COMPRESSION_THRESHOLD: u64 = 100 * 1024 * 1024;

/// Windows file buffer sizes
pub const WINDOWS_MEMORY_DUMP_BUFFER: usize = 4 * 1024 * 1024; // 4MB
pub const WINDOWS_MFT_BUFFER: usize = 2 * 1024 * 1024; // 2MB
pub const WINDOWS_EVENT_LOG_BUFFER: usize = 1 * 1024 * 1024; // 1MB
pub const WINDOWS_FILE_BUFFER_CAPACITY: usize = 8 * 1024 * 1024; // 8MB

// Platform-specific constants
/// macOS task dyld info count
pub const MACOS_TASK_DYLD_INFO_COUNT: u32 = 5;

// Error messages
pub const ERROR_FAILED_TO_CREATE_SESSION: &str = "Failed to create SSH session";
pub const ERROR_FAILED_TO_CREATE_SFTP: &str = "Failed to create SFTP subsystem";
pub const ERROR_FAILED_TO_CREATE_S3_STREAM: &str = "Failed to create S3 upload stream";
pub const ERROR_FAILED_TO_UPLOAD: &str = "Failed to upload file";
pub const ERROR_FAILED_TO_COMPRESS: &str = "Failed to compress file";
pub const ERROR_FAILED_TO_READ_FILE: &str = "Failed to read file";
pub const ERROR_FAILED_TO_WRITE_FILE: &str = "Failed to write file";
pub const ERROR_AUTHENTICATION_FAILED: &str = "Authentication failed";

// File paths and extensions
pub const PROC_PATH: &str = "/proc";
pub const SYS_PATH: &str = "/sys";
pub const DEV_PATH: &str = "/dev";

// Common file extensions
pub const COMPRESSED_EXTENSIONS: &[&str] = &[
    "zip", "gz", "xz", "bz2", "7z", "rar", "jpg", "jpeg", "png", "gif", "mp3", "mp4", "avi", "mov",
    "mpg", "mpeg",
];

pub const EXECUTABLE_EXTENSIONS: &[&str] = &["exe", "dll", "so", "dylib"];
pub const LOG_FILE_EXTENSIONS: &[&str] = &["log", "txt", "csv"];

// Default file names
pub const DEFAULT_OUTPUT_NAME: &str = "collected_artifacts.zip";
pub const DEFAULT_MEMORY_DUMP_NAME: &str = "memory_dump.bin";
pub const DEFAULT_VOLATILE_DATA_NAME: &str = "volatile_data.json";

// Test constants
#[cfg(test)]
pub mod test {
    /// Test memory size (8GB)
    pub const TEST_TOTAL_MEMORY: u64 = 8 * 1024 * 1024 * 1024;

    /// Test used memory (4GB)
    pub const TEST_USED_MEMORY: u64 = 4 * 1024 * 1024 * 1024;

    /// Test swap size (2GB)
    pub const TEST_TOTAL_SWAP: u64 = 2 * 1024 * 1024 * 1024;

    /// Test used swap (512MB)
    pub const TEST_USED_SWAP: u64 = 512 * 1024 * 1024;

    /// Test disk space (100GB)
    pub const TEST_TOTAL_DISK_SPACE: u64 = 100 * 1024 * 1024 * 1024;

    /// Test available disk space (50GB)
    pub const TEST_AVAILABLE_DISK_SPACE: u64 = 50 * 1024 * 1024 * 1024;

    /// Test data size (2MB)
    pub const TEST_DATA_SIZE: usize = 2 * 1024 * 1024;
}
