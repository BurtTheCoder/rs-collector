use serde_json::json;
use uuid::Uuid;
use anyhow::{Result, Context};

use crate::models::ArtifactMetadata;
use crate::collectors::volatile::models::VolatileDataSummary;
use crate::collectors::memory::models::MemoryCollectionSummary;

/// Create a JSON summary of the collection.
/// 
/// Generates a comprehensive JSON report containing metadata about all collected
/// artifacts, system information, and collection statistics. This summary is
/// crucial for chain of custody and forensic analysis documentation.
/// 
/// # Arguments
/// 
/// * `hostname` - The hostname of the system where collection occurred
/// * `timestamp` - ISO 8601 formatted timestamp of when collection started
/// * `artifacts` - Vector of tuples containing (path, metadata) for each collected artifact
/// * `volatile_data_summary` - Optional summary of volatile data collection
/// * `memory_collection_summary` - Optional summary of memory collection
/// 
/// # Returns
/// 
/// * `Ok(String)` - JSON formatted summary as a string
/// * `Err` - If JSON serialization fails
/// 
/// # Example Output
/// 
/// ```json
/// {
///   "collection_id": "550e8400-e29b-41d4-a716-446655440000",
///   "hostname": "workstation-01",
///   "collection_timestamp": "2024-01-15T14:30:52Z",
///   "artifact_count": 42,
///   "artifacts": [...],
///   "volatile_data": {...},
///   "process_memory": {...}
/// }
/// ```
pub fn create_collection_summary(
    hostname: &str, 
    timestamp: &str, 
    artifacts: &[(String, ArtifactMetadata)],
    volatile_data_summary: Option<&VolatileDataSummary>,
    memory_collection_summary: Option<&MemoryCollectionSummary>
) -> Result<String> {
    let artifact_list: Vec<_> = artifacts.iter()
        .map(|(path, meta)| {
            json!({
                "path": path,
                "original_path": meta.original_path,
                "collection_time": meta.collection_time,
                "file_size": meta.file_size,
                "created_time": meta.created_time,
                "accessed_time": meta.accessed_time,
                "modified_time": meta.modified_time,
                "is_locked": meta.is_locked
            })
        })
        .collect();
    
    let mut summary = json!({
        "collection_id": Uuid::new_v4().to_string(),
        "hostname": hostname,
        "collection_time": timestamp,
        "os_version": std::env::consts::OS,
        "collector_version": env!("CARGO_PKG_VERSION"),
        "artifacts": artifact_list,
        "organization": "file_system_based" // Indicate the new organization method
    });
    
    // Add volatile data summary if available
    if let Some(vd_summary) = volatile_data_summary {
        let volatile_data = json!({
            "system_name": vd_summary.system_name,
            "os_version": vd_summary.os_version,
            "cpu_count": vd_summary.cpu_count,
            "total_memory_mb": vd_summary.total_memory_mb,
            "process_count": vd_summary.process_count,
            "network_interface_count": vd_summary.network_interface_count,
            "disk_count": vd_summary.disk_count,
            "files": [
                "volatile/system-info.json",
                "volatile/processes.json",
                "volatile/network-connections.json",
                "volatile/memory.json",
                "volatile/disks.json"
            ]
        });
        
        if let Some(obj) = summary.as_object_mut() {
            obj.insert("volatile_data".to_string(), volatile_data);
        }
    }
    
    // Add memory collection summary if available
    if let Some(mem_summary) = memory_collection_summary {
        let memory_data = json!({
            "processes_examined": mem_summary.processes_examined,
            "processes_collected": mem_summary.processes_collected,
            "processes_skipped": mem_summary.processes_skipped,
            "processes_failed": mem_summary.processes_failed,
            "total_memory_collected": mem_summary.total_memory_collected,
            "collection_start_time": mem_summary.start_time,
            "collection_end_time": mem_summary.end_time,
            "duration_seconds": mem_summary.duration_seconds,
            "summary_file": "process_memory/memory_collection_summary.json"
        });
        
        if let Some(obj) = summary.as_object_mut() {
            obj.insert("process_memory".to_string(), memory_data);
        }
    }
    
    serde_json::to_string_pretty(&summary).context("Failed to serialize collection summary to JSON")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::Value;

    fn create_test_artifact_metadata() -> ArtifactMetadata {
        ArtifactMetadata {
            original_path: "/test/path/file.txt".to_string(),
            collection_time: Utc::now().to_rfc3339(),
            file_size: 1024,
            created_time: Some(Utc::now().to_rfc3339()),
            accessed_time: Some(Utc::now().to_rfc3339()),
            modified_time: Some(Utc::now().to_rfc3339()),
            is_locked: false,
        }
    }

    fn create_test_volatile_summary() -> VolatileDataSummary {
        VolatileDataSummary {
            system_name: Some("test-system".to_string()),
            os_version: Some("Test OS 1.0".to_string()),
            cpu_count: 4,
            total_memory_mb: 8192,
            process_count: 100,
            network_interface_count: 3,
            disk_count: 2,
        }
    }

    fn create_test_memory_summary() -> MemoryCollectionSummary {
        use std::collections::HashMap;
        MemoryCollectionSummary {
            processes_examined: 50,
            processes_collected: 45,
            processes_skipped: 3,
            processes_failed: 2,
            total_memory_collected: 1024 * 1024 * 1024,
            start_time: "2024-01-01T00:00:00Z".to_string(),
            end_time: "2024-01-01T00:05:00Z".to_string(),
            duration_seconds: 300.0,
            process_summaries: HashMap::new(),
        }
    }

    #[test]
    fn test_basic_summary_creation() {
        let artifacts = vec![
            ("artifact1.txt".to_string(), create_test_artifact_metadata()),
            ("artifact2.log".to_string(), create_test_artifact_metadata()),
        ];

        let result = create_collection_summary(
            "test-host",
            "2024-01-01T00:00:00Z",
            &artifacts,
            None,
            None
        );

        assert!(result.is_ok());
        let json_str = result.unwrap();
        let json: Value = serde_json::from_str(&json_str).unwrap();

        // Verify basic fields
        assert_eq!(json["hostname"], "test-host");
        assert_eq!(json["collection_time"], "2024-01-01T00:00:00Z");
        assert_eq!(json["organization"], "file_system_based");
        assert!(json["collection_id"].is_string());
        assert!(json["collector_version"].is_string());
        assert!(json["os_version"].is_string());

        // Verify artifacts
        assert_eq!(json["artifacts"].as_array().unwrap().len(), 2);
        assert_eq!(json["artifacts"][0]["path"], "artifact1.txt");
        assert_eq!(json["artifacts"][1]["path"], "artifact2.log");

        // Verify no volatile or memory data
        assert!(json["volatile_data"].is_null());
        assert!(json["process_memory"].is_null());
    }

    #[test]
    fn test_summary_with_volatile_data() {
        let artifacts = vec![
            ("test.txt".to_string(), create_test_artifact_metadata()),
        ];
        let volatile_summary = create_test_volatile_summary();

        let result = create_collection_summary(
            "test-host",
            "2024-01-01T00:00:00Z",
            &artifacts,
            Some(&volatile_summary),
            None
        );

        assert!(result.is_ok());
        let json_str = result.unwrap();
        let json: Value = serde_json::from_str(&json_str).unwrap();

        // Verify volatile data
        assert!(!json["volatile_data"].is_null());
        assert_eq!(json["volatile_data"]["system_name"], "test-system");
        assert_eq!(json["volatile_data"]["os_version"], "Test OS 1.0");
        assert_eq!(json["volatile_data"]["cpu_count"], 4);
        assert_eq!(json["volatile_data"]["total_memory_mb"], 8192);
        assert_eq!(json["volatile_data"]["process_count"], 100);
        assert_eq!(json["volatile_data"]["network_interface_count"], 3);
        assert_eq!(json["volatile_data"]["disk_count"], 2);
        
        // Verify files array
        let files = json["volatile_data"]["files"].as_array().unwrap();
        assert_eq!(files.len(), 5);
        assert!(files.contains(&json!("volatile/system-info.json")));
    }

    #[test]
    fn test_summary_with_memory_data() {
        let artifacts = vec![
            ("test.txt".to_string(), create_test_artifact_metadata()),
        ];
        let memory_summary = create_test_memory_summary();

        let result = create_collection_summary(
            "test-host",
            "2024-01-01T00:00:00Z",
            &artifacts,
            None,
            Some(&memory_summary)
        );

        assert!(result.is_ok());
        let json_str = result.unwrap();
        let json: Value = serde_json::from_str(&json_str).unwrap();

        // Verify memory data
        assert!(!json["process_memory"].is_null());
        assert_eq!(json["process_memory"]["processes_examined"], 50);
        assert_eq!(json["process_memory"]["processes_collected"], 45);
        assert_eq!(json["process_memory"]["processes_skipped"], 3);
        assert_eq!(json["process_memory"]["processes_failed"], 2);
        assert_eq!(json["process_memory"]["total_memory_collected"], 1024 * 1024 * 1024);
        assert_eq!(json["process_memory"]["duration_seconds"], 300.0);
        assert_eq!(json["process_memory"]["summary_file"], "process_memory/memory_collection_summary.json");
    }

    #[test]
    fn test_summary_with_all_data() {
        let artifacts = vec![
            ("test1.txt".to_string(), create_test_artifact_metadata()),
            ("test2.log".to_string(), create_test_artifact_metadata()),
        ];
        let volatile_summary = create_test_volatile_summary();
        let memory_summary = create_test_memory_summary();

        let result = create_collection_summary(
            "test-host",
            "2024-01-01T00:00:00Z",
            &artifacts,
            Some(&volatile_summary),
            Some(&memory_summary)
        );

        assert!(result.is_ok());
        let json_str = result.unwrap();
        let json: Value = serde_json::from_str(&json_str).unwrap();

        // Verify all sections are present
        assert!(json["collection_id"].is_string());
        assert_eq!(json["artifacts"].as_array().unwrap().len(), 2);
        assert!(!json["volatile_data"].is_null());
        assert!(!json["process_memory"].is_null());
    }

    #[test]
    fn test_empty_artifacts_list() {
        let artifacts = vec![];

        let result = create_collection_summary(
            "test-host",
            "2024-01-01T00:00:00Z",
            &artifacts,
            None,
            None
        );

        assert!(result.is_ok());
        let json_str = result.unwrap();
        let json: Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json["artifacts"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_unique_collection_ids() {
        let artifacts = vec![
            ("test.txt".to_string(), create_test_artifact_metadata()),
        ];

        // Create two summaries
        let result1 = create_collection_summary(
            "test-host",
            "2024-01-01T00:00:00Z",
            &artifacts,
            None,
            None
        ).unwrap();

        let result2 = create_collection_summary(
            "test-host",
            "2024-01-01T00:00:00Z",
            &artifacts,
            None,
            None
        ).unwrap();

        let json1: Value = serde_json::from_str(&result1).unwrap();
        let json2: Value = serde_json::from_str(&result2).unwrap();

        // Collection IDs should be different
        assert_ne!(json1["collection_id"], json2["collection_id"]);
    }

    #[test]
    fn test_special_characters_in_paths() {
        let mut metadata = create_test_artifact_metadata();
        metadata.original_path = "/path/with spaces/and-special@chars#.txt".to_string();
        
        let artifacts = vec![
            ("artifact with spaces.txt".to_string(), metadata),
        ];

        let result = create_collection_summary(
            "host-name-123",
            "2024-01-01T00:00:00Z",
            &artifacts,
            None,
            None
        );

        assert!(result.is_ok());
        let json_str = result.unwrap();
        let json: Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json["artifacts"][0]["path"], "artifact with spaces.txt");
        assert_eq!(json["artifacts"][0]["original_path"], "/path/with spaces/and-special@chars#.txt");
    }

    #[test]
    fn test_artifact_metadata_fields() {
        let metadata = create_test_artifact_metadata();
        let artifacts = vec![
            ("test.txt".to_string(), metadata.clone()),
        ];

        let result = create_collection_summary(
            "test-host",
            "2024-01-01T00:00:00Z",
            &artifacts,
            None,
            None
        );

        assert!(result.is_ok());
        let json_str = result.unwrap();
        let json: Value = serde_json::from_str(&json_str).unwrap();

        let artifact = &json["artifacts"][0];
        assert_eq!(artifact["original_path"], metadata.original_path);
        assert_eq!(artifact["file_size"], metadata.file_size);
        assert_eq!(artifact["is_locked"], metadata.is_locked);
        assert!(artifact["collection_time"].is_string());
        assert!(artifact["created_time"].is_string());
        assert!(artifact["accessed_time"].is_string());
        assert!(artifact["modified_time"].is_string());
    }

    #[test]
    fn test_json_pretty_formatting() {
        let artifacts = vec![
            ("test.txt".to_string(), create_test_artifact_metadata()),
        ];

        let result = create_collection_summary(
            "test-host",
            "2024-01-01T00:00:00Z",
            &artifacts,
            None,
            None
        );

        assert!(result.is_ok());
        let json_str = result.unwrap();
        
        // Pretty formatting should include newlines and indentation
        assert!(json_str.contains('\n'));
        assert!(json_str.contains("  ")); // Indentation
    }
}
