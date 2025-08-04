//! Security utilities and validation functions.
//!
//! This module provides security-related functionality including:
//! - Path validation to prevent directory traversal
//! - Input sanitization
//! - Privilege management helpers
//! - Security configuration and policies
//! - Credential scrubbing to prevent sensitive data exposure

pub mod config;
pub mod credential_scrubber;
pub mod path_validator;

pub use config::{log_security_event, SecurityConfig, SecurityEvent};
pub use credential_scrubber::{safe_error_message, scrub_credentials, scrub_path};
pub use path_validator::{sanitize_filename, validate_output_path, validate_path};
