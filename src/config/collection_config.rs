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
