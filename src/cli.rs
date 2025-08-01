use clap::{Parser, Subcommand, Args as ClapArgs, ValueEnum};
use std::path::PathBuf;

/// Command-line arguments for the rust-dfir-triage tool.
/// 
/// This struct defines all available command-line options for the forensic
/// collection tool. Options are organized into logical groups for cloud uploads,
/// local storage, collection configuration, and subcommands.
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

/// Target operating system for cross-compilation.
/// 
/// Used with the `build` subcommand to specify the target platform
/// when building a custom collector binary.
#[derive(Clone, Debug, ValueEnum, PartialEq)]
pub enum TargetOS {
    /// Microsoft Windows
    Windows,
    /// Linux distributions
    Linux,
    /// Apple macOS
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

/// Available subcommands for the collector.
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

/// Options for the build subcommand.
/// 
/// Controls the creation of custom collector binaries with embedded
/// configurations for deployment scenarios.
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_basic_args_parsing() {
        let args = Args::parse_from(&[
            "rust-dfir-triage",
            "--bucket", "test-bucket",
            "--output", "/tmp/output",
            "--verbose",
        ]);

        assert_eq!(args.bucket, Some("test-bucket".to_string()));
        assert_eq!(args.output, Some("/tmp/output".to_string()));
        assert!(args.verbose);
        assert!(!args.skip_upload);
        assert!(!args.encrypt);
    }

    #[test]
    fn test_s3_args() {
        let args = Args::parse_from(&[
            "rust-dfir-triage",
            "--bucket", "my-bucket",
            "--prefix", "custom-prefix",
            "--region", "us-west-2",
            "--profile", "dev",
            "--encrypt",
        ]);

        assert_eq!(args.bucket, Some("my-bucket".to_string()));
        assert_eq!(args.prefix, Some("custom-prefix".to_string()));
        assert_eq!(args.region, Some("us-west-2".to_string()));
        assert_eq!(args.profile, Some("dev".to_string()));
        assert!(args.encrypt);
    }

    #[test]
    fn test_sftp_args() {
        let args = Args::parse_from(&[
            "rust-dfir-triage",
            "--sftp-host", "sftp.example.com",
            "--sftp-port", "2222",
            "--sftp-user", "testuser",
            "--sftp-key", "/home/user/.ssh/id_rsa",
            "--sftp-path", "/remote/path",
            "--sftp-connections", "8",
        ]);

        assert_eq!(args.sftp_host, Some("sftp.example.com".to_string()));
        assert_eq!(args.sftp_port, 2222);
        assert_eq!(args.sftp_user, Some("testuser".to_string()));
        assert_eq!(args.sftp_key, Some(PathBuf::from("/home/user/.ssh/id_rsa")));
        assert_eq!(args.sftp_path, Some("/remote/path".to_string()));
        assert_eq!(args.sftp_connections, 8);
    }

    #[test]
    fn test_default_values() {
        let args = Args::parse_from(&["rust-dfir-triage"]);

        assert_eq!(args.sftp_port, 22);
        assert_eq!(args.sftp_connections, 4);
        assert_eq!(args.buffer_size, 8);
        assert_eq!(args.max_memory_size, 4096);
        assert_eq!(args.memory_regions, "all");
        assert!(!args.verbose);
        assert!(!args.force);
        assert!(!args.stream);
        assert!(!args.no_volatile_data);
        assert!(!args.dump_process_memory);
        assert!(!args.include_system_processes);
    }

    #[test]
    fn test_memory_dump_args() {
        let args = Args::parse_from(&[
            "rust-dfir-triage",
            "--dump-process-memory",
            "--process", "chrome,firefox",
            "--pid", "1234,5678",
            "--max-memory-size", "8192",
            "--include-system-processes",
            "--memory-regions", "heap,stack",
        ]);

        assert!(args.dump_process_memory);
        assert_eq!(args.process, Some("chrome,firefox".to_string()));
        assert_eq!(args.pid, Some("1234,5678".to_string()));
        assert_eq!(args.max_memory_size, 8192);
        assert!(args.include_system_processes);
        assert_eq!(args.memory_regions, "heap,stack");
    }

    #[test]
    fn test_init_config_subcommand() {
        let args = Args::parse_from(&[
            "rust-dfir-triage",
            "init-config",
            "custom-config.yaml",
            "--target-os", "windows",
        ]);

        match args.command {
            Some(Commands::InitConfig { path, target_os }) => {
                assert_eq!(path, PathBuf::from("custom-config.yaml"));
                assert_eq!(target_os, Some(TargetOS::Windows));
            }
            _ => panic!("Expected InitConfig command"),
        }
    }

    #[test]
    fn test_build_subcommand() {
        let args = Args::parse_from(&[
            "rust-dfir-triage",
            "build",
            "--config", "config.yaml",
            "--output", "build.sh",
            "--name", "custom-collector",
            "--target-os", "linux",
        ]);

        match args.command {
            Some(Commands::Build(build_opts)) => {
                assert_eq!(build_opts.config, PathBuf::from("config.yaml"));
                assert_eq!(build_opts.output, Some(PathBuf::from("build.sh")));
                assert_eq!(build_opts.name, Some("custom-collector".to_string()));
                assert_eq!(build_opts.target_os, Some(TargetOS::Linux));
            }
            _ => panic!("Expected Build command"),
        }
    }

    #[test]
    fn test_target_os_display() {
        assert_eq!(format!("{}", TargetOS::Windows), "windows");
        assert_eq!(format!("{}", TargetOS::Linux), "linux");
        assert_eq!(format!("{}", TargetOS::MacOS), "macos");
    }

    #[test]
    fn test_conflicting_args() {
        // Test that parser accepts both S3 and SFTP args (validation happens at runtime)
        let args = Args::parse_from(&[
            "rust-dfir-triage",
            "--bucket", "test-bucket",
            "--sftp-host", "sftp.example.com",
        ]);

        assert_eq!(args.bucket, Some("test-bucket".to_string()));
        assert_eq!(args.sftp_host, Some("sftp.example.com".to_string()));
    }

    #[test]
    fn test_stream_mode() {
        let args = Args::parse_from(&[
            "rust-dfir-triage",
            "--stream",
            "--buffer-size", "16",
            "--bucket", "stream-bucket",
        ]);

        assert!(args.stream);
        assert_eq!(args.buffer_size, 16);
        assert_eq!(args.bucket, Some("stream-bucket".to_string()));
    }

    #[test]
    fn test_memory_search_and_yara() {
        let args = Args::parse_from(&[
            "rust-dfir-triage",
            "--memory-search", "4D5A9000",
            "--memory-yara", "/path/to/rules.yar",
            "--dump-memory-region", "1234:0x400000:4096",
        ]);

        assert_eq!(args.memory_search, Some("4D5A9000".to_string()));
        assert_eq!(args.memory_yara, Some("/path/to/rules.yar".to_string()));
        assert_eq!(args.dump_memory_region, Some("1234:0x400000:4096".to_string()));
    }

    #[test]
    fn test_artifact_types_and_config() {
        let args = Args::parse_from(&[
            "rust-dfir-triage",
            "--config", "/path/to/config.yaml",
            "--artifact-types", "logs,registry,memory",
            "--target-os", "windows",
        ]);

        assert_eq!(args.config, Some(PathBuf::from("/path/to/config.yaml")));
        assert_eq!(args.artifact_types, Some("logs,registry,memory".to_string()));
        assert_eq!(args.target_os, Some(TargetOS::Windows));
    }

    #[test]
    fn test_no_subcommand_with_all_flags() {
        let args = Args::parse_from(&[
            "rust-dfir-triage",
            "--verbose",
            "--force",
            "--skip-upload",
            "--no-volatile-data",
            "--output", "/custom/output",
        ]);

        assert!(args.verbose);
        assert!(args.force);
        assert!(args.skip_upload);
        assert!(args.no_volatile_data);
        assert_eq!(args.output, Some("/custom/output".to_string()));
        assert!(args.command.is_none());
    }
}
