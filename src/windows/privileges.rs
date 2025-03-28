use std::ptr;
use std::mem;
use std::io;

use anyhow::{Result, anyhow};
use log::{info, warn, debug};
use widestring::U16CString;
use winapi::um::processthreadsapi::GetCurrentProcess;
use winapi::um::securitybaseapi::{AdjustTokenPrivileges, GetTokenInformation};
use winapi::um::winbase::LookupPrivilegeValueW;
use winapi::shared::minwindef::{FALSE, DWORD};
use winapi::um::winnt::{
    TOKEN_PRIVILEGES, TOKEN_ADJUST_PRIVILEGES, TOKEN_QUERY,
    SE_PRIVILEGE_ENABLED, LUID, HANDLE, TOKEN_INFORMATION_CLASS,
    TokenPrivileges, PRIVILEGE_SET, PRIVILEGE_SET_ALL_NECESSARY,
};
use winapi::um::processthreadsapi::OpenProcessToken;

/// Enable backup and restore privileges for accessing locked files
pub fn enable_privileges() -> Result<()> {
    info!("Enabling backup and restore privileges");
    
    let privileges = [
        "SeBackupPrivilege",
        "SeRestorePrivilege",
        "SeSecurityPrivilege",
        "SeTakeOwnershipPrivilege",
        "SeDebugPrivilege",
    ];
    
    let h_process = unsafe { GetCurrentProcess() };
    let mut h_token: HANDLE = ptr::null_mut();
    
    // Open the process token
    let token_result = unsafe {
        OpenProcessToken(
            h_process,
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut h_token,
        )
    };
    
    if token_result == 0 {
        let err = io::Error::last_os_error();
        return Err(anyhow!("Failed to open process token: {}", err));
    }
    
    let mut success_count = 0;
    for privilege in privileges.iter() {
        let privilege_result = enable_privilege(h_token, privilege);
        match privilege_result {
            Ok(enabled) => {
                if enabled {
                    success_count += 1;
                    info!("Successfully enabled privilege: {}", privilege);
                } else {
                    debug!("Privilege already enabled: {}", privilege);
                }
            },
            Err(e) => {
                warn!("Failed to enable privilege {}: {}", privilege, e);
            }
        }
    }
    
    if success_count > 0 {
        info!("Successfully enabled {} privileges", success_count);
        Ok(())
    } else {
        warn!("Failed to enable any privileges, collection may be limited");
        Ok(()) // Continue anyway, some files might be accessible
    }
}

/// Enable a specific privilege
fn enable_privilege(h_token: HANDLE, privilege_name: &str) -> Result<bool> {
    let mut luid = LUID {
        LowPart: 0,
        HighPart: 0,
    };
    
    // Convert privilege name to wide string
    let wide_name = U16CString::from_str(privilege_name)?;
    
    // Look up the privilege value
    let lookup_result = unsafe {
        LookupPrivilegeValueW(
            ptr::null(),
            wide_name.as_ptr(),
            &mut luid,
        )
    };
    
    if lookup_result == 0 {
        let err = io::Error::last_os_error();
        return Err(anyhow!("LookupPrivilegeValue failed: {}", err));
    }
    
    // Check if privilege is already enabled
    if is_privilege_enabled(h_token, &luid)? {
        return Ok(false); // Already enabled
    }
    
    // Create TOKEN_PRIVILEGES structure
    let mut tp = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        Privileges: [
            winapi::um::winnt::LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }
        ],
    };
    
    // Adjust token privileges
    let adjust_result = unsafe {
        AdjustTokenPrivileges(
            h_token,
            FALSE,
            &mut tp,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    
    if adjust_result == 0 {
        let err = io::Error::last_os_error();
        return Err(anyhow!("AdjustTokenPrivileges failed: {}", err));
    }
    
    let last_error = unsafe { winapi::um::errhandlingapi::GetLastError() };
    if last_error != 0 {
        let err = io::Error::from_raw_os_error(last_error as i32);
        return Err(anyhow!("Failed to enable privilege: {}", err));
    }
    
    Ok(true)
}

/// Check if a privilege is already enabled
fn is_privilege_enabled(h_token: HANDLE, luid: &LUID) -> Result<bool> {
    // First, get the required size for the buffer
    let mut return_length: DWORD = 0;
    let token_info_result = unsafe {
        GetTokenInformation(
            h_token,
            TokenPrivileges,
            ptr::null_mut(),
            0,
            &mut return_length,
        )
    };
    
    // Allocate buffer for privilege info
    let buffer_size = return_length as usize;
    let mut buffer = vec![0u8; buffer_size];
    
    // Get token privileges
    let token_info_result = unsafe {
        GetTokenInformation(
            h_token,
            TokenPrivileges,
            buffer.as_mut_ptr() as *mut _,
            buffer_size as DWORD,
            &mut return_length,
        )
    };
    
    if token_info_result == 0 {
        let err = io::Error::last_os_error();
        return Err(anyhow!("GetTokenInformation failed: {}", err));
    }
    
    // Create privilege set to check
    let mut privilege_set = PRIVILEGE_SET {
        PrivilegeCount: 1,
        Control: PRIVILEGE_SET_ALL_NECESSARY,
        Privilege: [
            winapi::um::winnt::LUID_AND_ATTRIBUTES {
                Luid: *luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }
        ],
    };
    
    let mut has_privilege: FALSE = 0;
    
    // Check if privilege is enabled
    let check_result = unsafe {
        winapi::um::securitybaseapi::PrivilegeCheck(
            h_token,
            &mut privilege_set,
            &mut has_privilege,
        )
    };
    
    if check_result == 0 {
        let err = io::Error::last_os_error();
        return Err(anyhow!("PrivilegeCheck failed: {}", err));
    }
    
    Ok(has_privilege != 0)
}