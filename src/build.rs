use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, Context, anyhow};
use log::{info, warn};

/// Build a binary with embedded configuration
/// 
/// This function handles the entire build process:
/// 1. Determines the target OS and architecture
/// 2. Copies the config file to the appropriate location
/// 3. Builds the binary with embedded configuration
/// 4. Copies the binary to the specified output location
pub fn build_binary_with_config(
    config_path: &Path, 
    output_path: Option<&Path>, 
    binary_name: Option<&str>,
    target_os: Option<&str>
) -> Result<PathBuf> {
    // Determine target OS and triple
    let target_os_normalized = match target_os.map(|s| s.to_lowercase()).as_deref() {
        Some("windows") | Some("win") => "windows".to_string(),
        Some("linux") => "linux".to_string(),
        Some("macos") | Some("darwin") => "macos".to_string(),
        Some(other) => return Err(anyhow!("Unsupported target OS: {}", other)),
        None => std::env::consts::OS.to_string(),
    };
    
    let target_triple = match target_os_normalized.as_str() {
        "windows" => "x86_64-pc-windows-msvc",
        "linux" => "x86_64-unknown-linux-gnu", 
        "macos" => "x86_64-apple-darwin",
        _ => return Err(anyhow!("Unsupported target OS: {}", target_os_normalized)),
    };
    
    // Prepare output name with appropriate extension
    let default_name = format!("rust_collector_{}", target_os_normalized);
    let mut final_name = binary_name.unwrap_or(&default_name).to_string();
    if target_os_normalized == "windows" && !final_name.ends_with(".exe") {
        final_name.push_str(".exe");
    }
    
    // Determine output path
    let output_file = match output_path {
        Some(path) => {
            let mut path_buf = path.to_path_buf();
            // Create directory if it doesn't exist
            if let Some(parent) = path_buf.parent() {
                fs::create_dir_all(parent)
                    .context(format!("Failed to create output directory: {}", parent.display()))?;
            }
            
            // If output_path is a directory, append the filename
            if path_buf.is_dir() {
                path_buf.push(&final_name);
            }
            path_buf
        },
        None => PathBuf::from(&final_name),
    };
    
    // Ensure config directory exists
    let config_dir = PathBuf::from("config");
    fs::create_dir_all(&config_dir)
        .context("Failed to create config directory")?;
    
    // Copy configuration file to OS-specific location
    let os_config_path = config_dir.join(format!("default_{}_config.yaml", target_os_normalized));
    fs::copy(config_path, &os_config_path)
        .context(format!("Failed to copy config to {}", os_config_path.display()))?;
    
    // Also copy to generic default config for fallback
    let generic_config_path = config_dir.join("default_config.yaml");
    fs::copy(config_path, &generic_config_path)
        .context(format!("Failed to copy config to {}", generic_config_path.display()))?;
    
    info!("Building for {} with target {}", target_os_normalized, target_triple);
    info!("Output will be saved to: {}", output_file.display());
    
    // Run cargo build
    let status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--features=embed_config")
        .arg("--target").arg(target_triple)
        .status()
        .context("Failed to execute cargo build")?;
    
    if !status.success() {
        return Err(anyhow!("Build failed with status: {}", status));
    }
    
    // Copy the binary to the final location
    let source_path = format!("target/{}/release/rust_collector{}", 
        target_triple, 
        if target_os_normalized == "windows" { ".exe" } else { "" }
    );
    
    fs::copy(&source_path, &output_file)
        .context(format!("Failed to copy binary from {} to {}", source_path, output_file.display()))?;
    
    info!("Build completed successfully. Binary available at: {}", output_file.display());
    Ok(output_file)
}

/// For backward compatibility - generates a build script but warns that it's deprecated
pub fn generate_build_script(
    config_path: &Path, 
    output_path: Option<&Path>, 
    binary_name: Option<&str>,
    target_os: Option<&str>
) -> Result<PathBuf> {
    warn!("generate_build_script is deprecated, use build_binary_with_config instead");
    
    // Determine the target OS (default to current OS)
    let target_os_str = target_os.unwrap_or(std::env::consts::OS);
    
    // Build the binary directly
    let output_file = build_binary_with_config(
        config_path,
        output_path,
        binary_name,
        Some(target_os_str)
    )?;
    
    // Return the path to the binary instead of a script
    Ok(output_file)
}

/// For backward compatibility - executes a build script but warns that it's deprecated
pub fn execute_build_script(script_path: &Path) -> Result<()> {
    warn!("execute_build_script is deprecated, use build_binary_with_config instead");
    warn!("The path provided ({}) is not a script but will be treated as a binary", script_path.display());
    
    // Just return success since the binary should already be built
    Ok(())
}
