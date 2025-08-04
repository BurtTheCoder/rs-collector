//! Cloud storage integration for artifact uploads.
//!
//! This module provides functionality to upload collected artifacts directly
//! to cloud storage providers without requiring local storage. This is especially
//! useful for remote forensic collections where local disk space is limited.
//!
//! ## Supported Providers
//!
//! - **Amazon S3**: Full support for S3 and S3-compatible storage
//! - **SFTP**: Secure file transfer to any SSH/SFTP server
//!
//! ## Features
//!
//! - **Streaming Uploads**: Data is uploaded as it's collected, minimizing memory usage
//! - **Parallel Transfers**: Multiple artifacts can be uploaded concurrently
//! - **Retry Logic**: Automatic retry with exponential backoff for failed uploads
//! - **Progress Tracking**: Real-time upload progress monitoring
//! - **Compression**: On-the-fly compression during upload
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌─────────────────┐
//! │   Collector     │────▶│ Streaming ZIP   │
//! └─────────────────┘     └────────┬────────┘
//!                                  │
//!                    ┌─────────────┴─────────────┐
//!                    │                           │
//!              ┌─────▼──────┐           ┌───────▼────────┐
//!              │ S3 Upload  │           │ SFTP Upload    │
//!              │  Stream    │           │    Stream      │
//!              └─────┬──────┘           └───────┬────────┘
//!                    │                           │
//!              ┌─────▼──────┐           ┌───────▼────────┐
//!              │  S3 Bucket │           │  SFTP Server   │
//!              └────────────┘           └────────────────┘
//! ```
//!
//! ## Usage Example
//!
//! ### S3 Upload
//!
//! ```no_run
//! use rust_collector::cloud::s3::UploadQueue;
//! use rust_collector::cloud::streaming::S3UploadStream;
//! use rusoto_core::Region;
//! use rusoto_s3::S3Client;
//! use std::sync::Arc;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create S3 client
//! let region = Region::UsEast1;
//! let client = Arc::new(S3Client::new(region.clone()));
//!
//! // Create upload stream
//! let upload_stream = S3UploadStream::new(
//!     client,
//!     "forensics-bucket",
//!     "collection-2024.zip",
//!     5  // buffer_size_mb
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### SFTP Upload
//!
//! ```no_run
//! use rust_collector::cloud::sftp::{upload_to_sftp, SFTPConfig};
//! use std::path::{Path, PathBuf};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let local_path = Path::new("/tmp/collection.zip");
//!
//! let config = SFTPConfig {
//!     host: "forensics.example.com".to_string(),
//!     port: 22,
//!     username: "investigator".to_string(),
//!     private_key_path: PathBuf::from("/home/user/.ssh/id_rsa"),
//!     remote_path: "/forensics/case123/".to_string(),
//!     connection_timeout_sec: 30,
//!     concurrent_connections: 4,
//!     buffer_size_mb: 8,
//!     max_retries: 3,
//! };
//!
//! upload_to_sftp(local_path, config).await?;
//!
//! println!("Upload completed successfully");
//! # Ok(())
//! # }
//! ```

/// Amazon S3 integration and configuration
pub mod s3;

/// S3 streaming upload implementation
pub mod streaming;

/// HTTP client utilities for cloud APIs
pub mod client;

/// SFTP configuration and basic upload functionality
pub mod sftp;

/// SFTP streaming upload implementation
pub mod sftp_streaming;

/// Common trait for streaming upload targets
pub mod streaming_target;
