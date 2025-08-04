use std::env;
use std::fs;
use std::path::PathBuf;

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
mod constants;

#[cfg(test)]
mod test_utils;

use cli::{Args, Commands};
use collectors::collector;
use utils::{summary, compress};
use config::{CollectionConfig, load_or_create_config, Artifact};
use models::ArtifactMetadata;
use privileges::enable_required_privileges;

fn main() -> Result<()> {
    // Parse arguments
    let args = Args::parse();
    
    // Initialize logging
    initialize_logging(args.verbose)?;
    
    // Handle subcommands
    if let Some(cmd) = &args.command {
        return handle_subcommand(cmd);
    }
    
    info!("Starting DFIR triage collection");
    
    // Load and process configuration
    let config = load_and_process_config(&args)?;
    let artifacts_to_collect = filter_artifacts_by_type(&config, &args);
    
    // Check privileges
    check_and_enable_privileges(&args)?;
    
    // Setup collection directories
    let (hostname, timestamp, artifact_dir) = setup_collection_directories(&args)?;
    
    // Collect volatile data
    let volatile_data_summary = collect_volatile_data(&artifact_dir, &args)?;
    
    // Collect process memory if requested
    let memory_collection_summary = handle_memory_operations(&artifact_dir, &args, &volatile_data_summary)?;
    
    // Collect artifacts
    let all_metadata = collect_artifacts(&artifact_dir, &artifacts_to_collect, &config)?;
    
    // Generate bodyfile if requested
    generate_bodyfile_if_requested(&artifact_dir, &config, &hostname);
    
    // Write collection summary
    write_collection_summary(
        &artifact_dir, 
        &hostname, 
        &timestamp, 
        &all_metadata, 
        &volatile_data_summary, 
        &memory_collection_summary
    )?;
    
    // Handle upload
    handle_upload(&artifact_dir, &hostname, &timestamp, &args)?;
    
    info!("DFIR triage completed successfully");
    Ok(())
}

/// Initialize logging with the specified verbosity level
fn initialize_logging(verbose: bool) -> Result<()> {
    let log_level = if verbose { LevelFilter::Debug } else { LevelFilter::Info };
    TermLogger::init(
        log_level,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    ).context("Failed to initialize logger")?;
    Ok(())
}

/// Handle subcommands (init-config and build)
fn handle_subcommand(cmd: &Commands) -> Result<()> {
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
            Ok(())
        },
        Commands::Build(build_opts) => {
            info!("Building standalone binary with embedded configuration");
            
            // Determine target OS
            let target_os = build_opts.target_os.as_ref().map(|os| os.to_string());
            
            // Build binary directly using the new approach
            let output_file = build::build_binary_with_config(
                &build_opts.config, 
                build_opts.output.as_deref(),
                build_opts.name.as_deref(),
                target_os.as_deref()
            )?;
            
            info!("Standalone binary created at: {}", output_file.display());
            Ok(())
        }
    }
}

/// Load configuration and process environment variables
fn load_and_process_config(args: &Args) -> Result<CollectionConfig> {
    let mut config = load_or_create_config(args.config.as_deref())?;
    config.process_environment_variables()?;
    Ok(config)
}

/// Filter artifacts by type if specified
fn filter_artifacts_by_type(config: &CollectionConfig, args: &Args) -> Vec<Artifact> {
    if let Some(types_str) = &args.artifact_types {
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
    }
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
    
    let runtime = Runtime::new().context("Failed to create Tokio runtime")?;
    
    // Upload to S3 if configured
    if args.bucket.is_some() {
        // Prepare artifact paths to upload
        let mut files_to_upload = vec![zip_path.clone()];
        
        // Also upload the summary JSON separately for easy access
        files_to_upload.push(summary_path.clone());
        
        // Upload all files concurrently
        let prefix = args.prefix.clone().unwrap_or_else(|| format!("triage-{}-{}", timestamp, hostname));
        
        let bucket = args.bucket.as_ref().ok_or_else(|| anyhow!("Bucket not provided"))?;
        info!("Starting concurrent upload of {} files to S3 bucket: {}", 
             files_to_upload.len(), bucket);
             
        let upload_result = runtime.block_on(cloud::s3::upload_files_concurrently(
            files_to_upload,
            bucket,
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
            host: args.sftp_host.as_ref().ok_or_else(|| anyhow!("SFTP host not provided"))?.clone(),
            port: args.sftp_port,
            username: args.sftp_user.as_ref().ok_or_else(|| anyhow!("SFTP user not provided"))?.clone(),
            private_key_path: args.sftp_key.as_ref().ok_or_else(|| anyhow!("SFTP key not provided"))?.clone(),
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
/// Check and enable privileges
fn check_and_enable_privileges(args: &Args) -> Result<()> {
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
    
    Ok(())
}

/// Setup collection directories and return hostname, timestamp, and artifact directory
fn setup_collection_directories(args: &Args) -> Result<(String, String, PathBuf)> {
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
    
    Ok((hostname, timestamp, artifact_dir))
}

/// Collect volatile data if not disabled
fn collect_volatile_data(artifact_dir: &PathBuf, args: &Args) -> Result<Option<collectors::volatile::models::VolatileDataSummary>> {
    if args.no_volatile_data {
        info!("Volatile data collection disabled, skipping...");
        return Ok(None);
    }
    
    info!("Starting volatile data collection...");
    
    let volatile_dir = artifact_dir.join("volatile");
    let mut collector = collectors::volatile::VolatileDataCollector::new();
    
    match collector.collect_all(&volatile_dir) {
        Ok(summary) => {
            info!("Volatile data collection completed successfully");
            Ok(Some(summary))
        },
        Err(e) => {
            warn!("Volatile data collection failed: {}", e);
            warn!("Continuing with regular artifact collection");
            Ok(None)
        }
    }
}

/// Handle memory operations (collection, search, YARA scanning)
fn handle_memory_operations(
    artifact_dir: &PathBuf,
    args: &Args,
    volatile_data_summary: &Option<collectors::volatile::models::VolatileDataSummary>
) -> Result<Option<collectors::memory::models::MemoryCollectionSummary>> {
    if !args.dump_process_memory && args.memory_search.is_none() && 
       args.memory_yara.is_none() && args.dump_memory_region.is_none() {
        return Ok(None);
    }
    
    info!("Starting process memory operations...");
    
    // Check if memory collection is available
    if !collectors::memory::is_memory_collection_available() {
        warn!("Memory collection is not available on this system");
        return Ok(None);
    }
    
    // Use volatile data if available
    if volatile_data_summary.is_none() {
        warn!("Process memory operations require volatile data collection. Run without --no-volatile-data flag.");
        return Ok(None);
    }
    
    // Read the processes from the file
    let processes_path = artifact_dir.join("volatile").join("processes.json");
    let processes_json = fs::read_to_string(&processes_path)
        .context("Failed to read processes file")?;
    let processes: Vec<collectors::volatile::models::ProcessInfo> = serde_json::from_str(&processes_json)
        .context("Failed to parse processes JSON")?;
    
    let mut memory_summary = None;
    
    // Handle memory collection if requested
    if args.dump_process_memory {
        match collectors::memory::collect_process_memory(
            &processes,
            artifact_dir,
            args.process.as_deref(),
            args.pid.as_deref(),
            args.include_system_processes,
            args.max_memory_size,
            &args.memory_regions,
        ) {
            Ok(summary) => {
                info!("Process memory collection completed successfully");
                memory_summary = Some(summary);
            },
            Err(e) => {
                warn!("Process memory collection failed: {}", e);
                warn!("Continuing with other operations");
            }
        }
    }
    
    // Handle memory search if requested
    if let Some(_pattern) = &args.memory_search {
        warn!("Memory pattern search is not yet implemented in the refactored code");
    }
    
    // Handle YARA scanning if requested
    if let Some(_rule) = &args.memory_yara {
        warn!("YARA memory scanning is not yet implemented in the refactored code");
    }
    
    // Handle specific memory region dump if requested
    if let Some(_spec) = &args.dump_memory_region {
        warn!("Memory region dump is not yet implemented in the refactored code");
    }
    
    Ok(memory_summary)
}

/// Collect configured artifacts
fn collect_artifacts(
    artifact_dir: &PathBuf,
    artifacts_to_collect: &[Artifact],
    _config: &CollectionConfig
) -> Result<Vec<(String, ArtifactMetadata)>> {
    info!("Starting artifact collection...");
    
    let mut all_metadata: Vec<(String, ArtifactMetadata)> = Vec::new();
    let required_artifacts: Vec<&Artifact> = artifacts_to_collect
        .iter()
        .filter(|a| a.required)
        .collect();
    
    info!("Collecting {} artifacts ({} required)", 
          artifacts_to_collect.len(), required_artifacts.len());
    
    for artifact in artifacts_to_collect {
        let artifact_type_str = format!("{}", artifact.artifact_type);
        let type_dir = artifact_dir.join(&artifact_type_str);
        
        if !type_dir.exists() {
            fs::create_dir_all(&type_dir)
                .context("Failed to create artifact type directory")?;
        }
        
        let metadata = collector::collect_artifacts(
            &[artifact.clone()], 
            &type_dir
        )?;
        
        all_metadata.extend(metadata.into_iter());
    }
    
    info!("Successfully collected {} artifacts", all_metadata.len());
    Ok(all_metadata)
}

/// Generate bodyfile if requested
fn generate_bodyfile_if_requested(artifact_dir: &PathBuf, config: &CollectionConfig, hostname: &str) {
    // Check if bodyfile generation is enabled
    let generate_bodyfile = config.global_options.get("generate_bodyfile")
        .map(|v| v == "true")
        .unwrap_or(true);
        
    if generate_bodyfile {
        #[cfg(not(target_os = "windows"))]
        {
            let bodyfile_path = artifact_dir.parent()
                .unwrap_or(artifact_dir)
                .join(format!("{}.body", hostname));
            
            info!("Generating bodyfile at {}", bodyfile_path.display());
            
            if let Err(e) = utils::bodyfile::generate_bodyfile(&bodyfile_path, &config.global_options) {
                warn!("Failed to generate bodyfile: {}", e);
            } else {
                info!("Bodyfile generation completed successfully");
            }
        }
    }
}

/// Write collection summary
fn write_collection_summary(
    artifact_dir: &PathBuf,
    hostname: &str,
    timestamp: &str,
    all_metadata: &[(String, ArtifactMetadata)],
    volatile_data_summary: &Option<collectors::volatile::models::VolatileDataSummary>,
    memory_collection_summary: &Option<collectors::memory::models::MemoryCollectionSummary>
) -> Result<PathBuf> {
    let summary_json = summary::create_collection_summary(
        hostname, 
        timestamp, 
        all_metadata,
        volatile_data_summary.as_ref(),
        memory_collection_summary.as_ref()
    )?;
    let summary_path = artifact_dir.join("collection_summary.json");
    
    fs::write(&summary_path, &summary_json)
        .context("Failed to write collection summary")?;
    
    info!("Collection summary written to {}", summary_path.display());
    
    Ok(summary_path)
}

/// Handle artifact upload (streaming or standard)
fn handle_upload(
    artifact_dir: &PathBuf,
    hostname: &str,
    timestamp: &str,
    args: &Args
) -> Result<()> {
    let summary_path = artifact_dir.join("collection_summary.json");
    
    // Check if streaming to cloud storage is enabled
    if !args.skip_upload && args.stream {
        handle_streaming_upload(artifact_dir, hostname, timestamp, &summary_path, args)?;
    } else {
        // Standard compression and upload
        compress_and_upload(artifact_dir, hostname, timestamp, &summary_path, args)?;
    }
    
    Ok(())
}

/// Handle streaming upload to S3 or SFTP
fn handle_streaming_upload(
    artifact_dir: &PathBuf,
    hostname: &str,
    timestamp: &str,
    summary_path: &PathBuf,
    args: &Args
) -> Result<()> {
    let runtime = Runtime::new().context("Failed to create Tokio runtime")?;
    
    // Check if we have S3 or SFTP options
    if args.bucket.is_some() {
        info!("Using streaming upload to S3...");
        
        let result = runtime.block_on(stream_to_s3(artifact_dir, hostname, timestamp, summary_path, args));
        
        match result {
            Ok(_) => {
                info!("Successfully streamed artifacts to S3");
            },
            Err(e) => {
                warn!("Streaming upload to S3 failed: {}", e);
                warn!("Falling back to standard upload method");
                // Continue with standard compression and upload
                compress_and_upload(artifact_dir, hostname, timestamp, summary_path, args)?;
            }
        }
    } else if args.sftp_host.is_some() && args.sftp_user.is_some() && args.sftp_key.is_some() {
        info!("Using streaming upload to SFTP...");
        
        let result = runtime.block_on(stream_to_sftp(artifact_dir, hostname, timestamp, summary_path, args));
        
        match result {
            Ok(_) => {
                info!("Successfully streamed artifacts to SFTP");
            },
            Err(e) => {
                warn!("Streaming upload to SFTP failed: {}", e);
                warn!("Falling back to standard upload method");
                // Continue with standard compression and upload
                compress_and_upload(artifact_dir, hostname, timestamp, summary_path, args)?;
            }
        }
    } else {
        warn!("Streaming enabled but no valid cloud storage options provided");
        warn!("Falling back to standard compression and upload");
        compress_and_upload(artifact_dir, hostname, timestamp, summary_path, args)?;
    }
    
    Ok(())
}

/// Stream artifacts to S3
async fn stream_to_s3(
    artifact_dir: &PathBuf,
    hostname: &str,
    timestamp: &str,
    summary_path: &PathBuf,
    args: &Args
) -> Result<()> {
    // Create S3 client
    let s3_client = cloud::client::create_s3_client(
        args.region.as_deref(), 
        args.profile.as_deref()
    )?;
    
    // Create key for ZIP file
    let default_prefix = format!("triage-{}-{}", timestamp, hostname);
    let prefix = args.prefix.as_deref().unwrap_or_else(|| default_prefix.as_str());
    let key = format!("{}/{}-{}.zip", prefix, hostname, timestamp);
    
    let bucket = args.bucket.as_ref().ok_or_else(|| anyhow!("Bucket not provided"))?;
    
    // Stream artifacts to S3
    collectors::streaming::stream_artifacts_to_s3(
        artifact_dir,
        s3_client.clone(),
        bucket,
        &key,
        args.buffer_size
    ).await?;
    
    // Also upload the summary JSON separately for easy access
    let summary_key = format!("{}/collection_summary.json", prefix);
    
    collectors::streaming::stream_file_to_s3(
        summary_path,
        s3_client,
        bucket,
        &summary_key,
        args.buffer_size
    ).await?;
    
    Ok(())
}

/// Stream artifacts to SFTP
async fn stream_to_sftp(
    artifact_dir: &PathBuf,
    hostname: &str,
    timestamp: &str,
    summary_path: &PathBuf,
    args: &Args
) -> Result<()> {
    // Create SFTP config
    let sftp_config = cloud::sftp::SFTPConfig {
        host: args.sftp_host.as_ref().ok_or_else(|| anyhow!("SFTP host not provided"))?.clone(),
        port: args.sftp_port,
        username: args.sftp_user.as_ref().ok_or_else(|| anyhow!("SFTP user not provided"))?.clone(),
        private_key_path: args.sftp_key.as_ref().ok_or_else(|| anyhow!("SFTP key not provided"))?.clone(),
        remote_path: args.sftp_path.clone().unwrap_or_else(|| "/".to_string()),
        concurrent_connections: args.sftp_connections,
        buffer_size_mb: args.buffer_size,
        connection_timeout_sec: 30, // Default timeout
        max_retries: 3, // Default retries
    };
    
    // Create remote path for ZIP file
    let remote_path = format!("{}/{}-{}.zip", 
        sftp_config.remote_path.trim_end_matches('/'),
        hostname, timestamp);
    
    // Stream artifacts to SFTP
    collectors::streaming::stream_artifacts_to_sftp(
        artifact_dir,
        sftp_config.clone(),
        &remote_path,
        args.buffer_size
    ).await?;
    
    // Also upload the summary JSON separately for easy access
    let summary_remote_path = format!("{}/collection_summary.json", 
        sftp_config.remote_path.trim_end_matches('/'));
    
    collectors::streaming::stream_file_to_sftp(
        summary_path,
        sftp_config,
        &summary_remote_path,
        args.buffer_size
    ).await?;
    
    Ok(())
}