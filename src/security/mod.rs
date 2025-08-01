//! Security utilities and validation functions.
//!
//! This module provides security-related functionality including:
//! - Path validation to prevent directory traversal
//! - Input sanitization
//! - Privilege management helpers
//! - Security configuration and policies

pub mod path_validator;
pub mod config;

pub use path_validator::{validate_path, sanitize_filename, validate_output_path};
pub use config::{SecurityConfig, SecurityEvent, log_security_event};