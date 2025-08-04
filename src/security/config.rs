//! Security configuration and policy enforcement.
//!
//! This module defines security policies and configuration options
//! that can be used to control the security behavior of the collector.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Security configuration for the collector.
///
/// This struct contains all security-related settings that control
/// how the collector operates from a security perspective.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable path validation to prevent directory traversal
    pub validate_paths: bool,

    /// Enable output file encryption
    pub encrypt_output: bool,

    /// Set restrictive file permissions on output
    pub restrictive_permissions: bool,

    /// Drop privileges after initialization
    pub drop_privileges: bool,

    /// Maximum file size to collect (in bytes)
    pub max_file_size: Option<u64>,

    /// Allowed output directories
    pub allowed_output_dirs: Vec<PathBuf>,

    /// Disallowed file extensions
    pub blocked_extensions: Vec<String>,

    /// Enable audit logging
    pub audit_logging: bool,

    /// Redact sensitive information from logs
    pub redact_sensitive_data: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            validate_paths: true,
            encrypt_output: false,
            restrictive_permissions: true,
            drop_privileges: true,
            max_file_size: Some(10 * 1024 * 1024 * 1024), // 10GB
            allowed_output_dirs: vec![],
            blocked_extensions: vec![
                ".key".to_string(),
                ".pem".to_string(),
                ".p12".to_string(),
                ".pfx".to_string(),
                ".keystore".to_string(),
            ],
            audit_logging: true,
            redact_sensitive_data: true,
        }
    }
}

impl SecurityConfig {
    /// Create a high-security configuration.
    ///
    /// This configuration enables all security features and
    /// uses the most restrictive settings.
    pub fn high_security() -> Self {
        Self {
            validate_paths: true,
            encrypt_output: true,
            restrictive_permissions: true,
            drop_privileges: true,
            max_file_size: Some(1024 * 1024 * 1024), // 1GB
            allowed_output_dirs: vec![],
            blocked_extensions: vec![
                ".key".to_string(),
                ".pem".to_string(),
                ".p12".to_string(),
                ".pfx".to_string(),
                ".keystore".to_string(),
                ".password".to_string(),
                ".secret".to_string(),
                ".private".to_string(),
            ],
            audit_logging: true,
            redact_sensitive_data: true,
        }
    }

    /// Create a low-security configuration for testing.
    ///
    /// WARNING: This configuration disables most security features
    /// and should only be used for testing purposes.
    pub fn low_security() -> Self {
        Self {
            validate_paths: false,
            encrypt_output: false,
            restrictive_permissions: false,
            drop_privileges: false,
            max_file_size: None,
            allowed_output_dirs: vec![],
            blocked_extensions: vec![],
            audit_logging: false,
            redact_sensitive_data: false,
        }
    }

    /// Check if a file extension is blocked.
    pub fn is_extension_blocked(&self, path: &std::path::Path) -> bool {
        if let Some(extension) = path.extension() {
            let ext = extension.to_string_lossy().to_lowercase();
            self.blocked_extensions.iter().any(|blocked| {
                blocked.to_lowercase() == format!(".{}", ext) || blocked.to_lowercase() == ext
            })
        } else {
            false
        }
    }

    /// Check if output directory is allowed.
    pub fn is_output_dir_allowed(&self, path: &std::path::Path) -> bool {
        if self.allowed_output_dirs.is_empty() {
            // If no restrictions, allow all
            true
        } else {
            // Check if path is within any allowed directory
            self.allowed_output_dirs
                .iter()
                .any(|allowed| path.starts_with(allowed))
        }
    }

    /// Apply security policy to a file size.
    pub fn is_file_size_allowed(&self, size: u64) -> bool {
        if let Some(max_size) = self.max_file_size {
            size <= max_size
        } else {
            true
        }
    }
}

/// Security audit event types.
#[derive(Debug, Clone, Serialize)]
pub enum SecurityEvent {
    /// Path validation failed
    PathValidationFailed { path: String, reason: String },

    /// Privilege escalation requested
    PrivilegeEscalation { privilege: String, success: bool },

    /// Sensitive file accessed
    SensitiveFileAccess { path: String, action: String },

    /// Security policy violation
    PolicyViolation { policy: String, details: String },

    /// Authentication attempt
    Authentication { method: String, success: bool },
}

/// Log a security event.
pub fn log_security_event(event: SecurityEvent) {
    use log::warn;

    match event {
        SecurityEvent::PathValidationFailed { path, reason } => {
            warn!(
                "Security: Path validation failed for '{}': {}",
                path, reason
            );
        }
        SecurityEvent::PrivilegeEscalation { privilege, success } => {
            if success {
                warn!("Security: Privilege '{}' acquired", privilege);
            } else {
                warn!("Security: Failed to acquire privilege '{}'", privilege);
            }
        }
        SecurityEvent::SensitiveFileAccess { path, action } => {
            warn!(
                "Security: Sensitive file access - Action: '{}' Path: '{}'",
                action, path
            );
        }
        SecurityEvent::PolicyViolation { policy, details } => {
            warn!(
                "Security: Policy violation - Policy: '{}' Details: {}",
                policy, details
            );
        }
        SecurityEvent::Authentication { method, success } => {
            if success {
                warn!(
                    "Security: Authentication successful using method '{}'",
                    method
                );
            } else {
                warn!("Security: Authentication failed using method '{}'", method);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert!(config.validate_paths);
        assert!(!config.encrypt_output);
        assert!(config.restrictive_permissions);
        assert!(config.drop_privileges);
        assert_eq!(config.max_file_size, Some(10 * 1024 * 1024 * 1024));
    }

    #[test]
    fn test_security_config_high_security() {
        let config = SecurityConfig::high_security();
        assert!(config.validate_paths);
        assert!(config.encrypt_output);
        assert!(config.restrictive_permissions);
        assert!(config.drop_privileges);
        assert_eq!(config.max_file_size, Some(1024 * 1024 * 1024));
        assert!(config.blocked_extensions.len() > 5);
    }

    #[test]
    fn test_security_config_low_security() {
        let config = SecurityConfig::low_security();
        assert!(!config.validate_paths);
        assert!(!config.encrypt_output);
        assert!(!config.restrictive_permissions);
        assert!(!config.drop_privileges);
        assert_eq!(config.max_file_size, None);
        assert!(config.blocked_extensions.is_empty());
    }

    #[test]
    fn test_is_extension_blocked() {
        let config = SecurityConfig::default();

        assert!(config.is_extension_blocked(Path::new("private.key")));
        assert!(config.is_extension_blocked(Path::new("cert.pem")));
        assert!(config.is_extension_blocked(Path::new("store.p12")));
        assert!(!config.is_extension_blocked(Path::new("data.txt")));
        assert!(!config.is_extension_blocked(Path::new("noextension")));
    }

    #[test]
    fn test_is_output_dir_allowed() {
        let mut config = SecurityConfig::default();

        // No restrictions by default
        assert!(config.is_output_dir_allowed(Path::new("/tmp/output")));
        assert!(config.is_output_dir_allowed(Path::new("/home/user/data")));

        // Add restrictions
        config.allowed_output_dirs =
            vec![PathBuf::from("/tmp"), PathBuf::from("/home/user/forensics")];

        assert!(config.is_output_dir_allowed(Path::new("/tmp/output")));
        assert!(config.is_output_dir_allowed(Path::new("/home/user/forensics/case1")));
        assert!(!config.is_output_dir_allowed(Path::new("/etc/output")));
        assert!(!config.is_output_dir_allowed(Path::new("/home/user/other")));
    }

    #[test]
    fn test_is_file_size_allowed() {
        let mut config = SecurityConfig::default();

        // Default 10GB limit
        assert!(config.is_file_size_allowed(1024 * 1024)); // 1MB
        assert!(config.is_file_size_allowed(5 * 1024 * 1024 * 1024)); // 5GB
        assert!(!config.is_file_size_allowed(11 * 1024 * 1024 * 1024)); // 11GB

        // No limit
        config.max_file_size = None;
        assert!(config.is_file_size_allowed(100 * 1024 * 1024 * 1024)); // 100GB
    }
}
