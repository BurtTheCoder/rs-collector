//! Streaming artifact collection
//!
//! This module handles streaming artifacts directly to remote storage

mod core;
mod s3;
mod sftp;

pub use s3::{stream_artifacts_to_s3, stream_file_to_s3};
pub use sftp::{stream_artifacts_to_sftp, stream_file_to_sftp};
