#!/bin/bash

# Performance testing script for rs-collector
# This script runs various performance tests and generates a report

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="${PROJECT_DIR}/perf_results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
REPORT_FILE="${OUTPUT_DIR}/perf_report_${TIMESTAMP}.md"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Create output directory
mkdir -p "${OUTPUT_DIR}"

echo -e "${GREEN}Starting rs-collector Performance Tests${NC}"
echo "Results will be saved to: ${REPORT_FILE}"

# Start report
cat > "${REPORT_FILE}" << EOF
# rs-collector Performance Test Report
Date: $(date)
System: $(uname -a)
CPU: $(nproc) cores
Memory: $(free -h | grep Mem | awk '{print $2}')

## Test Results

EOF

# Function to run a benchmark and capture results
run_benchmark() {
    local bench_name=$1
    local description=$2
    
    echo -e "\n${YELLOW}Running benchmark: ${bench_name}${NC}"
    echo -e "\n### ${description}\n" >> "${REPORT_FILE}"
    
    if cargo bench --bench "${bench_name}" 2>&1 | tee -a "${OUTPUT_DIR}/${bench_name}_${TIMESTAMP}.log"; then
        echo -e "${GREEN}✓ ${bench_name} completed${NC}"
        echo "Status: ✓ Completed" >> "${REPORT_FILE}"
        
        # Extract key metrics from criterion output
        if [ -f "target/criterion/${bench_name}/report/index.html" ]; then
            echo "Detailed results: file://${PROJECT_DIR}/target/criterion/${bench_name}/report/index.html" >> "${REPORT_FILE}"
        fi
    else
        echo -e "${RED}✗ ${bench_name} failed${NC}"
        echo "Status: ✗ Failed" >> "${REPORT_FILE}"
    fi
}

# Function to test real-world scenarios
test_scenario() {
    local name=$1
    local description=$2
    local command=$3
    
    echo -e "\n${YELLOW}Testing scenario: ${name}${NC}"
    echo -e "\n### Scenario: ${description}\n" >> "${REPORT_FILE}"
    
    # Create test data
    local test_dir=$(mktemp -d)
    
    # Measure execution time
    local start_time=$(date +%s.%N)
    
    if eval "${command}" > "${OUTPUT_DIR}/${name}_${TIMESTAMP}.log" 2>&1; then
        local end_time=$(date +%s.%N)
        local duration=$(echo "$end_time - $start_time" | bc)
        
        echo -e "${GREEN}✓ ${name} completed in ${duration}s${NC}"
        echo "Duration: ${duration} seconds" >> "${REPORT_FILE}"
    else
        echo -e "${RED}✗ ${name} failed${NC}"
        echo "Status: ✗ Failed" >> "${REPORT_FILE}"
    fi
    
    # Cleanup
    rm -rf "${test_dir}"
}

# Build release version for performance testing
echo -e "\n${YELLOW}Building release version...${NC}"
cargo build --release

# Run unit benchmarks
echo -e "\n${GREEN}=== Running Micro Benchmarks ===${NC}"

run_benchmark "hash_bench" "SHA256 Hash Calculation Performance"
run_benchmark "compression_bench" "ZIP Compression Performance"
run_benchmark "collector_bench" "Artifact Collection Performance"
run_benchmark "path_validation_bench" "Path Validation Performance"
run_benchmark "bodyfile_bench" "Bodyfile Generation Performance"

# Run real-world performance tests
echo -e "\n${GREEN}=== Running Real-World Scenarios ===${NC}"

# Create test data directory
TEST_DATA_DIR=$(mktemp -d)
echo "Creating test data in ${TEST_DATA_DIR}"

# Generate test files
echo "Generating test files..."
for i in {1..100}; do
    dd if=/dev/urandom of="${TEST_DATA_DIR}/file_${i}.bin" bs=1M count=1 2>/dev/null
done

# Test 1: Basic collection
test_scenario "basic_collection" "Collect 100 1MB files" \
    "${PROJECT_DIR}/target/release/rust_collector -o ${OUTPUT_DIR}/test1 -c ${PROJECT_DIR}/config/default_config.yaml"

# Test 2: Collection with compression
test_scenario "compressed_collection" "Collect and compress 100 1MB files" \
    "${PROJECT_DIR}/target/release/rust_collector -o ${OUTPUT_DIR}/test2.zip --compress"

# Test 3: Collection with hashing
test_scenario "hashed_collection" "Collect with SHA256 hashing" \
    "${PROJECT_DIR}/target/release/rust_collector -o ${OUTPUT_DIR}/test3 --hash"

# Test 4: Parallel collection
test_scenario "parallel_collection" "Parallel collection with all cores" \
    "${PROJECT_DIR}/target/release/rust_collector -o ${OUTPUT_DIR}/test4 --threads 0"

# Cleanup test data
rm -rf "${TEST_DATA_DIR}"

# Generate summary
echo -e "\n## Summary\n" >> "${REPORT_FILE}"
echo "All benchmarks completed. Detailed Criterion reports available in:" >> "${REPORT_FILE}"
echo "- target/criterion/*/report/index.html" >> "${REPORT_FILE}"

echo -e "\n${GREEN}Performance testing complete!${NC}"
echo "Report saved to: ${REPORT_FILE}"
echo "Benchmark logs saved to: ${OUTPUT_DIR}"

# Open report if possible
if command -v xdg-open > /dev/null; then
    xdg-open "${REPORT_FILE}" 2>/dev/null || true
elif command -v open > /dev/null; then
    open "${REPORT_FILE}" 2>/dev/null || true
fi