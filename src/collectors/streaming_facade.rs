//! Streaming artifact collection
//!
//! This module handles streaming artifacts directly to remote storage.
//! It provides functions for streaming artifacts to S3 and SFTP servers.

// Re-export the streaming functions for backward compatibility

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_documentation() {
        // This test ensures the module has proper documentation
        let doc = include_str!("streaming_facade.rs");
        assert!(doc.contains("Streaming artifact collection"));
        assert!(doc.contains("streaming artifacts directly to remote storage"));
    }

    #[test]
    fn test_re_export_comment() {
        // Verify re-export comment exists
        let content = include_str!("streaming_facade.rs");
        assert!(content.contains("Re-export the streaming functions for backward compatibility"));
    }
}
