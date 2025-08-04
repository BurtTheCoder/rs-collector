//! Permission error tracking and reporting
//!
//! This module provides utilities to track permission-related collection failures
//! and provide helpful guidance to users about running with elevated privileges.

use log::warn;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Tracks artifacts that failed due to permission errors
#[derive(Debug, Clone, Default)]
pub struct PermissionTracker {
    failed_artifacts: Arc<Mutex<HashSet<String>>>,
}

impl PermissionTracker {
    /// Create a new permission tracker
    pub fn new() -> Self {
        Self {
            failed_artifacts: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Record a permission failure for an artifact
    pub async fn record_permission_failure(&self, artifact_name: &str) {
        let mut failures = self.failed_artifacts.lock().await;
        failures.insert(artifact_name.to_string());
    }

    /// Check if an error message indicates a permission problem
    pub fn is_permission_error(error_msg: &str) -> bool {
        error_msg.contains("Permission denied")
            || error_msg.contains("PermissionDenied")
            || error_msg.contains("Access is denied")
            || error_msg.contains("elevated privileges")
    }

    /// Get the count of permission failures
    pub async fn failure_count(&self) -> usize {
        let failures = self.failed_artifacts.lock().await;
        failures.len()
    }

    /// Report permission failures and provide guidance
    pub async fn report_failures(&self) {
        let failures = self.failed_artifacts.lock().await;

        if failures.is_empty() {
            return;
        }

        warn!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        warn!("⚠️  Permission Issues Summary");
        warn!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        warn!("");
        warn!(
            "The following {} artifact(s) could not be collected due to insufficient permissions:",
            failures.len()
        );
        warn!("");

        for artifact in failures.iter() {
            warn!("  • {}", artifact);
        }

        warn!("");
        warn!("To collect these artifacts, try one of the following:");
        warn!("");

        #[cfg(target_os = "linux")]
        {
            warn!(
                "  1. Run with sudo: sudo {}",
                std::env::args().collect::<Vec<_>>().join(" ")
            );
            warn!("  2. Add your user to required groups (e.g., 'adm' for logs): sudo usermod -a -G adm $USER");
            warn!("  3. Adjust file permissions on specific files/directories");
        }

        #[cfg(target_os = "windows")]
        {
            warn!("  1. Run as Administrator (right-click and 'Run as administrator')");
            warn!("  2. Ensure your user account has appropriate permissions");
            warn!("  3. Check Windows security policies");
        }

        #[cfg(target_os = "macos")]
        {
            warn!(
                "  1. Run with sudo: sudo {}",
                std::env::args().collect::<Vec<_>>().join(" ")
            );
            warn!("  2. Grant Full Disk Access in System Preferences > Security & Privacy");
            warn!("  3. Check file permissions with 'ls -la'");
        }

        warn!("");
        warn!("Note: Collection continued for accessible artifacts.");
        warn!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }

    /// Check if we should suggest running with elevated privileges
    pub async fn should_suggest_elevation(&self) -> bool {
        let failures = self.failed_artifacts.lock().await;
        !failures.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_permission_tracker() {
        let tracker = PermissionTracker::new();

        // Initially no failures
        assert_eq!(tracker.failure_count().await, 0);
        assert!(!tracker.should_suggest_elevation().await);

        // Record a failure
        tracker.record_permission_failure("syslog").await;
        assert_eq!(tracker.failure_count().await, 1);
        assert!(tracker.should_suggest_elevation().await);

        // Duplicate failures only counted once
        tracker.record_permission_failure("syslog").await;
        assert_eq!(tracker.failure_count().await, 1);

        // Additional failure
        tracker.record_permission_failure("auth.log").await;
        assert_eq!(tracker.failure_count().await, 2);
    }

    #[test]
    fn test_permission_error_detection() {
        assert!(PermissionTracker::is_permission_error(
            "Permission denied accessing file"
        ));
        assert!(PermissionTracker::is_permission_error(
            "Error: PermissionDenied"
        ));
        assert!(PermissionTracker::is_permission_error("Access is denied"));
        assert!(PermissionTracker::is_permission_error(
            "Try running with elevated privileges"
        ));
        assert!(!PermissionTracker::is_permission_error("File not found"));
        assert!(!PermissionTracker::is_permission_error("Network error"));
    }
}
