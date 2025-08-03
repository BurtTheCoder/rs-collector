//! # rs-collector
//!
//! A high-performance, cross-platform Digital Forensics and Incident Response (DFIR) 
//! artifact collector written in Rust.
//!
//! ## Overview
//!
//! rs-collector is designed to efficiently collect forensic artifacts from Windows, macOS, 
//! and Linux systems. It supports parallel collection, streaming uploads to cloud storage,
//! memory collection, and various output formats.
//!
//! ## Features
//!
//! - **Cross-platform support**: Windows, macOS, and Linux
//! - **Parallel collection**: Utilizes all available CPU cores for faster collection
//! - **Streaming uploads**: Direct upload to S3 or SFTP without local storage
//! - **Memory collection**: Process memory dumps with search capabilities
//! - **Flexible configuration**: YAML-based artifact definitions
//! - **Multiple output formats**: ZIP archives, raw files, or streaming
//! - **Volatile data collection**: Processes, network connections, system info
//! - **Bodyfile generation**: Timeline analysis support
//!
//! ## Usage
//!
//! ### Basic Collection
//!
//! ```no_run
//! use rust_collector::config::CollectionConfig;
//! use rust_collector::collectors::collector::collect_artifacts;
//! use std::path::Path;
//!
//! # fn main() -> anyhow::Result<()> {
//! // Load configuration
//! let config = CollectionConfig::default();
//! 
//! // Collect artifacts
//! let results = collect_artifacts(&config.artifacts, Path::new("/tmp/output"))?;
//! 
//! println!("Collected {} artifacts", results.len());
//! # Ok(())
//! # }
//! ```
//!
//! ### Streaming Upload
//!
//! ```no_run
//! use rust_collector::cloud::s3::S3Config;
//! use rust_collector::collectors::streaming::s3::S3StreamingCollector;
//! # use rust_collector::config::CollectionConfig;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let s3_config = S3Config {
//!     bucket: "my-bucket".to_string(),
//!     region: "us-east-1".to_string(),
//!     access_key_id: "key".to_string(),
//!     secret_access_key: "secret".to_string(),
//!     buffer_size_mb: 5,
//! };
//!
//! let collector = S3StreamingCollector::new(s3_config).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Module Organization
//!
//! - [`cli`]: Command-line interface definitions and argument parsing
//! - [`models`]: Core data models and structures
//! - [`collectors`]: Artifact collection implementations
//! - [`config`]: Configuration management and artifact definitions
//! - [`cloud`]: Cloud storage upload functionality (S3, SFTP)
//! - [`utils`]: Utility functions for compression, hashing, etc.
//! - [`security`]: Security utilities including path validation and credential scrubbing
//! - [`privileges`]: Platform-specific privilege escalation
//! - [`constants`]: Application-wide constants
//!
//! ## Feature Flags
//!
//! - `memory_collection`: Enable memory collection capabilities
//! - `yara`: Enable YARA scanning in memory dumps
//! - `embed_config`: Embed default configurations in the binary
//!
//! ## Safety
//!
//! This crate uses `unsafe` code in specific scenarios:
//! - Windows raw disk access for locked file collection
//! - Memory collection on various platforms
//! - Platform-specific system calls
//!
//! All unsafe code is documented with safety invariants and is contained
//! within platform-specific modules.

/// Command-line interface definitions and argument parsing
pub mod cli;

/// Core data models and structures used throughout the application
pub mod models;

/// Windows-specific functionality including raw disk access
pub mod windows;

/// Artifact collectors for various platforms and artifact types
pub mod collectors;

/// Utility functions for compression, hashing, and file operations
pub mod utils;

/// Cloud storage integration (S3, SFTP)
pub mod cloud;

/// Configuration management and artifact definitions
pub mod config;

/// Build script generation for custom collection workflows
pub mod build;

/// Platform-specific privilege management
pub mod privileges;

/// Application constants and configuration values
pub mod constants;

/// Security utilities for path validation and credential protection
pub mod security;

/// Test utilities and helpers
#[cfg(test)]
pub mod test_utils;
