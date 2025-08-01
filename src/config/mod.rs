// Re-export all items from the submodules
mod artifact_types;
mod collection_config;
mod default_configs;
mod env_vars;
mod regex_config;

// Re-export artifact types
pub use artifact_types::{
    ArtifactType,
    WindowsArtifactType,
    LinuxArtifactType,
    MacOSArtifactType,
    VolatileDataType,
};

// Re-export collection config
pub use collection_config::{
    Artifact,
    CollectionConfig,
    load_or_create_config,
};

// Re-export environment variable functions
pub use env_vars::{
    parse_windows_env_vars,
    parse_unix_env_vars,
};

// Re-export regex config
pub use regex_config::RegexConfig;
