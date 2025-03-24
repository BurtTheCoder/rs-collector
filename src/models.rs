use serde::{Serialize, Deserialize};

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
