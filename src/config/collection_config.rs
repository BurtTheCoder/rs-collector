use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use log::{debug, info};
use serde::{Serialize, Deserialize};

use crate::config::artifact_types::ArtifactType;
use crate::config::env_vars::{parse_windows_env_vars, parse_unix_env_vars, normalize_path_for_os};
use crate::config::regex_config::RegexConfig;

// Include default config at compile time
#[cfg(feature = "embed_config")]
use include_dir::{include_dir, Dir};

#[cfg(feature = "embed_config")]
static CONFIG_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/config");

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Artifact {
    pub name: String,
    pub artifact_type: ArtifactType,
    pub source_path: String,
    pub destination_name: String,
    pub description: Option<String>,
    pub required: bool,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    #[serde(default)]
    pub regex: Option<RegexConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CollectionConfig {
    pub version: String,
    pub description: String,
    pub artifacts: Vec<Artifact>,
    #[serde(default)]
    pub global_options: HashMap<String, String>,
}

impl Default for CollectionConfig {
    fn default() -> Self {
        match std::env::consts::OS {
            "windows" => Self::default_windows(),
            "linux" => Self::default_linux(),
            "macos" => Self::default_macos(),
            _ => Self::default_minimal(),
        }
    }
}

impl CollectionConfig {
    /// Load configuration from a YAML file
    pub fn from_yaml_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .context(format!("Failed to read config file: {}", path.display()))?;
        
        let config: CollectionConfig = serde_yaml::from_str(&content)
            .context("Failed to parse YAML config")?;
        
        debug!("Loaded configuration from {}", path.display());
        Ok(config)
    }
    
    /// Save configuration to a YAML file
    pub fn save_to_yaml_file(&self, path: &Path) -> Result<()> {
        let yaml = serde_yaml::to_string(self)
            .context("Failed to serialize config to YAML")?;
        
        fs::write(path, yaml)
            .context(format!("Failed to write config to {}", path.display()))?;
        
        info!("Saved configuration to {}", path.display());
        Ok(())
    }
    
    /// Get the embedded default configuration
    #[cfg(feature = "embed_config")]
    pub fn get_embedded_config() -> Result<Self> {
        let os_name = std::env::consts::OS;
        
        // Try to find OS-specific config first
        let os_config_path = format!("default_{}_config.yaml", os_name);
        
        if let Some(file) = CONFIG_DIR.get_file(&os_config_path) {
            let content = file.contents_utf8()
                .ok_or_else(|| anyhow::anyhow!("Failed to read embedded OS-specific config as UTF-8"))?;
            
            let config: CollectionConfig = serde_yaml::from_str(content)
                .context("Failed to parse embedded OS-specific YAML config")?;
            
            info!("Using embedded OS-specific configuration for {}", os_name);
            Ok(config)
        } else if let Some(file) = CONFIG_DIR.get_file("default_config.yaml") {
            // Fall back to generic config
            let content = file.contents_utf8()
                .ok_or_else(|| anyhow::anyhow!("Failed to read embedded config as UTF-8"))?;
            
            let config: CollectionConfig = serde_yaml::from_str(content)
                .context("Failed to parse embedded YAML config")?;
            
            info!("Using generic embedded configuration");
            Ok(config)
        } else {
            info!("No embedded config found, using default for {}", os_name);
            Ok(Self::default())
        }
    }
    
    /// Process environment variables in paths
    /// Handles both Windows (%VAR%) and Unix ($VAR) style variables
    pub fn process_environment_variables(&mut self) -> Result<()> {
        for artifact in &mut self.artifacts {
            // Process Windows-style %VARIABLE% environment variables
            if artifact.source_path.contains('%') {
                let processed_path = parse_windows_env_vars(&artifact.source_path);
                artifact.source_path = processed_path;
            }
            
            // Process Unix-style $VARIABLE and ${VARIABLE} environment variables
            if artifact.source_path.contains('$') {
                let processed_path = parse_unix_env_vars(&artifact.source_path);
                artifact.source_path = processed_path;
            }
            
            // Normalize path separators for the current OS
            artifact.source_path = normalize_path_for_os(&artifact.source_path);
        }
        
        Ok(())
    }
    
    /// Create a default configuration YAML file
    pub fn create_default_config_file(path: &Path) -> Result<()> {
        let default_config = CollectionConfig::default();
        default_config.save_to_yaml_file(path)
    }
    
    /// Create an OS-specific default configuration file
    pub fn create_os_specific_config_file(path: &Path, target_os: &str) -> Result<()> {
        let config = match target_os {
            "windows" => Self::default_windows(),
            "linux" => Self::default_linux(),
            "macos" => Self::default_macos(),
            _ => Self::default_minimal(),
        };
        
        config.save_to_yaml_file(path)
    }
}

/// Load a configuration file or create a default one.
/// 
/// This function attempts to load a configuration in the following order:
/// 1. From the specified path if provided and exists
/// 2. From an OS-specific default config file if no path provided
/// 3. Creates a new default configuration if no files exist
/// 
/// # Arguments
/// 
/// * `config_path` - Optional path to a configuration file
/// 
/// # Returns
/// 
/// * `Ok(CollectionConfig)` - The loaded or created configuration
/// * `Err` - If config file exists but cannot be parsed
/// 
/// # Platform-Specific Behavior
/// 
/// When no config path is provided, the function looks for:
/// - Windows: `config/windows_default.yaml`
/// - Linux: `config/linux_default.yaml`
/// - macOS: `config/macos_default.yaml`
/// - Other: `config/default.yaml`
pub fn load_or_create_config(config_path: Option<&Path>) -> Result<CollectionConfig> {
    match config_path {
        Some(path) => {
            if path.exists() {
                CollectionConfig::from_yaml_file(path)
            } else {
                // Try to find an OS-specific default config
                let os_specific_path = match std::env::consts::OS {
                    "windows" => Path::new("config/windows_default.yaml"),
                    "linux" => Path::new("config/linux_default.yaml"),
                    "macos" => Path::new("config/macos_default.yaml"),
                    _ => Path::new("config/default.yaml"),
                };
                
                if os_specific_path.exists() {
                    info!("Using OS-specific default config: {}", os_specific_path.display());
                    CollectionConfig::from_yaml_file(os_specific_path)
                } else {
                    info!("Creating default config for {}", std::env::consts::OS);
                    let default_config = CollectionConfig::default();
                    default_config.save_to_yaml_file(path)?;
                    Ok(default_config)
                }
            }
        },
        None => {
            // Try embedded config if feature is enabled
            #[cfg(feature = "embed_config")]
            {
                CollectionConfig::get_embedded_config()
            }
            
            // Otherwise use default
            #[cfg(not(feature = "embed_config"))]
            {
                info!("No config path provided, using default configuration for {}", std::env::consts::OS);
                Ok(CollectionConfig::default())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{TempDir, NamedTempFile};
    use std::fs;

    fn create_test_artifact() -> Artifact {
        Artifact {
            name: "test_artifact".to_string(),
            artifact_type: ArtifactType::Logs,
            source_path: "/var/log/test.log".to_string(),
            destination_name: "test.log".to_string(),
            description: Some("Test artifact".to_string()),
            required: true,
            metadata: HashMap::new(),
            regex: None,
        }
    }

    fn create_test_config() -> CollectionConfig {
        CollectionConfig {
            version: "1.0".to_string(),
            description: "Test configuration".to_string(),
            artifacts: vec![create_test_artifact()],
            global_options: HashMap::new(),
        }
    }

    #[test]
    fn test_config_serialization_deserialization() {
        let config = create_test_config();
        
        // Serialize to YAML
        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("version: '1.0'"));
        assert!(yaml.contains("test_artifact"));
        
        // Deserialize back
        let deserialized: CollectionConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(deserialized.version, config.version);
        assert_eq!(deserialized.artifacts.len(), 1);
        assert_eq!(deserialized.artifacts[0].name, "test_artifact");
    }

    #[test]
    fn test_save_and_load_yaml_file() {
        let config = create_test_config();
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.yaml");
        
        // Save to file
        config.save_to_yaml_file(&config_path).unwrap();
        assert!(config_path.exists());
        
        // Load from file
        let loaded = CollectionConfig::from_yaml_file(&config_path).unwrap();
        assert_eq!(loaded.version, config.version);
        assert_eq!(loaded.artifacts.len(), config.artifacts.len());
    }

    #[test]
    fn test_default_configs() {
        // Test Windows default
        let windows_config = CollectionConfig::default_windows();
        assert!(windows_config.version == "1.0");
        assert!(windows_config.artifacts.iter().any(|a| a.name == "MFT"));
        assert!(windows_config.artifacts.iter().any(|a| a.name == "SYSTEM"));
        
        // Test Linux default
        let linux_config = CollectionConfig::default_linux();
        assert!(linux_config.artifacts.iter().any(|a| a.name == "syslog"));
        assert!(linux_config.artifacts.iter().any(|a| a.name == "auth.log"));
        
        // Test macOS default
        let macos_config = CollectionConfig::default_macos();
        assert!(macos_config.artifacts.iter().any(|a| a.name == "unified_logs"));
        assert!(macos_config.artifacts.iter().any(|a| a.name == "fseventsd"));
        
        // Test minimal default
        let minimal_config = CollectionConfig::default_minimal();
        assert!(minimal_config.artifacts.len() >= 2);
    }

    #[test]
    fn test_process_environment_variables() {
        let mut config = CollectionConfig {
            version: "1.0".to_string(),
            description: "Test".to_string(),
            artifacts: vec![
                Artifact {
                    name: "windows_env".to_string(),
                    artifact_type: ArtifactType::Logs,
                    source_path: "%TEMP%/test.log".to_string(),
                    destination_name: "test.log".to_string(),
                    description: None,
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
                Artifact {
                    name: "unix_env".to_string(),
                    artifact_type: ArtifactType::Logs,
                    source_path: "$HOME/test.log".to_string(),
                    destination_name: "test.log".to_string(),
                    description: None,
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
            ],
            global_options: HashMap::new(),
        };
        
        // Set test environment variables
        std::env::set_var("TEMP", "/tmp");
        std::env::set_var("HOME", "/home/user");
        
        config.process_environment_variables().unwrap();
        
        // Check that variables were expanded
        assert!(!config.artifacts[0].source_path.contains("%TEMP%"));
        assert!(!config.artifacts[1].source_path.contains("$HOME"));
    }

    #[test]
    fn test_load_or_create_config_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("existing.yaml");
        
        // Create a config file
        let test_config = create_test_config();
        test_config.save_to_yaml_file(&config_path).unwrap();
        
        // Load it
        let loaded = load_or_create_config(Some(&config_path)).unwrap();
        assert_eq!(loaded.version, test_config.version);
    }

    #[test]
    fn test_load_or_create_config_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("new.yaml");
        
        // Load non-existent file (should create default)
        let loaded = load_or_create_config(Some(&config_path)).unwrap();
        assert!(config_path.exists());
        assert_eq!(loaded.version, "1.0");
    }

    #[test]
    fn test_load_or_create_config_no_path() {
        // Load with no path (should use default)
        let loaded = load_or_create_config(None).unwrap();
        assert_eq!(loaded.version, "1.0");
    }

    #[test]
    fn test_create_os_specific_config_file() {
        let temp_dir = TempDir::new().unwrap();
        
        // Test Windows config
        let windows_path = temp_dir.path().join("windows.yaml");
        CollectionConfig::create_os_specific_config_file(&windows_path, "windows").unwrap();
        assert!(windows_path.exists());
        let windows_config = CollectionConfig::from_yaml_file(&windows_path).unwrap();
        assert!(windows_config.artifacts.iter().any(|a| a.name == "MFT"));
        
        // Test Linux config
        let linux_path = temp_dir.path().join("linux.yaml");
        CollectionConfig::create_os_specific_config_file(&linux_path, "linux").unwrap();
        assert!(linux_path.exists());
        let linux_config = CollectionConfig::from_yaml_file(&linux_path).unwrap();
        assert!(linux_config.artifacts.iter().any(|a| a.name == "syslog"));
    }

    #[test]
    fn test_artifact_with_regex() {
        let artifact = Artifact {
            name: "logs_with_pattern".to_string(),
            artifact_type: ArtifactType::Logs,
            source_path: "/var/log".to_string(),
            destination_name: "filtered_logs".to_string(),
            description: Some("Logs matching pattern".to_string()),
            required: false,
            metadata: HashMap::new(),
            regex: Some(RegexConfig {
                enabled: true,
                recursive: true,
                include_pattern: "error|warn".to_string(),
                exclude_pattern: "debug".to_string(),
                max_depth: Some(5),
            }),
        };
        
        // Serialize and deserialize
        let yaml = serde_yaml::to_string(&artifact).unwrap();
        let deserialized: Artifact = serde_yaml::from_str(&yaml).unwrap();
        
        assert!(deserialized.regex.is_some());
        let regex = deserialized.regex.unwrap();
        assert!(regex.enabled);
        assert!(regex.recursive);
        assert_eq!(regex.include_pattern, "error|warn");
        assert_eq!(regex.exclude_pattern, "debug");
        assert_eq!(regex.max_depth, Some(5));
    }

    #[test]
    fn test_invalid_yaml_error() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), "invalid: yaml: content:").unwrap();
        
        let result = CollectionConfig::from_yaml_file(temp_file.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to parse YAML"));
    }

    #[test]
    fn test_path_normalization() {
        let mut config = CollectionConfig {
            version: "1.0".to_string(),
            description: "Test".to_string(),
            artifacts: vec![
                Artifact {
                    name: "mixed_separators".to_string(),
                    artifact_type: ArtifactType::Logs,
                    source_path: "C:\\Users\\test/Documents\\file.txt".to_string(),
                    destination_name: "file.txt".to_string(),
                    description: None,
                    required: false,
                    metadata: HashMap::new(),
                    regex: None,
                },
            ],
            global_options: HashMap::new(),
        };
        
        config.process_environment_variables().unwrap();
        
        // Path should be normalized for the current OS
        let normalized_path = &config.artifacts[0].source_path;
        if cfg!(windows) {
            assert!(!normalized_path.contains('/'));
        } else {
            assert!(!normalized_path.contains('\\'));
        }
    }
}
