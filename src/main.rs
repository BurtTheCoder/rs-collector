use std::env;
use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;

use anyhow::{Context, Result, anyhow};
use log::{info, warn, LevelFilter};
use simplelog::{Config, TermLogger, TerminalMode, ColorChoice};
use tokio::runtime::Runtime;
use clap::Parser;

mod cli;
mod models;
mod windows;
mod collectors;
mod utils;
mod cloud;
mod config;
mod build;
mod privileges;

use cli::{Args, Commands};
use collectors::collector;
use utils::{summary, compress};
use config::{CollectionConfig, load_or_create_config};
use models::ArtifactMetadata;
use privileges::enable_required_privileges;

fn main() -> Result<()> {
    // Parse arguments
    let args = Args::parse();
    
    // Initialize logging
    let log_level = if args.verbose { LevelFilter::Debug } else { LevelFilter::Info };
    TermLogger::init(
        log_level,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    ).unwrap();
    
    // Handle subcommands
    if let Some(cmd) = &args.command {
        match cmd {
            Commands::InitConfig { path, target_os } => {
                if let Some(os) = target_os {
                    info!("Creating {} configuration file at {}", os, path.display());
                    CollectionConfig::create_os_specific_config_file(path, &os.to_string())?;
                } else {
                    info!("Creating default configuration file for current OS at {}", path.display());
                    CollectionConfig::create_default_config_file(path)?;
                }
                info!("Configuration created successfully");
                return Ok(());
            },
            Commands::Build(build_opts) => {
                info!("Building standalone binary with embedded configuration");
                
                // Determine target OS
                let target_os = build_opts.target_os.as_ref().map(|os| os.to_string());
                
                let script_path = build::generate_build_script(
                    &build_opts.config, 
                    build_opts.output.as_deref(),
                    build_opts.name.as_deref(),
                    target_os.as_deref()
                )?;
                
                // Execute the build script
                build::execute_build_script(&script_path)?;
                
                return Ok(());
            }
        }
    }
    
    info!("Starting DFIR triage collection");
    
    // Load configuration
    let mut config = load_or_create_config(args.config.as_deref())?;
    
    // Process environment variables in paths
    config.process_environment_variables()?;
    
    // Filter artifacts by type if specified
    let artifacts_to_collect = if let Some(types_str) = &args.artifact_types {
        let requested_types: Vec<&str> = types_str.split(',').collect();
        let mut filtered_artifacts = Vec::new();
        
        for artifact in &config.artifacts {
            let type_str = format!("{}", artifact.artifact_type).to_lowercase();
            if requested_types.iter().any(|&t| type_str.contains(&t.to_lowercase())) {
                filtered_artifacts.push(artifact.clone());
            }
        }
        
        if filtered_artifacts.is_empty() {
            warn!("No artifacts match the requested types: {}", types_str);
            info!("Using all artifacts from config instead");
            config.artifacts.clone()
        } else {
            filtered_artifacts
        }
    } else {
        config.artifacts.clone()
    };
    
    // Check if we have sufficient privileges
    if !privileges::is_elevated() {
        warn!("Running without elevated privileges - some artifacts may be inaccessible");
        
        if !args.force {
            return Err(anyhow!("Elevated privileges required. {} or use --force to continue anyway", 
                privileges::get_elevation_instructions()));
        }
    }
    
    // Enable necessary privileges based on OS
    if let Err(e) = enable_required_privileges() {
        warn!("Failed to enable privileges: {}", e);
    }
    
    // Create output directory
    let hostname = hostname::get()
        .map_err(|e| anyhow!("Failed to get hostname: {}", e))?
        .to_string_lossy()
        .to_string();
    
    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    
    let output_dir = match &args.output {
        Some(path) => PathBuf::from(path),
        None => {
            let temp_dir = env::temp_dir();
            temp_dir.join("dfir-triage")
        }
    };
    
    let artifact_dir = output_dir.join(&hostname);
    fs::create_dir_all(&artifact_dir)
        .context("Failed to create output directory")?;
    
    info!("Output directory created at {}", artifact_dir.display());
    
    // Collect volatile data first (unless disabled)
    let mut volatile_data_summary = None;
    if !args.no_volatile_data {
        info!("Starting volatile data collection...");
        
        let volatile_dir = artifact_dir.join("volatile");
        let mut collector = collectors::volatile::VolatileDataCollector::new();
        
        match collector.collect_all(&volatile_dir) {
            Ok(summary) => {
                info!("Volatile data collection completed successfully");
                volatile_data_summary = Some(summary);
            },
            Err(e) => {
                warn!("Volatile data collection failed: {}", e);
                warn!("Continuing with regular artifact collection");
            }
        }
    } else {
        info!("Volatile data collection disabled, skipping...");
    }
    
    // Collect process memory if requested
    let mut memory_collection_summary = None;
    if args.dump_process_memory || args.memory_search.is_some() || args.memory_yara.is_some() || args.dump_memory_region.is_some() {
        info!("Starting process memory operations...");
        
        // Check if memory collection is available
        if collectors::memory::is_memory_collection_available() {
            // Use volatile data if available, otherwise skip
            if let Some(_) = &volatile_data_summary {
                // Read the processes from the file
                let processes_path = artifact_dir.join("volatile").join("processes.json");
                let processes_json = fs::read_to_string(&processes_path)
                    .context("Failed to read processes file")?;
                let processes: Vec<collectors::volatile::models::ProcessInfo> = serde_json::from_str(&processes_json)
                    .context("Failed to parse processes JSON")?;
                
                // Handle memory collection if requested
                if args.dump_process_memory {
                    match collectors::memory::collect_process_memory(
                        &processes,
                        &artifact_dir,
                        args.process.as_deref(),
                        args.pid.as_deref(),
                        args.include_system_processes,
                        args.max_memory_size,
                        &args.memory_regions,
                    ) {
                        Ok(summary) => {
                            info!("Process memory collection completed successfully");
                            memory_collection_summary = Some(summary);
                        },
                        Err(e) => {
                            warn!("Process memory collection failed: {}", e);
                            warn!("Continuing with other operations");
                        }
                    }
                }
                
                // Handle memory search if requested
                if let Some(pattern_hex) = &args.memory_search {
                    info!("Searching for pattern in process memory: {}", pattern_hex);
                    
                    // Create memory collector
                    let collector = collectors::memory::collector::MemoryCollector::from_args(
                        args.process.as_deref(),
                        args.pid.as_deref(),
                        args.include_system_processes,
                        args.max_memory_size,
                        &args.memory_regions,
                    )?;
                    
                    // Convert hex pattern to bytes
                    let pattern = pattern_hex.replace(" ", "")
                        .chars()
                        .collect::<Vec<char>>()
                        .chunks(2)
                        .map(|c| u8::from_str_radix(&c.iter().collect::<String>(), 16))
                        .collect::<Result<Vec<u8>, _>>()
                        .context("Invalid hex pattern format")?;
                    
                    // Search in each process
                    let mut search_results = HashMap::new();
                    let filtered_processes: Vec<&collectors::volatile::models::ProcessInfo> = processes
                        .iter()
                        .filter(|p| {
                            if let Some(process_names) = &args.process {
                                process_names.split(',').any(|name| p.name.contains(name))
                            } else if let Some(pids) = &args.pid {
                                pids.split(',').any(|pid| pid.parse::<u32>().map(|id| id == p.pid).unwrap_or(false))
                            } else {
                                true
                            }
                        })
                        .collect();
                    
                    for process in filtered_processes {
                        match collector.search_memory(process, &pattern) {
                            Ok(addresses) => {
                                if !addresses.is_empty() {
                                    info!("Found {} matches in process {} ({})", 
                                          addresses.len(), process.name, process.pid);
                                    search_results.insert(process.pid, addresses);
                                }
                            },
                            Err(e) => {
                                warn!("Failed to search memory in process {} ({}): {}", 
                                      process.name, process.pid, e);
                            }
                        }
                    }
                    
                    // Write search results to file
                    let search_results_path = artifact_dir.join("memory_search_results.json");
                    fs::write(&search_results_path, serde_json::to_string_pretty(&search_results)?)
                        .context("Failed to write memory search results")?;
                    
                    info!("Memory search results written to {}", search_results_path.display());
                }
                
                // Handle YARA scanning if requested
                #[cfg(feature = "yara")]
                if let Some(yara_rule) = &args.memory_yara {
                    info!("Scanning process memory with YARA rules");
                    
                    // Create memory collector
                    let collector = collectors::memory::collector::MemoryCollector::from_args(
                        args.process.as_deref(),
                        args.pid.as_deref(),
                        args.include_system_processes,
                        args.max_memory_size,
                        &args.memory_regions,
                    )?;
                    
                    // Check if the rule is a file path or a rule string
                    let rules = if std::path::Path::new(yara_rule).exists() {
                        // Read rule from file
                        vec![fs::read_to_string(yara_rule)?]
                    } else {
                        // Use as inline rule
                        vec![yara_rule.clone()]
                    };
                    
                    // Convert to string slices
                    let rule_slices: Vec<&str> = rules.iter().map(|s| s.as_str()).collect();
                    
                    // Scan each process
                    let mut scan_results = HashMap::new();
                    let filtered_processes: Vec<&collectors::volatile::models::ProcessInfo> = processes
                        .iter()
                        .filter(|p| {
                            if let Some(process_names) = &args.process {
                                process_names.split(',').any(|name| p.name.contains(name))
                            } else if let Some(pids) = &args.pid {
                                pids.split(',').any(|pid| pid.parse::<u32>().map(|id| id == p.pid).unwrap_or(false))
                            } else {
                                true
                            }
                        })
                        .collect();
                    
                    for process in filtered_processes {
                        match collector.scan_memory_yara(process, &rule_slices) {
                            Ok(matches) => {
                                if !matches.is_empty() {
                                    info!("Found YARA matches in process {} ({})", 
                                          process.name, process.pid);
                                    scan_results.insert(process.pid, matches);
                                }
                            },
                            Err(e) => {
                                warn!("Failed to scan memory in process {} ({}): {}", 
                                      process.name, process.pid, e);
                            }
                        }
                    }
                    
                    // Write scan results to file
                    let scan_results_path = artifact_dir.join("memory_yara_results.json");
                    fs::write(&scan_results_path, serde_json::to_string_pretty(&scan_results)?)
                        .context("Failed to write YARA scan results")?;
                    
                    info!("YARA scan results written to {}", scan_results_path.display());
                }
                
                // Handle memory region dump if requested
                if let Some(dump_info) = &args.dump_memory_region {
                    info!("Dumping specific memory region: {}", dump_info);
                    
                    // Parse dump info (format: pid:address:size)
                    let parts: Vec<&str> = dump_info.split(':').collect();
                    if parts.len() != 3 {
                        return Err(anyhow!("Invalid memory region format. Expected pid:address:size"));
                    }
                    
                    let pid = parts[0].parse::<u32>()
                        .context("Invalid PID in memory region specification")?;
                    let address = if parts[1].starts_with("0x") {
                        u64::from_str_radix(&parts[1][2..], 16)
                    } else {
                        u64::from_str_radix(parts[1], 16)
                    }.context("Invalid address in memory region specification")?;
                    let size = parts[2].parse::<usize>()
                        .context("Invalid size in memory region specification")?;
                    
                    // Find the process
                    let process = processes.iter().find(|p| p.pid == pid)
                        .ok_or_else(|| anyhow!("Process with PID {} not found", pid))?;
                    
                    // Create memory collector
                    let collector = collectors::memory::collector::MemoryCollector::from_args(
                        Some(&process.name),
                        Some(&pid.to_string()),
                        args.include_system_processes,
                        args.max_memory_size,
                        &args.memory_regions,
                    )?;
                    
                    // Dump the memory region
                    match collector.dump_memory_region(process, address, size) {
                        Ok(memory) => {
                            // Write memory dump to file
                            let dump_path = artifact_dir.join(format!("memory_dump_{}_{:x}_{}.bin", 
                                                                    pid, address, size));
                            fs::write(&dump_path, &memory)
                                .context("Failed to write memory dump")?;
                            
                            info!("Memory dump written to {}", dump_path.display());
                        },
                        Err(e) => {
                            warn!("Failed to dump memory region at {:x} for process {} ({}): {}", 
                                  address, process.name, pid, e);
                        }
                    }
                }
            } else {
                warn!("Memory operations require volatile data collection");
                warn!("Skipping memory operations");
            }
        } else {
            warn!("Memory collection is not available on this platform or build");
            warn!("Recompile with the 'memory_collection' feature to enable it");
        }
    }
    
    // Collect artifacts using the collector system
    info!("Collecting {} artifacts", artifacts_to_collect.len());
    let collected_artifacts = collector::collect_artifacts(&artifacts_to_collect, &artifact_dir)?;
    
    // Convert HashMap to Vec for summary
    let all_metadata: Vec<(String, ArtifactMetadata)> = collected_artifacts.into_iter().collect();
    
    // Report collection statistics by artifact type
    let mut counts_by_type = HashMap::new();
    for artifact in &artifacts_to_collect {
        let type_str = format!("{}", artifact.artifact_type);
        *counts_by_type.entry(type_str).or_insert(0) += 1;
    }
    
    for (artifact_type, count) in counts_by_type {
        info!("Configured to collect {} {} artifacts", count, artifact_type);
    }
    
    info!("Successfully collected {} artifacts", all_metadata.len());
    
    // Write collection summary
    let summary_json = summary::create_collection_summary(
        &hostname, 
        &timestamp, 
        &all_metadata,
        volatile_data_summary.as_ref(),
        memory_collection_summary.as_ref()
    );
    let summary_path = artifact_dir.join("collection_summary.json");
    
    fs::write(&summary_path, &summary_json)
        .context("Failed to write collection summary")?;
    
    // Generate bodyfile if enabled in config and on supported OS
    if cfg!(any(target_os = "macos", target_os = "linux")) {
        if let Some(generate_bodyfile) = config.global_options.get("generate_bodyfile") {
            if generate_bodyfile == "true" {
                let bodyfile_path = artifact_dir.join(format!("{}.body", hostname));
                info!("Generating bodyfile at {}", bodyfile_path.display());
                
                if let Err(e) = utils::bodyfile::generate_bodyfile(&bodyfile_path, &config.global_options) {
                    warn!("Failed to generate bodyfile: {}", e);
                } else {
                    info!("Bodyfile generation completed successfully");
                }
            }
        }
    }
    
    // Check if streaming to cloud storage is enabled
    if !args.skip_upload && args.stream {
        let runtime = Runtime::new().unwrap();
        
        // Check if we have S3 or SFTP options
        if args.bucket.is_some() {
            info!("Using streaming upload to S3...");
            
            let result = runtime.block_on(async {
                // Create S3 client
                let s3_client = cloud::client::create_s3_client(
                    args.region.as_deref(), 
                    args.profile.as_deref()
                )?;
                
                // Create key for ZIP file
                let default_prefix = format!("triage-{}-{}", timestamp, hostname);
                let prefix = args.prefix.as_deref().unwrap_or_else(|| default_prefix.as_str());
                let key = format!("{}/{}-{}.zip", prefix, hostname, timestamp);
                
                // Stream artifacts to S3
                collectors::streaming::stream_artifacts_to_s3(
                    &artifact_dir,
                    s3_client.clone(),
                    args.bucket.as_ref().unwrap(),
                    &key,
                    args.buffer_size
                ).await?;
                
                // Also upload the summary JSON separately for easy access
                let summary_key = format!("{}/collection_summary.json", prefix);
                
                collectors::streaming::stream_file_to_s3(
                    &summary_path,
                    s3_client,
                    args.bucket.as_ref().unwrap(),
                    &summary_key,
                    args.buffer_size
                ).await?;
                
                Ok::<_, anyhow::Error>(())
            });
            
            match result {
                Ok(_) => {
                    info!("Successfully streamed artifacts to S3");
                },
                Err(e) => {
                    warn!("Streaming upload to S3 failed: {}", e);
                    warn!("Falling back to standard upload method");
                    // Continue with standard compression and upload
                    compress_and_upload(&artifact_dir, &hostname, &timestamp, &summary_path, &args)?;
                }
            }
        } else if args.sftp_host.is_some() && args.sftp_user.is_some() && args.sftp_key.is_some() {
            info!("Using streaming upload to SFTP...");
            
            // Create SFTP config
            let sftp_config = cloud::sftp::SFTPConfig {
                host: args.sftp_host.as_ref().unwrap().clone(),
                port: args.sftp_port,
                username: args.sftp_user.as_ref().unwrap().clone(),
                private_key_path: args.sftp_key.as_ref().unwrap().clone(),
                remote_path: args.sftp_path.clone().unwrap_or_else(|| "/".to_string()),
                concurrent_connections: args.sftp_connections,
                buffer_size_mb: args.buffer_size,
                connection_timeout_sec: 30, // Default timeout
                max_retries: 3, // Default retries
            };
            
            let result = runtime.block_on(async {
                // Create remote path for ZIP file
                let remote_path = format!("{}/{}-{}.zip", 
                    sftp_config.remote_path.trim_end_matches('/'),
                    hostname, timestamp);
                
                // Stream artifacts to SFTP
                collectors::streaming::stream_artifacts_to_sftp(
                    &artifact_dir,
                    sftp_config.clone(),
                    &remote_path,
                    args.buffer_size
                ).await?;
                
                // Also upload the summary JSON separately for easy access
                let summary_remote_path = format!("{}/collection_summary.json", 
                    sftp_config.remote_path.trim_end_matches('/'));
                
                collectors::streaming::stream_file_to_sftp(
                    &summary_path,
                    sftp_config,
                    &summary_remote_path,
                    args.buffer_size
                ).await?;
                
                Ok::<_, anyhow::Error>(())
            });
            
            match result {
                Ok(_) => {
                    info!("Successfully streamed artifacts to SFTP");
                },
                Err(e) => {
                    warn!("Streaming upload to SFTP failed: {}", e);
                    warn!("Falling back to standard upload method");
                    // Continue with standard compression and upload
                    compress_and_upload(&artifact_dir, &hostname, &timestamp, &summary_path, &args)?;
                }
            }
        } else {
            warn!("Streaming enabled but no valid cloud storage options provided");
            warn!("Falling back to standard compression and upload");
            compress_and_upload(&artifact_dir, &hostname, &timestamp, &summary_path, &args)?;
        }
    } else {
        // Standard compression and upload
        compress_and_upload(&artifact_dir, &hostname, &timestamp, &summary_path, &args)?;
    }
    
    info!("DFIR triage completed successfully");
    
    Ok(())
}

/// Compress artifacts and upload to cloud storage if needed
fn compress_and_upload(
    artifact_dir: &PathBuf,
    hostname: &str,
    timestamp: &str,
    summary_path: &PathBuf,
    args: &Args,
) -> Result<()> {
    // Compress artifacts
    let zip_path = compress::compress_artifacts(artifact_dir, hostname, timestamp)?;
    
    info!("Artifact archive: {}", zip_path.display());
    
    // Skip upload if requested
    if args.skip_upload {
        return Ok(());
    }
    
    let runtime = Runtime::new().unwrap();
    
    // Upload to S3 if configured
    if args.bucket.is_some() {
        // Prepare artifact paths to upload
        let mut files_to_upload = vec![zip_path.clone()];
        
        // Also upload the summary JSON separately for easy access
        files_to_upload.push(summary_path.clone());
        
        // Upload all files concurrently
        let prefix = args.prefix.clone().unwrap_or_else(|| format!("triage-{}-{}", timestamp, hostname));
        
        info!("Starting concurrent upload of {} files to S3 bucket: {}", 
             files_to_upload.len(), args.bucket.as_ref().unwrap());
             
        let upload_result = runtime.block_on(cloud::s3::upload_files_concurrently(
            files_to_upload,
            args.bucket.as_ref().unwrap(),
            &prefix,
            args.region.as_deref(),
            args.profile.as_deref(),
            args.encrypt
        ));
        
        match upload_result {
            Ok(_) => info!("Successfully uploaded all artifacts to S3"),
            Err(e) => warn!("Failed to upload artifacts to S3: {}", e)
        }
    }
    
    // Upload to SFTP if configured
    if args.sftp_host.is_some() && args.sftp_user.is_some() && args.sftp_key.is_some() {
        // Create SFTP config
        let sftp_config = cloud::sftp::SFTPConfig {
            host: args.sftp_host.as_ref().unwrap().clone(),
            port: args.sftp_port,
            username: args.sftp_user.as_ref().unwrap().clone(),
            private_key_path: args.sftp_key.as_ref().unwrap().clone(),
            remote_path: args.sftp_path.clone().unwrap_or_else(|| "/".to_string()),
            concurrent_connections: args.sftp_connections,
            buffer_size_mb: args.buffer_size,
            connection_timeout_sec: 30, // Default timeout
            max_retries: 3, // Default retries
        };
        
        // Prepare artifact paths to upload
        let files_to_upload = vec![zip_path.clone(), summary_path.clone()];
        
        info!("Starting upload of {} files to SFTP server: {}",
             files_to_upload.len(), sftp_config.host);
             
        let upload_result = runtime.block_on(cloud::sftp::upload_files_concurrently(
            files_to_upload,
            sftp_config
        ));
        
        match upload_result {
            Ok(_) => info!("Successfully uploaded all artifacts to SFTP"),
            Err(e) => warn!("Failed to upload artifacts to SFTP: {}", e)
        }
    }
    
    Ok(())
}
