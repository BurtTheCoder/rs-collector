#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

use anyhow::Result;
use log::info;

/// Enable required privileges for the current platform
pub fn enable_required_privileges() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        info!("Enabling Windows privileges");
        windows::enable_privileges()
    }
    #[cfg(target_os = "linux")]
    {
        info!("Enabling Linux privileges");
        linux::enable_privileges()
    }
    #[cfg(target_os = "macos")]
    {
        info!("Enabling macOS privileges");
        macos::enable_privileges()
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        info!("No platform-specific privileges to enable");
        Ok(())
    }
}

/// Check if the process is running with elevated privileges
pub fn is_elevated() -> bool {
    #[cfg(target_os = "windows")]
    {
        windows::is_admin()
    }
    #[cfg(target_os = "linux")]
    {
        linux::is_root()
    }
    #[cfg(target_os = "macos")]
    {
        macos::is_root()
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        false
    }
}

/// Get instructions for elevating privileges on the current platform
pub fn get_elevation_instructions() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "Run as Administrator by right-clicking the executable and selecting 'Run as administrator'"
    }
    #[cfg(target_os = "linux")]
    {
        "Run with sudo: 'sudo ./rust_collector'"
    }
    #[cfg(target_os = "macos")]
    {
        "Run with sudo: 'sudo ./rust_collector'"
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        "Run with elevated privileges appropriate for your operating system"
    }
}
