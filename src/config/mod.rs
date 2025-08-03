//! Configuration management for artifact collection.
//!
//! This module provides comprehensive configuration capabilities for defining
//! what artifacts to collect, how to collect them, and where to store them.
//!
//! ## Overview
//!
//! The configuration system is designed to be flexible and extensible, supporting:
//! - Platform-specific artifact definitions
//! - Environment variable expansion
//! - Regular expression-based file matching
//! - Default configurations for common use cases
//!
//! ## Configuration Format
//!
//! Configurations are typically stored in YAML format:
//!
//! ```yaml
//! version: "1.0"
//! description: "Windows DFIR collection"
//! artifacts:
//!   - name: "Windows Event Logs"
//!     artifact_type:
//!       Windows: EventLogs
//!     source_path: "%SystemRoot%\\System32\\winevt\\Logs"
//!     destination_name: "EventLogs"
//!     required: true
//! ```
//!
//! ## Usage Example
//!
//! ```no_run
//! use rust_collector::config::{CollectionConfig, load_or_create_config};
//! use std::path::Path;
//!
//! # fn main() -> anyhow::Result<()> {
//! // Load configuration from file or use defaults
//! let config = load_or_create_config(Some(Path::new("config.yaml")))?;
//!
//! // Access artifacts
//! for artifact in &config.artifacts {
//!     println!("Collecting: {}", artifact.name);
//! }
//! # Ok(())
//! # }
//! ```

// Re-export all items from the submodules
mod artifact_types;
mod collection_config;
mod default_configs;
mod env_vars;
mod regex_config;

/// Artifact type definitions for different platforms
///
/// This module defines the various types of artifacts that can be collected
/// on Windows, Linux, and macOS systems. Each platform has specific artifact
/// types that correspond to forensically relevant data sources.
pub use artifact_types::{
    ArtifactType,
    WindowsArtifactType,
    LinuxArtifactType,
    MacOSArtifactType,
    VolatileDataType,
};

/// Main configuration structures
///
/// These types define the structure of collection configurations, including
/// individual artifact definitions and the overall collection configuration.
pub use collection_config::{
    Artifact,
    CollectionConfig,
    load_or_create_config,
};

/// Environment variable parsing utilities
///
/// These functions handle platform-specific environment variable expansion,
/// allowing artifact paths to use variables like %SystemRoot% on Windows
/// or $HOME on Unix systems.
pub use env_vars::{
    parse_windows_env_vars,
    parse_unix_env_vars,
};

/// Regular expression configuration for file matching
///
/// Enables pattern-based artifact collection using regular expressions
/// to match files by name or path.
pub use regex_config::RegexConfig;
