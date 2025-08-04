use std::env;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=config/");
    println!("cargo:rerun-if-env-changed=RS_COLLECTOR_CONFIG");

    // Only run config embedding when feature is enabled
    if env::var("CARGO_FEATURE_EMBED_CONFIG").is_ok() {
        embed_appropriate_config()?;
    }

    Ok(())
}

fn embed_appropriate_config() -> Result<(), Box<dyn std::error::Error>> {
    // Determine target OS - use CARGO_CFG_TARGET_OS for cross-compilation
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_else(|_| env::consts::OS.to_string());

    // Normalize OS name
    let target_os = match target_os.as_str() {
        "macos" | "darwin" => "macos",
        "windows" => "windows",
        "linux" => "linux",
        _ => "generic",
    };

    println!("cargo:warning=Preparing config for target OS: {target_os}");

    // Config directory (create if needed)
    let config_dir = Path::new("config");
    fs::create_dir_all(config_dir)?;

    // OS-specific config file path
    let os_config_name = format!("default_{target_os}_config.yaml");
    let os_config_path = config_dir.join(&os_config_name);

    // Custom config source (from env var if provided)
    let custom_config = env::var("RS_COLLECTOR_CONFIG").ok();

    // Handle different config sources (priority order)
    if let Some(custom_path) = custom_config {
        let custom_path = Path::new(&custom_path);
        if custom_path.exists() {
            println!(
                "cargo:warning=Embedding custom config from {}",
                custom_path.display()
            );
            fs::copy(custom_path, &os_config_path)?;

            // Also copy to generic default config for fallback
            let generic_path = config_dir.join("default_config.yaml");
            fs::copy(&os_config_path, &generic_path)?;

            return Ok(());
        } else {
            println!(
                "cargo:warning=Specified custom config not found: {}",
                custom_path.display()
            );
            // Will fall back to next option
        }
    }

    // If OS-specific config doesn't exist yet, check if we have a template
    if !os_config_path.exists() {
        // Check for existing OS-specific config in the config directory
        let template_path = config_dir.join(format!("default_{target_os}_config.yaml"));
        if template_path.exists() {
            println!(
                "cargo:warning=Using existing OS-specific config: {}",
                template_path.display()
            );
            // No need to copy, it's already in the right place
        } else {
            // Check for generic default config
            let generic_path = config_dir.join("default_config.yaml");
            if generic_path.exists() {
                println!("cargo:warning=Using generic config as fallback");
                fs::copy(&generic_path, &os_config_path)?;
            } else {
                println!("cargo:warning=No config found, build may fail or use hardcoded defaults");
            }
        }
    }

    println!("cargo:warning=Config embedding completed for {target_os}");
    Ok(())
}
