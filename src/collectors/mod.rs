//! Artifact collection implementations.
//!
//! This module contains all the collectors responsible for gathering forensic
//! artifacts from various sources. Each collector is optimized for its specific
//! artifact type and platform.
//!
//! ## Architecture
//!
//! The collector system follows a modular architecture:
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │          Main Collector API             │
//! ├─────────────────────────────────────────┤
//! │        Platform Collectors              │
//! │  ┌─────────┬──────────┬──────────┐     │
//! │  │ Windows │  Linux   │  macOS   │     │
//! │  └─────────┴──────────┴──────────┘     │
//! ├─────────────────────────────────────────┤
//! │       Specialized Collectors            │
//! │  ┌─────────┬──────────┬──────────┐     │
//! │  │ Memory  │ Volatile │  Regex   │     │
//! │  └─────────┴──────────┴──────────┘     │
//! ├─────────────────────────────────────────┤
//! │         Output Handlers                 │
//! │  ┌─────────┬──────────┬──────────┐     │
//! │  │  Local  │   S3     │  SFTP    │     │
//! │  └─────────┴──────────┴──────────┘     │
//! └─────────────────────────────────────────┘
//! ```
//!
//! ## Collector Types
//!
//! - **Platform Collectors**: OS-specific implementations for Windows, Linux, and macOS
//! - **Memory Collectors**: Process memory collection with search capabilities
//! - **Volatile Collectors**: Runtime system information (processes, network, etc.)
//! - **Regex Collectors**: Pattern-based file collection
//! - **Streaming Collectors**: Direct-to-cloud upload without local storage
//!
//! ## Usage Example
//!
//! ```no_run
//! use rust_collector::collectors::collector::collect_artifacts_parallel;
//! use rust_collector::config::CollectionConfig;
//! use std::path::Path;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = CollectionConfig::default();
//! let output_dir = Path::new("/tmp/collection");
//!
//! // Collect artifacts in parallel
//! let results = collect_artifacts_parallel(&config.artifacts, output_dir).await?;
//! 
//! println!("Collected {} artifacts", results.len());
//! # Ok(())
//! # }
//! ```

/// Core collector trait and main collection functions
pub mod collector;

/// Platform-specific collector implementations
pub mod platforms;

/// Streaming upload collectors for cloud storage
pub mod streaming;

/// Facade for streaming collectors
pub mod streaming_facade;

/// Volatile data collectors (processes, network, system info)
pub mod volatile;

/// Memory collection functionality
pub mod memory;

/// Regular expression-based file collectors
pub mod regex;

/// Permission error tracking and reporting
pub mod permission_tracker;
