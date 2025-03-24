use anyhow::{Result, Context};
use log::{info, warn, debug};
use std::process::Command;

/// Enable necessary macOS privileges for artifact collection
pub fn enable_privileges() -> Result<()> {
    // Check if running as root
    if !is_root() {
        warn!("Not running as root, some artifacts may be inaccessible");
    } else {
        info!("Running as root");
    }
    
    // Check for Full Disk Access
    if let Err(e) = check_full_disk_access() {
        warn!("May not have Full Disk Access: {}", e);
        warn!("Some protected files may be inaccessible");
    } else {
        debug!("Full Disk Access appears to be granted");
    }
    
    Ok(())
}

/// Check if the process is running as root
pub fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

/// Check if the process has Full Disk Access
fn check_full_disk_access() -> Result<()> {
    // Try to access a protected file that requires Full Disk Access
    // For example, try to read a file in /Library/Application Support/com.apple.TCC
    let test_path = "/Library/Application Support/com.apple.TCC/TCC.db";
    
    let output = Command::new("ls")
        .arg("-la")
        .arg(test_path)
        .output()
        .context("Failed to execute ls command")?;
    
    if !output.status.success() {
        return Err(anyhow::anyhow!("Cannot access TCC database, Full Disk Access may not be granted"));
    }
    
    // If we can list the file, we likely have Full Disk Access
    debug!("Successfully accessed TCC database, Full Disk Access appears to be granted");
    
    Ok(())
}

/// Request TCC permissions (this would typically be done via entitlements in the app bundle)
#[allow(dead_code)]
fn request_tcc_permissions() -> Result<()> {
    // In a real implementation, this would use the TCC framework to request permissions
    // However, this requires app bundle entitlements and cannot be done at runtime
    // For a command-line tool, the user must grant Full Disk Access manually
    
    warn!("To access protected files, grant Full Disk Access to this application in System Preferences > Security & Privacy > Privacy > Full Disk Access");
    
    Ok(())
}
