//! Common helper functions for MemProcFS memory collection

#[cfg(feature = "memory_collection")]
use anyhow::{bail, Result};
#[cfg(feature = "memory_collection")]
use log::warn;
#[cfg(feature = "memory_collection")]
use memprocfs::*;
#[cfg(feature = "memory_collection")]
#[cfg(feature = "memory_collection")]
use std::env;
#[cfg(feature = "memory_collection")]
use std::path::Path;

#[cfg(feature = "memory_collection")]
use crate::collectors::memory::models::{MemoryProtection, MemoryRegionType};

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
    // Use Windows-style protection flags as memprocfs doesn't define VM_PROT_* constants
    MemoryProtection {
        read: protection & 0x1 != 0,    // PAGE_READONLY or PAGE_READWRITE
        write: protection & 0x2 != 0,   // PAGE_READWRITE
        execute: protection & 0x4 != 0, // PAGE_EXECUTE*
    }
}

/// Convert a memory region type from MemProcFS to our model
#[cfg(feature = "memory_collection")]
pub fn convert_region_type(region: &VmmProcessMapVadEntry) -> MemoryRegionType {
    // VAD entries don't have type_ex or protection fields
    // Use info field to determine type
    if region.info.contains("Stack") {
        return MemoryRegionType::Stack;
    } else if region.info.contains("Heap") {
        return MemoryRegionType::Heap;
    } else if !region.info.is_empty()
        && (region.info.ends_with(".exe") || region.info.ends_with(".dll"))
    {
        return MemoryRegionType::Code;
    } else if !region.info.is_empty() {
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
