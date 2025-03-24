use clap::{Parser, Subcommand, Args as ClapArgs, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(name = "rust-dfir-triage", about = "Cross-platform DFIR triage tool")]
pub struct Args {
    /// S3 bucket name for uploading artifacts
    #[clap(short, long)]
    pub bucket: Option<String>,

    /// S3 prefix for uploading artifacts (default: triage-{timestamp}-{hostname})
    #[clap(short, long)]
    pub prefix: Option<String>,

    /// AWS region for S3 uploads
    #[clap(long)]
    pub region: Option<String>,

    /// AWS profile to use for S3 uploads
    #[clap(long)]
    pub profile: Option<String>,

    /// Enable server-side encryption for S3 uploads
    #[clap(long)]
    pub encrypt: bool,

    /// SFTP server hostname for uploading artifacts
    #[clap(long)]
    pub sftp_host: Option<String>,

    /// SFTP server port (default: 22)
    #[clap(long, default_value = "22")]
    pub sftp_port: u16,

    /// SFTP username for authentication
    #[clap(long)]
    pub sftp_user: Option<String>,

    /// Path to private key file for SFTP authentication
    #[clap(long)]
    pub sftp_key: Option<PathBuf>,

    /// Remote path on SFTP server for uploading artifacts
    #[clap(long)]
    pub sftp_path: Option<String>,

    /// Number of concurrent connections for SFTP uploads
    #[clap(long, default_value = "4")]
    pub sftp_connections: usize,

    /// Local output path (default: %TEMP%/dfir-triage or /tmp/dfir-triage)
    #[clap(short, long)]
    pub output: Option<String>,

    /// Skip uploading to cloud storage (S3 or SFTP)
    #[clap(long)]
    pub skip_upload: bool,
    
    /// Verbose logging
    #[clap(short, long)]
    pub verbose: bool,
    
    /// Path to configuration YAML file
    #[clap(short = 'c', long)]
    pub config: Option<PathBuf>,
    
    /// Override default artifact types to collect (comma-separated)
    #[clap(short = 't', long)]
    pub artifact_types: Option<String>,
    
    /// Target operating system (windows, linux, macos)
    #[clap(long)]
    pub target_os: Option<TargetOS>,
    
    /// Continue even without elevated privileges
    #[clap(long)]
    pub force: bool,
    
    /// Stream artifacts directly to cloud storage (S3 or SFTP) without local storage
    #[clap(long, help = "Stream artifacts directly to cloud storage without storing locally")]
    pub stream: bool,

    /// Buffer size for streaming operations (in MB)
    #[clap(long, default_value = "8", help = "Buffer size for streaming operations (in MB)")]
    pub buffer_size: usize,
    
    /// Skip volatile data collection (running processes, network connections, etc.)
    #[clap(long, help = "Skip volatile data collection")]
    pub no_volatile_data: bool,
    
    /// Dump process memory for forensic analysis
    #[clap(long, help = "Dump process memory for forensic analysis")]
    pub dump_process_memory: bool,
    
    /// Specific processes to dump memory from (comma-separated names)
    #[clap(long, help = "Specific processes to dump memory from (comma-separated names)")]
    pub process: Option<String>,
    
    /// Specific process IDs to dump memory from (comma-separated PIDs)
    #[clap(long, help = "Specific process IDs to dump memory from (comma-separated PIDs)")]
    pub pid: Option<String>,
    
    /// Maximum total size for memory dumps (in MB)
    #[clap(long, default_value = "4096", help = "Maximum total size for memory dumps (in MB)")]
    pub max_memory_size: usize,
    
    /// Include system processes in memory dump
    #[clap(long, help = "Include system processes in memory dump")]
    pub include_system_processes: bool,
    
    /// Memory regions to dump (comma-separated: heap,stack,code,all)
    #[clap(long, default_value = "all", help = "Memory regions to dump (comma-separated: heap,stack,code,all)")]
    pub memory_regions: String,
    
    /// Search for a pattern in process memory (hex format, e.g. "4D5A90")
    #[clap(long, help = "Search for a pattern in process memory (hex format, e.g. \"4D5A90\")")]
    pub memory_search: Option<String>,
    
    /// Scan process memory with YARA rules (path to rule file or rule string)
    #[clap(long, help = "Scan process memory with YARA rules (path to rule file or rule string)")]
    pub memory_yara: Option<String>,
    
    /// Dump specific memory region (format: pid:address:size, e.g. "1234:0x400000:4096")
    #[clap(long, help = "Dump specific memory region (format: pid:address:size, e.g. \"1234:0x400000:4096\")")]
    pub dump_memory_region: Option<String>,
    
    /// Subcommands
    #[clap(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum TargetOS {
    Windows,
    Linux,
    MacOS,
}

impl std::fmt::Display for TargetOS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetOS::Windows => write!(f, "windows"),
            TargetOS::Linux => write!(f, "linux"),
            TargetOS::MacOS => write!(f, "macos"),
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Create a default configuration file
    InitConfig {
        /// Path to output configuration file
        #[clap(default_value = "config.yaml")]
        path: PathBuf,
        
        /// Target OS for the configuration (windows, linux, macos)
        #[clap(long)]
        target_os: Option<TargetOS>,
    },
    
    /// Build a standalone binary with embedded configuration
    #[clap(name = "build")]
    Build(BuildOpts),
}

#[derive(ClapArgs, Debug)]
pub struct BuildOpts {
    /// Path to configuration YAML file to embed
    #[clap(short, long)]
    pub config: PathBuf,
    
    /// Output path for the build script
    #[clap(short, long)]
    pub output: Option<PathBuf>,
    
    /// Custom name for the generated binary
    #[clap(short, long)]
    pub name: Option<String>,
    
    /// Target OS for the build (windows, linux, macos)
    #[clap(long)]
    pub target_os: Option<TargetOS>,
}
