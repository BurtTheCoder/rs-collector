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
//! use rust_collector::cloud::s3::{S3Config, create_s3_client};
//! use rust_collector::cloud::streaming::S3UploadStream;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = S3Config {
//!     bucket: "forensics-bucket".to_string(),
//!     region: "us-east-1".to_string(),
//!     access_key_id: "KEY".to_string(),
//!     secret_access_key: "SECRET".to_string(),
//!     buffer_size_mb: 5,
//! };
//!
//! let client = create_s3_client(&config)?;
//! let upload_stream = S3UploadStream::new(
//!     client,
//!     &config.bucket,
//!     "collection-2024.zip",
//!     config.buffer_size_mb
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### SFTP Upload
//!
//! ```no_run
//! use rust_collector::cloud::sftp::upload_to_sftp;
//! use std::path::Path;
//!
//! # fn example() -> anyhow::Result<()> {
//! let local_path = Path::new("/tmp/collection.zip");
//! let remote_path = "/forensics/case123/collection.zip";
//!
//! upload_to_sftp(
//!     "forensics.example.com",
//!     22,
//!     "investigator",
//!     Some("/home/user/.ssh/id_rsa"),
//!     local_path,
//!     remote_path,
//! )?;
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
