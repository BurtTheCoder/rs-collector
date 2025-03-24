# Rust Collector Usage Examples

## Basic Usage with Runtime Config

Run the collector with a YAML configuration file:

```bash
# Initialize with a default config file
./rust_collector init-config my_config.yaml

# Edit the config file to customize your artifacts
# ...

# Run with your custom config
./rust_collector -c my_config.yaml -o /path/to/output

# Collect only specific artifact types
./rust_collector -c my_config.yaml -t "Registry,EventLog" -o /path/to/output
```

## Creating a Standalone Binary

For incident response scenarios where you want a single executable with no dependencies:

```bash
# Build a standalone binary with embedded configuration
./rust_collector build -c examples/custom_config.yaml -n "ir_collector"

# This generates a build script and runs it, creating a binary called "ir_collector"
# The resulting binary has the configuration embedded and doesn't need any external files
```

## Sample Deployment Scenarios

### Basic Local Collection

```bash
# Collect all artifacts and store locally
./rust_collector -o /path/to/output

# Collect only MFT and Registry hives
./rust_collector -t "MFT,Registry" -o /path/to/output
```

### S3 Upload

```bash
# Collect artifacts and upload to S3
./rust_collector -b my-ir-bucket -p "incident-20250319" -o /path/to/output
```

### Air-Gapped Environment

For air-gapped environments or systems with limited connectivity:

1. Build a standalone binary with embedded configuration
   ```bash
   ./rust_collector build -c air_gap_config.yaml -n "standalone_collector"
   ```

2. Transfer the binary to the target system via USB or other means

3. Run on the target system with just:
   ```bash
   ./standalone_collector -o C:\collection
   ```

## Custom Artifact Collection

The `custom_config.yaml` example in this directory demonstrates how to configure additional artifacts like:

- USN Journal
- Prefetch files
- Amcache.hve
- SRUM database

You can easily extend this to collect any file-based artifacts needed for your investigation.