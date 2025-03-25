use serde::{Serialize, Deserialize};

/// Configuration for regex-based artifact collection
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RegexConfig {
    /// Whether regex matching is enabled for this artifact
    #[serde(default)]
    pub enabled: bool,
    
    /// Whether to recursively search directories
    #[serde(default)]
    pub recursive: bool,
    
    /// Regex pattern for files to include
    #[serde(default = "default_include_pattern")]
    pub include_pattern: String,
    
    /// Regex pattern for files to exclude
    #[serde(default)]
    pub exclude_pattern: String,
    
    /// Maximum directory depth for recursive searches
    #[serde(default)]
    pub max_depth: Option<usize>,
}

/// Default include pattern matches everything
fn default_include_pattern() -> String {
    ".*".to_string()
}
