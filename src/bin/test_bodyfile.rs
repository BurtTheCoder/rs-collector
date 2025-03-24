use std::path::PathBuf;
use std::collections::HashMap;
use anyhow::Result;
use rust_collector::utils::bodyfile;

fn main() -> Result<()> {
    // Create a bodyfile in the current directory
    let output_path = PathBuf::from("test_bodyfile.body");
    
    println!("Generating bodyfile at {}", output_path.display());
    
    // Set options for bodyfile generation
    let mut options = HashMap::new();
    options.insert("bodyfile_calculate_hash".to_string(), "true".to_string());
    options.insert("bodyfile_hash_max_size_mb".to_string(), "10".to_string());
    options.insert("bodyfile_use_iso8601".to_string(), "true".to_string());
    options.insert("bodyfile_skip_paths".to_string(), "/proc,/sys,/dev".to_string());
    
    // Generate a limited bodyfile for the current directory with options
    bodyfile::generate_limited_bodyfile_with_options(&output_path, &PathBuf::from("."), &options)?;
    
    println!("Bodyfile generation complete. File saved at {}", output_path.display());
    
    Ok(())
}
