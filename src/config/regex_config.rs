use serde::{Deserialize, Serialize};

/// Configuration for regex-based artifact collection
#[derive(Debug, Serialize, Deserialize, Clone)]
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

impl Default for RegexConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            recursive: false,
            include_pattern: default_include_pattern(),
            exclude_pattern: String::new(),
            max_depth: None,
        }
    }
}

/// Default include pattern matches everything
fn default_include_pattern() -> String {
    ".*".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_config_default() {
        let config = RegexConfig::default();

        // Test default values
        assert!(!config.enabled);
        assert!(!config.recursive);
        assert_eq!(config.include_pattern, ".*");
        assert_eq!(config.exclude_pattern, "");
        assert_eq!(config.max_depth, None);
    }

    #[test]
    fn test_regex_config_serialization() {
        let config = RegexConfig {
            enabled: true,
            recursive: true,
            include_pattern: r"\.log$".to_string(),
            exclude_pattern: r"\.tmp$".to_string(),
            max_depth: Some(5),
        };

        // Test JSON serialization
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("enabled"));
        assert!(json.contains(r"\.log$"));

        // Test deserialization
        let deserialized: RegexConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.enabled, config.enabled);
        assert_eq!(deserialized.recursive, config.recursive);
        assert_eq!(deserialized.include_pattern, config.include_pattern);
        assert_eq!(deserialized.exclude_pattern, config.exclude_pattern);
        assert_eq!(deserialized.max_depth, config.max_depth);
    }

    #[test]
    fn test_regex_config_yaml_serialization() {
        let config = RegexConfig {
            enabled: true,
            recursive: false,
            include_pattern: r"error|warn".to_string(),
            exclude_pattern: r"debug".to_string(),
            max_depth: Some(3),
        };

        // Test YAML serialization
        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("enabled: true"));
        assert!(yaml.contains("recursive: false"));
        assert!(yaml.contains("include_pattern"));

        // Test deserialization
        let deserialized: RegexConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(deserialized.enabled, config.enabled);
        assert_eq!(deserialized.include_pattern, config.include_pattern);
    }

    #[test]
    fn test_regex_config_partial_deserialization() {
        // Test that missing fields use defaults
        let yaml = r#"
enabled: true
include_pattern: "*.txt"
"#;

        let config: RegexConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.enabled);
        assert_eq!(config.include_pattern, "*.txt");
        assert!(!config.recursive); // Should use default
        assert_eq!(config.exclude_pattern, ""); // Should use default
        assert_eq!(config.max_depth, None); // Should use default
    }

    #[test]
    fn test_default_include_pattern() {
        // Test the default pattern function
        assert_eq!(default_include_pattern(), ".*");

        // Test that deserialization without include_pattern uses default
        let yaml = r#"
enabled: true
recursive: true
"#;

        let config: RegexConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.include_pattern, ".*");
    }

    #[test]
    fn test_regex_config_clone() {
        let original = RegexConfig {
            enabled: true,
            recursive: true,
            include_pattern: "test".to_string(),
            exclude_pattern: "exclude".to_string(),
            max_depth: Some(10),
        };

        let cloned = original.clone();
        assert_eq!(cloned.enabled, original.enabled);
        assert_eq!(cloned.recursive, original.recursive);
        assert_eq!(cloned.include_pattern, original.include_pattern);
        assert_eq!(cloned.exclude_pattern, original.exclude_pattern);
        assert_eq!(cloned.max_depth, original.max_depth);
    }

    #[test]
    fn test_regex_patterns_with_special_chars() {
        let config = RegexConfig {
            enabled: true,
            recursive: false,
            include_pattern: r"^[a-z]+\.(log|txt)$".to_string(),
            exclude_pattern: r"(temp|tmp|cache).*".to_string(),
            max_depth: None,
        };

        // Ensure special regex characters are preserved
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: RegexConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.include_pattern, r"^[a-z]+\.(log|txt)$");
        assert_eq!(deserialized.exclude_pattern, r"(temp|tmp|cache).*");
    }

    #[test]
    fn test_max_depth_variations() {
        // Test None
        let config1 = RegexConfig {
            enabled: true,
            recursive: true,
            include_pattern: ".*".to_string(),
            exclude_pattern: "".to_string(),
            max_depth: None,
        };

        let yaml1 = serde_yaml::to_string(&config1).unwrap();
        assert!(yaml1.contains("max_depth:") || !yaml1.contains("max_depth"));

        // Test Some(0)
        let config2 = RegexConfig {
            enabled: true,
            recursive: true,
            include_pattern: ".*".to_string(),
            exclude_pattern: "".to_string(),
            max_depth: Some(0),
        };

        let yaml2 = serde_yaml::to_string(&config2).unwrap();
        let deserialized2: RegexConfig = serde_yaml::from_str(&yaml2).unwrap();
        assert_eq!(deserialized2.max_depth, Some(0));

        // Test large depth
        let config3 = RegexConfig {
            enabled: true,
            recursive: true,
            include_pattern: ".*".to_string(),
            exclude_pattern: "".to_string(),
            max_depth: Some(999),
        };

        let yaml3 = serde_yaml::to_string(&config3).unwrap();
        let deserialized3: RegexConfig = serde_yaml::from_str(&yaml3).unwrap();
        assert_eq!(deserialized3.max_depth, Some(999));
    }

    #[test]
    fn test_empty_patterns() {
        let config = RegexConfig {
            enabled: true,
            recursive: false,
            include_pattern: "".to_string(),
            exclude_pattern: "".to_string(),
            max_depth: None,
        };

        // Empty patterns should be preserved
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: RegexConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.include_pattern, "");
        assert_eq!(deserialized.exclude_pattern, "");
    }
}
