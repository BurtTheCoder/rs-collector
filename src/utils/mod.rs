//! Utility functions for forensic artifact processing.
//!
//! This module provides essential utilities for handling collected artifacts,
//! including compression, hashing, timeline generation, and reporting.
//!
//! ## Components
//!
//! - **Compression**: ZIP archive creation with streaming support
//! - **Hashing**: SHA-256 calculation for file integrity
//! - **Bodyfile**: Timeline generation in Sleuthkit bodyfile format
//! - **Summary**: Collection summary and reporting
//!
//! ## Common Use Cases
//!
//! ### Creating a ZIP Archive
//!
//! ```no_run
//! use rust_collector::utils::compress::compress_artifacts;
//! use std::path::Path;
//!
//! # fn example() -> anyhow::Result<()> {
//! let source_dir = Path::new("/tmp/collected");
//! let hostname = "workstation01";
//! let timestamp = "20240101_120000";
//!
//! let zip_path = compress_artifacts(source_dir, hostname, timestamp)?;
//! println!("Created archive: {}", zip_path.display());
//! # Ok(())
//! # }
//! ```
//!
//! ### Generating File Hashes
//!
//! ```no_run
//! use rust_collector::utils::hash::calculate_sha256;
//! use std::path::Path;
//!
//! # fn example() -> anyhow::Result<()> {
//! let file_path = Path::new("/evidence/suspicious.exe");
//! let max_size = 1024 * 1024 * 1024; // 1GB limit
//!
//! match calculate_sha256(file_path, max_size)? {
//!     Some(hash) => println!("SHA-256: {}", hash),
//!     None => println!("File exceeds size limit"),
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### Creating a Timeline
//!
//! ```no_run
//! use rust_collector::utils::bodyfile::generate_bodyfile;
//! use std::path::Path;
//! use std::collections::HashMap;
//!
//! # fn example() -> anyhow::Result<()> {
//! let output_path = Path::new("/tmp/timeline.bodyfile");
//! let mut options = HashMap::new();
//! options.insert("root_path".to_string(), "/mnt/evidence".to_string());
//!
//! generate_bodyfile(output_path, &options)?;
//! println!("Generated bodyfile successfully");
//! # Ok(())
//! # }
//! ```

/// Collection summary generation and reporting
pub mod summary;

/// File compression and ZIP archive creation
pub mod compress;

/// Bodyfile timeline generation for forensic analysis
pub mod bodyfile;

/// Cryptographic hash calculation utilities
pub mod hash;

/// Streaming ZIP archive creation for large collections
pub mod streaming_zip;
