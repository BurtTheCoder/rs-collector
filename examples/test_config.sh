#!/bin/bash
set -e

echo "Building Rust Collector..."
cargo build

echo "Creating a default configuration file..."
./target/debug/rust_collector init-config test_config.yaml

echo "Running with the configuration file..."
./target/debug/rust_collector -c test_config.yaml -v

echo "Testing artifact type filtering..."
./target/debug/rust_collector -c test_config.yaml -t "Registry,Custom" -v

echo "Building a standalone binary with embedded configuration..."
./target/debug/rust_collector build -c test_config.yaml -n standalone_collector

echo "Testing the standalone binary..."
./standalone_collector -v

echo "Tests completed successfully!"