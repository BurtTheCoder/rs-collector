use anyhow::Result;
use log::{info, warn};

/// Enable necessary Windows privileges for artifact collection
#[allow(dead_code)]
pub fn enable_privileges() -> Result<()> {
    // Check if running as administrator
    let is_admin = is_admin();

    if !is_admin {
        warn!("Not running as Administrator, some artifacts may be inaccessible");
    } else {
        info!("Running as Administrator");
    }

    // Import the existing Windows privilege code
    crate::windows::enable_privileges()?;

    Ok(())
}

/// Check if the process is running as administrator
#[cfg(target_os = "windows")]
#[allow(dead_code)]
pub fn is_admin() -> bool {
    use winapi::um::shellapi::IsUserAnAdmin;
    unsafe { IsUserAnAdmin() != 0 }
}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn is_admin() -> bool {
    // On non-Windows platforms, this is just a mock
    false
}
