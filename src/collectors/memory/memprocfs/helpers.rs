//! Common helper functions for MemProcFS memory collection

#[cfg(feature = "memory_collection")]
use anyhow::{Result, bail, Context};
#[cfg(feature = "memory_collection")]  
use log::{debug, warn, info, error};
#[cfg(feature = "memory_collection")]
use std::path::Path;
#[cfg(feature = "memory_collection")]
use std::fs;
#[cfg(feature = "memory_collection")]
use std::env;
#[cfg(feature = "memory_collection")]
use memprocfs::*;

#[cfg(feature = "memory_collection")]
use crate::collectors::memory::models::{
    MemoryRegionInfo, MemoryRegionType, MemoryProtection, ModuleInfo,
};
#[cfg(feature = "memory_collection")]
use crate::collectors::volatile::models::ProcessInfo;

/// Get the appropriate MemProcFS library path
#[cfg(feature = "memory_collection")]
pub fn get_library_path() -> Result<String> {
    // First check if the user has specified a custom library path
    if let Ok(path) = env::var("RUST_COLLECTOR_MEMPROCFS_PATH") {
        if Path::new(&path).exists() {
            return Ok(path);
        }
        warn!("Custom MemProcFS library path does not exist: {}", path);
    }

    // Then check common locations based on OS
    #[cfg(target_os = "windows")]
    {
        let paths = [
            "vmm.dll",
            "C:\\Program Files\\MemProcFS\\vmm.dll",
            "C:\\MemProcFS\\vmm.dll",
        ];
        
        for path in paths {
            if Path::new(path).exists() {
                return Ok(path.to_string());
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let paths = [
            "./vmm.so",
            "/usr/local/lib/vmm.so",
            "/usr/lib/vmm.so",
            "/opt/memprocfs/vmm.so",
        ];
        
        for path in paths {
            if Path::new(path).exists() {
                return Ok(path.to_string());
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let paths = [
            "./vmm.dylib",
            "/usr/local/lib/vmm.dylib",
            "/opt/memprocfs/vmm.dylib",
        ];
        
        for path in paths {
            if Path::new(path).exists() {
                return Ok(path.to_string());
            }
        }
    }

    bail!("MemProcFS library not found. Please install MemProcFS or set RUST_COLLECTOR_MEMPROCFS_PATH environment variable.")
}

/// Convert MemProcFS memory protection to our MemoryProtection model
#[cfg(feature = "memory_collection")]
pub fn convert_protection(protection: u32) -> MemoryProtection {
    MemoryProtection {
        read: protection & VM_PROT_READ != 0,
        write: protection & VM_PROT_WRITE != 0,
        execute: protection & VM_PROT_EXECUTE != 0,
    }
}

/// Convert a memory region type from MemProcFS to our model
#[cfg(feature = "memory_collection")]
pub fn convert_region_type(region: &VmmMapVadEntry) -> MemoryRegionType {
    if region.type_ex == "Stack" {
        return MemoryRegionType::Stack;
    } else if region.type_ex == "Heap" {
        return MemoryRegionType::Heap;
    } else if region.protection & VM_PROT_EXECUTE != 0 {
        return MemoryRegionType::Code;
    } else if !region.filename.is_empty() {
        return MemoryRegionType::MappedFile;
    } else {
        return MemoryRegionType::Other;
    }
}

/// Format a memory dump for output
#[cfg(feature = "memory_collection")]
pub fn format_memory_dump(data: &[u8], max_length: usize) -> String {
    use pretty_hex::*;
    let truncated = if data.len() > max_length {
        &data[..max_length]
    } else {
        data
    };
    format!("{:?}", truncated.hex_dump())
}

#[cfg(not(feature = "memory_collection"))]
pub fn format_memory_dump(data: &[u8], max_length: usize) -> String {
    let truncated = if data.len() > max_length {
        &data[..max_length]
    } else {
        data
    };
    format!("{:?}", truncated)
}
