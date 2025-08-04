//! macOS-specific memory collection implementation
//!
//! This module provides macOS-specific implementation for memory collection
//! using the task_for_pid and mach APIs.

use anyhow::{bail, Context, Result};
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::ffi::CStr;
use std::mem;
use std::path::Path;
use std::slice;

#[cfg(feature = "memory_collection")]
use mach_sys::kern_return::KERN_SUCCESS;
#[cfg(feature = "memory_collection")]
use mach_sys::mach_types::task_port_t;
#[cfg(feature = "memory_collection")]
use mach_sys::port::mach_port_t;
#[cfg(feature = "memory_collection")]
use mach_sys::task::task_info;
#[cfg(feature = "memory_collection")]
use mach_sys::task_info::{task_dyld_info_data_t, TASK_DYLD_INFO};
#[cfg(feature = "memory_collection")]
use mach_sys::traps::mach_task_self;
#[cfg(feature = "memory_collection")]
use mach_sys::traps::task_for_pid;
#[cfg(feature = "memory_collection")]
use mach_sys::vm::{mach_vm_read_overwrite, mach_vm_region};
#[cfg(feature = "memory_collection")]
use mach_sys::vm_prot::{VM_PROT_EXECUTE, VM_PROT_READ, VM_PROT_WRITE};
#[cfg(feature = "memory_collection")]
use mach_sys::vm_region::{vm_region_basic_info_data_64_t, VM_REGION_BASIC_INFO_64};
#[cfg(feature = "memory_collection")]
use mach_sys::vm_types::{mach_vm_address_t, mach_vm_size_t};
#[cfg(feature = "memory_collection")]
type task_info_t = *mut ::std::os::raw::c_int;
#[cfg(feature = "memory_collection")]
const TASK_DYLD_INFO_COUNT: mach_msg_type_number_t = 5;
#[cfg(feature = "memory_collection")]
use mach_sys::dyld_images::{dyld_all_image_infos, dyld_image_info};
#[cfg(feature = "memory_collection")]
use mach_sys::ffi::c_int;
#[cfg(feature = "memory_collection")]
use mach_sys::message::mach_msg_type_number_t;

use crate::collectors::memory::models::{
    MemoryProtection, MemoryRegionInfo, MemoryRegionType, ModuleInfo,
};
use crate::collectors::memory::platforms::MemoryCollectorImpl;
use crate::collectors::volatile::models::ProcessInfo;

/// macOS memory collector implementation
pub struct MacOSMemoryCollector {
    #[cfg(feature = "memory_collection")]
    task_ports: HashMap<u32, task_port_t>,
}

impl MemoryCollectorImpl for MacOSMemoryCollector {
    fn new() -> Result<Self> {
        // Check if we're running as root, which is required for task_for_pid on macOS
        let is_root = unsafe { libc::geteuid() == 0 };
        if !is_root {
            warn!("Memory collection on macOS requires root privileges for full access");
        }

        info!("Initialized macOS memory collector");

        #[cfg(feature = "memory_collection")]
        {
            Ok(Self {
                task_ports: HashMap::new(),
            })
        }

        #[cfg(not(feature = "memory_collection"))]
        {
            bail!(
                "Memory collection is not enabled. Recompile with the 'memory_collection' feature."
            );
        }
    }

    fn get_memory_regions(&self, process: &ProcessInfo) -> Result<Vec<MemoryRegionInfo>> {
        #[cfg(feature = "memory_collection")]
        {
            let pid = process.pid;

            // Get task port for the process
            let task = self.get_task_port(pid)?;

            // Enumerate memory regions
            let mut regions = Vec::new();
            let mut address: mach_vm_address_t = 0;

            loop {
                let mut info: vm_region_basic_info_data_64_t = unsafe { mem::zeroed() };
                let mut count: mach_msg_type_number_t =
                    (mem::size_of::<vm_region_basic_info_data_64_t>() / mem::size_of::<c_int>())
                        as mach_msg_type_number_t;
                let mut object_name: mach_port_t = 0;
                let mut size: mach_vm_size_t = 0;

                let kr = unsafe {
                    mach_vm_region(
                        task,
                        &mut address,
                        &mut size,
                        VM_REGION_BASIC_INFO_64,
                        (&mut info as *mut _) as *mut c_int,
                        &mut count,
                        &mut object_name,
                    )
                };

                if kr != KERN_SUCCESS {
                    // End of regions
                    break;
                }

                // Determine region type
                let region_type = if info.protection & VM_PROT_EXECUTE != 0 {
                    MemoryRegionType::Code
                } else if address >= 0x7fff00000000 {
                    // Heuristic for stack regions on macOS (high addresses)
                    MemoryRegionType::Stack
                } else if info.protection & VM_PROT_WRITE != 0 {
                    // Writable regions that aren't code or stack are likely heap
                    MemoryRegionType::Heap
                } else {
                    MemoryRegionType::Other
                };

                // Convert protection flags
                let protection = MemoryProtection {
                    read: info.protection & VM_PROT_READ != 0,
                    write: info.protection & VM_PROT_WRITE != 0,
                    execute: info.protection & VM_PROT_EXECUTE != 0,
                };

                // Create region info
                let region = MemoryRegionInfo {
                    base_address: address,
                    size,
                    region_type,
                    protection,
                    name: Some(format!("{} ({})", process.name, region_type)),
                    mapped_file: None, // We'll try to fill this in from modules later
                    dumped: false,
                    dump_path: None,
                };

                regions.push(region);

                // Move to next region
                address += size;
            }

            // Try to match regions with modules
            if let Ok(modules) = self.get_modules(process) {
                for module in &modules {
                    for region in &mut regions {
                        // If this region contains the module's base address
                        if region.base_address <= module.base_address
                            && region.base_address + region.size
                                >= module.base_address + module.size
                        {
                            region.mapped_file = Some(module.path.clone());
                            region.name = Some(module.name.clone());
                            break;
                        }
                    }
                }
            }

            debug!("Found {} memory regions for process {}", regions.len(), pid);

            Ok(regions)
        }

        #[cfg(not(feature = "memory_collection"))]
        {
            bail!(
                "Memory collection is not enabled. Recompile with the 'memory_collection' feature."
            );
        }
    }

    fn read_memory(&self, pid: u32, address: u64, size: usize) -> Result<Vec<u8>> {
        #[cfg(feature = "memory_collection")]
        {
            // Get task port for the process
            let task = self.get_task_port(pid)?;

            // Allocate buffer for the memory
            let mut buffer = vec![0u8; size];
            let mut out_size: mach_vm_size_t = 0;

            // Read memory
            let kr = unsafe {
                mach_vm_read_overwrite(
                    task,
                    address,
                    size as mach_vm_size_t,
                    buffer.as_mut_ptr() as mach_vm_address_t,
                    &mut out_size,
                )
            };

            if kr != KERN_SUCCESS {
                return Err(anyhow!(
                    "Failed to read memory at address {:x} for process {}: {}",
                    address,
                    pid,
                    kr
                ));
            }

            // Resize buffer to actual bytes read
            buffer.truncate(out_size as usize);

            debug!(
                "Read {} bytes from address {:x} for process {}",
                out_size, address, pid
            );

            Ok(buffer)
        }

        #[cfg(not(feature = "memory_collection"))]
        {
            bail!(
                "Memory collection is not enabled. Recompile with the 'memory_collection' feature."
            );
        }
    }

    fn get_modules(&self, process: &ProcessInfo) -> Result<Vec<ModuleInfo>> {
        #[cfg(feature = "memory_collection")]
        {
            let pid = process.pid;

            // Get task port for the process
            let task = self.get_task_port(pid)?;

            // Get the dyld info from the task
            let mut dyld_info: task_dyld_info_data_t = unsafe { mem::zeroed() };
            let mut count = TASK_DYLD_INFO_COUNT;

            let kr = unsafe {
                task_info(
                    task,
                    TASK_DYLD_INFO,
                    &mut dyld_info as *mut _ as task_info_t,
                    &mut count,
                )
            };

            if kr != KERN_SUCCESS {
                return Err(anyhow!("Failed to get dyld info for process {}: {}", pid, kr));
            }

            // Read the all_image_infos structure
            let mut all_image_infos: dyld_all_image_infos = unsafe { mem::zeroed() };
            let mut out_size: mach_vm_size_t = 0;

            let kr = unsafe {
                mach_vm_read_overwrite(
                    task,
                    dyld_info.all_image_info_addr,
                    mem::size_of::<dyld_all_image_infos>() as mach_vm_size_t,
                    &mut all_image_infos as *mut _ as mach_vm_address_t,
                    &mut out_size,
                )
            };

            if kr != KERN_SUCCESS {
                return Err(anyhow!("Failed to read all_image_infos for process {}: {}", pid, kr));
            }

            // Read the image_info array
            let image_count = all_image_infos.infoArrayCount as usize;
            let image_array_size = image_count * mem::size_of::<dyld_image_info>();
            let mut image_array_bytes = vec![0u8; image_array_size];

            let kr = unsafe {
                mach_vm_read_overwrite(
                    task,
                    all_image_infos.infoArray as mach_vm_address_t,
                    image_array_size as mach_vm_size_t,
                    image_array_bytes.as_mut_ptr() as mach_vm_address_t,
                    &mut out_size,
                )
            };

            if kr != KERN_SUCCESS {
                return Err(anyhow!("Failed to read image array for process {}: {}", pid, kr));
            }

            // Convert bytes to array of dyld_image_info
            let image_array = unsafe {
                slice::from_raw_parts(
                    image_array_bytes.as_ptr() as *const dyld_image_info,
                    image_count,
                )
            };

            let mut modules = Vec::new();

            // Process each image
            for image in image_array {
                // Read the image path
                let mut path_buffer = [0u8; 1024];
                let mut path_size: mach_vm_size_t = 0;

                let kr = unsafe {
                    mach_vm_read_overwrite(
                        task,
                        image.imageFilePath as mach_vm_address_t,
                        path_buffer.len() as mach_vm_size_t,
                        path_buffer.as_mut_ptr() as mach_vm_address_t,
                        &mut path_size,
                    )
                };

                if kr != KERN_SUCCESS {
                    continue; // Skip this image if we can't read the path
                }

                // Convert C string to Rust string
                let path = unsafe {
                    CStr::from_ptr(path_buffer.as_ptr() as *const i8)
                        .to_string_lossy()
                        .to_string()
                };

                // Extract module name from path
                let name = Path::new(&path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                // Create module info
                let module = ModuleInfo {
                    base_address: image.imageLoadAddress as u64,
                    size: 0, // We don't know the size from dyld info
                    path,
                    name,
                    version: None,
                };

                modules.push(module);
            }

            debug!("Found {} modules for process {}", modules.len(), pid);

            Ok(modules)
        }

        #[cfg(not(feature = "memory_collection"))]
        {
            bail!(
                "Memory collection is not enabled. Recompile with the 'memory_collection' feature."
            );
        }
    }
}

#[cfg(feature = "memory_collection")]
impl MacOSMemoryCollector {
    /// Get task port for a process, caching the result
    fn get_task_port(&self, pid: u32) -> Result<task_port_t> {
        // Check if we already have the task port
        if let Some(&task) = self.task_ports.get(&pid) {
            return Ok(task);
        }

        // Get the task port
        let mut task: task_port_t = 0;
        let kr = unsafe { task_for_pid(mach_task_self(), pid as i32, &mut task) };

        if kr != KERN_SUCCESS {
            return Err(anyhow!("Failed to get task port for process {}: {}", pid, kr));
        }

        // Cache the task port
        let mut task_ports = self.task_ports.clone();
        task_ports.insert(pid, task);

        Ok(task)
    }
}
