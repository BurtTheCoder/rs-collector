use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, Context, anyhow};
use log::info;

/// Generate a build script to build a binary with embedded configuration
pub fn generate_build_script(
    config_path: &Path, 
    output_path: Option<&Path>, 
    binary_name: Option<&str>,
    target_os: Option<&str>
) -> Result<PathBuf> {
    // Determine the target OS (default to current OS)
    let target_os = target_os.unwrap_or(std::env::consts::OS);
    
    // Determine the script path
    let script_path = match output_path {
        Some(path) => path.join(format!("build_{}.sh", target_os)),
        None => PathBuf::from(format!("build_{}.sh", target_os)),
    };
    
    // Create directory if it doesn't exist
    if let Some(parent) = script_path.parent() {
        fs::create_dir_all(parent)
            .context(format!("Failed to create directory: {}", parent.display()))?;
    }
    
    // Determine the binary name - store in a variable to avoid temporary value issue
    let default_name = format!("rust_collector_{}", target_os);
    let binary_name = binary_name.unwrap_or(&default_name);
    
    // Create the build script with target-specific options
    let script_content = match target_os {
        "windows" => format!(
            r#"#!/bin/bash
set -e

# Windows build script for embedded configuration rust_collector
# Generated automatically

# Ensure config directory exists
mkdir -p config/

# Copy configuration file
cp "{}" config/default_windows_config.yaml

# Build with embedded configuration
cargo build --release --features="embed_config" --target x86_64-pc-windows-gnu

# Copy the binary with the specified name
cp target/x86_64-pc-windows-gnu/release/rust_collector.exe {}.exe

echo "Build completed successfully. Binary created at: {}.exe"
"#,
            config_path.display(),
            binary_name,
            binary_name
        ),
        "linux" => format!(
            r#"#!/bin/bash
set -e

# Linux build script for embedded configuration rust_collector
# Generated automatically

# Ensure config directory exists
mkdir -p config/

# Copy configuration file
cp "{}" config/default_linux_config.yaml

# Build with embedded configuration
cargo build --release --features="embed_config" --target x86_64-unknown-linux-gnu

# Copy the binary with the specified name
cp target/x86_64-unknown-linux-gnu/release/rust_collector {}

echo "Build completed successfully. Binary created at: {}"
"#,
            config_path.display(),
            binary_name,
            binary_name
        ),
        "macos" => format!(
            r#"#!/bin/bash
set -e

# macOS build script for embedded configuration rust_collector
# Generated automatically

# Ensure config directory exists
mkdir -p config/

# Copy configuration file
cp "{}" config/default_macos_config.yaml

# Build with embedded configuration
cargo build --release --features="embed_config" --target x86_64-apple-darwin

# Copy the binary with the specified name
cp target/x86_64-apple-darwin/release/rust_collector {}

echo "Build completed successfully. Binary created at: {}"
"#,
            config_path.display(),
            binary_name,
            binary_name
        ),
        _ => return Err(anyhow!("Unsupported target OS: {}", target_os)),
    };
    
    // Write the build script
    fs::write(&script_path, script_content)
        .context(format!("Failed to write build script to {}", script_path.display()))?;
    
    // Make the script executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms)?;
    }
    
    info!("Build script for {} generated at {}", target_os, script_path.display());
    
    Ok(script_path)
}

/// Execute the build script
pub fn execute_build_script(script_path: &Path) -> Result<()> {
    info!("Executing build script: {}", script_path.display());
    
    let output = Command::new(script_path)
        .output()
        .context("Failed to execute build script")?;
    
    if output.status.success() {
        info!("Build completed successfully");
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("Build failed: {}", error))
    }
}
