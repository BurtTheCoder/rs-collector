#!/bin/bash
set -e

# macOS build script for embedded configuration rust_collector
# Generated automatically

# Ensure config directory exists
mkdir -p config/

# Copy configuration file
cp "test_config.yaml" config/default_macos_config.yaml

# Build with embedded configuration
cargo build --release --features="embed_config" --target x86_64-apple-darwin

# Copy the binary with the specified name
cp target/x86_64-apple-darwin/release/rust_collector macos_collector

echo "Build completed successfully. Binary created at: macos_collector"
