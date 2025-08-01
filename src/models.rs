use serde::{Serialize, Deserialize};

/// Metadata for a collected forensic artifact.
/// 
/// This struct contains comprehensive metadata about each artifact collected
/// during forensic acquisition. It's designed to maintain chain of custody
/// and provide investigators with crucial file system metadata.
/// 
/// # Fields
/// 
/// * `original_path` - The original file system path where the artifact was located
/// * `collection_time` - ISO 8601 timestamp of when the artifact was collected
/// * `file_size` - Size of the file in bytes
/// * `created_time` - Optional file creation timestamp (ISO 8601 format)
/// * `accessed_time` - Optional last access timestamp (ISO 8601 format)
/// * `modified_time` - Optional last modification timestamp (ISO 8601 format)
/// * `is_locked` - Whether the file was locked/in-use during collection
/// 
/// # Serialization
/// 
/// This struct supports JSON and other serde-compatible formats for easy
/// integration with analysis tools and long-term storage.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArtifactMetadata {
    pub original_path: String,
    pub collection_time: String,
    pub file_size: u64,
    pub created_time: Option<String>,
    pub accessed_time: Option<String>,
    pub modified_time: Option<String>,
    pub is_locked: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artifact_metadata_serialization() {
        let metadata = ArtifactMetadata {
            original_path: "/path/to/file.txt".to_string(),
            collection_time: "2024-01-01T00:00:00Z".to_string(),
            file_size: 1024,
            created_time: Some("2024-01-01T00:00:00Z".to_string()),
            accessed_time: Some("2024-01-01T00:00:00Z".to_string()),
            modified_time: Some("2024-01-01T00:00:00Z".to_string()),
            is_locked: false,
        };

        // Test JSON serialization
        let json = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains("original_path"));
        assert!(json.contains("/path/to/file.txt"));
        assert!(json.contains("1024"));

        // Test deserialization
        let deserialized: ArtifactMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.original_path, metadata.original_path);
        assert_eq!(deserialized.file_size, metadata.file_size);
        assert_eq!(deserialized.is_locked, metadata.is_locked);
    }

    #[test]
    fn test_artifact_metadata_with_none_values() {
        let metadata = ArtifactMetadata {
            original_path: "/test/file".to_string(),
            collection_time: "2024-01-01T00:00:00Z".to_string(),
            file_size: 0,
            created_time: None,
            accessed_time: None,
            modified_time: None,
            is_locked: true,
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: ArtifactMetadata = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.created_time, None);
        assert_eq!(deserialized.accessed_time, None);
        assert_eq!(deserialized.modified_time, None);
        assert!(deserialized.is_locked);
    }

    #[test]
    fn test_artifact_metadata_clone() {
        let original = ArtifactMetadata {
            original_path: "/path/to/file.txt".to_string(),
            collection_time: "2024-01-01T00:00:00Z".to_string(),
            file_size: 2048,
            created_time: Some("2023-12-01T00:00:00Z".to_string()),
            accessed_time: None,
            modified_time: Some("2023-12-15T00:00:00Z".to_string()),
            is_locked: false,
        };

        let cloned = original.clone();
        assert_eq!(cloned.original_path, original.original_path);
        assert_eq!(cloned.collection_time, original.collection_time);
        assert_eq!(cloned.file_size, original.file_size);
        assert_eq!(cloned.created_time, original.created_time);
        assert_eq!(cloned.accessed_time, original.accessed_time);
        assert_eq!(cloned.modified_time, original.modified_time);
        assert_eq!(cloned.is_locked, original.is_locked);
    }

    #[test]
    fn test_artifact_metadata_debug() {
        let metadata = ArtifactMetadata {
            original_path: "/debug/test".to_string(),
            collection_time: "2024-01-01T00:00:00Z".to_string(),
            file_size: 100,
            created_time: Some("2024-01-01T00:00:00Z".to_string()),
            accessed_time: None,
            modified_time: None,
            is_locked: false,
        };

        let debug_str = format!("{:?}", metadata);
        assert!(debug_str.contains("ArtifactMetadata"));
        assert!(debug_str.contains("original_path"));
        assert!(debug_str.contains("/debug/test"));
    }

    #[test]
    fn test_large_file_size() {
        let metadata = ArtifactMetadata {
            original_path: "/large/file.bin".to_string(),
            collection_time: "2024-01-01T00:00:00Z".to_string(),
            file_size: u64::MAX,
            created_time: None,
            accessed_time: None,
            modified_time: None,
            is_locked: false,
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: ArtifactMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.file_size, u64::MAX);
    }

    #[test]
    fn test_special_characters_in_path() {
        let metadata = ArtifactMetadata {
            original_path: "/path with spaces/special@chars#.txt".to_string(),
            collection_time: "2024-01-01T00:00:00Z".to_string(),
            file_size: 512,
            created_time: None,
            accessed_time: None,
            modified_time: None,
            is_locked: false,
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: ArtifactMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.original_path, "/path with spaces/special@chars#.txt");
    }

    #[test]
    fn test_yaml_serialization() {
        let metadata = ArtifactMetadata {
            original_path: "/yaml/test.yml".to_string(),
            collection_time: "2024-01-01T00:00:00Z".to_string(),
            file_size: 256,
            created_time: Some("2024-01-01T00:00:00Z".to_string()),
            accessed_time: Some("2024-01-01T01:00:00Z".to_string()),
            modified_time: Some("2024-01-01T00:30:00Z".to_string()),
            is_locked: true,
        };

        let yaml = serde_yaml::to_string(&metadata).unwrap();
        assert!(yaml.contains("original_path:"));
        assert!(yaml.contains("/yaml/test.yml"));
        
        let deserialized: ArtifactMetadata = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(deserialized.original_path, metadata.original_path);
        assert_eq!(deserialized.is_locked, metadata.is_locked);
    }

    #[test]
    fn test_empty_strings() {
        let metadata = ArtifactMetadata {
            original_path: "".to_string(),
            collection_time: "".to_string(),
            file_size: 0,
            created_time: Some("".to_string()),
            accessed_time: None,
            modified_time: None,
            is_locked: false,
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: ArtifactMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.original_path, "");
        assert_eq!(deserialized.collection_time, "");
        assert_eq!(deserialized.created_time, Some("".to_string()));
    }

    #[test]
    fn test_partial_deserialization() {
        // Test that we can deserialize even if some fields are missing (though this shouldn't happen with our struct)
        let json = r#"{
            "original_path": "/test",
            "collection_time": "2024-01-01T00:00:00Z",
            "file_size": 100,
            "created_time": null,
            "accessed_time": null,
            "modified_time": null,
            "is_locked": false
        }"#;

        let metadata: ArtifactMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(metadata.original_path, "/test");
        assert_eq!(metadata.file_size, 100);
        assert!(!metadata.is_locked);
    }
}
