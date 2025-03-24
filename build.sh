#!/bin/bash
set -e

# Build script for embedded configuration rust_collector
# Generated automatically

# Ensure config directory exists
mkdir -p config/

# Copy configuration file
cp "custom_config.yaml" config/default_config.yaml

# Build with embedded configuration
cargo build --release --features="embed_config"

# Copy the binary with the specified name
cp target/release/rust_collector malware_collector

echo "Build completed successfully. Binary created at: malware_collector"
