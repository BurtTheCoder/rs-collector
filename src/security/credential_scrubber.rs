//! Credential scrubbing utilities for preventing sensitive data exposure.
//!
//! This module provides functions to detect and scrub sensitive information
//! like passwords, API keys, and tokens from strings before they are logged
//! or displayed to users.

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Regex patterns for detecting various types of credentials
    static ref CREDENTIAL_PATTERNS: Vec<(Regex, &'static str)> = vec![
        // AWS Access Key ID
        (Regex::new(r"(?i)(aws[_-]?access[_-]?key[_-]?id|aws[_-]?key[_-]?id|access[_-]?key[_-]?id)\s*[:=]\s*([A-Z0-9]{16,32})").unwrap(),
         "$1=<REDACTED_AWS_KEY>"),

        // AWS Secret Access Key
        (Regex::new(r"(?i)(aws[_-]?secret[_-]?access[_-]?key|aws[_-]?secret[_-]?key|secret[_-]?access[_-]?key|secret[_-]?key)\s*[:=]\s*([A-Za-z0-9/+=]{32,})").unwrap(),
         "$1=<REDACTED_AWS_SECRET>"),

        // Generic API keys
        (Regex::new(r"(?i)(api[_-]?key|apikey)\s*[:=]\s*([A-Za-z0-9\-_]{20,})").unwrap(),
         "$1=<REDACTED_API_KEY>"),

        // Generic passwords
        (Regex::new(r"(?i)(password|passwd|pwd)\s*[:=]\s*([^\s]+)").unwrap(),
         "$1=<REDACTED_PASSWORD>"),

        // SSH private key paths
        (Regex::new(r"(?i)(private[_-]?key|ssh[_-]?key|key[_-]?file)\s*[:=]\s*([^\s]+\.pem|[^\s]+\.key|[^\s]+id_rsa[^\s]*)").unwrap(),
         "$1=<REDACTED_KEY_PATH>"),

        // Bearer tokens
        (Regex::new(r"(?i)(bearer|authorization)\s*[:=]\s*(bearer\s+)?([A-Za-z0-9\-._~+/]+=*)").unwrap(),
         "$1=<REDACTED_TOKEN>"),

        // GitHub tokens
        (Regex::new(r"(?i)(github[_-]?token|gh[_-]?token)\s*[:=]\s*([A-Za-z0-9_]{35,40})").unwrap(),
         "$1=<REDACTED_GITHUB_TOKEN>"),

        // Generic tokens
        (Regex::new(r"(?i)(token|access[_-]?token|auth[_-]?token)\s*[:=]\s*([A-Za-z0-9\-._~+/]{20,})").unwrap(),
         "$1=<REDACTED_TOKEN>"),

        // Database connection strings
        (Regex::new(r"(?i)(mysql|postgres|postgresql|mongodb|redis|mssql|oracle)://([^:]+):([^@]+)@").unwrap(),
         "$1://<REDACTED_USER>:<REDACTED_PASS>@"),

        // Basic auth in URLs
        (Regex::new(r"(https?://)([^:]+):([^@]+)@").unwrap(),
         "$1<REDACTED_USER>:<REDACTED_PASS>@"),
    ];

    /// Regex for detecting potential file paths containing credentials
    static ref SENSITIVE_PATH_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)\.ssh/").unwrap(),
        Regex::new(r"(?i)\.aws/").unwrap(),
        Regex::new(r"(?i)\.kube/").unwrap(),
        Regex::new(r"(?i)\.docker/").unwrap(),
        Regex::new(r"(?i)\.gnupg/").unwrap(),
        Regex::new(r"(?i)id_rsa").unwrap(),
        Regex::new(r"(?i)id_dsa").unwrap(),
        Regex::new(r"(?i)id_ecdsa").unwrap(),
        Regex::new(r"(?i)id_ed25519").unwrap(),
        Regex::new(r"(?i)\.pem$").unwrap(),
        Regex::new(r"(?i)\.key$").unwrap(),
        Regex::new(r"(?i)\.p12$").unwrap(),
        Regex::new(r"(?i)\.pfx$").unwrap(),
        Regex::new(r"(?i)credentials").unwrap(),
        Regex::new(r"(?i)password").unwrap(),
        Regex::new(r"(?i)secret").unwrap(),
    ];
}

/// Scrub credentials from a string.
///
/// This function detects and replaces various types of credentials with
/// safe placeholder values to prevent accidental exposure in logs or error
/// messages.
///
/// # Arguments
///
/// * `input` - The string to scrub
///
/// # Returns
///
/// A new string with credentials replaced by placeholders
///
/// # Example
///
/// ```
/// use rust_collector::security::credential_scrubber::scrub_credentials;
///
/// let input = "Failed to connect with password=secret123";
/// let scrubbed = scrub_credentials(input);
/// assert_eq!(scrubbed, "Failed to connect with password=<REDACTED_PASSWORD>");
/// ```
pub fn scrub_credentials(input: &str) -> String {
    let mut result = input.to_string();

    // Apply all credential patterns
    for (pattern, replacement) in CREDENTIAL_PATTERNS.iter() {
        result = pattern.replace_all(&result, *replacement).to_string();
    }

    result
}

/// Check if a path might contain sensitive files.
///
/// This function checks if a file path appears to contain credentials or
/// other sensitive data based on common patterns.
///
/// # Arguments
///
/// * `path` - The file path to check
///
/// # Returns
///
/// * `true` if the path might contain sensitive data
/// * `false` otherwise
pub fn is_sensitive_path(path: &str) -> bool {
    for pattern in SENSITIVE_PATH_PATTERNS.iter() {
        if pattern.is_match(path) {
            return true;
        }
    }
    false
}

/// Scrub a file path to hide sensitive information.
///
/// This function replaces the filename portion of sensitive paths with
/// a placeholder while preserving the directory structure for debugging.
///
/// # Arguments
///
/// * `path` - The file path to scrub
///
/// # Returns
///
/// A scrubbed version of the path
pub fn scrub_path(path: &str) -> String {
    if !is_sensitive_path(path) {
        return path.to_string();
    }

    // Find the last path separator
    if let Some(pos) = path.rfind(|c| c == '/' || c == '\\') {
        let dir = &path[..pos];
        format!("{}/[REDACTED_SENSITIVE_FILE]", dir)
    } else {
        "[REDACTED_SENSITIVE_FILE]".to_string()
    }
}

/// Create a safe error message that scrubs credentials.
///
/// This is a convenience function that combines credential scrubbing
/// with common error formatting patterns.
///
/// # Arguments
///
/// * `context` - The error context
/// * `error` - The underlying error
///
/// # Returns
///
/// A formatted error string with credentials scrubbed
pub fn safe_error_message(context: &str, error: &impl std::fmt::Display) -> String {
    let raw_message = format!("{}: {}", context, error);
    scrub_credentials(&raw_message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrub_aws_credentials() {
        let input = "Failed with aws_access_key_id=AKIAIOSFODNN7EXAMPLE";
        let result = scrub_credentials(input);
        assert_eq!(result, "Failed with aws_access_key_id=<REDACTED_AWS_KEY>");

        let input = "AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let result = scrub_credentials(input);
        assert_eq!(result, "AWS_SECRET_ACCESS_KEY=<REDACTED_AWS_SECRET>");
    }

    #[test]
    fn test_scrub_passwords() {
        let input = "Connection failed: password=mysecret123";
        let result = scrub_credentials(input);
        assert_eq!(result, "Connection failed: password=<REDACTED_PASSWORD>");

        let input = "Login with passwd: supersecret!@#";
        let result = scrub_credentials(input);
        assert_eq!(result, "Login with passwd=<REDACTED_PASSWORD>");
    }

    #[test]
    fn test_scrub_api_keys() {
        let input = "Using api_key=abcdef123456789012345678901234567890";
        let result = scrub_credentials(input);
        assert_eq!(result, "Using api_key=<REDACTED_API_KEY>");
    }

    #[test]
    fn test_scrub_database_urls() {
        let input = "postgres://user:pass123@localhost:5432/db";
        let result = scrub_credentials(input);
        assert_eq!(
            result,
            "postgres://<REDACTED_USER>:<REDACTED_PASS>@localhost:5432/db"
        );
    }

    #[test]
    fn test_scrub_basic_auth_urls() {
        let input = "Failed to connect to https://admin:secret@example.com/api";
        let result = scrub_credentials(input);
        assert_eq!(
            result,
            "Failed to connect to https://<REDACTED_USER>:<REDACTED_PASS>@example.com/api"
        );
    }

    #[test]
    fn test_is_sensitive_path() {
        assert!(is_sensitive_path("/home/user/.ssh/id_rsa"));
        assert!(is_sensitive_path("/Users/test/.aws/credentials"));
        assert!(is_sensitive_path("C:\\Users\\test\\.kube\\config"));
        assert!(is_sensitive_path("/etc/ssl/private/server.key"));
        assert!(is_sensitive_path("/var/lib/app/secrets.yaml"));

        assert!(!is_sensitive_path("/usr/bin/ls"));
        assert!(!is_sensitive_path("/home/user/documents/report.pdf"));
    }

    #[test]
    fn test_scrub_path() {
        assert_eq!(
            scrub_path("/home/user/.ssh/id_rsa"),
            "/home/user/.ssh/[REDACTED_SENSITIVE_FILE]"
        );

        assert_eq!(
            scrub_path("C:\\Users\\test\\.aws\\credentials"),
            "C:\\Users\\test\\.aws/[REDACTED_SENSITIVE_FILE]"
        );

        assert_eq!(scrub_path("/usr/bin/ls"), "/usr/bin/ls");
    }

    #[test]
    fn test_safe_error_message() {
        let error = "password=secret123";
        let result = safe_error_message("Authentication failed", &error);
        assert_eq!(
            result,
            "Authentication failed: password=<REDACTED_PASSWORD>"
        );
    }

    #[test]
    fn test_multiple_credentials_in_string() {
        let input = "Config: api_key=abc123 password=secret aws_secret_key=xyz789";
        let result = scrub_credentials(input);
        assert!(result.contains("<REDACTED_API_KEY>"));
        assert!(result.contains("<REDACTED_PASSWORD>"));
        assert!(result.contains("<REDACTED_AWS_SECRET>"));
        assert!(!result.contains("abc123"));
        assert!(!result.contains("secret"));
        assert!(!result.contains("xyz789"));
    }
}
