#[cfg(target_os = "windows")]
pub mod raw_access;
#[cfg(target_os = "windows")]
mod privileges;

#[cfg(not(target_os = "windows"))]
mod mock_impl;

#[cfg(target_os = "windows")]
pub use privileges::enable_privileges;
#[cfg(target_os = "windows")]
pub use raw_access::collect_with_raw_handle;
#[cfg(target_os = "windows")]
pub use raw_access::check_backup_api_available;

#[cfg(not(target_os = "windows"))]
pub use mock_impl::{enable_privileges, collect_with_raw_handle};

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn check_backup_api_available() -> bool {
    false
}
