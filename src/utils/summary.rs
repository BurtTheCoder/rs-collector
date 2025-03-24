use serde_json::json;
use uuid::Uuid;

use crate::models::ArtifactMetadata;
use crate::collectors::volatile::models::VolatileDataSummary;
use crate::collectors::memory::models::MemoryCollectionSummary;

/// Create a JSON summary of the collection
pub fn create_collection_summary(
    hostname: &str, 
    timestamp: &str, 
    artifacts: &[(String, ArtifactMetadata)],
    volatile_data_summary: Option<&VolatileDataSummary>,
    memory_collection_summary: Option<&MemoryCollectionSummary>
) -> String {
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
    
    serde_json::to_string_pretty(&summary).unwrap()
}
